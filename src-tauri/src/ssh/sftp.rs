//! SFTP 文件传输：在独立的 SSH 连接上开 sftp 子系统（与终端标签解耦）。
//! 目录浏览 + 上传/下载（分块，带进度帧）+ 基本增删改。

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use russh::client::Handle;
use russh_sftp::client::SftpSession;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use toolset_core::error::{AppError, AppResult};

use super::client::Handler;
use super::SshState;

/// 一个活动 SFTP 会话：持有 sftp 会话，并保留 SSH 句柄以维持底层连接。
pub struct SftpHolder {
    sftp: SftpSession,
    _handle: Handle<Handler>,
}

/// 传输进度帧（后端 → 前端，单次传输一条 Channel）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TransferFrame {
    Progress { transferred: u64, total: u64 },
}

/// 远端目录项。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpEntry {
    name: String,
    is_dir: bool,
    is_link: bool,
    size: u64,
    mtime: u64, // unix 秒
    permissions: u32,
}

const CHUNK: usize = 64 * 1024;

fn holder(state: &SshState, id: &str) -> AppResult<Arc<SftpHolder>> {
    state
        .sftps
        .get(id)
        .map(|r| r.value().clone())
        .ok_or_else(|| AppError::Invalid("SFTP 会话不存在或已关闭".into()))
}

/// 打开一个 SFTP 会话（独立连接 + sftp 子系统）。
#[tauri::command]
pub async fn sftp_open(state: State<'_, SshState>, conn_id: String) -> AppResult<String> {
    let (params, host, port) = super::commands::build_params(&state, &conn_id)?;
    let (handle, fp, first_seen) = super::client::connect(params).await?;
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| AppError::Invalid(format!("打开通道失败：{e}")))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| AppError::Invalid(format!("请求 sftp 子系统失败：{e}")))?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| AppError::Invalid(format!("SFTP 初始化失败：{e}")))?;

    if first_seen {
        super::commands::record_fp(&state, &host, port, fp);
    }
    let id = Uuid::new_v4().to_string();
    state
        .sftps
        .insert(id.clone(), Arc::new(SftpHolder { sftp, _handle: handle }));
    Ok(id)
}

/// 远端家目录（规范化的当前路径）。
#[tauri::command]
pub async fn sftp_home(state: State<'_, SshState>, sftp_id: String) -> AppResult<String> {
    let h = holder(&state, &sftp_id)?;
    h.sftp
        .canonicalize(".")
        .await
        .map_err(|e| AppError::Invalid(format!("解析路径失败：{e}")))
}

#[tauri::command]
pub async fn sftp_list(
    state: State<'_, SshState>,
    sftp_id: String,
    path: String,
) -> AppResult<Vec<SftpEntry>> {
    let h = holder(&state, &sftp_id)?;
    let rd = h
        .sftp
        .read_dir(&path)
        .await
        .map_err(|e| AppError::Invalid(format!("列目录失败：{e}")))?;
    let mut out = Vec::new();
    for e in rd {
        let ft = e.file_type();
        let m = e.metadata();
        out.push(SftpEntry {
            name: e.file_name(),
            is_dir: ft.is_dir(),
            is_link: ft.is_symlink(),
            size: m.size.unwrap_or(0),
            mtime: m.mtime.unwrap_or(0) as u64,
            permissions: m.permissions.unwrap_or(0),
        });
    }
    // 目录在前、再按名称
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(out)
}

#[tauri::command]
pub async fn sftp_mkdir(state: State<'_, SshState>, sftp_id: String, path: String) -> AppResult<()> {
    let h = holder(&state, &sftp_id)?;
    h.sftp
        .create_dir(path)
        .await
        .map_err(|e| AppError::Invalid(format!("新建目录失败：{e}")))
}

#[tauri::command]
pub async fn sftp_remove(state: State<'_, SshState>, sftp_id: String, path: String) -> AppResult<()> {
    let h = holder(&state, &sftp_id)?;
    h.sftp
        .remove_file(path)
        .await
        .map_err(|e| AppError::Invalid(format!("删除文件失败：{e}")))
}

#[tauri::command]
pub async fn sftp_rmdir(state: State<'_, SshState>, sftp_id: String, path: String) -> AppResult<()> {
    let h = holder(&state, &sftp_id)?;
    h.sftp
        .remove_dir(path)
        .await
        .map_err(|e| AppError::Invalid(format!("删除目录失败：{e}")))
}

