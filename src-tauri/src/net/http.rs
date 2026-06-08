//! HTTP 单次请求：reqwest(rustls)。绕过 webview 的 CORS、可设任意方法/请求头、看原始响应。
//! 变量替换与 URL+query 拼接复用 `toolset_core::httpclient`（纯函数、可单测）。

use std::time::{Duration, Instant};

use reqwest::redirect::Policy;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use toolset_core::error::{AppError, AppResult};
use toolset_core::httpclient::{build_url, parse_curl, substitute_vars};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kv {
    key: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpReq {
    method: String,
    url: String,
    #[serde(default)]
    params: Vec<Kv>,
    #[serde(default)]
    headers: Vec<Kv>,
    #[serde(default)]
    body_type: String, // none|json|raw|form
    #[serde(default)]
    body: String,
    #[serde(default)]
    form: Vec<Kv>,
    #[serde(default)]
    vars: Vec<Kv>, // 启用中的环境变量
    #[serde(default)]
    follow_redirects: bool,
    #[serde(default)]
    timeout_ms: u64,
    #[serde(default)]
    skip_tls_verify: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KvOut {
    key: String,
    value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResp {
    status: u16,
    status_text: String,
    headers: Vec<KvOut>,
    body: String,
    size: u64,
    time_ms: u64,
    final_url: String,
}

#[tauri::command]
pub async fn http_send(req: HttpReq) -> AppResult<HttpResp> {
    let vars: Vec<(String, String)> =
        req.vars.iter().map(|k| (k.key.clone(), k.value.clone())).collect();
    let sub = |s: &str| substitute_vars(s, &vars);

    // URL + query（先替换变量再拼接；query 键值也替换）
    let base = sub(&req.url);
    let params: Vec<(String, String)> =
        req.params.iter().map(|k| (sub(&k.key), sub(&k.value))).collect();
    let url = build_url(&base, &params);

    let method = Method::from_bytes(req.method.to_uppercase().as_bytes())
        .map_err(|_| AppError::Invalid(format!("非法请求方法：{}", req.method)))?;

    let timeout = if req.timeout_ms == 0 { 30_000 } else { req.timeout_ms };
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(req.skip_tls_verify)
        .redirect(if req.follow_redirects {
            Policy::limited(10)
        } else {
            Policy::none()
        })
        .timeout(Duration::from_millis(timeout))
        .build()
        .map_err(|e| AppError::Invalid(format!("HTTP 客户端构建失败：{e}")))?;

    let mut rb = client.request(method, &url);

    let mut has_ct = false;
    for h in &req.headers {
        let k = sub(&h.key);
        if k.is_empty() {
            continue;
        }
        if k.eq_ignore_ascii_case("content-type") {
            has_ct = true;
        }
        rb = rb.header(&k, sub(&h.value));
    }

    match req.body_type.as_str() {
        "json" => {
            if !has_ct {
                rb = rb.header("content-type", "application/json");
            }
            rb = rb.body(sub(&req.body));
        }
        "raw" => {
            rb = rb.body(sub(&req.body));
        }
        "form" => {
            if !has_ct {
                rb = rb.header("content-type", "application/x-www-form-urlencoded");
            }
            let enc = req
                .form
                .iter()
                .map(|k| format!("{}={}", urlencode(&sub(&k.key)), urlencode(&sub(&k.value))))
                .collect::<Vec<_>>()
                .join("&");
            rb = rb.body(enc);
        }
        _ => {}
    }

    let start = Instant::now();
    let resp = rb
        .send()
        .await
        .map_err(|e| AppError::Invalid(format!("请求失败：{e}")))?;
    let status = resp.status();
    let final_url = resp.url().to_string();
    let headers: Vec<KvOut> = resp
        .headers()
        .iter()
        .map(|(k, v)| KvOut {
            key: k.to_string(),
            value: v.to_str().unwrap_or("").to_string(),
        })
        .collect();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::Invalid(format!("读取响应失败：{e}")))?;
    let time_ms = start.elapsed().as_millis() as u64;

    Ok(HttpResp {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers,
        size: bytes.len() as u64,
        body: String::from_utf8_lossy(&bytes).to_string(),
        time_ms,
        final_url,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurlParsed {
    method: String,
    url: String,
    headers: Vec<KvOut>,
    body: String,
    user: Option<String>,
}

/// 解析 curl 命令为结构化请求（前端「导入 curl」用）。
#[tauri::command]
pub fn curl_parse(text: String) -> AppResult<CurlParsed> {
    let p = parse_curl(&text)?;
    Ok(CurlParsed {
        method: p.method,
        url: p.url,
        headers: p
            .headers
            .into_iter()
            .map(|(k, v)| KvOut { key: k, value: v })
            .collect(),
        body: p.body,
        user: p.user,
    })
}

/// query/form 组件百分号编码。
fn urlencode(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}
