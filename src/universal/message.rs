use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

pub type Thread = Vec<Message>;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: String,
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

/// User content that can be either string or array (matching AI SDK Message)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum UserContent {
    String(String),
    Array(Vec<UserContentPart>),
}

/// Assistant content that can be either string or array (matching AI SDK Message)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum AssistantContent {
    String(String),
    Array(Vec<AssistantContentPart>),
}

/// Reusable tool result content part for tagged unions
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct ToolResultContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "unknown")]
    pub output: serde_json::Value,
    pub provider_options: Option<ProviderOptions>,
}

/// Reusable text content part for tagged unions
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct TextContentPart {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
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

/// Assistant content parts - text, file, reasoning, tool calls, and tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum AssistantContentPart {
    Text(TextContentPart),
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
    ToolCall {
        tool_call_id: String,
        tool_name: String,
        #[ts(type = "any")]
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_executed: Option<bool>,
    },
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        #[ts(type = "any")]
        output: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
}

/// Tool content parts - only tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ToolContentPart {
    ToolResult(ToolResultContentPart),
}

/// Tool content - array of tool content parts
pub type ToolContent = Vec<ToolContentPart>;

/// Collection of response messages with rich output metadata
pub type ResponseMessages = Vec<ResponseMessage>;

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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(tag = "sourceType")]
pub enum SourceContentPart {
    Url {
        id: String,
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_metadata: Option<ProviderMetadata>,
    },
    Document {
        id: String,
        media_type: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Generated file content part - matching AI SDK GeneratedFile
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct GeneratedFileContentPart {
    #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
    pub file: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool call content part for response messages
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase", optional_fields)]
pub struct ToolCallContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "unknown")]
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_executed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool result content part for response messages
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct ToolResultResponsePart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "any")]
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool error content part for response messages
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct ToolErrorContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Content part for response messages - matches AI SDK's ContentPart exactly
/// This is the output equivalent of Message content parts
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ResponseContentPart {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    Source {
        #[ts(rename = "sourceType")]
        source_type: SourceType,
        id: String,
        // URL source fields
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        title: Option<String>,
        // Document source fields
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "mediaType")]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    File {
        #[ts(type = "any")] // GeneratedFile type from AI SDK
        file: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    ToolCall {
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[ts(rename = "toolName")]
        tool_name: String,
        #[ts(type = "any")]
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerExecuted")]
        provider_executed: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    ToolResult {
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[ts(rename = "toolName")]
        tool_name: String,
        #[ts(type = "any")]
        output: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    ToolError {
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[ts(rename = "toolName")]
        tool_name: String,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Response message - the output equivalent of Message with rich metadata
/// This includes all the sources, reasoning, etc. that get stripped when converting to Message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResponseMessage {
    pub role: String,
    pub content: Vec<ResponseContentPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
}
