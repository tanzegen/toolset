// SSH 终端工具的命令封装（与后端 ssh::commands 一一对应）。
// Tauri 默认把 JS camelCase 参数转为 Rust snake_case，故这里用 camelCase。

import { invoke, Channel } from "@tauri-apps/api/core";

export interface ConnView {
  id: string;
  name: string;
  group: string;
  host: string;
  port: number;
  username: string;
  auth: string; // "password" | "key"
  keyPath: string | null;
  note: string;
  hasPassword: boolean;
  hasKeyPem: boolean;
  hasKeyPass: boolean;
}

export interface ConnList {
  groups: string[];
  connections: ConnView[];
}

export interface VaultStatus {
  hasMaster: boolean;
  unlocked: boolean;
}

export interface ConnInput {
  id?: string; // 省略 = 新建
  name: string;
  group: string;
  host: string;
  port: number;
  username: string;
  auth: string;
  keyPath?: string | null;
  note: string;
}

// 密码类字段：undefined=保持原样，""=清除，非空=设置
export interface SecretInput {
  password?: string;
  keyPem?: string;
  keyPass?: string;
}

export interface ImportResult {
  imported: number;
  secretsRecovered: number;
  secretsDropped: number;
}

export type SshFrame =
  | { kind: "data"; data: string }
  | { kind: "status"; state: string; msg: string };

export interface SftpEntry {
  name: string;
  isDir: boolean;
  isLink: boolean;
  size: number;
  mtime: number;
  permissions: number;
}

export type TransferFrame = { kind: "progress"; transferred: number; total: number };

export const VAULT_LOCKED = "__VAULT_LOCKED__";

export const ssh = {
  vaultStatus: () => invoke<VaultStatus>("ssh_vault_status"),
  vaultSetMaster: (password: string) => invoke<void>("ssh_vault_set_master", { password }),
  vaultUnlock: (password: string) => invoke<boolean>("ssh_vault_unlock", { password }),
  vaultLock: () => invoke<void>("ssh_vault_lock"),
  vaultReset: (newPassword: string) => invoke<void>("ssh_vault_reset", { newPassword }),

  connList: () => invoke<ConnList>("ssh_conn_list"),
  connSave: (conn: ConnInput, secrets?: SecretInput) =>
    invoke<ConnView>("ssh_conn_save", { conn, secrets }),
  connDelete: (id: string) => invoke<void>("ssh_conn_delete", { id }),
  connClone: (id: string) => invoke<ConnView>("ssh_conn_clone", { id }),
  connExport: (path: string) => invoke<void>("ssh_conn_export", { path }),
  connImport: (path: string, fileMaster?: string) =>
    invoke<ImportResult>("ssh_conn_import", { path, fileMaster }),

  connect: (connId: string, channel: Channel<SshFrame>) =>
    invoke<string>("ssh_connect", { connId, channel }),
  // data 可为终端字符串或 trzsz 的二进制块；统一 base64 后回传，后端解码为原始字节。
  write: (sessionId: string, data: string | Uint8Array) =>
    invoke<void>("ssh_write", { sessionId, data: toB64(data) }),
  resize: (sessionId: string, cols: number, rows: number) =>
    invoke<void>("ssh_resize", { sessionId, cols, rows }),
  close: (sessionId: string) => invoke<void>("ssh_close", { sessionId }),

  // —— SFTP ——
  sftpOpen: (connId: string) => invoke<string>("sftp_open", { connId }),
  sftpHome: (sftpId: string) => invoke<string>("sftp_home", { sftpId }),
  sftpList: (sftpId: string, path: string) => invoke<SftpEntry[]>("sftp_list", { sftpId, path }),
  sftpMkdir: (sftpId: string, path: string) => invoke<void>("sftp_mkdir", { sftpId, path }),
  sftpRemove: (sftpId: string, path: string) => invoke<void>("sftp_remove", { sftpId, path }),
  sftpRmdir: (sftpId: string, path: string) => invoke<void>("sftp_rmdir", { sftpId, path }),
  sftpRename: (sftpId: string, from: string, to: string) =>
    invoke<void>("sftp_rename", { sftpId, from, to }),
  sftpDownload: (sftpId: string, remote: string, local: string, channel: Channel<TransferFrame>) =>
    invoke<void>("sftp_download", { sftpId, remote, local, channel }),
  sftpUpload: (sftpId: string, local: string, remote: string, channel: Channel<TransferFrame>) =>
    invoke<void>("sftp_upload", { sftpId, local, remote, channel }),
  sftpUploadBytes: (sftpId: string, remote: string, dataB64: string) =>
    invoke<void>("sftp_upload_bytes", { sftpId, remote, data: dataB64 }),
  sftpClose: (sftpId: string) => invoke<void>("sftp_close", { sftpId }),
};

/** Uint8Array → base64（用于拖拽上传把 File 字节回传后端）。 */
export function bytesToBase64(bytes: Uint8Array): string {
  let s = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    s += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(s);
}

/** 后端 base64 字节 → Uint8Array（喂给 xterm.write）。 */
export function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** 字符串（UTF-8）或字节 → base64，用于把终端输入/trzsz 二进制回传后端。 */
function toB64(data: string | Uint8Array): string {
  const bytes = typeof data === "string" ? new TextEncoder().encode(data) : data;
  let s = "";
  for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
  return btoa(s);
}
