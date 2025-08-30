/*!
OpenAI chat completion request types.

These types match the OpenAI TypeScript SDK v5.16.0 exactly.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main chat completion request parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionCreateParams {
    /// The conversation messages
    pub messages: Vec<ChatCompletionMessageParam>,
    
    /// Model identifier (e.g., "gpt-4o", "gpt-3.5-turbo")
    pub model: String,
    
    /// Audio configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionAudioParam>,
    
    /// Frequency penalty between -2.0 and 2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    
    /// Logit bias map
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f64>>,
    
    /// Whether to return log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    
    /// Maximum number of completion tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    
    /// Maximum tokens (deprecated, use max_completion_tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    /// Request metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    
    /// Output modalities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,
    
    /// Number of completions to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    
    /// Whether to enable parallel tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    
    /// Presence penalty between -2.0 and 2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    
    /// Reasoning effort for o1/o3 models
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    
    /// Response format specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    
    /// Random seed for deterministic outputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    
    /// Service tier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequences>,
    
    /// Whether to stream responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    
    /// Sampling temperature between 0 and 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    
    /// Tool choice configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,
    
    /// Available tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,
    
    /// Nucleus sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    
    /// User identifier (deprecated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Chat completion message parameter (union type)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatCompletionMessageParam {
    #[serde(rename = "system")]
    System {
        content: MessageContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "user")]
    User {
        content: MessageContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<MessageContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        audio: Option<AudioReference>,
        #[serde(skip_serializing_if = "Option::is_none")]
        function_call: Option<FunctionCall>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refusal: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,
    },
    #[serde(rename = "tool")]
    Tool {
        content: String,
        tool_call_id: String,
    },
    #[serde(rename = "function")]
    Function {
        content: String,
        name: String,
    },
    #[serde(rename = "developer")]
    Developer {
        content: MessageContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

/// Message content (can be string or array of content parts)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ChatCompletionContentPart>),
}

/// Content part types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionContentPart {
    Text {
        text: String,
    },
    ImageUrl {
        image_url: ImageUrl,
    },
    InputAudio {
        input_audio: InputAudio,
    },
    #[serde(rename = "file")]
    File {
        #[serde(rename = "file")]
        file_data: FileData,
    },
    Refusal {
        refusal: String,
    },
}

/// Image URL configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>, // "auto" | "low" | "high"
}

/// Input audio configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputAudio {
    pub data: String, // Base64 encoded
    pub format: String, // "wav" | "mp3"
}

/// File data reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileData {
    pub id: String,
}

/// Audio reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioReference {
    pub id: String,
}

/// Audio parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAudioParam {
    pub voice: String,
    pub format: String,
}

/// Function call (deprecated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub arguments: String,
    pub name: String,
}

/// Tool call in messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionMessageToolCall {
    Function {
        id: String,
        function: FunctionCall,
    },
    #[serde(rename = "custom")]
    Custom {
        id: String,
        custom: serde_json::Value,
    },
}

/// Response format options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema {
        json_schema: JsonSchema,
    },
}

/// JSON Schema definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchema {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Service tier options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}

/// Stop sequences
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopSequences {
    Single(String),
    Multiple(Vec<String>),
}

/// Tool choice options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionToolChoiceOption {
    Auto(String), // "auto" | "none" | "required"
    Named {
        #[serde(rename = "type")]
        tool_type: String,
        function: NamedToolChoice,
    },
}

/// Named tool choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedToolChoice {
    pub name: String,
}

/// Chat completion tool definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionTool {
    Function {
        function: FunctionTool,
    },
    #[serde(rename = "custom")]
    Custom {
        custom: CustomTool,
    },
}

/// Function tool definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>, // JSON Schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Custom tool definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<serde_json::Value>,
}