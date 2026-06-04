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
}
