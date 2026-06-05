# SSH / SFTP 终端工具 · 实施计划（plan）

> 配套规格见 [`spec.md`](./spec.md)。本文定架构、模块、IPC 协议与分期任务。

## 1. 架构总览

三层，沿用并扩展现有约定：

```
前端 (Svelte 5 + xterm.js)
  └─ tools/SshTerminal.svelte ── 通过 ipc.ts 调命令 / 订阅 Channel
         │  invoke 命令（同步请求）          Channel<bytes>（流式输出）
         ▼                                         ▲
Tauri 接入层 (src-tauri/src/ssh/)  ← 新增「有状态」层
  └─ SessionManager（tauri::State，常驻）：管理活动会话、tokio 任务、russh 句柄
  └─ 配置文件 I/O + 应用数据目录解析
         │ 调用纯逻辑
         ▼
toolset-core（纯逻辑，可单测，无 GUI/异步）
  └─ vault：Argon2id KDF + 字段级 AES-GCM 加解密 + verifier
  └─ sshconfig：连接配置 serde 模型 + 导入导出合并逻辑
```

**关键判断**：会话管理（russh、tokio、长连接、Channel）**不进 core**——core 的契约是「无 GUI/异步依赖、纯函数可单测」。只有 vault 加密与配置模型这类纯逻辑进 core。这是本仓首个有状态工具，SessionManager 是新模式。

## 2. 模块划分

### 2.1 toolset-core（纯逻辑 + 单测）
- `core/src/vault.rs`
  - `derive_key(master: &str, kdf: &KdfParams) -> [u8;32]`（Argon2id）
  - `encrypt_field(key, conn_id, field, plaintext) -> SecretBlob` / `decrypt_field(...) -> String`（AES-256-GCM，AAD = `conn_id:field`）
  - `make_verifier(key)` / `check_verifier(key, &Verifier) -> bool`
  - 单测：往返加解密、错密码 verifier 失败、AAD 不匹配解密失败、KDF 确定性。
- `core/src/sshconfig.rs`
  - serde 结构（§spec 5 的模型）、`import_merge(current, incoming, ...)` 合并策略、克隆逻辑。
  - 单测：导入合并、克隆、明文/密文分离序列化。

> vault 复用 core 已有依赖 `aes-gcm`、`base64`；新增 `argon2`、`zeroize`。

### 2.2 src-tauri/src/ssh/（有状态接入层，新增）
- `mod.rs`：导出命令、注册 State。
- `manager.rs`：`SessionManager { sessions: DashMap<SessionId, SessionHandle> }`，作为 `tauri::State` 注入。`SessionHandle` 持有写端（`mpsc::Sender<Vec<u8>>`）、resize 句柄、关闭信号。
- `client.rs`：实现 `russh::client::Handler`（`check_server_key` 做 TOFU 指纹校验）；建连、认证（password / publickey）、`request_pty` + `request_shell`；spawn 读循环：channel data → `Channel.send(bytes)`。
- `commands.rs`：vault / 连接 CRUD / 会话命令（见 §4）。
- `store.rs`：应用数据目录（`app.path().app_config_dir()`）下读写 `connections.json`；调用 core 的 vault/sshconfig。
- P2 追加 `sftp.rs`（russh-sftp）；P3 追加 `transfer.rs`（trzsz/lrzsz）。

### 2.3 前端（src/lib/）
- `tools/SshTerminal.svelte`：整体布局——左连接树、右标签页 + 终端。
- `components/ssh/ConnTree.svelte`（分组树/增删改克隆）、`ConnForm.svelte`（编辑表单）、`TermTabs.svelte`、`TermView.svelte`（xterm 实例 + Channel 订阅 + 输入回传 + resize 观察）、`VaultUnlock.svelte`（主密码弹窗）、`ImportExport.svelte`。
- `lib/ssh.ts`：命令与 Channel 的强类型封装（与现有 `ipc.ts` 风格一致）。
- 依赖：`@xterm/xterm` + `@xterm/addon-fit`（轻量，无重型链）。

## 3. 依赖与构建

| 位置 | 新增依赖 | 用途 |
|------|---------|------|
| core | `argon2`、`zeroize` | KDF、内存擦除 |
| src-tauri | `russh`、`russh-sftp`(P2)、`tokio`(已随 Tauri)、`dashmap`、`bytes` | SSH 会话、并发表 |
| 前端 | `@xterm/xterm`、`@xterm/addon-fit` | 终端渲染 |
| capabilities | `dialog:default`（选私钥/导入导出/下载路径） | 文件对话框 |

- 构建约束：全部走 GNU（P0 已验证 russh 0.61 + ring + russh-sftp 在 `D:\minGw64` 编译链接通过）。
- 编译期开销：russh/ring 较重，按需在 `[profile.dev.package.*]` 视情况加 opt-level（参考现有 rsa 的处理）。

## 4. IPC 协议

