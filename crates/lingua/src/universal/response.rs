/*!
Universal response types for cross-provider transformation.

This module provides a canonical representation of LLM responses that can be
converted to/from any provider format.
*/

use crate::capabilities::ProviderFormat;
use crate::serde_json::{self, Value};
use crate::universal::defaults::PLACEHOLDER_ID;
use crate::universal::message::Message;
use serde::{Deserialize, Serialize};

/// Universal response envelope for LLM API responses.
///
/// This type captures the common structure across all provider response formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalResponse {
    /// Original response ID from the provider (e.g. "msg_abc123"), and the
    /// format it came from. Both are skipped during serialization — IDs are
    /// format-specific and not semantically comparable across providers.
    #[serde(skip_serializing)]
    pub id: Option<String>,
    #[serde(skip_serializing)]
    pub id_format: Option<ProviderFormat>,

    /// Model that generated the response
    pub model: Option<String>,

    /// Response messages (may be multiple for multi-choice responses)
    pub messages: Vec<Message>,

    /// Token usage statistics
    pub usage: Option<UniversalUsage>,

    /// Why the model stopped generating
    pub finish_reason: Option<FinishReason>,
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

    /// Reasoning/thinking tokens used in the completion.
    /// `Some(n)` only when `n > 0`; otherwise `None`.
    pub completion_reasoning_tokens: Option<i64>,
}

/// Reason why the model stopped generating.
///
/// Normalized across provider-specific values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    /// Normal completion (OpenAI: "stop", Anthropic: "end_turn", Google: "STOP")
    #[serde(alias = "stop")]
    Stop,

    /// Hit token limit (OpenAI: "length", Anthropic: "max_tokens")
    #[serde(alias = "length")]
    Length,

    /// Model wants to call tools (OpenAI: "tool_calls", Anthropic: "tool_use")
    #[serde(alias = "tool_calls")]
    ToolCalls,

    /// Content was filtered
    #[serde(alias = "content_filter")]
    ContentFilter,

    /// Provider-specific reason not in the canonical set
    Other(String),
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display as canonical (OpenAI) format strings
        let s = match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ToolCalls => "tool_calls",
            Self::ContentFilter => "content_filter",
            Self::Other(s) => s,
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for FinishReason {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "stop" | "end_turn" | "stop_sequence" | "completed" => FinishReason::Stop,
            "length" | "max_tokens" | "max_output_tokens" | "incomplete" => FinishReason::Length,
            "tool_calls" | "tool_use" => FinishReason::ToolCalls,
            "content_filter" | "content_filtered" | "safety" => FinishReason::ContentFilter,
            _ => FinishReason::Other(s.to_string()),
        })
    }
}

impl FinishReason {
    /// Parse a provider-specific finish reason string to universal FinishReason.
    ///
    /// This is the inverse of `to_provider_string()` and handles provider-specific
    /// string variants:
    /// - OpenAI Chat: "stop", "length", "tool_calls", "content_filter"
    /// - OpenAI Responses: "completed", "incomplete"
    /// - Anthropic: "end_turn", "stop_sequence", "max_tokens", "tool_use"
    /// - Bedrock: "end_turn", "stop_sequence", "max_tokens", "tool_use", "content_filtered"
    /// - Google: "STOP", "MAX_TOKENS", "TOOL_CALLS", "SAFETY", "RECITATION", "OTHER"
    pub fn from_provider_string(s: &str, provider: ProviderFormat) -> Self {
        match (s, provider) {
            // Stop variants
            (
                "end_turn" | "stop_sequence",
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic
                | ProviderFormat::Converse,
            ) => Self::Stop,
            ("STOP", ProviderFormat::Google) => Self::Stop,
            ("completed", ProviderFormat::Responses) => Self::Stop,
            ("stop", _) => Self::Stop,

            // Length variants
            (
                "max_tokens",
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic
                | ProviderFormat::Converse,
            ) => Self::Length,
            ("MAX_TOKENS", ProviderFormat::Google) => Self::Length,
            ("incomplete", ProviderFormat::Responses) => Self::Length,
            ("length", _) => Self::Length,

            // ToolCalls variants
            (
                "tool_use",
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic
                | ProviderFormat::Converse,
            ) => Self::ToolCalls,
            ("TOOL_CALLS", ProviderFormat::Google) => Self::ToolCalls,
            ("tool_calls", _) => Self::ToolCalls,

            // ContentFilter variants
            ("content_filtered", ProviderFormat::Converse) => Self::ContentFilter,
            ("SAFETY" | "RECITATION" | "OTHER", ProviderFormat::Google) => Self::ContentFilter,
            ("content_filter", _) => Self::ContentFilter,

            // Unknown - pass through
            (other, _) => Self::Other(other.to_string()),
        }
    }

