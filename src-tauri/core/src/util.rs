//! 跨工具共享的小工具：时区解析、相对时间。

use crate::error::{AppError, AppResult};
use chrono_tz::Tz;

pub fn parse_tz(tz: &str) -> AppResult<Tz> {
    let t = tz.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("utc") {
        return Ok(Tz::UTC);
    }
    t.parse::<Tz>()
        .map_err(|_| AppError::Invalid(format!("未知时区: {tz}")))
}

/// 中文相对时间。delta = 目标时刻 - 现在（秒）。
pub fn humanize(delta_secs: i64) -> String {
    if delta_secs == 0 {
        return "刚刚".to_string();
    }
    let s = delta_secs.abs();
    let (val, unit) = if s < 60 {
        (s, "秒")
    } else if s < 3600 {
        (s / 60, "分钟")
    } else if s < 86_400 {
        (s / 3600, "小时")
    } else if s < 2_592_000 {
        (s / 86_400, "天")
    } else if s < 31_536_000 {
        (s / 2_592_000, "个月")
    } else {
        (s / 31_536_000, "年")
    };
    if delta_secs > 0 {
        format!("{val}{unit}后")
    } else {
        format!("{val}{unit}前")
    }
}
