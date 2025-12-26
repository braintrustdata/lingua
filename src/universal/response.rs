/*!
Universal response types for cross-provider transformation.

This module provides a canonical representation of LLM responses that can be
converted to/from any provider format.
*/

use crate::serde_json::{Map, Value};
use crate::universal::message::Message;

/// Universal response envelope for LLM API responses.
///
/// This type captures the common structure across all provider response formats.
#[derive(Debug, Clone)]
pub struct UniversalResponse {
    /// Model that generated the response
    pub model: Option<String>,

    /// Response messages (may be multiple for multi-choice responses)
    pub messages: Vec<Message>,

    /// Token usage statistics
    pub usage: Option<UniversalUsage>,

    /// Why the model stopped generating
    pub finish_reason: Option<FinishReason>,

    /// Provider-specific response fields
    pub extras: Map<String, Value>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default)]
pub struct UniversalUsage {
    /// Tokens in the prompt/input
    pub prompt_tokens: Option<i64>,

    /// Tokens in the completion/output
    pub completion_tokens: Option<i64>,

    /// Cached tokens in the prompt (from prompt caching)
    pub prompt_cached_tokens: Option<i64>,

    /// Tokens written to cache during this request
    pub prompt_cache_creation_tokens: Option<i64>,
}

/// Reason why the model stopped generating.
///
/// Normalized across provider-specific values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    /// Normal completion (OpenAI: "stop", Anthropic: "end_turn", Google: "STOP")
    Stop,

    /// Hit token limit (OpenAI: "length", Anthropic: "max_tokens")
    Length,

    /// Model wants to call tools (OpenAI: "tool_calls", Anthropic: "tool_use")
    ToolCalls,

    /// Content was filtered
    ContentFilter,

    /// Provider-specific reason not in the canonical set
    Other(String),
}

impl std::str::FromStr for FinishReason {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "stop" | "end_turn" | "completed" => FinishReason::Stop,
            "length" | "max_tokens" | "max_output_tokens" => FinishReason::Length,
            "tool_calls" | "tool_use" => FinishReason::ToolCalls,
            "content_filter" => FinishReason::ContentFilter,
            _ => FinishReason::Other(s.to_string()),
        })
    }
}
