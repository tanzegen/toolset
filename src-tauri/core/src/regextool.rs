//! 正则测试器：匹配 + 捕获组 + 可选替换。基于 `regex`（RE2 语义，不支持反向引用/环视）。

use crate::error::{AppError, AppResult};
use regex::RegexBuilder;
use serde::Serialize;

#[derive(Serialize)]
pub struct RegexGroup {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Serialize)]
pub struct RegexMatch {
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub groups: Vec<RegexGroup>,
}

#[derive(Serialize)]
pub struct RegexResult {
    pub matches: Vec<RegexMatch>,
    pub count: usize,
    pub replaced: Option<String>,
}

const MAX_MATCHES: usize = 1000;

pub fn regex_test(
    pattern: String,
    flags: String,
    text: String,
    replacement: Option<String>,
) -> AppResult<RegexResult> {
    if pattern.is_empty() {
        return Err(AppError::Empty);
    }
    let mut b = RegexBuilder::new(&pattern);
    b.case_insensitive(flags.contains('i'));
    b.multi_line(flags.contains('m'));
    b.dot_matches_new_line(flags.contains('s'));
    b.ignore_whitespace(flags.contains('x'));
    b.swap_greed(flags.contains('U'));
    let re = b
        .build()
        .map_err(|e| AppError::Invalid(format!("正则无效: {e}")))?;

    // 组名（含未命名组用序号）。索引 0 是整体匹配，跳过。
    let names: Vec<Option<String>> = re
        .capture_names()
        .map(|o| o.map(|s| s.to_string()))
        .collect();

    let mut matches = Vec::new();
    for (i, caps) in re.captures_iter(&text).enumerate() {
        if i >= MAX_MATCHES {
            break;
        }
        let m0 = caps.get(0).unwrap();
        let mut groups = Vec::new();
        for gi in 1..names.len() {
            let label = names[gi]
                .clone()
                .unwrap_or_else(|| gi.to_string());
            groups.push(RegexGroup {
                name: label,
                value: caps.get(gi).map(|m| m.as_str().to_string()),
            });
        }
        matches.push(RegexMatch {
            index: i,
            start: m0.start(),
            end: m0.end(),
            text: m0.as_str().to_string(),
            groups,
        });
    }

    let count = matches.len();
    let replaced = replacement.map(|r| re.replace_all(&text, r.as_str()).into_owned());
    Ok(RegexResult {
        matches,
        count,
        replaced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_and_groups() {
        let r = regex_test(
            r"(\d{4})-(\d{2})".into(),
            "".into(),
            "2023-11 and 2024-06".into(),
            None,
        )
        .unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(r.matches[0].text, "2023-11");
        assert_eq!(r.matches[0].groups[0].value.as_deref(), Some("2023"));
        assert_eq!(r.matches[0].groups[1].value.as_deref(), Some("11"));
    }

    #[test]
    fn named_group() {
        let r = regex_test(r"(?<year>\d{4})".into(), "".into(), "2023".into(), None).unwrap();
        assert_eq!(r.matches[0].groups[0].name, "year");
    }

    #[test]
    fn case_insensitive_flag() {
        let r = regex_test("abc".into(), "i".into(), "ABCabc".into(), None).unwrap();
        assert_eq!(r.count, 2);
    }

    #[test]
    fn replace_mode() {
        let r = regex_test(
            r"\d+".into(),
            "".into(),
            "a1b2".into(),
            Some("#".into()),
        )
        .unwrap();
        assert_eq!(r.replaced.as_deref(), Some("a#b#"));
    }

    #[test]
    fn invalid_pattern_errors() {
        assert!(regex_test("(".into(), "".into(), "x".into(), None).is_err());
    }
}
