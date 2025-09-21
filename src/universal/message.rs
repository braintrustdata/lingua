use serde::{Deserialize, Serialize};
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
#[ts(export)]
#[serde(tag = "type")]
pub enum UserContentPart {
    #[serde(rename = "text")]
    Text(TextContentPart),
    #[serde(rename = "image")]
    Image {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        image: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "mediaType")]
        #[ts(optional)]
        #[ts(rename = "mediaType")]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
    #[serde(rename = "file")]
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        #[serde(rename = "mediaType")]
        #[ts(rename = "mediaType")]
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
}

/// Assistant content parts - text, file, reasoning, tool calls, and tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum AssistantContentPart {
    #[serde(rename = "text")]
    Text(TextContentPart),
    #[serde(rename = "file")]
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        #[serde(rename = "mediaType")]
        #[ts(rename = "mediaType")]
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
    #[serde(rename = "reasoning")]
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
    #[serde(rename = "tool-call")]
    ToolCall {
        #[serde(rename = "toolCallId")]
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        #[ts(rename = "toolName")]
        tool_name: String,
        #[ts(type = "any")]
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerExecuted")]
        #[ts(optional)]
        #[ts(rename = "providerExecuted")]
        provider_executed: Option<bool>,
    },
    #[serde(rename = "tool-result")]
    ToolResult {
        #[serde(rename = "toolCallId")]
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        #[ts(rename = "toolName")]
        tool_name: String,
        #[ts(type = "any")]
        output: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<ProviderOptions>,
    },
}

/// Tool content parts - only tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ToolContentPart {
    #[serde(rename = "tool-result")]
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
#[ts(export)]
#[serde(tag = "sourceType")]
pub enum SourceContentPart {
    #[serde(rename = "url")]
    Url {
        id: String,
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "document")]
    Document {
        id: String,
        #[serde(rename = "mediaType")]
        #[ts(rename = "mediaType")]
        media_type: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Generated file content part - matching AI SDK GeneratedFile
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GeneratedFileContentPart {
    #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
    pub file: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerMetadata")]
    #[ts(optional)]
    #[ts(rename = "providerMetadata")]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool call content part for response messages
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolCallContentPart {
    #[serde(rename = "toolCallId")]
    #[ts(rename = "toolCallId")]
    pub tool_call_id: String,
    #[serde(rename = "toolName")]
    #[ts(rename = "toolName")]
    pub tool_name: String,
    #[ts(type = "any")]
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerExecuted")]
    #[ts(optional)]
    #[ts(rename = "providerExecuted")]
    pub provider_executed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerMetadata")]
    #[ts(optional)]
    #[ts(rename = "providerMetadata")]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool result content part for response messages
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolResultResponsePart {
    #[serde(rename = "toolCallId")]
    #[ts(rename = "toolCallId")]
    pub tool_call_id: String,
    #[serde(rename = "toolName")]
    #[ts(rename = "toolName")]
    pub tool_name: String,
    #[ts(type = "any")]
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerMetadata")]
    #[ts(optional)]
    #[ts(rename = "providerMetadata")]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool error content part for response messages
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolErrorContentPart {
    #[serde(rename = "toolCallId")]
    #[ts(rename = "toolCallId")]
    pub tool_call_id: String,
    #[serde(rename = "toolName")]
    #[ts(rename = "toolName")]
    pub tool_name: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "providerMetadata")]
    #[ts(optional)]
    #[ts(rename = "providerMetadata")]
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Content part for response messages - matches AI SDK's ContentPart exactly
/// This is the output equivalent of Message content parts
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ResponseContentPart {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "reasoning")]
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "source")]
    Source {
        #[serde(rename = "sourceType")]
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
        #[serde(rename = "mediaType")]
        #[ts(optional)]
        #[ts(rename = "mediaType")]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "file")]
    File {
        #[ts(type = "any")] // GeneratedFile type from AI SDK
        file: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "tool-call")]
    ToolCall {
        #[serde(rename = "toolCallId")]
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        #[ts(rename = "toolName")]
        tool_name: String,
        #[ts(type = "any")]
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerExecuted")]
        #[ts(optional)]
        #[ts(rename = "providerExecuted")]
        provider_executed: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "tool-result")]
    ToolResult {
        #[serde(rename = "toolCallId")]
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        #[ts(rename = "toolName")]
        tool_name: String,
        #[ts(type = "any")]
        output: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "tool-error")]
    ToolError {
        #[serde(rename = "toolCallId")]
        #[ts(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        #[ts(rename = "toolName")]
        tool_name: String,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
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
    #[serde(rename = "providerOptions")]
    #[ts(optional)]
    #[ts(rename = "providerOptions")]
    pub provider_options: Option<ProviderOptions>,
}
