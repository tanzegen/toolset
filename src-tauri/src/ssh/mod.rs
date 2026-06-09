//! SSH 终端工具的「有状态」接入层：会话管理 + russh 客户端 + Tauri 命令。
//!
//! 纯加密/配置逻辑在 toolset-core::{vault, sshconfig}（可单测）；本层负责文件 I/O、
//! 网络、会话生命周期——这是本仓首个有状态工具（区别于其余无状态纯函数工具）。

pub mod client;
pub mod commands;
pub mod local;
pub mod sftp;
pub mod store;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;
use zeroize::Zeroize;

use toolset_core::sshconfig::Store;

/// 后端 → 前端的流式帧（单会话一条 Tauri Channel）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SshFrame {
    /// 远端输出，base64 编码字节。
    Data { data: String },
    /// 会话状态变化：connected / closed / error。
    Status { state: String, msg: String },
}

/// 会话任务的控制指令（命令层 → 会话 tokio 任务）。
pub enum SessionCmd {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
    Close,
}

/// 一个活动会话的句柄。
pub struct SessionHandle {
    pub tx: UnboundedSender<SessionCmd>,
}

/// SSH 工具的全局状态（注入为 Tauri State）。
pub struct SshState {
    /// connections.json 路径（应用配置目录下）。
    pub path: PathBuf,
    /// 内存中的配置；变更后落盘。
    pub store: Mutex<Store>,
    /// 解锁后缓存的 vault 密钥；None = 锁定。
    pub vault_key: Mutex<Option<[u8; 32]>>,
    /// 活动会话表（并发安全）。用 Arc 以便克隆进会话任务做自清理（异常断开时移除自身）。
    pub sessions: Arc<DashMap<String, SessionHandle>>,
    /// 活动 SFTP 会话表。值用 Arc 包裹，便于在 await 前从 DashMap 取出、不跨 await 持锁。
    pub sftps: DashMap<String, Arc<sftp::SftpHolder>>,
}

impl SshState {
    pub fn new(path: PathBuf) -> Self {
        let mut store = store::load(&path);
        store.normalize_order(); // 未手动排序则按名称归一化，保证显示/导出顺序一致
        SshState {
            path,
            store: Mutex::new(store),
            vault_key: Mutex::new(None),
            sessions: Arc::new(DashMap::new()),
            sftps: DashMap::new(),
        }
    }

    /// 取缓存 vault 密钥的副本（[u8;32] 实现 Copy）；None 表示未解锁。
    pub fn key(&self) -> Option<[u8; 32]> {
        *self.vault_key.lock().unwrap()
    }

    /// 设置/清空缓存密钥；清空时擦除旧密钥。
    pub fn set_key(&self, key: Option<[u8; 32]>) {
        let mut slot = self.vault_key.lock().unwrap();
        if let Some(old) = slot.as_mut() {
            old.zeroize();
        }
        *slot = key;
    }
}
