//! russh 客户端：建连、认证、PTY/shell，以及把会话 I/O 桥接到 Tauri Channel。
//!
//! `connect` 负责建连 + 认证 + 主机指纹 TOFU，被终端与 SFTP 共用；
//! `start_shell` 在已认证连接上开 PTY/shell 并 spawn 会话任务。
//! TOFU：首次接受并记录；与已记录不一致时**在认证前拒绝**（绝不外发凭据）。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use russh::client::{self, Handle};
use russh::keys::{decode_secret_key, HashAlg, PrivateKeyWithHashAlg};
use dashmap::DashMap;
use russh::{ChannelMsg, Disconnect};
use tauri::ipc::Channel;
use tokio::sync::mpsc;

use toolset_core::error::{AppError, AppResult};

use super::{SessionCmd, SessionHandle, SshFrame};

/// 发起一次连接所需的（已解密的）参数。
pub struct ConnParams {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: String, // "password" | "key"
    pub password: Option<String>,
    pub key_pem: Option<String>,
    pub key_pass: Option<String>,
    /// 已记录的主机指纹（None = 首次连接）。
    pub known_fp: Option<String>,
}

/// TOFU 结果：在 check_server_key 内写入，连接返回后读取。
#[derive(Default)]
struct HostKeyReport {
    fingerprint: Option<String>,
    mismatch: bool,
}

pub(crate) struct Handler {
    known: Option<String>,
    report: Arc<Mutex<HostKeyReport>>,
}

impl client::Handler for Handler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let fp = key.fingerprint(HashAlg::Sha256).to_string();
        let mut r = self.report.lock().unwrap();
        r.fingerprint = Some(fp.clone());
        match &self.known {
            None => Ok(true),                // 首次：接受，成功后由命令层落盘
            Some(k) if *k == fp => Ok(true), // 指纹一致
            Some(_) => {
                r.mismatch = true;
                Ok(false) // 不一致：认证前拒绝
            }
        }
    }
}

/// 建连 + 认证 + TOFU。返回已认证句柄、本次主机指纹、是否首次见到该主机。
pub async fn connect(p: ConnParams) -> AppResult<(Handle<Handler>, Option<String>, bool)> {
    let report = Arc::new(Mutex::new(HostKeyReport::default()));
    let first_seen = p.known_fp.is_none();
    let handler = Handler {
        known: p.known_fp.clone(),
        report: report.clone(),
    };

    let config = Arc::new(client::Config {
        // 不因空闲而主动断开——只要服务器不断、连接没真正失活，就保持连接。
        inactivity_timeout: None,
        // 周期性 keepalive 维持空闲连接（穿透 NAT/防火墙），并据此发现真正失活的对端
        // （连续 keepalive_max 次无响应才断开，等同 OpenSSH 的 ServerAliveInterval/CountMax）。
        keepalive_interval: Some(Duration::from_secs(30)),
        keepalive_max: 3,
        ..Default::default()
    });

    let mut handle = match client::connect(config, (p.host.as_str(), p.port), handler).await {
        Ok(h) => h,
        Err(e) => {
            if report.lock().unwrap().mismatch {
                return Err(AppError::Invalid(
                    "主机密钥与已记录的不一致，连接已中止（疑似中间人或主机变更）".into(),
                ));
            }
            return Err(AppError::Invalid(format!("连接失败：{e}")));
        }
    };

    let auth_ok = match p.auth.as_str() {
        "key" => {
            let pem = p.key_pem.ok_or_else(|| AppError::Invalid("缺少私钥".into()))?;
            let key = decode_secret_key(&pem, p.key_pass.as_deref())
                .map_err(|e| AppError::Invalid(format!("私钥解析失败：{e}")))?;
            let hash = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|e| AppError::Invalid(format!("算法协商失败：{e}")))?
                .flatten();
            handle
                .authenticate_publickey(
                    p.username.clone(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                )
                .await
        }
        _ => {
            let pw = p.password.ok_or_else(|| AppError::Invalid("缺少密码".into()))?;
            handle.authenticate_password(p.username.clone(), pw).await
        }
    }
    .map_err(|e| AppError::Invalid(format!("认证出错：{e}")))?;

    if !auth_ok.success() {
        return Err(AppError::Invalid("认证失败：用户名、密码或私钥不正确".into()));
    }

    let fp = report.lock().unwrap().fingerprint.clone();
    Ok((handle, fp, first_seen))
}

/// 在已认证连接上开 PTY + shell，并 spawn 会话任务（多路复用远端输出与本地控制）。
/// 任务会把自身登记进 `sessions`，并在退出（正常关闭或异常断开）时移除，避免连接/句柄泄露。
pub async fn start_shell(
    handle: Handle<Handler>,
    frame: Channel<SshFrame>,
    sessions: Arc<DashMap<String, SessionHandle>>,
    session_id: String,
) -> AppResult<()> {
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| AppError::Invalid(format!("打开通道失败：{e}")))?;
    channel
        .request_pty(false, "xterm-256color", 80, 24, 0, 0, &[])
        .await
        .map_err(|e| AppError::Invalid(format!("请求 PTY 失败：{e}")))?;
    channel
        .request_shell(true)
        .await
        .map_err(|e| AppError::Invalid(format!("启动 shell 失败：{e}")))?;

    let (tx, mut rx) = mpsc::unbounded_channel::<SessionCmd>();
    sessions.insert(session_id.clone(), SessionHandle { tx });

    tauri::async_runtime::spawn(async move {
        let _ = frame.send(SshFrame::Status {
            state: "connected".into(),
            msg: String::new(),
        });
        // 区分干净退出（shell exit / 用户关闭）与异常断开，供前端决定是否自动重连。
        let mut clean = false;
        loop {
            tokio::select! {
                msg = channel.wait() => {
                    match msg {
                        Some(ChannelMsg::Data { ref data }) => {
                            let _ = frame.send(SshFrame::Data { data: B64.encode(data) });
                        }
                        Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                            let _ = frame.send(SshFrame::Data { data: B64.encode(data) });
                        }
                        Some(ChannelMsg::ExitStatus { .. }) => clean = true,
                        Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                        _ => {}
                    }
                }
                cmd = rx.recv() => {
                    match cmd {
                        Some(SessionCmd::Data(bytes)) => {
                            let _ = channel.data(&bytes[..]).await;
                        }
                        Some(SessionCmd::Resize { cols, rows }) => {
                            let _ = channel.window_change(cols, rows, 0, 0).await;
                        }
                        Some(SessionCmd::Close) | None => {
                            clean = true; // 用户主动关闭
                            let _ = channel.eof().await;
                            break;
                        }
                    }
                }
            }
        }
        // 自清理：无论正常关闭还是异常断开，都移除会话条目并断开底层连接。
        sessions.remove(&session_id);
        let _ = handle.disconnect(Disconnect::ByApplication, "", "").await;
        let _ = frame.send(SshFrame::Status {
            state: if clean { "exited" } else { "closed" }.into(),
            msg: String::new(),
        });
    });

    Ok(())
}
