//! 编码工具：Base64（标准 / URL-safe）与 URL 百分号编解码。

use crate::error::{AppError, AppResult};
use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::Engine as _;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

/// encodeURIComponent 风格：保留 RFC 3986 unreserved（A-Za-z0-9 - _ . ~）。
const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub fn base64_encode(input: String, url_safe: bool) -> String {
    if url_safe {
        URL_SAFE.encode(input.as_bytes())
    } else {
        STANDARD.encode(input.as_bytes())
    }
}

pub fn base64_decode(input: String, url_safe: bool) -> AppResult<String> {
    let trimmed: String = input.split_whitespace().collect();
    if trimmed.is_empty() {
        return Err(AppError::Empty);
    }
    let engine = if url_safe { URL_SAFE } else { STANDARD };
    let bytes = engine
        .decode(trimmed.as_bytes())
        .map_err(|e| AppError::Invalid(format!("Base64 解码失败: {e}")))?;
    String::from_utf8(bytes).map_err(|_| AppError::Invalid("解码结果不是合法的 UTF-8 文本".to_string()))
}

pub fn url_encode(input: String) -> String {
    utf8_percent_encode(&input, COMPONENT).to_string()
}

pub fn url_decode(input: String) -> AppResult<String> {
    percent_decode_str(&input)
        .decode_utf8()
        .map(|s| s.into_owned())
        .map_err(|_| AppError::Invalid("URL 解码结果不是合法的 UTF-8 文本".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        let enc = base64_encode("Hello, 世界".into(), false);
        let dec = base64_decode(enc, false).unwrap();
        assert_eq!(dec, "Hello, 世界");
    }

    #[test]
    fn base64_known_vector() {
        assert_eq!(base64_encode("abc".into(), false), "YWJj");
    }

    #[test]
    fn url_roundtrip() {
        let enc = url_encode("a b&c=世界".into());
        assert!(!enc.contains(' '));
        assert_eq!(url_decode(enc).unwrap(), "a b&c=世界");
    }

    #[test]
    fn url_encode_keeps_unreserved() {
        assert_eq!(url_encode("a-_.~".into()), "a-_.~");
    }
}
