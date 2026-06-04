//! 简繁转换：基于 zhconv（MediaWiki + OpenCC 规则，词组级），把文本转换到目标变体。

use crate::error::{AppError, AppResult};
use zhconv::{zhconv, Variant};

/// target: hans(简体) | hant(繁体) | tw(台湾正体) | hk(香港繁体) | cn(大陆简体)
pub fn zh_convert(text: String, target: String) -> AppResult<String> {
    if text.is_empty() {
        return Err(AppError::Empty);
    }
    let variant = match target.as_str() {
        "hans" => Variant::ZhHans,
        "hant" => Variant::ZhHant,
        "tw" => Variant::ZhTW,
        "hk" => Variant::ZhHK,
        "cn" => Variant::ZhCN,
        other => return Err(AppError::Invalid(format!("未知目标变体: {other}"))),
    };
    Ok(zhconv(&text, variant))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simp_to_trad() {
        assert_eq!(zh_convert("简体中文".into(), "hant".into()).unwrap(), "簡體中文");
    }

    #[test]
    fn trad_to_simp() {
        assert_eq!(zh_convert("簡體中文".into(), "hans".into()).unwrap(), "简体中文");
    }

    #[test]
    fn phrase_level() {
        // 词组级：电脑 → 電腦（而非逐字硬转）
        assert!(zh_convert("电脑".into(), "hant".into()).unwrap().contains("電腦"));
        assert!(zh_convert("電腦".into(), "hans".into()).unwrap().contains("电脑"));
    }

    #[test]
    fn empty_errors() {
        assert!(zh_convert(String::new(), "hant".into()).is_err());
    }

    #[test]
    fn unknown_target_errors() {
        assert!(zh_convert("x".into(), "xx".into()).is_err());
    }
}
