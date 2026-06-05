//! connections.json 的读写（应用配置目录）。配置模型来自 toolset-core::sshconfig。

use std::path::Path;

use toolset_core::sshconfig::Store;

/// 读取配置；文件不存在或损坏时返回空配置（不报错，首次启动即空）。
pub fn load(path: &Path) -> Store {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Store::default(),
    }
}

/// 落盘（pretty JSON）。自动创建父目录。
pub fn save(path: &Path, store: &Store) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = std::fs::write(path, json);
    }
}
