//! HTTP / WebSocket 客户端的接入层。
//!
//! HTTP 是无状态单次请求（`http.rs`）；WebSocket 是长连接会话（`ws.rs`），
//! 复用 SSH 那套「会话表 + Tauri Channel 流式」的模式：用 `Arc<DashMap>` 以便
//! 克隆进会话任务做自清理（断开时移除自身），避免会话泄露。

pub mod http;
pub mod ws;

use std::sync::Arc;

use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

/// 后端 → 前端的 WS 帧（每个 WS 会话一条 Channel）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WsFrame {
    /// 收到的消息。text=true 为文本（data 即文本）；否则二进制（data 为 hex）。
    Message { text: bool, data: String },
    /// 状态变化：open / closed / error。
    Status { state: String, msg: String },
}

/// 控制指令（命令层 → WS 会话任务）。
pub enum WsCmd {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>), // 协议级心跳 Ping 帧
    Close,
}

pub struct WsHandle {
    pub tx: UnboundedSender<WsCmd>,
}

/// 网络工具全局状态：WS 会话表（注入为 Tauri State）。
pub struct NetState {
    pub ws: Arc<DashMap<String, WsHandle>>,
}

impl Default for NetState {
    fn default() -> Self {
        NetState {
            ws: Arc::new(DashMap::new()),
        }
    }
}
