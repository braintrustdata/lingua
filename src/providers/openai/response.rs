/*!
OpenAI chat completion response types.

These types match the OpenAI TypeScript SDK exactly, extracted from the latest version.
All fields and nested types are preserved to ensure full API compatibility.
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

    /// Service tier used for processing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,

    /// Backend configuration fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,

    /// Token usage statistics
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

    /// Chat completion message
    pub message: ChatCompletionMessage,
}

/// Log probability information for a choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceLogprobs {
    /// Content token log probabilities
    pub content: Option<Vec<ChatCompletionTokenLogprob>>,

    /// Refusal token log probabilities
    pub refusal: Option<Vec<ChatCompletionTokenLogprob>>,
}

/// Token log probability information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionTokenLogprob {
    /// The token
    pub token: String,

    /// UTF-8 bytes representation
    pub bytes: Option<Vec<u32>>,

    /// Log probability of this token
    pub logprob: f64,

    /// Most likely alternative tokens
    pub top_logprobs: Vec<TokenLogprob>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenLogprob {
    /// The token
    pub token: String,

    /// UTF-8 bytes representation
    pub bytes: Option<Vec<u32>>,

    /// Log probability of this token
    pub logprob: f64,
}

/// Chat completion message from the model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    /// Message content
    pub content: Option<String>,

    /// Refusal message
    pub refusal: Option<String>,

    /// Message role (always "assistant")
    pub role: String,

    /// Message annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<MessageAnnotation>>,

    /// Audio response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionAudioResponse>,

    /// Deprecated: use tool_calls instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallResponse>,

    /// Tool calls made by the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<MessageToolCall>>,
}

/// Message annotations for web search results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageAnnotation {
    #[serde(rename = "url_citation")]
    UrlCitation { url_citation: UrlCitation },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlCitation {
    /// End index of citation in message
    pub end_index: u32,

    /// Start index of citation in message
    pub start_index: u32,

    /// Title of web resource
    pub title: String,

    /// URL of web resource
    pub url: String,
}

/// Audio response data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAudioResponse {
    /// Audio response identifier
    pub id: String,

    /// Base64 encoded audio data
    pub data: String,

    /// Expiration timestamp
    pub expires_at: u64,

    /// Audio transcript
    pub transcript: String,
}

/// Function call response (deprecated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCallResponse {
    /// Function arguments as JSON string
    pub arguments: String,

    /// Function name
    pub name: String,
}

/// Tool call made by the model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageToolCall {
    #[serde(rename = "function")]
    Function {
        id: String,
        function: FunctionCallResponse,
    },
    #[serde(rename = "custom")]
    Custom {
        id: String,
        custom: CustomToolCallResponse,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomToolCallResponse {
    /// Custom tool input
    pub input: String,

    /// Custom tool name
    pub name: String,
}

/// Streamed chat completion chunk
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier (same across all chunks)
    pub id: String,

    /// List of completion choices
    pub choices: Vec<ChatCompletionChunkChoice>,

    /// Unix timestamp of creation
    pub created: u64,

    /// Model identifier
    pub model: String,

    /// Object type (always "chat.completion.chunk")
    pub object: String,

    /// Service tier used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,

    /// System fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,

    /// Token usage (only in final chunk if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<CompletionUsage>,
}

/// Streaming completion choice delta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionChunkChoice {
    /// Delta containing incremental changes
    pub delta: ChunkDelta,

    /// Finish reason (null until completion)
    pub finish_reason: Option<FinishReason>,

    /// Choice index
    pub index: u32,

    /// Log probability information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChoiceLogprobs>,
}

/// Delta for streaming updates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkDelta {
    /// Content delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Deprecated: use tool_calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallDelta>,

    /// Refusal delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,

    /// Role (only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Tool call deltas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    /// Function arguments delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,

    /// Function name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Tool call index
    pub index: u32,

    /// Tool call ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Function delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCallDelta>,

    /// Tool type
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub tool_type: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    /// Accepted prediction tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_prediction_tokens: Option<u32>,

    /// Audio output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u32>,

    /// Reasoning tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,

    /// Rejected prediction tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_prediction_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    /// Audio input tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u32>,

    /// Cached tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

/// Chat completion deletion response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionDeleted {
    /// ID of deleted completion
    pub id: String,

    /// Whether deletion succeeded
    pub deleted: bool,

    /// Object type
    pub object: String,
}

/// Stored chat completion message (extended version)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionStoreMessage {
    /// Message identifier
    pub id: String,

    /// Message content
    pub content: Option<String>,

    /// Refusal message
    pub refusal: Option<String>,

    /// Message role
    pub role: String,

    /// Message annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<MessageAnnotation>>,

    /// Audio response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionAudioResponse>,

    /// Content parts array
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_parts: Option<Vec<ContentPart>>,

    /// Deprecated function call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallResponse>,

    /// Tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<MessageToolCall>>,
}

/// Content part for stored messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlData },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageUrlData {
    /// Image URL
    pub url: String,

    /// Detail level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Enumeration types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}
