//! JSON 工具：美化（可配缩进）、压缩、校验（报错带行列）。

use crate::error::{AppError, AppResult};
use serde::Serialize;
use serde_json::Value;

fn parse(input: &str) -> AppResult<Value> {
    if input.trim().is_empty() {
        return Err(AppError::Empty);
    }
    serde_json::from_str::<Value>(input)
        .map_err(|e| AppError::Invalid(format!("第 {} 行第 {} 列: {e}", e.line(), e.column())))
}

fn pretty_with_indent(v: &Value, indent: usize) -> String {
    let pad = " ".repeat(indent.clamp(1, 8));
    let buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(pad.as_bytes());
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
    v.serialize(&mut ser).expect("Value 序列化不会失败");
    String::from_utf8(ser.into_inner()).expect("serde_json 输出始终是 UTF-8")
}

pub fn json_format(input: String, indent: u8) -> AppResult<String> {
    let v = parse(&input)?;
    Ok(pretty_with_indent(&v, indent as usize))
}

pub fn json_minify(input: String) -> AppResult<String> {
    let v = parse(&input)?;
    serde_json::to_string(&v).map_err(|e| AppError::Invalid(e.to_string()))
}

// —— 字段提取（投影）：按勾选的点路径，从对象/对象数组里挑出字段组成新 JSON ——
// 语义：路径是一串键，遇到数组就「逐元素 map」（根数组与深层数组一致，路径不含下标）。

/// 字段路径前缀树。某节点 children 为空 = 选中的叶子，对应「整段子树原样保留」。
#[derive(Default)]
struct PathTrie {
    children: std::collections::BTreeMap<String, PathTrie>,
}

fn build_trie(paths: &[String]) -> PathTrie {
    let mut root = PathTrie::default();
    for p in paths {
        let mut node = &mut root;
        for seg in p.split('.').filter(|s| !s.is_empty()) {
            node = node.children.entry(seg.to_string()).or_default();
        }
    }
    root
}

/// 按 trie 投影：trie 到叶子（无 children）即整段保留；否则数组逐元素 map、
/// 对象按源键序挑选 trie 命中的键（保留原始字段顺序）；标量但路径还要往深 → 丢弃。
fn project(value: &Value, trie: &PathTrie) -> Option<Value> {
    if trie.children.is_empty() {
        return Some(value.clone());
    }
    match value {
        Value::Array(arr) => Some(Value::Array(
            arr.iter().filter_map(|el| project(el, trie)).collect(),
        )),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if let Some(sub) = trie.children.get(k) {
                    if let Some(pv) = project(v, sub) {
                        out.insert(k.clone(), pv);
                    }
                }
            }
            Some(Value::Object(out))
        }
        _ => None,
    }
}

/// 从 JSON 中挑出 paths 指定的字段，组成新 JSON（数组→逐元素投影，对象→投影一次）。
pub fn json_pick(input: String, paths: Vec<String>) -> AppResult<String> {
    let v = parse(&input)?;
    if paths.is_empty() {
        // 未选字段：给出与输入同形的空容器，提示「请勾选」，而非误返回整篇。
        let empty = match &v {
            Value::Array(_) => Value::Array(Vec::new()),
            _ => Value::Object(serde_json::Map::new()),
        };
        return Ok(pretty_with_indent(&empty, 2));
    }
    let projected = project(&v, &build_trie(&paths)).unwrap_or(Value::Null);
    Ok(pretty_with_indent(&projected, 2))
}

#[derive(Serialize)]
pub struct JsonValidateResult {
    pub valid: bool,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

pub fn json_validate(input: String) -> JsonValidateResult {
    match serde_json::from_str::<Value>(&input) {
        Ok(_) => JsonValidateResult {
            valid: true,
            message: "JSON 合法".to_string(),
            line: 0,
            column: 0,
        },
        Err(e) => JsonValidateResult {
            valid: false,
            message: e.to_string(),
            line: e.line(),
            column: e.column(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_indent_2() {
        let out = json_format(r#"{"a":1,"b":[2,3]}"#.into(), 2).unwrap();
        assert_eq!(out, "{\n  \"a\": 1,\n  \"b\": [\n    2,\n    3\n  ]\n}");
    }

    #[test]
    fn format_indent_4() {
        let out = json_format(r#"{"a":1}"#.into(), 4).unwrap();
        assert_eq!(out, "{\n    \"a\": 1\n}");
    }

    #[test]
    fn minify_strips_whitespace() {
        let out = json_minify("{\n  \"a\": 1\n}".into()).unwrap();
        assert_eq!(out, r#"{"a":1}"#);
    }

    #[test]
    fn validate_reports_error_position() {
        let r = json_validate("{\"a\": }".into());
        assert!(!r.valid);
        assert!(r.column > 0);
    }

    #[test]
    fn validate_ok() {
        assert!(json_validate(r#"{"a":1}"#.into()).valid);
    }

    #[test]
    fn format_invalid_errors() {
        assert!(json_format("{bad}".into(), 2).is_err());
    }

    fn pick(input: &str, paths: &[&str]) -> Value {
        let out = json_pick(input.into(), paths.iter().map(|s| s.to_string()).collect()).unwrap();
        serde_json::from_str(&out).unwrap()
    }

    #[test]
    fn pick_top_level_from_array() {
        let v = pick(r#"[{"id":1,"name":"A","age":30},{"id":2,"name":"B","age":25}]"#, &["id", "name"]);
        assert_eq!(v, serde_json::json!([{"id":1,"name":"A"},{"id":2,"name":"B"}]));
    }

    #[test]
    fn pick_nested_object_path() {
        let v = pick(r#"[{"id":1,"addr":{"city":"BJ","zip":"100"}}]"#, &["id", "addr.city"]);
        assert_eq!(v, serde_json::json!([{"id":1,"addr":{"city":"BJ"}}]));
    }

    #[test]
    fn pick_drills_into_nested_array() {
        let v = pick(r#"[{"id":1,"items":[{"sku":"A","qty":2},{"sku":"B","qty":1}]}]"#, &["items.sku"]);
        assert_eq!(v, serde_json::json!([{"items":[{"sku":"A"},{"sku":"B"}]}]));
    }

    #[test]
    fn pick_omits_missing_field() {
        let v = pick(r#"[{"id":1,"name":"A"},{"id":2}]"#, &["name"]);
        assert_eq!(v, serde_json::json!([{"name":"A"},{}]));
    }

    #[test]
    fn pick_single_object_root() {
        let v = pick(r#"{"a":1,"b":2,"c":3}"#, &["a", "c"]);
        assert_eq!(v, serde_json::json!({"a":1,"c":3}));
    }

    #[test]
    fn pick_preserves_key_order() {
        // 依赖 serde_json preserve_order：输出键序跟随输入（b 在 a 前）
        let out = json_pick(r#"{"b":1,"a":2}"#.into(), vec!["b".into(), "a".into()]).unwrap();
        assert_eq!(out, "{\n  \"b\": 1,\n  \"a\": 2\n}");
    }

    #[test]
    fn pick_empty_selection_returns_empty_container() {
        let out = json_pick(r#"[{"a":1}]"#.into(), vec![]).unwrap();
        assert_eq!(out, "[]");
    }
}