**命令（invoke，请求/响应）**
- Vault：`vault_status() -> {hasMaster, unlocked}`、`vault_set_master(pwd)`、`vault_unlock(pwd) -> bool`、`vault_lock()`、`vault_reset()`（清密文留明文）。
- 连接：`conn_list() -> Connection[]`（明文部分，密文标记是否存在）、`conn_save(conn, secrets?)`、`conn_delete(id)`、`conn_clone(id) -> Connection`、`conn_export(path)`、`conn_import(path, fileMasterPwd?) -> 合并结果`。
- 会话：`ssh_connect(connId, channel) -> sessionId`（需 vault 已解锁；`channel` 为前端建的 `tauri::ipc::Channel`，后端经它推 stdout 字节与状态帧）、`ssh_write(sessionId, bytes)`、`ssh_resize(sessionId, cols, rows)`、`ssh_close(sessionId)`。

**Channel 帧**（后端 → 前端，单会话一条 Channel）：
```jsonc
{ "kind": "data",   "bytes": [...] }      // 远端输出（base64 或 byte array）
{ "kind": "status", "state": "connected|closed|error", "msg": "..." }
{ "kind": "hostkey","fingerprint": "...", "changed": true }   // TOFU 告警
```

## 5. 连接时序（P1）

1. 前端 `ssh_connect(connId, channel)`。
2. 后端：若需密码而 vault 未解锁 → 返回错误码 `VaultLocked`，前端弹 `VaultUnlock` 后重试。
3. 解 vault → 取该连接密文字段 → russh TCP 连接 → `check_server_key`（TOFU，比对 `knownHosts`，不一致发 `hostkey changed` 帧）→ 认证 → `request_pty`/`request_shell`。
4. SessionManager 存入句柄，spawn 读循环把远端字节经 Channel 推前端；前端 xterm 写出。
5. 前端按键 → `ssh_write`；面板尺寸变 → `ssh_resize`；关闭标签 → `ssh_close`。

## 6. 权限与安全落地

- 仅加 `dialog`（选文件/路径）。SSH socket 在 Rust 后端，**不需要** webview 网络权限，也不开放 fs 给前端。
- 私钥/密码读出后只在后端内存停留；解锁 key 用 `zeroize` 在 lock/退出时擦除。
- `connections.json` 落在 app 配置目录，不进仓库（加 `.gitignore` 提示）。

## 7. 分期任务清单

### P1（终端 + 连接管理）
- [ ] core：`vault.rs`（KDF + 字段加解密 + verifier）+ 单测
- [ ] core：`sshconfig.rs`（模型 + 导入合并 + 克隆）+ 单测
- [ ] src-tauri：`store.rs` 配置读写（app 目录）
- [ ] src-tauri：`manager.rs` + `client.rs`（russh 建连/认证/PTY/读循环 + TOFU）
- [ ] src-tauri：`commands.rs`（vault / conn / ssh 命令）+ 注册 State 与 handler
- [ ] capabilities：加 `dialog`
- [ ] 前端：`ssh.ts`、`SshTerminal.svelte`、ConnTree/ConnForm/TermTabs/TermView/VaultUnlock/ImportExport
- [ ] 前端：xterm 接入 + Channel 流式 + 输入回传 + resize + 快捷键 + 字号
- [ ] 注册到 `tools.ts`（新「远程」分类，icon=server/terminal）
- [ ] 验收：spec §8 P1 全流程手测

### P2（SFTP）
- [ ] `sftp.rs`（russh-sftp，复用连接）+ 命令（列目录/上传/下载/进度）
- [ ] 前端 SFTP 双栏面板 + 拖拽 + 进度
- [ ] 验收：spec §8 P2

### P3（trzsz）—— 已确定只做 trzsz，lrzsz 暂缓
> 调研结论：crates.io 无 `trzsz` Rust 库，只能走前端 `trzsz.js`；lrzsz 需后端 `zmodem2`
> 且较脆，而 SFTP(P2) 已覆盖传输。用户选「只做 trzsz」。故 P3 不写后端 `transfer.rs`。
- [x] 前端：`TrzszFilter`（npm `trzsz`）接入 `TermView.svelte`，处理 processServerOutput/processTerminalInput
- [x] 后端：`ssh_write` 改为 base64 承载，支持 trzsz 二进制块
- [ ] 运行期验证：trzsz 浏览器模式依赖 WebView2 的 File System Access API（showOpenFilePicker/
      showDirectoryPicker）在 Tauri 安全上下文可用——待真机确认；若被拦截再评估替代方案
- [ ] 验收：远端 `trz`/`tsz` 上传下载，spec §8 P3（lrzsz 部分本期不做）

## 8. 测试与验证

- **core 单测**（`cargo test -p toolset-core`）：vault 往返、错密码失败、AAD 防挪用、导入合并、克隆。
- **构建校验**：`npm run build` + `cargo build`（GNU）通过；新依赖链接成功。
- **手动验收**：按 spec §8 分期跑。SSH 端到端需真实/容器化 sshd，列为各期收尾手测。

## 9. 风险

- R-1 russh 0.61 API 仍在演进，PTY/认证细节以实际版本为准（P0 probe 已过编译，连通行为待 P1 实跑校验）。
- R-2 trzsz/lrzsz 协议实现是 P3 最大不确定项，已前置为调研任务。
- R-3 有状态会话 + tokio 是本仓新模式，需确保会话泄漏/关闭清理正确（SessionManager 负责生命周期与 zeroize）。
