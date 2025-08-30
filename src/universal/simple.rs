use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Minimal message role for testing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub enum SimpleRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Minimal message for testing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct SimpleMessage {
    /// Role of the message
    pub role: SimpleRole,
    /// Text content
    pub content: String,
}

impl SimpleMessage {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: SimpleRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: SimpleRole::Assistant,
            content: content.into(),
        }
    }
}
