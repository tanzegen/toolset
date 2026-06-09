//! SSH 连接配置模型：明文元数据与密码类字段（密文）物理分离。
//!
//! - 明文字段（host/port/user/group/note 等）任何时候可读、可管理、可导出；
//! - `secret` 三个字段为 `SecretBlob`（密文），忘记主密码也只丢这三类。
//! 纯数据结构 + 克隆/分组合并等无 I/O 逻辑，可单测。导入时的密文重加密由上层编排。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::vault::{KdfParams, SecretBlob, Verifier};

fn default_port() -> u16 {
    22
}
fn default_version() -> u32 {
    1
}

/// 密码类字段集合：全部为可选密文，缺省即未设置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Secrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<SecretBlob>,
    #[serde(default, rename = "keyPem", skip_serializing_if = "Option::is_none")]
    pub key_pem: Option<SecretBlob>,
    #[serde(default, rename = "keyPass", skip_serializing_if = "Option::is_none")]
    pub key_pass: Option<SecretBlob>,
}

/// 一份连接配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub group: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub auth: String, // "password" | "key"
    #[serde(default, rename = "keyPath", skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub secret: Secrets,
}

/// vault 元信息（KDF 参数 + 主密码校验块）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub kdf: KdfParams,
    pub verifier: Verifier,
}

/// 持久化根结构（落盘为 connections.json）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub connections: Vec<Connection>,
    /// 是否手动拖动排过序。false=「默认按名称」，新增/导入会重排到名称序；
    /// true=用户已手动排序，保持 connections 的物理顺序不再自动重排。随导出一同带出。
    #[serde(default, rename = "manualOrder")]
    pub manual_order: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultMeta>,
    #[serde(default, rename = "knownHosts")]
    pub known_hosts: BTreeMap<String, String>,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            version: 1,
            groups: Vec::new(),
            connections: Vec::new(),
            manual_order: false,
            vault: None,
            known_hosts: BTreeMap::new(),
        }
    }
}

impl Store {
    pub fn find(&self, id: &str) -> Option<&Connection> {
        self.connections.iter().find(|c| c.id == id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Connection> {
        self.connections.iter_mut().find(|c| c.id == id)
    }

    /// 新增或就地更新（按 id）。同时确保其分组已登记。
    pub fn upsert(&mut self, conn: Connection) {
        self.ensure_group(&conn.group);
        match self.connections.iter_mut().find(|c| c.id == conn.id) {
            Some(slot) => *slot = conn,
            None => self.connections.push(conn),
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.connections.len();
        self.connections.retain(|c| c.id != id);
        before != self.connections.len()
    }

    /// 登记一个分组名（非空且未存在时）。
    pub fn ensure_group(&mut self, group: &str) {
        if !group.is_empty() && !self.groups.iter().any(|g| g == group) {
            self.groups.push(group.to_string());
        }
    }

    /// 按名称稳定排序（不区分大小写）。供「默认按名称」在未手动排序时归一化。
    pub fn sort_by_name(&mut self) {
        self.connections
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    /// 未手动排序时按名称归一化；已手动排序则保持原序。
    pub fn normalize_order(&mut self) {
        if !self.manual_order {
            self.sort_by_name();
        }
    }
}

/// 克隆一份连接：换新 id、名字加「副本」后缀；密码类字段一并带过去（上层在 vault
/// 已解锁时会重加密到新 id 的 AAD 下；未解锁则上层应清空 secret 再保存）。
pub fn clone_connection(src: &Connection, new_id: String) -> Connection {
    Connection {
        id: new_id,
        name: format!("{} 副本", src.name),
        ..src.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(id: &str, group: &str) -> Connection {
        Connection {
            id: id.into(),
            name: format!("host-{id}"),
            group: group.into(),
            host: "10.0.0.1".into(),
            port: 22,
            username: "root".into(),
            auth: "password".into(),
            key_path: None,
            note: String::new(),
            secret: Secrets::default(),
        }
    }

    fn conn_named(id: &str, name: &str) -> Connection {
        let mut c = conn(id, "");
        c.name = name.into();
        c
    }

    #[test]
    fn serde_keeps_plaintext_and_omits_empty_secret() {
        let mut store = Store::default();
        store.upsert(conn("a", "prod"));
        let json = serde_json::to_string(&store).unwrap();
        // 明文字段在；空 secret 不序列化出任何密文块（无 ct 字段、secret 为空对象）
        assert!(json.contains("\"host\":\"10.0.0.1\""));
        assert!(json.contains("\"secret\":{}"));
        assert!(!json.contains("\"ct\""));
        // 回环
        let back: Store = serde_json::from_str(&json).unwrap();
        assert_eq!(back.connections.len(), 1);
        assert_eq!(back.find("a").unwrap().group, "prod");
    }

    #[test]
    fn upsert_registers_group_and_updates_in_place() {
        let mut store = Store::default();
        store.upsert(conn("a", "prod"));
        store.upsert(conn("a", "prod")); // 同 id 覆盖，不新增
        assert_eq!(store.connections.len(), 1);
        assert_eq!(store.groups, vec!["prod".to_string()]);
    }

    #[test]
    fn remove_works() {
        let mut store = Store::default();
        store.upsert(conn("a", ""));
        assert!(store.remove("a"));
        assert!(!store.remove("a"));
    }

    #[test]
    fn clone_gets_new_id_and_suffix() {
        let c = conn("a", "prod");
        let cloned = clone_connection(&c, "b".into());
        assert_eq!(cloned.id, "b");
        assert!(cloned.name.contains("副本"));
        assert_eq!(cloned.host, c.host);
    }

    #[test]
    fn sort_by_name_is_case_insensitive_and_stable() {
        let mut store = Store::default();
        store.connections.push(conn_named("1", "Bravo"));
        store.connections.push(conn_named("2", "alpha"));
        store.connections.push(conn_named("3", "Charlie"));
        store.sort_by_name();
        let names: Vec<_> = store.connections.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Bravo", "Charlie"]);
    }

    #[test]
    fn normalize_respects_manual_flag() {
        let mut store = Store::default();
        store.connections.push(conn_named("1", "zzz"));
        store.connections.push(conn_named("2", "aaa"));
        store.manual_order = true;
        store.normalize_order(); // 手动模式：保持物理序
        assert_eq!(store.connections[0].name, "zzz");
        store.manual_order = false;
        store.normalize_order(); // 默认模式：按名称
        assert_eq!(store.connections[0].name, "aaa");
    }

    #[test]
    fn manual_order_roundtrips_in_json() {
        let mut store = Store::default();
        store.manual_order = true;
        let json = serde_json::to_string(&store).unwrap();
        assert!(json.contains("\"manualOrder\":true"));
        let back: Store = serde_json::from_str(&json).unwrap();
        assert!(back.manual_order);
    }
}
