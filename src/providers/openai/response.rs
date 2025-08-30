/*!
OpenAI chat completion response types.

These types match the OpenAI TypeScript SDK v5.16.0 exactly.
*/

use serde::{Deserialize, Serialize};

/// Main chat completion response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletion {
    /// Unique identifier for the chat completion
    pub id: String,
    
    /// List of completion choices
    pub choices: Vec<ChatCompletionChoice>,
    
    /// Unix timestamp of creation
    pub created: u64,
    
    /// Model used for completion
    pub model: String,
    
    /// Object type (always "chat.completion")
    pub object: String,
    
    /// Service tier used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    
    /// System fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<CompletionUsage>,
}

/// Individual completion choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    /// Reason why completion finished
    pub finish_reason: FinishReason,
    
    /// Index of this choice
    pub index: u32,
    
    /// Log probability information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChoiceLogprobs>,
    
    /// The completion message
    pub message: ChatCompletionMessage,
}

/// Finish reason options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
}

/// Log probabilities for a choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceLogprobs {
    /// Content token log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ChatCompletionTokenLogprob>>,
    
    /// Refusal token log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Vec<ChatCompletionTokenLogprob>>,
}

/// Token log probability information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionTokenLogprob {
    /// The token string
    pub token: String,
    
    /// Token bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
    
    /// Log probability of the token
    pub logprob: f64,
    
    /// Top alternative tokens with their log probabilities
    pub top_logprobs: Vec<TopLogprob>,
}

/// Top log probability alternative
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogprob {
    /// The alternative token
    pub token: String,
    
    /// Token bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
    
    /// Log probability of this alternative
    pub logprob: f64,
}

/// Chat completion message in response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    /// Message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    
    /// Refusal content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    
    /// Message role (always "assistant" in responses)
    pub role: String,
    
    /// Content annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<MessageAnnotation>>,
    
    /// Audio output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionAudio>,
    
    /// Function call (deprecated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,
}

/// Message annotation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageAnnotation {
    UrlCitation {
        url_citation: UrlCitation,
    },
}

/// URL citation annotation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlCitation {
    pub end_index: u32,
    pub start_index: u32,
    pub title: String,
    pub url: String,
}

/// Audio output in completion
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAudio {
    pub id: String,
    pub expires_at: u64,
    pub data: String, // Base64 encoded
    pub transcript: String,
}

/// Function call (deprecated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function arguments as JSON string
    pub arguments: String,
    
    /// Function name
    pub name: String,
}

/// Tool call made by assistant
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

/// Token usage statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionUsage {
    /// Number of tokens in completion
    pub completion_tokens: u32,
    
    /// Number of tokens in prompt
    pub prompt_tokens: u32,
    
    /// Total tokens used
    pub total_tokens: u32,
    
    /// Detailed completion token breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    
    /// Detailed prompt token breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

/// Detailed completion token usage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    /// Accepted prediction tokens (for caching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_prediction_tokens: Option<u32>,
    
    /// Audio output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u32>,
    
    /// Reasoning tokens (for o1/o3 models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
    
    /// Rejected prediction tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_prediction_tokens: Option<u32>,
}

/// Detailed prompt token usage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    /// Audio input tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u32>,
    
    /// Cached tokens (for context caching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

// Streaming response types

/// Chat completion chunk (streaming response)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for the completion
    pub id: String,
    
    /// List of delta choices
    pub choices: Vec<ChatCompletionChunkChoice>,
    
    /// Unix timestamp of creation
    pub created: u64,
    
    /// Model used
    pub model: String,
    
    /// Object type (always "chat.completion.chunk")
    pub object: String,
    
    /// Service tier used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    
    /// System fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    
    /// Token usage (only in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<CompletionUsage>,
}

/// Choice in streaming chunk
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionChunkChoice {
    /// Delta (incremental changes)
    pub delta: ChatCompletionChunkDelta,
    
    /// Finish reason (only in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    
    /// Choice index
    pub index: u32,
    
    /// Log probabilities for this chunk
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChoiceLogprobs>,
}

/// Delta changes in streaming chunk
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionChunkDelta {
    /// Content delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    
    /// Function call delta (deprecated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallDelta>,
    
    /// Refusal delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    
    /// Role (only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    
    /// Tool calls delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCallDelta>>,
}

/// Function call delta (deprecated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Tool call delta in streaming
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageToolCallDelta {
    pub index: u32,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub tool_type: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCallDelta>,
}