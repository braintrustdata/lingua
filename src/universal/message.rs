use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Message role in a conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub enum MessageRole {
    /// System instruction that guides model behavior
    System,
    /// User input message
    User,
    /// Model response message  
    Assistant,
    /// Tool call result or tool invocation
    Tool,
}

/// Content type within a message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub enum ContentType {
    /// Plain text content
    Text,
    /// Image content (base64 or URL)
    Image,
    /// Audio content
    Audio,
    /// Video content
    Video,
    /// Document/file content
    Document,
    /// Tool call invocation
    ToolCall,
    /// Tool execution result
    ToolResult,
    /// Thinking/reasoning content (visible reasoning)
    Thinking,
}

/// A piece of content within a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct ContentBlock {
    /// Type of this content block
    pub content_type: ContentType,
    /// The actual content data (format depends on content_type)
    pub data: String,
    /// Optional metadata for this content block
    #[ts(optional)]
    #[ts(type = "Record<string, any>")]
    pub metadata: Option<serde_json::Value>,
}

/// Metadata associated with a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct MessageMetadata {
    /// Unique identifier for this message
    #[ts(optional)]
    pub id: Option<String>,
    /// Timestamp when message was created
    #[ts(optional)]
    pub timestamp: Option<i64>,
    /// Token usage information
    #[ts(optional)]
    pub usage: Option<crate::universal::TokenUsage>,
    /// Cost information
    #[ts(optional)]
    pub cost: Option<crate::universal::CostInfo>,
    /// Provider-specific state/continuation data
    #[ts(optional)]
    #[ts(type = "Record<string, any>")]
    pub provider_state: Option<serde_json::Value>,
    /// Reasoning context for multi-turn reasoning
    #[ts(optional)]
    pub reasoning_context: Option<String>,
    /// Cache reference information
    #[ts(optional)]
    pub cache_key: Option<String>,
}

/// A complete message in a conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content blocks that make up this message
    pub content: Vec<ContentBlock>,
    /// Optional metadata for this message
    #[ts(optional)]
    pub metadata: Option<MessageMetadata>,
}

impl Message {
    /// Create a simple text message
    pub fn text(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentBlock {
                content_type: ContentType::Text,
                data: content.into(),
                metadata: None,
            }],
            metadata: None,
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::text(MessageRole::System, content)
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::text(MessageRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::text(MessageRole::Assistant, content)
    }
}