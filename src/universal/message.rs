use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Universal message prompt - collection of model messages
pub type ModelPrompt = Vec<ModelMessage>;

/// Legacy alias for backwards compatibility
#[deprecated(note = "Use ModelPrompt instead")]
pub type LanguageModelV2Prompt = Vec<ModelMessage>;

/// User content that can be either string or array (matching AI SDK ModelMessage)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(untagged)]
pub enum UserContent {
    String(String),
    Array(Vec<UserContentPart>),
}

/// Assistant content that can be either string or array (matching AI SDK ModelMessage)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(untagged)]
pub enum AssistantContent {
    String(String),
    Array(Vec<AssistantContentPart>),
}

/// Universal ModelMessage from AI SDK - user-facing format
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
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

/// Legacy aliases for backwards compatibility
#[deprecated(note = "Use ModelMessage instead")]
pub type LanguageModelV2Message = ModelMessage;

#[deprecated(note = "Use UserContent instead")]
pub type UserContentValue = UserContent;

#[deprecated(note = "Use AssistantContent instead")]
pub type AssistantContentValue = AssistantContent;

/// Text content part of a message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
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
#[ts(export, export_to = "../../bindings/typescript/")]
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
#[ts(export, export_to = "../../bindings/typescript/")]
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
#[ts(export, export_to = "../../bindings/typescript/")]
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
#[ts(export, export_to = "../../bindings/typescript/")]
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
#[ts(export, export_to = "../../bindings/typescript/")]
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

/// Legacy content union type - deprecated
#[deprecated(note = "Use individual content part structs instead")]
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
        provider_metadata: Option<ProviderMetadata>,
    },
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
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
        provider_metadata: Option<ProviderMetadata>,
    },
    Source {
        #[serde(rename = "sourceType")]
        #[ts(rename = "sourceType")]
        source_type: SourceType,
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
        #[ts(optional)]
        #[serde(rename = "isError")]
        #[ts(rename = "isError")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// User content parts - text, image, and file parts allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(untagged)]
pub enum UserContentPart {
    Text(TextPart),
    Image(ImagePart),
    File(FilePart),
}

/// Assistant content parts - text, file, reasoning, tool calls, and tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
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

/// Legacy user content - deprecated
#[deprecated(note = "Use UserContentPart instead")]
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
        provider_metadata: Option<ProviderMetadata>,
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
        provider_metadata: Option<ProviderMetadata>,
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
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Legacy assistant content - deprecated
#[deprecated(note = "Use AssistantContentPart instead")]
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
        provider_metadata: Option<ProviderMetadata>,
    },
    Reasoning {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
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
        #[ts(optional)]
        #[serde(rename = "isError")]
        #[ts(rename = "isError")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "providerMetadata")]
        #[ts(optional)]
        #[ts(rename = "providerMetadata")]
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Legacy tool content - deprecated
#[deprecated(note = "Use ToolContent type alias instead")]
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
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Source type enum
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Url,
    Document,
}

/// Legacy source type alias
#[deprecated(note = "Use SourceType instead")]
pub type LanguageModelV2SourceType = SourceType;

/// Provider options - matching AI SDK ModelMessage format
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[ts(type = "Record<string, any>")]
pub struct ProviderOptions {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub options: serde_json::Map<String, serde_json::Value>,
}

/// Provider metadata
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../bindings/typescript/")]
#[ts(type = "Record<string, any>")]
pub struct ProviderMetadata {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// Legacy provider options aliases
#[deprecated(note = "Use ProviderOptions instead")]
pub type SharedV2ProviderOptions = ProviderOptions;

#[deprecated(note = "Use ProviderMetadata instead")]
pub type SharedV2ProviderMetadata = ProviderMetadata;
