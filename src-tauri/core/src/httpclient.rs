//! HTTP 客户端的纯逻辑：环境变量替换、URL+query 拼接、curl 解析。无 I/O，可单测。

use crate::error::{AppError, AppResult};

/// 把文本里的 `{{key}}` 替换为变量值（按给定顺序逐个替换）。
pub fn substitute_vars(text: &str, vars: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (k, v) in vars {
        if k.is_empty() {
            continue;
        }
        let needle = ["{{", k.as_str(), "}}"].concat();
        if out.contains(&needle) {
            out = out.replace(&needle, v);
        }
    }
    out
}

/// query 组件百分号编码（保留 RFC3986 unreserved，其余编码）。
fn enc_component(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

/// 在 base URL 后追加 query 参数（已有 `?` 则用 `&` 续接）。键值做百分号编码。
pub fn build_url(base: &str, params: &[(String, String)]) -> String {
    if params.is_empty() {
        return base.to_string();
    }
    let qs = params
        .iter()
        .map(|(k, v)| format!("{}={}", enc_component(k), enc_component(v)))
        .collect::<Vec<_>>()
        .join("&");
    let sep = if base.contains('?') { "&" } else { "?" };
    format!("{base}{sep}{qs}")
}

/// 解析后的 curl 命令。
#[derive(Debug, Default, PartialEq)]
pub struct ParsedCurl {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub user: Option<String>, // -u user:pass
}

/// 把一段 curl 命令拆成 token（支持单/双引号、反斜杠续行）。
fn tokenize(input: &str) -> Vec<String> {
    let cleaned = input.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut has = false;
    for c in cleaned.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else {
                    cur.push(c);
                }
            }
            None => match c {
                '\'' | '"' => {
                    quote = Some(c);
                    has = true;
                }
                c if c.is_whitespace() => {
                    if has {
                        toks.push(std::mem::take(&mut cur));
                        has = false;
                    }
                }
                _ => {
                    cur.push(c);
                    has = true;
                }
            },
        }
    }
    if has {
        toks.push(cur);
    }
    toks
}

/// 解析常见 curl 命令（覆盖浏览器/Postman 的「Copy as cURL」主流写法）。
pub fn parse_curl(input: &str) -> AppResult<ParsedCurl> {
    let toks = tokenize(input);
    let mut r = ParsedCurl::default();
    let mut i = 0;
    if toks.first().map(|s| s.as_str()) == Some("curl") {
        i = 1;
    }
    while i < toks.len() {
        let t = toks[i].clone();
        match t.as_str() {
            "-X" | "--request" => {
                i += 1;
                if let Some(m) = toks.get(i) {
                    r.method = m.to_uppercase();
                }
            }
            "-H" | "--header" => {
                i += 1;
                if let Some(h) = toks.get(i) {
                    if let Some((k, v)) = h.split_once(':') {
                        r.headers.push((k.trim().to_string(), v.trim().to_string()));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                i += 1;
                if let Some(d) = toks.get(i) {
                    r.body.push_str(d);
                }
            }
            "-u" | "--user" => {
                i += 1;
                if let Some(u) = toks.get(i) {
                    r.user = Some(u.clone());
                }
            }
            "--url" => {
                i += 1;
                if let Some(u) = toks.get(i) {
                    r.url = u.clone();
                }
            }
            // 无参开关，忽略
            "--compressed" | "-L" | "--location" | "-k" | "--insecure" | "-s" | "--silent"
            | "-i" | "--include" | "-v" | "--verbose" | "-g" | "--globoff" => {}
            // 其它带值未知开关：粗略吃掉其值，避免被误判为 URL
            s if s.starts_with('-') => {
                if let Some(n) = toks.get(i + 1) {
                    if !n.starts_with('-') {
                        i += 1;
                    }
                }
            }
            // 位置参数 = URL
            _ => {
                if r.url.is_empty() {
                    r.url = t;
                }
            }
        }
        i += 1;
    }
    if r.url.is_empty() {
        return Err(AppError::Invalid("未识别到 URL".into()));
    }
    if r.method.is_empty() {
        r.method = if r.body.is_empty() { "GET".into() } else { "POST".into() };
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn substitute_basic_and_missing() {
        let vars = kv(&[("base", "https://api.test"), ("id", "42")]);
        assert_eq!(
            substitute_vars("{{base}}/users/{{id}}", &vars),
            "https://api.test/users/42"
        );
        // 未提供的变量原样保留
        assert_eq!(substitute_vars("{{x}}/y", &vars), "{{x}}/y");
    }

    #[test]
    fn build_url_appends_and_encodes() {
        assert_eq!(build_url("https://a.com", &[]), "https://a.com");
        assert_eq!(
            build_url("https://a.com", &kv(&[("q", "hello world"), ("k", "a&b")])),
            "https://a.com?q=hello%20world&k=a%26b"
        );
        // 已有 ? 用 & 续接
        assert_eq!(
            build_url("https://a.com?x=1", &kv(&[("y", "2")])),
            "https://a.com?x=1&y=2"
        );
    }

    #[test]
    fn parse_curl_post_with_headers_and_body() {
        let c = r#"curl -X POST 'https://api.test/x' -H 'Content-Type: application/json' -H "Authorization: Bearer t" --data-raw '{"a":1}'"#;
        let p = parse_curl(c).unwrap();
        assert_eq!(p.method, "POST");
        assert_eq!(p.url, "https://api.test/x");
        assert_eq!(p.headers.len(), 2);
        assert_eq!(p.headers[0], ("Content-Type".into(), "application/json".into()));
        assert_eq!(p.body, "{\"a\":1}");
    }

    #[test]
    fn parse_curl_bare_url_defaults_get() {
        let p = parse_curl("curl https://example.com/foo").unwrap();
        assert_eq!(p.method, "GET");
        assert_eq!(p.url, "https://example.com/foo");
        assert!(p.headers.is_empty());
    }

    #[test]
    fn parse_curl_data_defaults_post() {
        let p = parse_curl("curl https://x.com -d name=joe").unwrap();
        assert_eq!(p.method, "POST");
        assert_eq!(p.body, "name=joe");
    }
}
