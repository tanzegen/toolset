//! JSON → 语言结构：递归推断类型，生成 Go struct / TS interface / Rust struct。

use crate::error::{AppError, AppResult};
use heck::{ToSnakeCase, ToUpperCamelCase};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Go,
    Ts,
    Rust,
}

impl Lang {
    fn any(self) -> &'static str {
        match self {
            Lang::Go => "interface{}",
            Lang::Ts => "any",
            Lang::Rust => "serde_json::Value",
        }
    }
    fn bool(self) -> &'static str {
        match self {
            Lang::Go => "bool",
            Lang::Ts => "boolean",
            Lang::Rust => "bool",
        }
    }
    fn int(self) -> &'static str {
        match self {
            Lang::Go => "int64",
            Lang::Ts => "number",
            Lang::Rust => "i64",
        }
    }
    fn float(self) -> &'static str {
        match self {
            Lang::Go => "float64",
            Lang::Ts => "number",
            Lang::Rust => "f64",
        }
    }
    fn string(self) -> &'static str {
        match self {
            Lang::Go => "string",
            Lang::Ts => "string",
            Lang::Rust => "String",
        }
    }
    fn array_of(self, t: &str) -> String {
        match self {
            Lang::Go => format!("[]{t}"),
            Lang::Ts => format!("{t}[]"),
            Lang::Rust => format!("Vec<{t}>"),
        }
    }
}

fn type_name(key: &str) -> String {
    key.to_upper_camel_case()
}

/// 取数组元素的命名：复数去 s，否则加 Item，避免与根名/字段名撞车。
fn singular(s: &str) -> String {
    if s.len() > 1 && s.ends_with('s') {
        s[..s.len() - 1].to_string()
    } else {
        format!("{s}Item")
    }
}

fn type_ref(
    v: &Value,
    suggested: &str,
    lang: Lang,
    defs: &mut Vec<String>,
    seen: &mut HashSet<String>,
) -> String {
    match v {
        Value::Null => lang.any().to_string(),
        Value::Bool(_) => lang.bool().to_string(),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                lang.int().to_string()
            } else {
                lang.float().to_string()
            }
        }
        Value::String(_) => lang.string().to_string(),
        Value::Array(arr) => {
            let elem = arr.iter().find(|e| !e.is_null());
            let et = match elem {
                Some(e) => type_ref(e, &singular(suggested), lang, defs, seen),
                None => lang.any().to_string(),
            };
            lang.array_of(&et)
        }
        Value::Object(_) => {
            emit_struct(v, suggested, lang, defs, seen);
            suggested.to_string()
        }
    }
}

fn emit_struct(
    v: &Value,
    name: &str,
    lang: Lang,
    defs: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Value::Object(map) = v else {
        return;
    };
    if !seen.insert(name.to_string()) {
        return;
    }
    // (原始 key, 类型字符串, 是否 null)
    let mut fields: Vec<(String, String, bool)> = Vec::new();
    for (k, val) in map {
        let ty = type_ref(val, &type_name(k), lang, defs, seen);
        fields.push((k.clone(), ty, val.is_null()));
    }
    defs.push(render_struct(lang, name, &fields));
}

fn render_struct(lang: Lang, name: &str, fields: &[(String, String, bool)]) -> String {
    match lang {
        Lang::Go => {
            let mut s = format!("type {name} struct {{\n");
            for (k, ty, _) in fields {
                s.push_str(&format!(
                    "\t{} {} `json:\"{}\"`\n",
                    k.to_upper_camel_case(),
                    ty,
                    k
                ));
            }
            s.push('}');
            s
        }
        Lang::Ts => {
            let mut s = format!("interface {name} {{\n");
            for (k, ty, is_null) in fields {
                let opt = if *is_null { "?" } else { "" };
                s.push_str(&format!("  {k}{opt}: {ty};\n"));
            }
            s.push('}');
            s
        }
        Lang::Rust => {
            let mut s = format!("#[derive(Debug, Serialize, Deserialize)]\npub struct {name} {{\n");
            for (k, ty, _) in fields {
                let snake = k.to_snake_case();
                if &snake != k {
                    s.push_str(&format!("    #[serde(rename = \"{k}\")]\n"));
                }
                s.push_str(&format!("    pub {snake}: {ty},\n"));
            }
            s.push('}');
            s
        }
    }
}

fn render_alias(lang: Lang, name: &str, ty: &str) -> String {
    match lang {
        Lang::Go => format!("type {name} {ty}"),
        Lang::Ts => format!("type {name} = {ty};"),
        Lang::Rust => format!("pub type {name} = {ty};"),
    }
}

pub fn json_to_struct(json: String, lang: String, root_name: String) -> AppResult<String> {
    if json.trim().is_empty() {
        return Err(AppError::Empty);
    }
    let value: Value = serde_json::from_str(&json)
        .map_err(|e| AppError::Invalid(format!("第 {} 行第 {} 列: {e}", e.line(), e.column())))?;
    let lang = match lang.as_str() {
        "go" => Lang::Go,
        "ts" => Lang::Ts,
        "rust" => Lang::Rust,
        _ => return Err(AppError::Invalid("不支持的语言".to_string())),
    };
    let root = {
        let r = root_name.trim();
        if r.is_empty() {
            "Root".to_string()
        } else {
            r.to_upper_camel_case()
        }
    };

    let mut defs: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let top = type_ref(&value, &root, lang, &mut defs, &mut seen);
    if !matches!(value, Value::Object(_)) {
        defs.push(render_alias(lang, &root, &top));
    }
    if defs.is_empty() {
        return Err(AppError::Invalid("无法从该 JSON 生成结构".to_string()));
    }
    defs.reverse(); // 根类型在前
    Ok(defs.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_basic() {
        let out = json_to_struct(
            r#"{"id":1,"name":"a","active":true}"#.into(),
            "go".into(),
            "User".into(),
        )
        .unwrap();
        assert!(out.contains("type User struct"));
        assert!(out.contains("Id int64 `json:\"id\"`"));
        assert!(out.contains("Name string `json:\"name\"`"));
        assert!(out.contains("Active bool `json:\"active\"`"));
    }

    #[test]
    fn ts_nested_and_array() {
        let out = json_to_struct(
            r#"{"user":{"id":1},"tags":["x"]}"#.into(),
            "ts".into(),
            "Root".into(),
        )
        .unwrap();
        assert!(out.contains("interface Root"));
        assert!(out.contains("interface User"));
        assert!(out.contains("tags: string[];"));
        assert!(out.contains("user: User;"));
    }

    #[test]
    fn rust_rename_and_float() {
        let out = json_to_struct(
            r#"{"user_name":"a","ratio":1.5}"#.into(),
            "rust".into(),
            "Item".into(),
        )
        .unwrap();
        assert!(out.contains("pub struct Item"));
        assert!(out.contains("pub user_name: String,"));
        assert!(out.contains("pub ratio: f64,"));
    }

    #[test]
    fn root_array_alias() {
        let out = json_to_struct(r#"[{"id":1}]"#.into(), "go".into(), "Row".into()).unwrap();
        assert!(out.contains("type Row []RowItem"));
        assert!(out.contains("type RowItem struct"));
    }

    #[test]
    fn invalid_json_errors() {
        assert!(json_to_struct("{bad}".into(), "go".into(), "X".into()).is_err());
    }
}