    /// Convert a universal FinishReason to the provider-specific string representation.
    ///
    /// Each provider uses different strings for finish reasons:
    /// - OpenAI Chat: "stop", "length", "tool_calls", "content_filter"
    /// - OpenAI Responses: "completed", "incomplete"
    /// - Anthropic: "end_turn", "max_tokens", "tool_use"
    /// - Bedrock: "end_turn", "max_tokens", "tool_use", "content_filtered"
    /// - Google: "STOP", "MAX_TOKENS", "TOOL_CALLS", "SAFETY"
    /// - Mistral: uses OpenAI format
    pub fn to_provider_string(&self, provider: ProviderFormat) -> &str {
        match (self, provider) {
            // Stop variants
            (
                Self::Stop,
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic
                | ProviderFormat::Converse,
            ) => "end_turn",
            (Self::Stop, ProviderFormat::Google) => "STOP",
            (Self::Stop, ProviderFormat::Responses) => "completed",
            (
                Self::Stop,
                ProviderFormat::ChatCompletions
                | ProviderFormat::Mistral
                | ProviderFormat::Universal
                | ProviderFormat::Unknown,
            ) => "stop",

            // Length variants
            (
                Self::Length,
                ProviderFormat::ChatCompletions
                | ProviderFormat::Mistral
                | ProviderFormat::Universal
                | ProviderFormat::Unknown,
            ) => "length",
            (Self::Length, ProviderFormat::Responses) => "incomplete",
            (Self::Length, ProviderFormat::Google) => "MAX_TOKENS",
            (
                Self::Length,
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic
                | ProviderFormat::Converse,
            ) => "max_tokens",

            // ToolCalls variants
            (
                Self::ToolCalls,
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic
                | ProviderFormat::Converse,
            ) => "tool_use",
            (Self::ToolCalls, ProviderFormat::Google) => "STOP",
            (Self::ToolCalls, ProviderFormat::Responses) => "completed", // Tool calls also complete
            (
                Self::ToolCalls,
                ProviderFormat::ChatCompletions
                | ProviderFormat::Mistral
                | ProviderFormat::Universal
                | ProviderFormat::Unknown,
            ) => "tool_calls",

            // ContentFilter variants
            (Self::ContentFilter, ProviderFormat::Converse) => "content_filtered",
            (Self::ContentFilter, ProviderFormat::Google) => "SAFETY",
            (Self::ContentFilter, ProviderFormat::Responses) => "incomplete",
            (
                Self::ContentFilter,
                ProviderFormat::ChatCompletions
                | ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic
                | ProviderFormat::Mistral
                | ProviderFormat::Universal
                | ProviderFormat::Unknown,
            ) => "content_filter",

            // Other - pass through as-is
            (Self::Other(s), _) => s.as_str(),
        }
    }
}

impl UniversalResponse {
    /// Return the response ID to use when serializing to a given provider format.
    ///
    /// If the stored ID originated from the same format, it is returned as-is so
    /// that round-trips preserve the original value.  Otherwise we attempt to
    /// generate a vaguely reasonable-looking placeholder (e.g.
    /// `"msg_transformed"`, `"chatcmpl-transformed"`, `"universal_transformed"`).
    /// Extract the `id` field from a provider response payload using typed
    /// deserialization, avoiding direct `Value::get` access.
    pub fn extract_id_from_payload(payload: &Value) -> Option<String> {
        #[derive(Deserialize)]
        struct IdView {
            id: Option<String>,
        }
        serde_json::from_value::<IdView>(payload.clone())
            .ok()
            .and_then(|v| v.id)
    }

    pub fn id_for(&self, format: ProviderFormat) -> String {
        let prefix = match format {
            ProviderFormat::Anthropic => "msg_",
            ProviderFormat::BedrockAnthropic => "msg_bdrk_",
            ProviderFormat::VertexAnthropic => "msg_vrtx_",
            ProviderFormat::ChatCompletions | ProviderFormat::Mistral => "chatcmpl-",
            ProviderFormat::Universal => "universal_",
            ProviderFormat::Unknown => "resp_",
            ProviderFormat::Responses => "resp_",
            ProviderFormat::Google => "resp_",
            ProviderFormat::Converse => "msg_",
        };
        if let Some(id) = self.id.as_deref() {
            if self.id_format == Some(format) {
                return id.to_string();
            }
            let unique_part = [
                "msg_bdrk_",
                "msg_vrtx_",
                "universal_",
                "resp_",
                "chatcmpl-",
                "msg_",
            ]
            .iter()
            .find_map(|p| id.strip_prefix(p))
            .unwrap_or(id);
            if !unique_part.is_empty() && unique_part != PLACEHOLDER_ID {
                return format!("{}{}", prefix, unique_part);
            }
        }
        format!("{}{}", prefix, PLACEHOLDER_ID)
    }
}

