# 工具集 · DevTools（Windows 桌面端）

面向开发者的本地工具集，第一期支持 Windows 桌面端。基于 **Tauri v2（Rust + 系统 WebView2）+ Svelte 5 + Tailwind v4**，
追求低内存、界面美观、结构清晰。所有计算在 Rust 端完成，前端只做展示，数据不出本机。

## 内置工具

| 工具 | 能力 |
|------|------|
| 时间戳转换 | 自动识别 秒/毫秒/微秒/纳秒，按 IANA 时区双向换算，给出 UTC / 本地 / ISO8601 / RFC2822 / 相对时间 |
| Cron 表达式 | 解析 5/6 字段 cron，字段拆解 + 预测未来 N 次运行（按时区） |
| Hex / Float | IEEE 754 位模式 ↔ 浮点（32/64 位），含符号·阶码·尾数拆解与整数解释 |
| 进制转换 | 二/八/十/十六进制互转，按位宽（8/16/32/64）给出有符号/无符号与位视图 |
| JSON 工具 | 格式化（2/4 空格）、压缩、校验（错误带行列） |
| Base64 / URL | Base64 标准与 URL-safe、URL 百分号编解码，全 UTF-8 |
| 正则测试器 | 实时匹配、捕获组、替换预览（RE2 语义，flags i/m/s/x） |
| 命名风格转换 | camel / Pascal / snake / SCREAMING / kebab / Train / Title 互转 |
| 文本 Diff | 行级 LCS 比较，高亮新增/删除（自实现，无第三方依赖） |
| JSON 转结构 | 递归推断类型，生成 Go struct / TS interface / Rust struct |
| 子网计算器 | CIDR / IP+掩码解析，网络·广播·可用范围·掩码，IPv4 完整 + IPv6 基本 |
| 内网 IP | 枚举本机网卡地址与主用出口 IP（纯本地，不联网） |
| 公网 IP | 检测当前公网出口 IP（回显服务，端点可配置，需联网） |
| 域名解析 | DoH 查询 A/AAAA/CNAME/MX/TXT/NS（端点可配置，需联网） |
| 哈希 / UUID | 文本 MD5 / SHA1 / SHA256 / SHA512，UUID v4 批量生成 |

## 目录职责

```
toolset/
├─ src/                       前端（SvelteKit 静态 SPA，仅客户端渲染）
│  ├─ routes/+layout.svelte   全局样式与主题挂载
│  ├─ routes/+page.svelte     应用外壳：侧栏 + 主面板
│  ├─ app.css                 Tailwind 入口与基础样式
│  └─ lib/
│     ├─ ipc.ts               后端命令的强类型封装（invoke）
│     ├─ state.svelte.ts      全局状态（当前工具 / 主题）
│     ├─ tools.ts             工具注册表（新增工具只改这一处）
│     ├─ ui.ts                共享 Tailwind 类名片段
│     ├─ components/          Sidebar / ToolPanel / SegmentedControl / ResultRow / CopyButton / Icon
│     └─ tools/               6 个工具视图组件
├─ src-tauri/                 后端（Rust）
│  ├─ core/                   toolset-core：纯逻辑 + 单测（无 GUI/Tauri 依赖）
│  │  └─ src/                 error/util/timestamp/cron/numeric/encoding/json/
│  │                          jsonstruct/naming/textdiff/regextool/subnet/hashing
│  ├─ src/lib.rs              #[tauri::command] 薄包装 + 应用装配（含 WebView2 省内存参数）
│  ├─ src/main.rs             入口
│  ├─ capabilities/           Tauri v2 权限（core + clipboard 写入）
│  └─ tauri.conf.json         窗口与打包配置
├─ .cargo/config.toml.example 本地构建配置模板（真实 config.toml 本机自建、不入库）
└─ build/                     前端构建产物（generate_context! 嵌入）
```

## 环境前置（一次性）

- **Node** ≥ 20、**Rust（GNU 工具链）**：`stable-x86_64-pc-windows-gnu`。
- **MinGW-w64（64 位）**：本仓库复用 `D:\minGw64`（GCC 8.1，`x86_64-w64-mingw32`）。
- **WebView2 Runtime**：Win11 自带。
- **本地构建配置**：把 `.cargo/config.toml.example` 复制为 `.cargo/config.toml`（已 gitignore、不入库），按本机取消注释——
  - GNU 链接器锁到 `D:/minGw64/bin/gcc.exe`，并保证 `D:\minGw64\bin` 在 PATH 中位于旧的 32 位 `D:\minGw\bin` **之前**（供 `windres`/`dlltool` 解析）。
  - crates.io 国内不稳定时，启用 rsproxy.cn 稀疏镜像（校验和仍由 `Cargo.lock` 把关）。

安装 Rust（GNU）：
```powershell
winget install --id Rustlang.Rustup -e
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

## 入口命令

> 每条命令前，先让工具链就位：
> ```powershell
> $env:PATH = "$env:USERPROFILE\.cargo\bin;D:\minGw64\bin;$env:PATH"
> ```

| 目的 | 命令 |
|------|------|
| 安装前端依赖 | `npm install` |
| 开发运行（热重载 + 窗口） | `npm run tauri dev` |
| 仅构建前端 | `npm run build` → 产物到 `build/` |
| 前端类型检查 | `npm run check` |
| 打包桌面应用 | `npm run tauri build` |
| 后端单测 | `cargo test --manifest-path src-tauri/core/Cargo.toml` |

## 验证方式

- **后端单测**：`cargo test` 覆盖关键向量（π 的 IEEE754 位模式 `0x40490FDB`/`0x400921FB54442D18`、
  时间戳四单位识别与时区换算、进制有/无符号、Base64/URL 往返、`SHA256("abc")`、JSON 格式化/校验）。
- **运行验证**：`npm run tauri dev` 启动后逐工具手测；任务管理器确认空闲内存远低于“几百 MB”。
- **构建验证**：`npm run tauri build` 产物可独立运行，记录二进制体积与内存实测值。

## 设计要点

- 计算全在 Rust：精度（64 位整数、IEEE754 位重解释）、性能与可单测性。纯逻辑在 `toolset-core`，零 GUI 依赖、可独立单测。
- 前端极薄：Svelte 5 + Tailwind，零重型依赖，客户端包 gzip 约几十 KB。
- 低内存：复用系统 WebView2；release 开启 `opt-level="z" / lto / strip / panic=abort`。

## 实测指标（release，Windows 11）

| 指标 | 数值 |
|------|------|
| 可执行体积 | **4.19 MB** |
| 私有工作集（独占 RAM，公允值） | **~120 MB** |
| 总工作集（含共享 Chromium 页，偏高） | ~405 MB |
| 进程数 | 7（WebView2 多进程） |

> **关于内存**：WebView2 基于 Chromium 多进程，私有内存约 120MB 是 webview 类方案（Tauri/Wails/Electron 同理）的固有下限——但远小于 Electron（自带 Chromium，300–500MB）。
> 已在 `run()` 注入 `--disable-gpu --renderer-process-limit=1` 等参数，把私有工作集从 ~160MB 压到 ~120MB。
> 若需 30–60MB 级别，只能改用纯原生工具包（如 Fyne），代价是放弃当前 Web 技术栈带来的精致 UI。
