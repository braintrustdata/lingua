use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Exact port of LanguageModelV2Prompt from Vercel AI SDK
pub type LanguageModelV2Prompt = Vec<LanguageModelV2Message>;

/// Exact port of LanguageModelV2Message from Vercel AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum LanguageModelV2Message {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
    User {
        content: Vec<LanguageModelV2Content>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
    Assistant {
        content: Vec<LanguageModelV2Content>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
    Tool {
        content: Vec<LanguageModelV2Content>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerOptions")]
        provider_options: Option<SharedV2ProviderOptions>,
    },
}

/// Exact port of LanguageModelV2Content union type
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LanguageModelV2Content {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    File {
        #[ts(type = "string")] // data URL in TypeScript
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    Source {
        #[serde(rename = "sourceType")]
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
        #[serde(rename = "mediaType")]
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        media_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    #[serde(rename = "tool-call")]
    ToolCall {
        id: String,
        name: String,
        #[ts(type = "any")]
        args: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
    #[serde(rename = "tool-result")]
    ToolResult {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[ts(type = "any")]
        result: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "isError")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(rename = "providerMetadata")]
        provider_metadata: Option<SharedV2ProviderMetadata>,
    },
}

// Note: In Rust, we use a single enum for all content types instead of union types
// This is more idiomatic and provides better type safety

/// Source type enum - exact port from AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum LanguageModelV2SourceType {
    Url,
    Document,
}

/// Shared provider options - exact port from AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(type = "Record<string, any>")]
pub struct SharedV2ProviderOptions {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub options: serde_json::Map<String, serde_json::Value>,
}

/// Shared provider metadata - exact port from AI SDK
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(type = "Record<string, any>")]
pub struct SharedV2ProviderMetadata {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
