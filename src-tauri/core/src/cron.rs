//! Cron 表达式：字段拆解 + 预测未来 N 次运行（按时区）。

use crate::error::{AppError, AppResult};
use crate::util::{humanize, parse_tz};
use chrono::Utc;
use croner::parser::{CronParser, Seconds};
use serde::Serialize;

#[derive(Serialize)]
pub struct CronField {
    pub label: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct CronRun {
    pub local: String,
    pub utc: String,
    pub relative: String,
}

#[derive(Serialize)]
pub struct CronResult {
    pub fields: Vec<CronField>,
    pub next_runs: Vec<CronRun>,
    pub timezone: String,
}

pub fn cron_explain(expr: String, tz: String, count: u32) -> AppResult<CronResult> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(AppError::Empty);
    }
    let parts: Vec<&str> = expr.split_whitespace().collect();
    let labels: &[&str] = match parts.len() {
        5 => &["分钟", "小时", "日", "月", "星期"],
        6 => &["秒", "分钟", "小时", "日", "月", "星期"],
        _ => return Err(AppError::Invalid("cron 需为 5 或 6 个字段".to_string())),
    };
    let fields = labels
        .iter()
        .zip(parts.iter())
        .map(|(l, v)| CronField {
            label: l.to_string(),
            value: v.to_string(),
        })
        .collect();

    let cron = CronParser::builder()
        .seconds(Seconds::Optional)
        .build()
        .parse(expr)
        .map_err(|e| AppError::Invalid(format!("解析失败: {e}")))?;

    let tzv = parse_tz(&tz)?;
    let now = Utc::now();
    let n = count.clamp(1, 20) as usize;

    let mut next_runs = Vec::new();
    for t in cron.iter_after(now.with_timezone(&tzv)).take(n) {
        let utc = t.with_timezone(&Utc);
        next_runs.push(CronRun {
            local: t.format("%Y-%m-%d %H:%M:%S").to_string(),
            utc: utc.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            relative: humanize(utc.timestamp() - now.timestamp()),
        });
    }

    Ok(CronResult {
        fields,
        next_runs,
        timezone: tzv.name().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_field_parses() {
        let r = cron_explain("*/5 * * * *".into(), "UTC".into(), 3).unwrap();
        assert_eq!(r.fields.len(), 5);
        assert_eq!(r.fields[0].label, "分钟");
        assert_eq!(r.next_runs.len(), 3);
    }

    #[test]
    fn six_field_parses() {
        let r = cron_explain("0 0 12 * * *".into(), "UTC".into(), 2).unwrap();
        assert_eq!(r.fields.len(), 6);
        assert_eq!(r.fields[0].label, "秒");
    }

    #[test]
    fn bad_field_count_errors() {
        assert!(cron_explain("* * *".into(), "UTC".into(), 1).is_err());
    }

    #[test]
    fn invalid_expr_errors() {
        assert!(cron_explain("99 * * * *".into(), "UTC".into(), 1).is_err());
    }
}
