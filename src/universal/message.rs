use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Exact port of LanguageModelV2Prompt from Vercel AI SDK
pub type LanguageModelV2Prompt = Vec<LanguageModelV2Message>;

/// User content that can be either string or array (matching AI SDK beta)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(untagged)]
pub enum UserContentValue {
    String(String),
    Array(Vec<LanguageModelV2UserContent>),
}

/// Assistant content that can be either string or array (matching AI SDK beta)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(untagged)]
pub enum AssistantContentValue {
    String(String),
    Array(Vec<LanguageModelV2AssistantContent>),
}

/// Exact port of LanguageModelV2Message from Vercel AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum LanguageModelV2Message {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
    User {
        content: UserContentValue,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
    Assistant {
        content: AssistantContentValue,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
    Tool {
        content: Vec<LanguageModelV2ToolContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerOptions")]
        #[ts(optional)]
        #[ts(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
}

/// Exact port of LanguageModelV2Content union type
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LanguageModelV2Content {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[serde(rename = "mediaType")]
        #[ts(rename = "mediaType")]
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    Source {
        #[serde(rename = "sourceType")]
        #[ts(rename = "sourceType")]
        source_type: LanguageModelV2SourceType,
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "mediaType")]
        #[ts(rename = "mediaType")]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
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
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
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
        #[ts(optional)]
        #[serde(rename = "isError")]
        #[ts(rename = "isError")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
}

/// Role-specific content types matching AI SDK exactly

/// User content - only text and file parts allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LanguageModelV2UserContent {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    Image {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        image: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "mediaType")]
        #[ts(rename = "mediaType")]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[serde(rename = "mediaType")]
        #[ts(rename = "mediaType")]
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
}

/// Assistant content - text, file, reasoning, tool calls, and tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LanguageModelV2AssistantContent {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
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
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
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
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
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
        #[ts(optional)]
        #[serde(rename = "isError")]
        #[ts(rename = "isError")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
}

/// Tool content - only tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LanguageModelV2ToolContent {
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
        #[ts(optional)]
        #[serde(rename = "isError")]
        #[ts(rename = "isError")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
}

/// Source type enum - exact port from AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(rename_all = "lowercase")]
pub enum LanguageModelV2SourceType {
    Url,
    Document,
}

/// Shared provider options - exact port from AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[ts(type = "Record<string, any>")]
pub struct SharedV2ProviderOptions {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub options: serde_json::Map<String, serde_json::Value>,
}

/// Shared provider metadata - exact port from AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[ts(type = "Record<string, any>")]
pub struct SharedV2ProviderMetadata {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
