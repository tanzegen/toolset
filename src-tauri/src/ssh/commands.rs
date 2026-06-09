//! SSH 工具的 Tauri 命令：vault（主密码）、连接 CRUD/克隆/导入导出、会话生命周期。

use std::collections::{HashMap, HashSet};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;
use uuid::Uuid;

use toolset_core::error::{AppError, AppResult};
use toolset_core::sshconfig::{clone_connection, Connection, Secrets, VaultMeta};
use toolset_core::vault::{self, KdfParams, SecretBlob};

use super::client::{self, ConnParams};
use super::{store, SessionCmd, SshFrame, SshState};

// —— 字段名常量：参与密文 AAD，加解密两侧必须一致 ——
const F_PASSWORD: &str = "password";
const F_KEY_PEM: &str = "keyPem";
const F_KEY_PASS: &str = "keyPass";

// ============================ DTO ============================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnView {
    id: String,
    name: String,
    group: String,
    host: String,
    port: u16,
    username: String,
    auth: String,
    key_path: Option<String>,
    note: String,
    has_password: bool,
    has_key_pem: bool,
    has_key_pass: bool,
}

impl From<&Connection> for ConnView {
    fn from(c: &Connection) -> Self {
        ConnView {
            id: c.id.clone(),
            name: c.name.clone(),
            group: c.group.clone(),
            host: c.host.clone(),
            port: c.port,
            username: c.username.clone(),
            auth: c.auth.clone(),
            key_path: c.key_path.clone(),
            note: c.note.clone(),
            has_password: c.secret.password.is_some(),
            has_key_pem: c.secret.key_pem.is_some(),
            has_key_pass: c.secret.key_pass.is_some(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnList {
    groups: Vec<String>,
    connections: Vec<ConnView>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnInput {
    id: Option<String>, // None = 新建
    name: String,
    #[serde(default)]
    group: String,
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    username: String,
    #[serde(default)]
    auth: String,
    key_path: Option<String>,
    #[serde(default)]
    note: String,
}

fn default_port() -> u16 {
    22
}

/// 密码类字段输入语义：None=保持原样，Some("")=清除，Some(非空)=设置。
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecretInput {
    password: Option<String>,
    key_pem: Option<String>,
    key_pass: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    has_master: bool,
    unlocked: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    imported: usize,
    secrets_recovered: usize,
    secrets_dropped: usize,
}

// ============================ vault ============================

#[tauri::command]
pub fn ssh_vault_status(state: State<SshState>) -> VaultStatus {
    let store = state.store.lock().unwrap();
    VaultStatus {
        has_master: store.vault.is_some(),
        unlocked: state.key().is_some(),
    }
}

#[tauri::command]
pub fn ssh_vault_set_master(state: State<SshState>, password: String) -> AppResult<()> {
    if password.is_empty() {
        return Err(AppError::Invalid("主密码不能为空".into()));
    }
    let mut store = state.store.lock().unwrap();
    if store.vault.is_some() {
        return Err(AppError::Invalid("主密码已设置（如需更换请用重置）".into()));
    }
    let kdf = KdfParams::generate();
    let key = vault::derive_key(&password, &kdf)?;
    let verifier = vault::make_verifier(&key)?;
    store.vault = Some(VaultMeta { kdf, verifier });
    store::save(&state.path, &store);
    drop(store);
    state.set_key(Some(key));
    Ok(())
}

#[tauri::command]
pub fn ssh_vault_unlock(state: State<SshState>, password: String) -> AppResult<bool> {
    let meta = {
        let store = state.store.lock().unwrap();
        store.vault.clone()
    };
    let Some(meta) = meta else {
        return Err(AppError::Invalid("尚未设置主密码".into()));
    };
    let key = vault::derive_key(&password, &meta.kdf)?;
    if vault::check_verifier(&key, &meta.verifier) {
        state.set_key(Some(key));
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn ssh_vault_lock(state: State<SshState>) {
    state.set_key(None);
}

/// 重置 vault：清空所有密码类字段（保留全部明文连接），重设新主密码。
#[tauri::command]
pub fn ssh_vault_reset(state: State<SshState>, new_password: String) -> AppResult<()> {
    if new_password.is_empty() {
        return Err(AppError::Invalid("主密码不能为空".into()));
    }
    let mut store = state.store.lock().unwrap();
    for c in store.connections.iter_mut() {
        c.secret = Secrets::default();
    }
    let kdf = KdfParams::generate();
    let key = vault::derive_key(&new_password, &kdf)?;
    let verifier = vault::make_verifier(&key)?;
    store.vault = Some(VaultMeta { kdf, verifier });
    store::save(&state.path, &store);
    drop(store);
    state.set_key(Some(key));
    Ok(())
}

// ============================ 连接 CRUD ============================

#[tauri::command]
pub fn ssh_conn_list(state: State<SshState>) -> ConnList {
    let store = state.store.lock().unwrap();
    ConnList {
        groups: store.groups.clone(),
        connections: store.connections.iter().map(ConnView::from).collect(),
    }
}

#[tauri::command]
pub fn ssh_conn_save(
    state: State<SshState>,
    conn: ConnInput,
    secrets: Option<SecretInput>,
) -> AppResult<ConnView> {
    let id = conn.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut store = state.store.lock().unwrap();

    // 保留原有 secret（未显式改动的字段不动）
    let mut secret = store.find(&id).map(|e| e.secret.clone()).unwrap_or_default();

    if let Some(sec) = secrets {
        let key = state.key();
        apply_secret(&mut secret.password, &id, F_PASSWORD, sec.password, key.as_ref())?;
        apply_secret(&mut secret.key_pem, &id, F_KEY_PEM, sec.key_pem, key.as_ref())?;
        apply_secret(&mut secret.key_pass, &id, F_KEY_PASS, sec.key_pass, key.as_ref())?;
    }

    let c = Connection {
        id: id.clone(),
        name: conn.name,
        group: conn.group,
        host: conn.host,
        port: conn.port,
        username: conn.username,
        auth: conn.auth,
        key_path: conn.key_path,
        note: conn.note,
        secret,
    };
    let view = ConnView::from(&c);
    store.upsert(c);
    store.normalize_order(); // 未手动排序则新增/改名后按名称归位
    store::save(&state.path, &store);
    Ok(view)
}

/// 手动拖动排序：按给定的完整 id 顺序重排 connections，并置为手动排序模式。
#[tauri::command]
pub fn ssh_conn_reorder(state: State<SshState>, ids: Vec<String>) -> AppResult<()> {
    let mut store = state.store.lock().unwrap();
    let pos: HashMap<&String, usize> = ids.iter().enumerate().map(|(i, id)| (id, i)).collect();
    // 未列入 ids 的连接（理论上不会有）按 usize::MAX 落到末尾，保持稳定。
    store
        .connections
        .sort_by_key(|c| pos.get(&c.id).copied().unwrap_or(usize::MAX));
    store.manual_order = true;
    store::save(&state.path, &store);
    Ok(())
}

#[tauri::command]
pub fn ssh_conn_delete(state: State<SshState>, id: String) -> AppResult<()> {
    let mut store = state.store.lock().unwrap();
    store.remove(&id);
    store::save(&state.path, &store);
    Ok(())
}

#[tauri::command]
pub fn ssh_conn_clone(state: State<SshState>, id: String) -> AppResult<ConnView> {
    let mut store = state.store.lock().unwrap();
    let src = store
        .find(&id)
        .ok_or_else(|| AppError::Invalid("连接不存在".into()))?
        .clone();
    let new_id = Uuid::new_v4().to_string();
    let mut c = clone_connection(&src, new_id.clone());

    if has_any_secret(&src.secret) {
        match state.key() {
            Some(k) => c.secret = reencrypt(&src.secret, &id, &k, &new_id, &k)?,
            None => c.secret = Secrets::default(), // 未解锁则克隆不带密码
        }
    }
    let view = ConnView::from(&c);
    store.upsert(c);
    store.normalize_order(); // 未手动排序则克隆产物按名称归位
    store::save(&state.path, &store);
    Ok(view)
}

// ============================ 导入 / 导出 ============================

/// 选择部分连接导出，并可为这份导出**重设主密码**。
/// - new_master 为 Some(非空)：用新主密码重新加密选中连接的密码类字段（需当前 vault 已解锁
///   以解密原密文），导出文件自带新 KDF/校验块，导入端用新主密码即可恢复。
/// - new_master 为 None/空：密文原样带出，导出文件沿用当前 vault 元信息（导入端用**当前**主密码恢复）。
/// 返回导出的连接条数。
#[tauri::command]
pub fn ssh_conn_export_selected(
    state: State<SshState>,
    ids: Vec<String>,
    path: String,
    new_master: Option<String>,
) -> AppResult<usize> {
    let store = state.store.lock().unwrap();
    // 按 store 的物理顺序过滤（而非 ids 的传入顺序），保证部分导出仍保持原相对顺序。
    let want: HashSet<&String> = ids.iter().collect();
    let selected: Vec<Connection> = store
        .connections
        .iter()
        .filter(|c| want.contains(&c.id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return Err(AppError::Invalid("未选择任何连接".into()));
    }

    // 新主密码 → 新 KDF/校验块/密钥（用于重加密导出内容）。
    let new_vault = match new_master.as_deref() {
        Some(pw) if !pw.is_empty() => {
            let kdf = KdfParams::generate();
            let key = vault::derive_key(pw, &kdf)?;
            let verifier = vault::make_verifier(&key)?;
            Some((key, VaultMeta { kdf, verifier }))
        }
        _ => None,
    };
    let cur_key = state.key();

    let mut out = toolset_core::sshconfig::Store::default();
    for src in &selected {
        let mut c = src.clone();
        if has_any_secret(&src.secret) {
            match new_vault.as_ref() {
                // 重设主密码：把密码从当前密钥重加密到新密钥（AAD 仍用连接 id，导入端据此解密）
                Some((nk, _)) => match cur_key.as_ref() {
                    Some(ck) => c.secret = reencrypt(&src.secret, &src.id, ck, &src.id, nk)?,
                    None => return Err(AppError::Invalid("__VAULT_LOCKED__".into())),
                },
                // 不重设主密码：密文原样带出（导入端用当前主密码恢复）
                None => {}
            }
        }
        out.ensure_group(&c.group);
        let hk = host_key(&c.host, c.port);
        if let Some(fp) = store.known_hosts.get(&hk) {
            out.known_hosts.insert(hk, fp.clone());
        }
        out.connections.push(c);
    }
    out.vault = match new_vault {
        Some((_, meta)) => Some(meta),
        None => store.vault.clone(),
    };
    out.manual_order = store.manual_order; // 让顺序随导出一同带出

    let json = serde_json::to_string_pretty(&out)
        .map_err(|e| AppError::Invalid(format!("序列化失败：{e}")))?;
    std::fs::write(&path, json).map_err(|e| AppError::Invalid(format!("写入失败：{e}")))?;
    Ok(out.connections.len())
}

#[tauri::command]
pub fn ssh_conn_import(
    state: State<SshState>,
    path: String,
    file_master: Option<String>,
) -> AppResult<ImportResult> {
    let text =
        std::fs::read_to_string(&path).map_err(|e| AppError::Invalid(format!("读取失败：{e}")))?;
    let incoming: toolset_core::sshconfig::Store =
        serde_json::from_str(&text).map_err(|e| AppError::Invalid(format!("文件格式错误：{e}")))?;

    // 用文件自带的 KDF + 主密码派生「文件密钥」，用于解密文件里的密文字段
    let file_key = match (&incoming.vault, &file_master) {
        (Some(meta), Some(pw)) => {
            let k = vault::derive_key(pw, &meta.kdf)?;
            if vault::check_verifier(&k, &meta.verifier) {
                Some(k)
            } else {
                return Err(AppError::Invalid("导入文件的主密码不正确".into()));
            }
        }
        _ => None,
    };
    let cur_key = state.key(); // 用当前 vault 密钥重新加密入库

    let mut store = state.store.lock().unwrap();
    let pre = store.connections.len(); // 导入前已有连接数，用于判断是否「整库导入」
    let (mut imported, mut recovered, mut dropped) = (0usize, 0usize, 0usize);
    for src in &incoming.connections {
        let new_id = Uuid::new_v4().to_string();
        let mut c = src.clone();
        c.id = new_id.clone();
        if has_any_secret(&src.secret) {
            match (file_key.as_ref(), cur_key.as_ref()) {
                (Some(fk), Some(ck)) => {
                    c.secret = reencrypt(&src.secret, &src.id, fk, &new_id, ck)?;
                    recovered += count_secrets(&c.secret);
                }
                _ => {
                    dropped += count_secrets(&src.secret);
                    c.secret = Secrets::default();
                }
            }
        }
        store.upsert(c);
        imported += 1;
    }
    for g in &incoming.groups {
        store.ensure_group(g);
    }
    // 顺序处理：导入一份手动排序的整库（之前无连接）→ 采纳其顺序并转为手动模式；
    // 否则在非手动模式下按名称归一化（导入项按字母序插入），手动模式则保留追加顺序。
    if incoming.manual_order && pre == 0 {
        store.manual_order = true;
    } else {
        store.normalize_order();
    }
    store::save(&state.path, &store);
    Ok(ImportResult {
        imported,
        secrets_recovered: recovered,
        secrets_dropped: dropped,
    })
}

// ============================ 会话 ============================

#[tauri::command]
pub async fn ssh_connect(
    state: State<'_, SshState>,
    conn_id: String,
    channel: Channel<SshFrame>,
) -> AppResult<String> {
    let (params, host, port) = build_params(&state, &conn_id)?;
    let (handle, fp, first_seen) = client::connect(params).await?;
    let session_id = Uuid::new_v4().to_string();
    // start_shell 会自行登记会话并在退出时移除，无需在此 insert。
    client::start_shell(handle, channel, state.sessions.clone(), session_id.clone()).await?;
    if first_seen {
        record_fp(&state, &host, port, fp);
    }
    Ok(session_id)
}

/// 从配置取连接并解密所需密文，组装 ConnParams（终端与 SFTP 共用）。
/// 未解锁时返回哨兵错误 `__VAULT_LOCKED__`，前端据此弹主密码框后重试。
pub(crate) fn build_params(
    state: &SshState,
    conn_id: &str,
) -> AppResult<(ConnParams, String, u16)> {
    let c = {
        let store = state.store.lock().unwrap();
        store
            .find(conn_id)
            .ok_or_else(|| AppError::Invalid("连接不存在".into()))?
            .clone()
    };
    let known_fp = {
        let store = state.store.lock().unwrap();
        store.known_hosts.get(&host_key(&c.host, c.port)).cloned()
    };

    let need_secret = match c.auth.as_str() {
        "key" => c.secret.key_pem.is_some(),
        _ => c.secret.password.is_some(),
    };
    let key = if need_secret {
        Some(state.key().ok_or_else(|| AppError::Invalid("__VAULT_LOCKED__".into()))?)
    } else {
        None
    };
    let (password, key_pem, key_pass) = if c.auth == "key" {
        (
            None,
            decrypt_opt(&c.secret.key_pem, conn_id, F_KEY_PEM, key.as_ref())?,
            decrypt_opt(&c.secret.key_pass, conn_id, F_KEY_PASS, key.as_ref())?,
        )
    } else {
        (
            decrypt_opt(&c.secret.password, conn_id, F_PASSWORD, key.as_ref())?,
            None,
            None,
        )
    };
    let params = ConnParams {
        host: c.host.clone(),
        port: c.port,
        username: c.username.clone(),
        auth: c.auth.clone(),
        password,
        key_pem,
        key_pass,
        known_fp,
    };
    Ok((params, c.host, c.port))
}

/// 首次连接成功后记录主机指纹（TOFU）。
pub(crate) fn record_fp(state: &SshState, host: &str, port: u16, fp: Option<String>) {
    if let Some(fp) = fp {
        let mut store = state.store.lock().unwrap();
        store.known_hosts.insert(host_key(host, port), fp);
        store::save(&state.path, &store);
    }
}

#[tauri::command]
pub fn ssh_write(state: State<SshState>, session_id: String, data: String) -> AppResult<()> {
    // data 为 base64（前端统一编码：终端字符串或 trzsz 二进制块）。
    let bytes = B64
        .decode(data.as_bytes())
        .map_err(|_| AppError::Invalid("写入数据非法".into()))?;
    if let Some(h) = state.sessions.get(&session_id) {
        h.tx.send(SessionCmd::Data(bytes))
            .map_err(|_| AppError::Invalid("会话已结束".into()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn ssh_resize(
    state: State<SshState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> AppResult<()> {
    if let Some(h) = state.sessions.get(&session_id) {
        let _ = h.tx.send(SessionCmd::Resize { cols, rows });
    }
    Ok(())
}

#[tauri::command]
pub fn ssh_close(state: State<SshState>, session_id: String) {
    if let Some((_, h)) = state.sessions.remove(&session_id) {
        let _ = h.tx.send(SessionCmd::Close);
    }
}

// ============================ 辅助 ============================

fn host_key(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

fn has_any_secret(s: &Secrets) -> bool {
    s.password.is_some() || s.key_pem.is_some() || s.key_pass.is_some()
}

fn count_secrets(s: &Secrets) -> usize {
    s.password.is_some() as usize + s.key_pem.is_some() as usize + s.key_pass.is_some() as usize
}

/// 解密一个可选密文字段；blob 为 None 则返回 None。
fn decrypt_opt(
    blob: &Option<SecretBlob>,
    conn_id: &str,
    field: &str,
    key: Option<&[u8; 32]>,
) -> AppResult<Option<String>> {
    match blob {
        None => Ok(None),
        Some(b) => {
            let k = key.ok_or_else(|| AppError::Invalid("vault 未解锁".into()))?;
            Ok(Some(vault::decrypt_field(k, conn_id, field, b)?))
        }
    }
}

/// 按输入语义更新单个密文字段（None 保持 / Some("") 清除 / Some(非空) 加密设置）。
fn apply_secret(
    target: &mut Option<SecretBlob>,
    conn_id: &str,
    field: &str,
    input: Option<String>,
    key: Option<&[u8; 32]>,
) -> AppResult<()> {
    match input {
        None => {}
        Some(s) if s.is_empty() => *target = None,
        Some(s) => {
            let k = key.ok_or_else(|| AppError::Invalid("vault 未解锁，无法保存密码".into()))?;
            *target = Some(vault::encrypt_field(k, conn_id, field, &s)?);
        }
    }
    Ok(())
}

/// 把一份 Secrets 从 (old_id, old_key) 重加密到 (new_id, new_key)。
fn reencrypt(
    src: &Secrets,
    old_id: &str,
    old_key: &[u8; 32],
    new_id: &str,
    new_key: &[u8; 32],
) -> AppResult<Secrets> {
    let move_field = |field: &str, blob: &Option<SecretBlob>| -> AppResult<Option<SecretBlob>> {
        match blob {
            None => Ok(None),
            Some(b) => {
                let pt = vault::decrypt_field(old_key, old_id, field, b)?;
                Ok(Some(vault::encrypt_field(new_key, new_id, field, &pt)?))
            }
        }
    };
    Ok(Secrets {
        password: move_field(F_PASSWORD, &src.password)?,
        key_pem: move_field(F_KEY_PEM, &src.key_pem)?,
        key_pass: move_field(F_KEY_PASS, &src.key_pass)?,
    })
}
