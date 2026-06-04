//! 命名风格转换：一段标识符 → 各种风格。基于 heck。

use crate::error::{AppError, AppResult};
use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase,
    ToTrainCase, ToUpperCamelCase,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct CaseResult {
    pub lower_camel: String,
    pub upper_camel: String,
    pub snake: String,
    pub shouty_snake: String,
    pub kebab: String,
    pub shouty_kebab: String,
    pub train: String,
    pub title: String,
}

pub fn convert_case(input: String) -> AppResult<CaseResult> {
    let s = input.trim();
    if s.is_empty() {
        return Err(AppError::Empty);
    }
    Ok(CaseResult {
        lower_camel: s.to_lower_camel_case(),
        upper_camel: s.to_upper_camel_case(),
        snake: s.to_snake_case(),
        shouty_snake: s.to_shouty_snake_case(),
        kebab: s.to_kebab_case(),
        shouty_kebab: s.to_shouty_kebab_case(),
        train: s.to_train_case(),
        title: s.to_title_case(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_snake() {
        let r = convert_case("user_profile_id".into()).unwrap();
        assert_eq!(r.lower_camel, "userProfileId");
        assert_eq!(r.upper_camel, "UserProfileId");
        assert_eq!(r.snake, "user_profile_id");
        assert_eq!(r.shouty_snake, "USER_PROFILE_ID");
        assert_eq!(r.kebab, "user-profile-id");
    }

    #[test]
    fn from_camel() {
        let r = convert_case("getHTTPResponseCode".into()).unwrap();
        assert_eq!(r.snake, "get_http_response_code");
    }

    #[test]
    fn empty_errors() {
        assert!(convert_case("   ".into()).is_err());
    }
}
