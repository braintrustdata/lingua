/*!
Amazon Bedrock Converse API request types.

These types mirror the AWS Bedrock Converse API structure with full serde support
for JSON serialization and compatibility with the Elmir format system.
*/

use crate::serde_json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Main Converse API request parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct ConverseRequest {
    /// Model identifier for the foundation model
    pub model_id: String,

    /// List of messages in the conversation
    pub messages: Vec<BedrockMessage>,

    /// System instructions for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<BedrockSystemContentBlock>>,

    /// Inference configuration parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<BedrockInferenceConfiguration>,

    /// Tool configuration for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<BedrockToolConfiguration>,

    /// Guardrail configuration for content filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrail_config: Option<BedrockGuardrailConfiguration>,

    /// Additional model-specific request fields
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub additional_model_request_fields: Option<serde_json::Value>,

    /// Paths for additional model response fields to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_response_field_paths: Option<Vec<String>>,

    /// Variables for prompt templates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_variables: Option<HashMap<String, PromptVariableValues>>,
}

/// Streaming Converse API request parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct ConverseStreamRequest {
    /// Model identifier for the foundation model
    pub model_id: String,

    /// List of messages in the conversation
    pub messages: Vec<BedrockMessage>,

    /// System instructions for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<BedrockSystemContentBlock>>,

    /// Inference configuration parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<BedrockInferenceConfiguration>,

    /// Tool configuration for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<BedrockToolConfiguration>,

    /// Guardrail configuration for content filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrail_config: Option<BedrockGuardrailConfiguration>,

    /// Additional model-specific request fields
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub additional_model_request_fields: Option<serde_json::Value>,

    /// Paths for additional model response fields to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_model_response_field_paths: Option<Vec<String>>,

    /// Variables for prompt templates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_variables: Option<HashMap<String, PromptVariableValues>>,
}

/// Message in a conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockMessage {
    /// Role of the message sender
    pub role: BedrockConversationRole,

    /// Content blocks in the message
    pub content: Vec<BedrockContentBlock>,
}

/// Role in a conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
#[serde(rename_all = "lowercase")]
pub enum BedrockConversationRole {
    User,
    Assistant,
}

/// Content block types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type")]
pub enum BedrockContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image { image: BedrockImageBlock },

    #[serde(rename = "toolUse")]
    ToolUse { tool_use: BedrockToolUseBlock },

    #[serde(rename = "toolResult")]
    ToolResult { tool_result: BedrockToolResultBlock },
}

/// Image content block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockImageBlock {
    /// Image format
    pub format: BedrockImageFormat,

    /// Image source
    pub source: BedrockImageSource,
}

/// Image format
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
#[serde(rename_all = "lowercase")]
pub enum BedrockImageFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
}

/// Image source
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockImageSource {
    /// Base64 encoded image bytes
    pub bytes: String,
}

/// Tool use content block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockToolUseBlock {
    /// Unique identifier for the tool use
    pub tool_use_id: String,

    /// Name of the tool
    pub name: String,

    /// Input parameters for the tool
    #[ts(type = "any")]
    pub input: serde_json::Value,
}

/// Tool result content block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockToolResultBlock {
    /// Tool use ID this result corresponds to
    pub tool_use_id: String,

    /// Content of the tool result
    pub content: Vec<BedrockToolResultContent>,

    /// Status of the tool execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BedrockToolResultStatus>,
}

/// Tool result content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type")]
pub enum BedrockToolResultContent {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "json")]
    Json {
        #[ts(type = "any")]
        json: serde_json::Value,
    },

    #[serde(rename = "image")]
    Image { image: BedrockImageBlock },
}

/// Tool result status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
#[serde(rename_all = "lowercase")]
pub enum BedrockToolResultStatus {
    Success,
    Error,
}

/// System content block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockSystemContentBlock {
    /// Text content of the system message
    pub text: String,
}

/// Inference configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockInferenceConfiguration {
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Sampling temperature (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-p sampling parameter (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Stop sequences to end generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Tool configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockToolConfiguration {
    /// Available tools
    pub tools: Vec<BedrockTool>,

    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<BedrockToolChoice>,
}

/// Tool definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockTool {
    /// Tool specification
    pub tool_spec: BedrockToolSpec,
}

/// Tool specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockToolSpec {
    /// Tool name
    pub name: String,

    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Input schema for the tool
    pub input_schema: BedrockToolInputSchema,
}

/// Tool input schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockToolInputSchema {
    /// JSON schema for tool input
    #[ts(type = "any")]
    pub json: serde_json::Value,
}

/// Tool choice strategy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type")]
pub enum BedrockToolChoice {
    #[serde(rename = "auto")]
    Auto,

    #[serde(rename = "any")]
    Any,

    #[serde(rename = "tool")]
    Tool { name: String },
}

/// Guardrail configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct BedrockGuardrailConfiguration {
    /// Guardrail identifier
    pub guardrail_identifier: String,

    /// Guardrail version
    pub guardrail_version: String,

    /// Whether to enable trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<BedrockGuardrailTrace>,
}

/// Guardrail trace setting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
#[serde(rename_all = "lowercase")]
pub enum BedrockGuardrailTrace {
    Enabled,
    Disabled,
}

/// Values for prompt template variables
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
// #[ts(export, export_to = "bindings/typescript/")]
pub struct PromptVariableValues {
    /// Text value for the variable
    pub text: String,
}
