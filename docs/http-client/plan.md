# HTTP / WebSocket 客户端 — 实施计划

## 分层落点

| 层 | 内容 |
|---|---|
| `toolset-core/src/httpclient.rs` | 纯函数：`substitute_vars`、`build_url`、`parse_curl`、`to_curl` + 单测 |
| `src-tauri/src/net/mod.rs` | `NetState`（WS 会话表 DashMap）+ 帧/指令类型 |
| `src-tauri/src/net/http.rs` | `http_send` 命令（reqwest，rustls） |
| `src-tauri/src/net/ws.rs` | `ws_connect` / `ws_send` / `ws_close`（tokio-tungstenite） |
| `src-tauri/src/lib.rs` | 注册命令 + manage(NetState) |
| `src/lib/net.ts` | 前端命令封装 + 类型 |
| `src/lib/tools/HttpClient.svelte` | HTTP 客户端视图 |
| `src/lib/tools/WsClient.svelte` | WebSocket 客户端视图 |
| `src/lib/tools.ts` | 注册两个工具 |

## 数据模型（前端，持久化 localStorage）

```ts
KV = { key: string; value: string; on: boolean }
HttpRequest = {
  id, name, method, url,
  params: KV[], headers: KV[],
  bodyType: "none"|"json"|"raw"|"form", body: string, form: KV[],
  auth: { type:"none"|"bearer"|"basic"|"header", token?, user?, pass?, headerKey?, headerVal? },
  followRedirects: boolean, timeoutMs: number, skipTlsVerify: boolean,
}
VarSet = { vars: KV[] }            // 一套全局环境变量
HttpResponse = { status, statusText, headers: KV[], body, size, timeMs, finalUrl }
```

## 阶段

### P1 — HTTP 核心闭环
1. core `httpclient.rs`：`substitute_vars` + `build_url` + 单测 → 验证：`cargo test`
2. 后端 `http_send`（reqwest/rustls：方法/头/体/超时/重定向/跳过校验，变量替换+URL 拼接走 core）→ 验证：`cargo build`
3. 前端 `HttpClient.svelte`：请求构建（方法/URL/params/headers/body/auth）+ 发送 + 响应展示（状态/耗时/大小/头/body 美化）+ 环境变量编辑 + 历史 → 验证：发请求看响应

### P2 — HTTP 增强
4. 保存请求（集合分组）+ 历史持久化
5. curl 导入/导出（core `parse_curl`/`to_curl` + 单测）
6. 跳过 TLS 校验开关、超时/重定向选项 UI

### P3 — WebSocket
7. 后端 `net/ws.rs`：`ws_connect`(自定义头) / `ws_send`(text|binary) / `ws_close`，Channel 推消息+状态
8. 前端 `WsClient.svelte`：连接/收发/时间线/重连/保存 URL → 验证：连 echo 服务收发

## 依赖（Cargo.toml）
- `reqwest = { version="0.12", default-features=false, features=["rustls-tls","gzip","brotli","deflate"] }`
- `tokio-tungstenite = { version="0.24", default-features=false, features=["connect","rustls-tls-webpki-roots"] }`
- `futures-util = "0.3"`、`http = "1"`

## 验证基线
- `cargo test`（core）、`cargo build`、`npm run check`、`npm run build` 全绿。
- httpbin GET/POST、WS echo 收发手测。