impl UniversalUsage {
    /// Parse usage from provider-specific JSON value.
    ///
    /// Different providers use different field names:
    /// - OpenAI Chat: prompt_tokens, completion_tokens, prompt_tokens_details.cached_tokens
    /// - OpenAI Responses: input_tokens, output_tokens, input_tokens_details.cached_tokens
    /// - Anthropic: input_tokens, output_tokens, cache_read_input_tokens
    /// - Bedrock: inputTokens, outputTokens, cacheReadInputTokens
    /// - Mistral: uses OpenAI format
    pub fn from_provider_value(usage: &Value, provider: ProviderFormat) -> Self {
        match provider {
            ProviderFormat::Universal => {
                Self::from_provider_value(usage, ProviderFormat::ChatCompletions)
            }
            // OpenAI, Mistral, and Unknown use OpenAI format
            ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown => {
                Self {
                    prompt_tokens: usage.get("prompt_tokens").and_then(Value::as_i64),
                    completion_tokens: usage.get("completion_tokens").and_then(Value::as_i64),
                    prompt_cached_tokens: usage
                        .get("prompt_tokens_details")
                        .and_then(|d| d.get("cached_tokens"))
                        .and_then(Value::as_i64),
                    prompt_cache_creation_tokens: None, // OpenAI doesn't report cache creation tokens
                    // Treat 0 as None: 0 reasoning tokens means "no reasoning" = semantically None
                    completion_reasoning_tokens: usage
                        .get("completion_tokens_details")
                        .and_then(|d| d.get("reasoning_tokens"))
                        .and_then(Value::as_i64)
                        .filter(|&v| v > 0),
                }
            }
            ProviderFormat::Responses => Self {
                prompt_tokens: usage.get("input_tokens").and_then(Value::as_i64),
                completion_tokens: usage.get("output_tokens").and_then(Value::as_i64),
                prompt_cached_tokens: usage
                    .get("input_tokens_details")
                    .and_then(|d| d.get("cached_tokens"))
                    .and_then(Value::as_i64),
                prompt_cache_creation_tokens: None,
                // Treat 0 as None: 0 reasoning tokens means "no reasoning" = semantically None
                completion_reasoning_tokens: usage
                    .get("output_tokens_details")
                    .and_then(|d| d.get("reasoning_tokens"))
                    .and_then(Value::as_i64)
                    .filter(|&v| v > 0),
            },
            ProviderFormat::Anthropic
            | ProviderFormat::BedrockAnthropic
            | ProviderFormat::VertexAnthropic => Self {
                prompt_tokens: usage.get("input_tokens").and_then(Value::as_i64),
                completion_tokens: usage.get("output_tokens").and_then(Value::as_i64),
                prompt_cached_tokens: usage.get("cache_read_input_tokens").and_then(Value::as_i64),
                prompt_cache_creation_tokens: usage
                    .get("cache_creation_input_tokens")
                    .and_then(Value::as_i64),
                completion_reasoning_tokens: None, // Anthropic doesn't expose thinking tokens separately
            },
            ProviderFormat::Converse => Self {
                prompt_tokens: usage.get("inputTokens").and_then(Value::as_i64),
                completion_tokens: usage.get("outputTokens").and_then(Value::as_i64),
                prompt_cached_tokens: usage.get("cacheReadInputTokens").and_then(Value::as_i64),
                prompt_cache_creation_tokens: usage
                    .get("cacheWriteInputTokens")
                    .and_then(Value::as_i64),
                completion_reasoning_tokens: None, // Bedrock doesn't expose thinking tokens separately
            },
            ProviderFormat::Google => unreachable!("Google usage is handled via typed From trait"),
        }
    }

    /// Extract usage from a response payload, handling provider-specific key names.
    ///
    /// Most providers use "usage", but Google uses "usageMetadata".
    pub fn extract_from_response(payload: &Value, provider: ProviderFormat) -> Option<Self> {
        payload
            .get("usage")
            .map(|u| Self::from_provider_value(u, provider))
    }

