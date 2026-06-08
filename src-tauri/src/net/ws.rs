//! WebSocket 会话：tokio-tungstenite(rustls)。可设握手请求头，长连接收发文本/二进制。
//! 会话任务多路复用「远端消息」与「本地控制指令」，断开时自清理会话表。

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use toolset_core::error::{AppError, AppResult};

use super::{NetState, WsCmd, WsFrame, WsHandle};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsKv {
    key: String,
    value: String,
}

#[tauri::command]
pub async fn ws_connect(
    state: State<'_, NetState>,
    url: String,
    headers: Vec<WsKv>,
    channel: Channel<WsFrame>,
) -> AppResult<String> {
    // 先生成带标准 WS 握手头的请求，再追加用户自定义头。
    let mut request = url
        .into_client_request()
        .map_err(|e| AppError::Invalid(format!("URL 非法：{e}")))?;
    for h in &headers {
        if h.key.is_empty() {
            continue;
        }
        let name = http::header::HeaderName::from_bytes(h.key.as_bytes())
            .map_err(|_| AppError::Invalid(format!("非法请求头名：{}", h.key)))?;
        let val = http::header::HeaderValue::from_str(&h.value)
            .map_err(|_| AppError::Invalid(format!("非法请求头值：{}", h.value)))?;
        request.headers_mut().insert(name, val);
    }

    let (stream, _resp) = connect_async(request)
        .await
        .map_err(|e| AppError::Invalid(format!("连接失败：{e}")))?;

    let session_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsCmd>();
    state.ws.insert(session_id.clone(), WsHandle { tx });

    let (mut write, mut read) = stream.split();
    let map = state.ws.clone();
    let sid = session_id.clone();

    tauri::async_runtime::spawn(async move {
        let _ = channel.send(WsFrame::Status {
            state: "open".into(),
            msg: String::new(),
        });
        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(t))) => {
                            let _ = channel.send(WsFrame::Message { text: true, data: t.as_str().to_string() });
                        }
                        Some(Ok(Message::Binary(b))) => {
                            let _ = channel.send(WsFrame::Message { text: false, data: hex(&b) });
                        }
                        Some(Ok(Message::Ping(p))) => {
                            let _ = write.send(Message::Pong(p)).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(_)) => {} // Pong / Frame 忽略
                        Some(Err(e)) => {
                            let _ = channel.send(WsFrame::Status { state: "error".into(), msg: e.to_string() });
                            break;
                        }
                    }
                }
                cmd = rx.recv() => {
                    match cmd {
                        Some(WsCmd::Text(t)) => {
                            if write.send(Message::Text(t.into())).await.is_err() { break; }
                        }
                        Some(WsCmd::Binary(b)) => {
                            if write.send(Message::Binary(b.into())).await.is_err() { break; }
                        }
                        Some(WsCmd::Ping(p)) => {
                            if write.send(Message::Ping(p.into())).await.is_err() { break; }
                        }
                        Some(WsCmd::Close) | None => {
                            let _ = write.send(Message::Close(None)).await;
                            break;
                        }
                    }
                }
            }
        }
        map.remove(&sid); // 自清理，避免会话泄露
        let _ = channel.send(WsFrame::Status {
            state: "closed".into(),
            msg: String::new(),
        });
    });

    Ok(session_id)
}

#[tauri::command]
pub fn ws_send(
    state: State<NetState>,
    session_id: String,
    text: bool,
    data: String,
) -> AppResult<()> {
    if let Some(h) = state.ws.get(&session_id) {
        let cmd = if text {
            WsCmd::Text(data)
        } else {
            WsCmd::Binary(decode_hex(&data)?)
        };
        h.tx.send(cmd)
            .map_err(|_| AppError::Invalid("会话已关闭".into()))?;
    }
    Ok(())
}

/// 发送协议级心跳 Ping 帧（服务端回 Pong，连接不会被判空闲断开）。
#[tauri::command]
pub fn ws_ping(state: State<NetState>, session_id: String) -> AppResult<()> {
    if let Some(h) = state.ws.get(&session_id) {
        h.tx.send(WsCmd::Ping(Vec::new()))
            .map_err(|_| AppError::Invalid("会话已关闭".into()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn ws_close(state: State<NetState>, session_id: String) {
    if let Some((_, h)) = state.ws.remove(&session_id) {
        let _ = h.tx.send(WsCmd::Close);
    }
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn decode_hex(s: &str) -> AppResult<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return Err(AppError::Invalid("hex 长度需为偶数".into()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| AppError::Invalid("非法 hex".into())))
        .collect()
}
