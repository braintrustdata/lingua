use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Universal message prompt - collection of model messages
pub type ModelPrompt = Vec<ModelMessage>;

/// User content that can be either string or array (matching AI SDK ModelMessage)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum UserContent {
    String(String),
    Array(Vec<UserContentPart>),
}

/// Assistant content that can be either string or array (matching AI SDK ModelMessage)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum AssistantContent {
    String(String),
    Array(Vec<AssistantContentPart>),
}

/// Universal ModelMessage from AI SDK - user-facing format
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ModelMessage {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
    User {
        content: UserContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
    Assistant {
        content: AssistantContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
    Tool {
        content: ToolContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
}

/// Text content part of a message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TextPart {
    #[serde(rename = "type")]
    #[serde(default = "text_type")]
    #[ts(type = "'text'")]
    pub r#type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerOptions")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
}

fn text_type() -> String {
    "text".to_string()
}

/// Image content part of a message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImagePart {
    #[serde(rename = "type")]
    #[serde(default = "image_type")]
    #[ts(type = "'image'")]
    pub r#type: String,
    #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
    pub image: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "mediaType")]
    #[ts(optional)]
    #[ts(rename = "mediaType")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerOptions")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
}

fn image_type() -> String {
    "image".to_string()
}

/// File content part of a message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FilePart {
    #[serde(rename = "type")]
    #[serde(default = "file_type")]
    #[ts(type = "'file'")]
    pub r#type: String,
    #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub filename: Option<String>,
    #[serde(rename = "mediaType")]
    #[ts(rename = "mediaType")]
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerOptions")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
}

fn file_type() -> String {
    "file".to_string()
}

/// Reasoning content part of a message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReasoningPart {
    #[serde(rename = "type")]
    #[serde(default = "reasoning_type")]
    #[ts(type = "'reasoning'")]
    pub r#type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerOptions")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
}

fn reasoning_type() -> String {
    "reasoning".to_string()
}

/// Tool call content part of a message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolCallPart {
    #[serde(rename = "type")]
    #[serde(default = "tool_call_type")]
    #[ts(type = "'tool-call'")]
    pub r#type: String,
    #[serde(rename = "toolCallId")]
    #[ts(rename = "toolCallId")]
    pub tool_call_id: String,
    #[serde(rename = "toolName")]
    #[ts(rename = "toolName")]
    pub tool_name: String,
    #[ts(type = "any")]
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerOptions")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerExecuted")]
    #[ts(optional)]
    #[ts(rename = "providerExecuted")]
    pub provider_executed: Option<bool>,
}

fn tool_call_type() -> String {
    "tool-call".to_string()
}

/// Tool result content part of a message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolResultPart {
    #[serde(rename = "type")]
    #[serde(default = "tool_result_type")]
    #[ts(type = "'tool-result'")]
    pub r#type: String,
    #[serde(rename = "toolCallId")]
    #[ts(rename = "toolCallId")]
    pub tool_call_id: String,
    #[serde(rename = "toolName")]
    #[ts(rename = "toolName")]
    pub tool_name: String,
    #[ts(type = "any")]
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerOptions")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
}

fn tool_result_type() -> String {
    "tool-result".to_string()
}

/// User content parts - text, image, and file parts allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum UserContentPart {
    Text(TextPart),
    Image(ImagePart),
    File(FilePart),
}

/// Assistant content parts - text, file, reasoning, tool calls, and tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum AssistantContentPart {
    Text(TextPart),
    File(FilePart),
    Reasoning(ReasoningPart),
    ToolCall(ToolCallPart),
    ToolResult(ToolResultPart),
}

/// Tool content - only tool results allowed
pub type ToolContent = Vec<ToolResultPart>;

/// Source type enum
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Url,
    Document,
}

/// Provider options - matching AI SDK ModelMessage format
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(type = "Record<string, any>")]
pub struct ProviderOptions {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub options: serde_json::Map<String, serde_json::Value>,
}

/// Provider metadata
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(type = "Record<string, any>")]
pub struct ProviderMetadata {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
