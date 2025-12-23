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
    /// Tokens in the input/prompt
    pub input_tokens: Option<i64>,

    /// Tokens in the output/completion
    pub output_tokens: Option<i64>,
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

impl FinishReason {
    /// Parse a finish reason string into a canonical variant.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "stop" | "end_turn" | "completed" => FinishReason::Stop,
            "length" | "max_tokens" | "max_output_tokens" => FinishReason::Length,
            "tool_calls" | "tool_use" => FinishReason::ToolCalls,
            "content_filter" => FinishReason::ContentFilter,
            _ => FinishReason::Other(s.to_string()),
        }
    }
}