    /// Convert to provider-specific JSON representation.
    ///
    /// Returns a JSON object with provider-specific field names.
    pub fn to_provider_value(&self, provider: ProviderFormat) -> Value {
        let prompt = self.prompt_tokens.unwrap_or(0);
        let completion = self.completion_tokens.unwrap_or(0);

        match provider {
            ProviderFormat::Universal => self.to_provider_value(ProviderFormat::ChatCompletions),
            // OpenAI, Mistral, and Unknown use OpenAI format
            ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown => {
                let mut map = serde_json::Map::new();
                map.insert("prompt_tokens".into(), serde_json::json!(prompt));
                map.insert("completion_tokens".into(), serde_json::json!(completion));
                map.insert(
                    "total_tokens".into(),
                    serde_json::json!(prompt + completion),
                );

                if let Some(cached_tokens) = self.prompt_cached_tokens {
                    map.insert(
                        "prompt_tokens_details".into(),
                        serde_json::json!({ "cached_tokens": cached_tokens }),
                    );
                }

                if let Some(reasoning_tokens) = self.completion_reasoning_tokens {
                    map.insert(
                        "completion_tokens_details".into(),
                        serde_json::json!({ "reasoning_tokens": reasoning_tokens }),
                    );
                }

                Value::Object(map)
            }
            ProviderFormat::Responses => {
                let mut map = serde_json::Map::new();
                map.insert("input_tokens".into(), serde_json::json!(prompt));
                map.insert("output_tokens".into(), serde_json::json!(completion));
                map.insert(
                    "total_tokens".into(),
                    serde_json::json!(prompt + completion),
                );

                let cached = self.prompt_cached_tokens.unwrap_or(0);
                map.insert(
                    "input_tokens_details".into(),
                    serde_json::json!({ "cached_tokens": cached }),
                );

                let reasoning = self.completion_reasoning_tokens.unwrap_or(0);
                map.insert(
                    "output_tokens_details".into(),
                    serde_json::json!({ "reasoning_tokens": reasoning }),
                );

                Value::Object(map)
            }
            ProviderFormat::Anthropic
            | ProviderFormat::BedrockAnthropic
            | ProviderFormat::VertexAnthropic => {
                let mut map = serde_json::Map::new();
                if let Some(p) = self.prompt_tokens {
                    map.insert("input_tokens".into(), serde_json::json!(p));
                }
                if let Some(c) = self.completion_tokens {
                    map.insert("output_tokens".into(), serde_json::json!(c));
                }

                if let Some(cache_creation) = self.prompt_cache_creation_tokens {
                    map.insert(
                        "cache_creation_input_tokens".into(),
                        serde_json::json!(cache_creation),
                    );
                }

                if let Some(cache_read) = self.prompt_cached_tokens {
                    map.insert(
                        "cache_read_input_tokens".into(),
                        serde_json::json!(cache_read),
                    );
                }

                Value::Object(map)
            }
            ProviderFormat::Converse => serde_json::json!({
                "inputTokens": prompt,
                "outputTokens": completion
            }),
            ProviderFormat::Google => {
                let mut map = serde_json::Map::new();

                if let Some(p) = self.prompt_tokens {
                    map.insert("promptTokenCount".into(), serde_json::json!(p));
                }
                if let Some(c) = self.completion_tokens {
                    map.insert("candidatesTokenCount".into(), serde_json::json!(c));
                }

                if self.prompt_tokens.is_some() || self.completion_tokens.is_some() {
                    map.insert(
                        "totalTokenCount".into(),
                        serde_json::json!(prompt + completion),
                    );
                }

                if let Some(cached_tokens) = self.prompt_cached_tokens {
                    map.insert(
                        "cachedContentTokenCount".into(),
                        serde_json::json!(cached_tokens),
                    );
                }

                if let Some(reasoning_tokens) = self.completion_reasoning_tokens {
                    map.insert(
                        "thoughtsTokenCount".into(),
                        serde_json::json!(reasoning_tokens),
                    );
                }

                Value::Object(map)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_with_id(id: Option<&str>, id_format: Option<ProviderFormat>) -> UniversalResponse {
        UniversalResponse {
            id: id.map(ToString::to_string),
            id_format,
            model: None,
            messages: Vec::new(),
            usage: None,
            finish_reason: None,
        }
    }

    #[test]
    fn id_for_uses_neutral_prefixes_for_universal_and_unknown() {
        let response = response_with_id(None, None);

        assert_eq!(
            response.id_for(ProviderFormat::Universal),
            "universal_transformed"
        );
        assert_eq!(response.id_for(ProviderFormat::Unknown), "resp_transformed");
    }

    #[test]
    fn id_for_rewrites_universal_prefix_for_provider_targets() {
        let response =
            response_with_id(Some("universal_existing"), Some(ProviderFormat::Universal));

        assert_eq!(
            response.id_for(ProviderFormat::ChatCompletions),
            "chatcmpl-existing"
        );
        assert_eq!(response.id_for(ProviderFormat::Responses), "resp_existing");
    }
}
