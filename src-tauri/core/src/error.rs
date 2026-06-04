//! 统一错误类型：核心层统一返回 `AppResult<T>`，错误以友好字符串序列化给前端。

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("输入为空")]
    Empty,
    #[error("{0}")]
    Invalid(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
