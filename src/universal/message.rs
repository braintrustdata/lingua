use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

pub type Thread = Vec<Message>;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: UserContent,
    },
    User {
        content: UserContent,
    },
    Assistant {
        content: AssistantContent,
        id: Option<String>,
    },
    Tool {
        content: ToolContent,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum UserContent {
    String(String),
    Array(Vec<UserContentPart>),
}

/// User content parts - text, image, and file parts allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "type")]
#[skip_serializing_none]
pub enum UserContentPart {
    Text(TextContentPart),
    Image {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        image: serde_json::Value,
        #[ts(optional)]
        media_type: Option<String>,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[ts(optional)]
        filename: Option<String>,
        media_type: String,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum AssistantContent {
    String(String),
    Array(Vec<AssistantContentPart>),
}

/// Assistant content parts - text, file, reasoning, tool calls, and tool results allowed
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum AssistantContentPart {
    Text(TextContentPart),
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[ts(optional)]
        filename: Option<String>,
        media_type: String,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
    Reasoning {
        text: String,
        /// Providers will occasionally return encrypted content for reasoning parts which can
        /// be useful when you send a follow up message.
        #[ts(optional)]
        encrypted_content: Option<String>,
    },
    ToolCall {
        tool_call_id: String,
        tool_name: String,
        arguments: ToolCallArguments,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
        #[ts(optional)]
        provider_executed: Option<bool>,
    },
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        #[ts(type = "unknown")]
        output: serde_json::Value,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum ToolCallArguments {
    Valid(#[ts(type = "Record<string, unknown>")] serde_json::Map<String, serde_json::Value>),
    Invalid(String),
}

/// Tool content - array of tool content parts
pub type ToolContent = Vec<ToolContentPart>;

/// Reusable tool result content part for tagged unions
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct ToolResultContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "any")]
    pub output: serde_json::Value,
    pub provider_options: Option<ProviderOptions>,
}

/// Reusable text content part for tagged unions
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct TextContentPart {
    pub text: String,
    pub provider_options: Option<ProviderOptions>,
}

/// Tool content parts - only tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ToolContentPart {
    ToolResult(ToolResultContentPart),
}

/// Source type enum - matches AI SDK Source sourceType
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Url,
    Document,
}

/// Provider options - matching AI SDK Message format
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

/// Source content part - matching AI SDK Source type
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "sourceType")]
pub enum SourceContentPart {
    Url {
        id: String,
        url: String,
        #[ts(optional)]
        title: Option<String>,
        #[ts(optional)]
        provider_metadata: Option<ProviderMetadata>,
    },
    Document {
        id: String,
        media_type: String,
        title: String,
        #[ts(optional)]
        filename: Option<String>,
        #[ts(optional)]
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Generated file content part - matching AI SDK GeneratedFile
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct GeneratedFileContentPart {
    #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
    pub file: serde_json::Value,
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool call content part for response messages
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct ToolCallContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "any")]
    pub input: serde_json::Value,
    pub provider_executed: Option<bool>,
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool result content part for response messages
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct ToolResultResponsePart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "any")]
    pub output: serde_json::Value,
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool error content part for response messages
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct ToolErrorContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    pub error: String,
    pub provider_metadata: Option<ProviderMetadata>,
}
