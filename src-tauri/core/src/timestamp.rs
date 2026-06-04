//! 时间戳工具：自动识别 秒/毫秒/微秒/纳秒，按 IANA 时区换算，支持反向（日期时间 → 时间戳）。

use crate::error::{AppError, AppResult};
use crate::util::{humanize, parse_tz};
use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Serialize;

#[derive(Serialize)]
pub struct TimestampResult {
    /// 识别/使用的单位："s" | "ms" | "us" | "ns" | "datetime"
    pub detected_unit: String,
    // 四种单位的等值时间戳（字符串避免 JS Number 精度丢失）
    pub epoch_seconds: String,
    pub epoch_millis: String,
    pub epoch_micros: String,
    pub epoch_nanos: String,
    pub utc: String,
    pub local: String,
    pub iso8601: String,
    pub rfc2822: String,
    pub relative: String,
    pub weekday: String,
    pub timezone: String,
}

/// 按 |整数部分| 的位数推断单位，锚定当前年代（约 1990–2100）。
fn detect_unit(abs_digits: usize) -> &'static str {
    match abs_digits {
        0..=11 => "s",
        12..=14 => "ms",
        15..=16 => "us",
        _ => "ns",
    }
}

fn unit_to_nanos(value: i128, unit: &str) -> AppResult<i128> {
    let mul: i128 = match unit {
        "s" => 1_000_000_000,
        "ms" => 1_000_000,
        "us" => 1_000,
        "ns" => 1,
        other => return Err(AppError::Invalid(format!("未知单位: {other}"))),
    };
    Ok(value * mul)
}

fn build_result(
    total_nanos: i128,
    unit_label: String,
    tz: Tz,
    now: DateTime<Utc>,
) -> AppResult<TimestampResult> {
    let secs = total_nanos.div_euclid(1_000_000_000) as i64;
    let sub = total_nanos.rem_euclid(1_000_000_000) as u32;
    let dt_utc = DateTime::<Utc>::from_timestamp(secs, sub)
        .ok_or_else(|| AppError::Invalid("时间戳超出可表示范围".to_string()))?;
    let dt = dt_utc.with_timezone(&tz);
    let delta = dt_utc.timestamp() - now.timestamp();

    Ok(TimestampResult {
        detected_unit: unit_label,
        epoch_seconds: (total_nanos / 1_000_000_000).to_string(),
        epoch_millis: (total_nanos / 1_000_000).to_string(),
        epoch_micros: (total_nanos / 1_000).to_string(),
        epoch_nanos: total_nanos.to_string(),
        utc: dt_utc.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string(),
        local: dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        iso8601: dt.to_rfc3339(),
        rfc2822: dt.to_rfc2822(),
        relative: humanize(delta),
        weekday: dt.format("%A").to_string(),
        timezone: tz.name().to_string(),
    })
}

pub fn timestamp_convert(input: String, unit: String, tz: String) -> AppResult<TimestampResult> {
    let s: String = input
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != ',')
        .collect();
    if s.is_empty() {
        return Err(AppError::Empty);
    }
    let value: i128 = s
        .parse()
        .map_err(|_| AppError::Invalid("不是合法的整数时间戳".to_string()))?;
    let abs_digits = s.trim_start_matches('-').len();
    let detected = if unit == "auto" || unit.is_empty() {
        detect_unit(abs_digits)
    } else {
        unit.as_str()
    };
    let total_nanos = unit_to_nanos(value, detected)?;
    let tz = parse_tz(&tz)?;
    build_result(total_nanos, detected.to_string(), tz, Utc::now())
}

fn parse_naive(s: &str) -> AppResult<NaiveDateTime> {
    let s = s.trim();
    for f in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, f) {
            return Ok(dt);
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).unwrap());
    }
    Err(AppError::Invalid(
        "无法解析日期时间，请用 YYYY-MM-DD HH:MM:SS".to_string(),
    ))
}

pub fn timestamp_from_datetime(datetime: String, tz: String) -> AppResult<TimestampResult> {
    if datetime.trim().is_empty() {
        return Err(AppError::Empty);
    }
    let tzv = parse_tz(&tz)?;
    let naive = parse_naive(&datetime)?;
    let local = match tzv.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(dt, _) => dt,
        LocalResult::None => {
            return Err(AppError::Invalid(
                "该本地时间在所选时区不存在（夏令时跳变）".to_string(),
            ))
        }
    };
    let dt_utc = local.with_timezone(&Utc);
    let total_nanos =
        dt_utc.timestamp() as i128 * 1_000_000_000 + dt_utc.timestamp_subsec_nanos() as i128;
    build_result(total_nanos, "datetime".to_string(), tzv, Utc::now())
}

pub fn now_millis() -> String {
    Utc::now().timestamp_millis().to_string()
}

pub fn list_timezones() -> Vec<String> {
    chrono_tz::TZ_VARIANTS
        .iter()
        .map(|t| t.name().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn detect_units_by_digits() {
        assert_eq!(detect_unit(10), "s");
        assert_eq!(detect_unit(13), "ms");
        assert_eq!(detect_unit(16), "us");
        assert_eq!(detect_unit(19), "ns");
    }

    #[test]
    fn seconds_to_utc() {
        let r = build_result(1_700_000_000i128 * 1_000_000_000, "s".into(), Tz::UTC, fixed_now())
            .unwrap();
        assert_eq!(r.epoch_seconds, "1700000000");
        assert_eq!(r.epoch_millis, "1700000000000");
        assert_eq!(r.epoch_micros, "1700000000000000");
        assert_eq!(r.epoch_nanos, "1700000000000000000");
        assert!(r.utc.starts_with("2023-11-14"), "got {}", r.utc);
    }

    #[test]
    fn all_units_same_instant() {
        let secs = timestamp_convert("1700000000".into(), "auto".into(), "UTC".into()).unwrap();
        let ms = timestamp_convert("1700000000000".into(), "auto".into(), "UTC".into()).unwrap();
        let us = timestamp_convert("1700000000000000".into(), "auto".into(), "UTC".into()).unwrap();
        let ns =
            timestamp_convert("1700000000000000000".into(), "auto".into(), "UTC".into()).unwrap();
        assert_eq!(secs.detected_unit, "s");
        assert_eq!(ms.detected_unit, "ms");
        assert_eq!(us.detected_unit, "us");
        assert_eq!(ns.detected_unit, "ns");
        let day = &secs.utc[..10];
        assert_eq!(&ms.utc[..10], day);
        assert_eq!(&us.utc[..10], day);
        assert_eq!(&ns.utc[..10], day);
    }

    #[test]
    fn timezone_offset_shanghai() {
        // 1700000000 = 2023-11-14 22:13:20 UTC -> +08:00 次日 06:13:20
        let r = timestamp_convert("1700000000".into(), "s".into(), "Asia/Shanghai".into()).unwrap();
        assert!(r.local.starts_with("2023-11-15 06:13:20"), "got {}", r.local);
        assert_eq!(r.timezone, "Asia/Shanghai");
    }

    #[test]
    fn reverse_datetime_roundtrip() {
        let r = timestamp_from_datetime("2023-11-15 06:13:20".into(), "Asia/Shanghai".into())
            .unwrap();
        assert_eq!(r.epoch_seconds, "1700000000");
    }

    #[test]
    fn empty_input_errors() {
        assert!(timestamp_convert("".into(), "auto".into(), "UTC".into()).is_err());
    }
}