#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, SshState>,
    sftp_id: String,
    from: String,
    to: String,
) -> AppResult<()> {
    let h = holder(&state, &sftp_id)?;
    h.sftp
        .rename(from, to)
        .await
        .map_err(|e| AppError::Invalid(format!("重命名失败：{e}")))
}

/// 下载：远端 → 本地，分块并发进度帧。
#[tauri::command]
pub async fn sftp_download(
    state: State<'_, SshState>,
    sftp_id: String,
    remote: String,
    local: String,
    channel: Channel<TransferFrame>,
) -> AppResult<()> {
    let h = holder(&state, &sftp_id)?;
    let total = h
        .sftp
        .metadata(remote.clone())
        .await
        .ok()
        .and_then(|m| m.size)
        .unwrap_or(0);
    let mut rf = h
        .sftp
        .open(remote)
        .await
        .map_err(|e| AppError::Invalid(format!("打开远端文件失败：{e}")))?;
    let mut lf = tokio::fs::File::create(&local)
        .await
        .map_err(|e| AppError::Invalid(format!("创建本地文件失败：{e}")))?;

    let mut buf = vec![0u8; CHUNK];
    let mut done: u64 = 0;
    loop {
        let n = rf
            .read(&mut buf)
            .await
            .map_err(|e| AppError::Invalid(format!("读取失败：{e}")))?;
        if n == 0 {
            break;
        }
        lf.write_all(&buf[..n])
            .await
            .map_err(|e| AppError::Invalid(format!("写入失败：{e}")))?;
        done += n as u64;
        let _ = channel.send(TransferFrame::Progress { transferred: done, total });
    }
    lf.flush().await.ok();
    Ok(())
}

/// 上传：本地 → 远端，分块并发进度帧。
#[tauri::command]
pub async fn sftp_upload(
    state: State<'_, SshState>,
    sftp_id: String,
    local: String,
    remote: String,
    channel: Channel<TransferFrame>,
) -> AppResult<()> {
    let h = holder(&state, &sftp_id)?;
    let mut lf = tokio::fs::File::open(&local)
        .await
        .map_err(|e| AppError::Invalid(format!("打开本地文件失败：{e}")))?;
    let total = lf.metadata().await.map(|m| m.len()).unwrap_or(0);
    let mut rf = h
        .sftp
        .create(remote)
        .await
        .map_err(|e| AppError::Invalid(format!("创建远端文件失败：{e}")))?;

    let mut buf = vec![0u8; CHUNK];
    let mut done: u64 = 0;
    loop {
        let n = lf
            .read(&mut buf)
            .await
            .map_err(|e| AppError::Invalid(format!("读取失败：{e}")))?;
        if n == 0 {
            break;
        }
        rf.write_all(&buf[..n])
            .await
            .map_err(|e| AppError::Invalid(format!("上传失败：{e}")))?;
        done += n as u64;
        let _ = channel.send(TransferFrame::Progress { transferred: done, total });
    }
    rf.flush().await.ok();
    rf.shutdown().await.ok();
    Ok(())
}

/// 拖拽上传：直接写入字节（前端读取拖入的 File 内容并 base64 传来）。
/// 适合从资源管理器拖文件进面板；大文件会整块进内存，超大文件请用「上传」按钮（走路径流式）。
#[tauri::command]
pub async fn sftp_upload_bytes(
    state: State<'_, SshState>,
    sftp_id: String,
    remote: String,
    data: String,
) -> AppResult<()> {
    let h = holder(&state, &sftp_id)?;
    let bytes = B64
        .decode(data.as_bytes())
        .map_err(|_| AppError::Invalid("上传数据非法".into()))?;
    let mut rf = h
        .sftp
        .create(remote)
        .await
        .map_err(|e| AppError::Invalid(format!("创建远端文件失败：{e}")))?;
    rf.write_all(&bytes)
        .await
        .map_err(|e| AppError::Invalid(format!("上传失败：{e}")))?;
    rf.flush().await.ok();
    rf.shutdown().await.ok();
    Ok(())
}

#[tauri::command]
pub fn sftp_close(state: State<SshState>, sftp_id: String) {
    // 丢弃 Arc<SftpHolder> → 关闭 sftp 会话与底层 SSH 连接
    state.sftps.remove(&sftp_id);
}
