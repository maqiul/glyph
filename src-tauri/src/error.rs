//! Glyph 统一错误类型

use serde::Serialize;

/// 统一错误类型。所有跨 IPC 边界的错误都必须 `Serialize`。
#[derive(Debug, thiserror::Error)]
pub enum GlyphError {
    #[error("io: {0}")]
    Io(String),

    #[error("parse: {0}")]
    Parse(String),

    #[error("encoding: {0}")]
    Encoding(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("permission denied: {0}")]
    Permission(String),

    #[error("internal: {0}")]
    Internal(String),
}

/// 自动把 GlyphError 转成 tauri command 可序列化的错误响应
impl Serialize for GlyphError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for GlyphError {
    fn from(e: std::io::Error) -> Self {
        GlyphError::Io(e.to_string())
    }
}