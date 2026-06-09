//! 本地终端：用 PTY（portable-pty，Windows 走 ConPTY、类 Unix 走 openpty）拉起本机
//! shell（Windows：PowerShell / cmd；macOS / Linux：$SHELL / bash / zsh），把 PTY I/O
//! 桥接到与 SSH **相同**的会话通道（state.sessions），从而复用
//! ssh_write / ssh_resize / ssh_close —— 本地会话只需多一个 `local_open` 入口。
//!
//! PTY 的读/写/调整大小都是阻塞操作，放在独立 OS 线程，不占用 tokio 运行时。

use std::io::{Read, Write};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::mpsc;
use uuid::Uuid;

use toolset_core::error::{AppError, AppResult};

use super::{SessionCmd, SessionHandle, SshFrame, SshState};

/// 把前端传入的 shell 标识映射为可执行命令；未识别则按程序名直接执行。
fn build_command(shell: &str) -> CommandBuilder {
    let mut cmd = resolve_shell(shell);
    // 在用户主目录启动，行为更接近原生终端（Windows 否则落在 system32）。
    // Windows 用 USERPROFILE，类 Unix 用 HOME。
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        cmd.cwd(home);
    }
    #[cfg(unix)]
    {
        // 声明终端类型为 xterm-256color，与前端 xterm.js 的能力匹配。GUI 启动的进程
        // 通常没有 TERM，shell 的行编辑器(zle)缺少光标控制能力，自动补全等重绘会错乱
        // （如输入 ls 变成 lsls）。终端模拟器本就应主动声明自身 TERM，故无条件设置。
        cmd.env("TERM", "xterm-256color");
        // GUI 启动的进程通常也不带 locale 环境变量，shell 会落到 C 区域，
        // 导致 ls 等把中文文件名显示成 ?/乱码。环境里完全没有 locale 时补一个 UTF-8 字符集。
        let has_locale = std::env::var_os("LC_ALL").is_some()
            || std::env::var_os("LC_CTYPE").is_some()
            || std::env::var_os("LANG").is_some();
        if !has_locale {
            // macOS/BSD 允许只设字符集、不强制语言；Linux 需要完整 locale。
            #[cfg(target_os = "macos")]
            cmd.env("LC_CTYPE", "UTF-8");
            #[cfg(not(target_os = "macos"))]
            cmd.env("LANG", "C.UTF-8");
        }
    }
    cmd
}

/// Windows 默认 PowerShell；"default" / "" 同样回退到 PowerShell。
#[cfg(windows)]
fn resolve_shell(shell: &str) -> CommandBuilder {
    match shell {
        "cmd" => CommandBuilder::new("cmd.exe"),
        "pwsh" => CommandBuilder::new("pwsh.exe"),
        "wsl" => CommandBuilder::new("wsl.exe"),
        "powershell" | "default" | "" => CommandBuilder::new("powershell.exe"),
        other => CommandBuilder::new(other),
    }
}

/// 类 Unix（macOS / Linux）：默认取 $SHELL（用户登录 shell），缺失再回退 /bin/sh。
#[cfg(unix)]
fn resolve_shell(shell: &str) -> CommandBuilder {
    match shell {
        "bash" => CommandBuilder::new("bash"),
        "zsh" => CommandBuilder::new("zsh"),
        "sh" => CommandBuilder::new("sh"),
        "default" | "" => {
            CommandBuilder::new(std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()))
        }
        other => CommandBuilder::new(other),
    }
}

/// 开一个本地 PTY 会话；返回 session_id（与 SSH 会话同命名空间，复用读写/关闭命令）。
#[tauri::command]
pub fn local_open(
    state: State<SshState>,
    shell: String,
    channel: Channel<SshFrame>,
) -> AppResult<String> {
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AppError::Invalid(format!("打开本地 PTY 失败：{e}")))?;

    let cmd = build_command(&shell);
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| AppError::Invalid(format!("启动本地 shell 失败：{e}")))?;
    drop(pair.slave); // Windows 上必须释放 slave，否则读端不会收到 EOF

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| AppError::Invalid(format!("获取 PTY 读取端失败：{e}")))?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|e| AppError::Invalid(format!("获取 PTY 写入端失败：{e}")))?;
    let master = pair.master; // 留给控制线程做 resize

    let session_id = Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel::<SessionCmd>();
    state.sessions.insert(session_id.clone(), SessionHandle { tx });

    // 控制线程：输入 / 调整大小 / 关闭（均阻塞）。rx 关闭（会话被移除）或收到 Close 即收尾。
    std::thread::spawn(move || {
        while let Some(cmd) = rx.blocking_recv() {
            match cmd {
                SessionCmd::Data(bytes) => {
                    if writer.write_all(&bytes).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
                SessionCmd::Resize { cols, rows } => {
                    let _ = master.resize(PtySize {
                        rows: rows as u16,
                        cols: cols as u16,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                SessionCmd::Close => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        drop(master);
    });

    // 读取线程：阻塞读 PTY 输出 → Channel 推给前端；EOF 即本地 shell 结束。
    let sessions = state.sessions.clone();
    let sid = session_id.clone();
    std::thread::spawn(move || {
        let _ = channel.send(SshFrame::Status {
            state: "connected".into(),
            msg: String::new(),
        });
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = channel.send(SshFrame::Data {
                        data: B64.encode(&buf[..n]),
                    });
                }
            }
        }
        sessions.remove(&sid); // 自清理，避免会话泄露
        // 本地 shell 退出（exit / 关闭窗口）一律视为干净退出，前端据此关闭标签页。
        let _ = channel.send(SshFrame::Status {
            state: "exited".into(),
            msg: String::new(),
        });
    });

    Ok(session_id)
}
