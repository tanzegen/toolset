# HTTP / WebSocket 客户端 — 规格

## 目标

桌面端 API 调试工具：**绕过浏览器 CORS、可设任意请求方法与请求头、查看原始响应**——这正是浏览器 `fetch` 做不到、必须走原生（Rust）的地方。两个侧栏工具：**HTTP 客户端**、**WebSocket 客户端**。所有数据落 localStorage，不出本机。

## 范围（迷你够用版）

### HTTP 客户端
- 方法：GET / POST / PUT / PATCH / DELETE / HEAD / OPTIONS
- URL + query 参数（键值表，可逐项启停）
- 请求头（键值表，可逐项启停）
- Body：`none` / `json` / `raw`(文本) / `form`(x-www-form-urlencoded 键值表)
- 认证：None / Bearer / Basic / 自定义头（前端解析为请求头后发送）
- 选项：跟随重定向、超时、**跳过 TLS 校验**（内网自签用，危险标注）
- 响应：状态码 + 耗时 + 大小、响应头、Body（JSON 美化 / 原始 / 可搜索）
- **环境变量 `{{var}}`**：一套全局变量，作用于 url / params / headers / body
- 历史记录 + 保存请求（按集合分组）
- **curl 导入 / 导出**

### WebSocket 客户端
- 连接 ws / wss、自定义握手请求头、子协议
- 收发文本 / 二进制(hex)、消息时间线（收发分色 + 时间戳）
- ping/pong、清空、自动重连（可选）、保存常用 URL

## 非目标（本期不做，后续可加）
- Postman 级：多环境切换、预请求脚本、测试断言、cookie jar、GraphQL、multipart 文件上传

## 架构（契合现有分层 + GNU 工具链）
- **HTTP 执行**：`reqwest`（`default-features=false` + `rustls-tls`/`gzip`/`brotli`/`deflate`，避开 openssl/native-tls）。单次请求 = async 命令 `http_send`，**无常驻状态**。
- **WebSocket**：`tokio-tungstenite`（rustls），**有状态会话**，复用 SSH 会话模式：`WsState`(DashMap) + Tauri Channel 流式推消息/状态；命令 `ws_connect` / `ws_send` / `ws_close`。
- **纯函数进 `toolset-core::httpclient`**（变量替换、URL+query 拼接、curl 解析/生成）+ `#[cfg(test)]` 单测。
- **前端**两个工具组件，复用 `persist` 持久化、标签、可调宽度、快捷键。

## 安全
- 认证密钥（token / Basic 口令）按用户选择存 localStorage 明文（本机自用，与其它工具一致）。
- “跳过 TLS 校验”须显式开关并标红，默认关闭。

## 验证
- core：`substitute_vars` / `build_url` / `parse_curl` / `to_curl` 单测（含变量替换、参数拼接、curl 往返）。
- 运行：对公开 API（如 httpbin）GET/POST、看响应头与耗时；WS 连 `wss://echo.websocket.events` 收发。
- 构建：`cargo build`、`npm run check`、`npm run build` 全绿。
