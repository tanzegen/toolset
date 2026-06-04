//! Tauri 接入层：把 toolset-core 的纯函数包成 #[tauri::command] 并注册。
//! 所有计算逻辑与单测都在 toolset-core，本层只做参数透传与命令注册。

use toolset_core::cron::CronResult;
use toolset_core::error::AppResult;
use toolset_core::hashing::HashResult;
use toolset_core::json::JsonValidateResult;
use toolset_core::localip::LocalIpResult;
use toolset_core::naming::CaseResult;
use toolset_core::numeric::{BaseResult, FloatResult};
use toolset_core::regextool::RegexResult;
use toolset_core::subnet::SubnetResult;
use toolset_core::textdiff::DiffResult;
use toolset_core::timestamp::TimestampResult;
use toolset_core::{
    cron, encoding, hashing, json, jsonstruct, localip, naming, numeric, regextool, subnet,
    textdiff, timestamp,
};

#[tauri::command]
fn timestamp_convert(input: String, unit: String, tz: String) -> AppResult<TimestampResult> {
    timestamp::timestamp_convert(input, unit, tz)
}

#[tauri::command]
fn timestamp_from_datetime(datetime: String, tz: String) -> AppResult<TimestampResult> {
    timestamp::timestamp_from_datetime(datetime, tz)
}

#[tauri::command]
fn now_millis() -> String {
    timestamp::now_millis()
}

#[tauri::command]
fn list_timezones() -> Vec<String> {
    timestamp::list_timezones()
}

#[tauri::command]
fn hex_to_float(hex: String, width: String) -> AppResult<FloatResult> {
    numeric::hex_to_float(hex, width)
}

#[tauri::command]
fn float_to_hex(value: String, width: String) -> AppResult<FloatResult> {
    numeric::float_to_hex(value, width)
}

#[tauri::command]
fn base_convert(input: String, from_base: u32, bit_width: u32) -> AppResult<BaseResult> {
    numeric::base_convert(input, from_base, bit_width)
}

#[tauri::command]
fn base64_encode(input: String, url_safe: bool) -> String {
    encoding::base64_encode(input, url_safe)
}

#[tauri::command]
fn base64_decode(input: String, url_safe: bool) -> AppResult<String> {
    encoding::base64_decode(input, url_safe)
}

#[tauri::command]
fn url_encode(input: String) -> String {
    encoding::url_encode(input)
}

#[tauri::command]
fn url_decode(input: String) -> AppResult<String> {
    encoding::url_decode(input)
}

#[tauri::command]
fn json_format(input: String, indent: u8) -> AppResult<String> {
    json::json_format(input, indent)
}

#[tauri::command]
fn json_minify(input: String) -> AppResult<String> {
    json::json_minify(input)
}

#[tauri::command]
fn json_validate(input: String) -> JsonValidateResult {
    json::json_validate(input)
}

#[tauri::command]
fn hash_text(input: String) -> HashResult {
    hashing::hash_text(input)
}

#[tauri::command]
fn uuid_v4(count: u32) -> Vec<String> {
    hashing::uuid_v4(count)
}

#[tauri::command]
fn cron_explain(expr: String, tz: String, count: u32) -> AppResult<CronResult> {
    cron::cron_explain(expr, tz, count)
}

#[tauri::command]
fn regex_test(
    pattern: String,
    flags: String,
    text: String,
    replacement: Option<String>,
) -> AppResult<RegexResult> {
    regextool::regex_test(pattern, flags, text, replacement)
}

#[tauri::command]
fn subnet_calc(input: String) -> AppResult<SubnetResult> {
    subnet::subnet_calc(input)
}

#[tauri::command]
fn json_to_struct(json: String, lang: String, root_name: String) -> AppResult<String> {
    jsonstruct::json_to_struct(json, lang, root_name)
}

#[tauri::command]
fn convert_case(input: String) -> AppResult<CaseResult> {
    naming::convert_case(input)
}

#[tauri::command]
fn text_diff(left: String, right: String) -> AppResult<DiffResult> {
    textdiff::text_diff(left, right)
}

#[tauri::command]
fn local_ips() -> AppResult<LocalIpResult> {
    localip::local_ips()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 降低 WebView2 内存占用：单渲染进程、关闭独立 GPU 进程、裁剪 Edge 专有特性。
    // WebView2 运行时会读取该环境变量，需在创建 webview 前设置。
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--disable-gpu --renderer-process-limit=1 --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            timestamp_convert,
            timestamp_from_datetime,
            now_millis,
            list_timezones,
            hex_to_float,
            float_to_hex,
            base_convert,
            base64_encode,
            base64_decode,
            url_encode,
            url_decode,
            json_format,
            json_minify,
            json_validate,
            hash_text,
            uuid_v4,
            cron_explain,
            regex_test,
            subnet_calc,
            json_to_struct,
            convert_case,
            text_diff,
            local_ips,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
