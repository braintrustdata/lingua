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
#[derive(Debug, Clone, Serialize)]
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

    /// Why each choice stopped generating.
    #[serde(skip_serializing)]
    pub finish_reasons: Vec<FinishReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseReuseSignals {
    pub complete: bool,
    pub content_is_json: bool,
    pub saw_terminal_finish: bool,
}

impl ResponseReuseSignals {
    pub fn reusable_for_request(self, requires_json: bool) -> bool {
        self.saw_terminal_finish && self.complete && (!requires_json || self.content_is_json)
    }
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

    /// Tokens written to the 5-minute-TTL cache (Anthropic split cache writes)
    pub prompt_cache_creation_5m_tokens: Option<i64>,

    /// Tokens written to the 1-hour-TTL cache (Anthropic split cache writes)
    pub prompt_cache_creation_1h_tokens: Option<i64>,

    /// True when `prompt_tokens` excludes the cache read/creation buckets.
    /// Anthropic-style usage reports `input_tokens` exclusive of cache
    /// tokens, while OpenAI-style usage reports `prompt_tokens` inclusive of
    /// them. Consumers that want an OpenAI-style inclusive prompt total must
    /// add the cache buckets back when this is set; see
    /// [`UniversalUsage::inclusive_prompt_tokens`]. Consumers that want an
    /// Anthropic-style exclusive input count must subtract the cache buckets
    /// when this is not set; see [`UniversalUsage::exclusive_prompt_tokens`].
    pub prompt_tokens_exclude_cache: bool,

    /// Reasoning/thinking tokens used in the completion.
    /// `Some(n)` only when `n > 0`; otherwise `None`.
    pub completion_reasoning_tokens: Option<i64>,
}

/// Reason why the model stopped generating.
///
/// Normalized across provider-specific values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
            "content_filter" | "content_filtered" | "safety" | "refusal" => {
                FinishReason::ContentFilter
            }
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
            (
                "refusal",
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic,
            ) => Self::ContentFilter,
            ("content_filtered", ProviderFormat::Converse) => Self::ContentFilter,
            (
                "SAFETY" | "RECITATION" | "OTHER" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII"
                | "IMAGE_SAFETY" | "ESCALATION",
                ProviderFormat::Google,
            ) => Self::ContentFilter,
            ("content_filter", _) => Self::ContentFilter,

            // Unknown - pass through
            (other, _) => Self::Other(other.to_string()),
        }
    }

    pub fn is_incomplete(&self) -> bool {
        matches!(self, Self::Length | Self::ContentFilter)
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
                ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown,
            ) => "stop",

            // Length variants
            (
                Self::Length,
                ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown,
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
                ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown,
            ) => "tool_calls",

            // ContentFilter variants
            (
                Self::ContentFilter,
                ProviderFormat::Anthropic
                | ProviderFormat::BedrockAnthropic
                | ProviderFormat::VertexAnthropic,
            ) => "refusal",
            (Self::ContentFilter, ProviderFormat::Converse) => "content_filtered",
            (Self::ContentFilter, ProviderFormat::Google) => "SAFETY",
            (Self::ContentFilter, ProviderFormat::Responses) => "incomplete",
            (
                Self::ContentFilter,
                ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown,
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
    /// `"msg_transformed"`, `"chatcmpl-transformed"`).
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

    pub fn content_is_json(&self) -> bool {
        self.assistant_texts()
            .iter()
            .all(|content| serde_json::from_str::<Value>(content).is_ok())
    }

    pub fn is_complete(&self) -> bool {
        !self.finish_reasons.iter().any(FinishReason::is_incomplete)
            && !self
                .finish_reason
                .as_ref()
                .is_some_and(FinishReason::is_incomplete)
    }

    pub fn assistant_texts(&self) -> Vec<String> {
        let mut contents: Vec<String> = self
            .messages
            .iter()
            .filter_map(|message| match message {
                Message::Assistant { content, .. } => content.text(),
                _ => None,
            })
            .collect();
        if contents.is_empty() {
            if let Some(text) = self.messages.last().and_then(message_text) {
                contents.push(text);
            }
        }
        contents
    }

    pub fn reuse_signals(&self) -> ResponseReuseSignals {
        ResponseReuseSignals {
            complete: self.is_complete(),
            content_is_json: self.content_is_json(),
            saw_terminal_finish: true,
        }
    }

    pub fn id_for(&self, format: ProviderFormat) -> String {
        let prefix = match format {
            ProviderFormat::Anthropic => "msg_",
            ProviderFormat::BedrockAnthropic => "msg_bdrk_",
            ProviderFormat::VertexAnthropic => "msg_vrtx_",
            ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown => {
                "chatcmpl-"
            }
            ProviderFormat::Responses => "resp_",
            ProviderFormat::Google => "resp_",
            ProviderFormat::Converse => "msg_",
        };
        if let Some(id) = self.id.as_deref() {
            if self.id_format == Some(format) {
                return id.to_string();
            }
            let unique_part = ["msg_bdrk_", "msg_vrtx_", "resp_", "chatcmpl-", "msg_"]
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

fn message_text(message: &Message) -> Option<String> {
    match message {
        Message::Assistant { content, .. } => content.text(),
        Message::System { .. }
        | Message::Developer { .. }
        | Message::User { .. }
        | Message::Tool { .. } => None,
    }
}

#[derive(Default, Deserialize)]
struct AnthropicUsageView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    output_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    cache_read_input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    cache_creation_input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_cache_creation")]
    cache_creation: Option<AnthropicCacheCreationView>,
}

#[derive(Default, Deserialize)]
struct AnthropicCacheCreationView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    ephemeral_5m_input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    ephemeral_1h_input_tokens: Option<i64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OptionalI64View {
    Integer(i64),
    Other(serde::de::IgnoredAny),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OptionalCacheCreationView {
    CacheCreation(AnthropicCacheCreationView),
    Other(serde::de::IgnoredAny),
}

fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(
        match Option::<OptionalI64View>::deserialize(deserializer)? {
            Some(OptionalI64View::Integer(value)) => Some(value),
            Some(OptionalI64View::Other(_)) | None => None,
        },
    )
}

fn deserialize_optional_cache_creation<'de, D>(
    deserializer: D,
) -> Result<Option<AnthropicCacheCreationView>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(
        match Option::<OptionalCacheCreationView>::deserialize(deserializer)? {
            Some(OptionalCacheCreationView::CacheCreation(value)) => Some(value),
            Some(OptionalCacheCreationView::Other(_)) | None => None,
        },
    )
}

fn anthropic_usage_view(usage: &Value) -> AnthropicUsageView {
    serde_json::from_value::<AnthropicUsageView>(usage.clone()).unwrap_or_default()
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
                    prompt_cache_creation_5m_tokens: None,
                    prompt_cache_creation_1h_tokens: None,
                    // OpenAI's prompt_tokens already includes cached tokens
                    prompt_tokens_exclude_cache: false,
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
                prompt_cache_creation_5m_tokens: None,
                prompt_cache_creation_1h_tokens: None,
                // OpenAI's input_tokens already includes cached tokens
                prompt_tokens_exclude_cache: false,
                // Treat 0 as None: 0 reasoning tokens means "no reasoning" = semantically None
                completion_reasoning_tokens: usage
                    .get("output_tokens_details")
                    .and_then(|d| d.get("reasoning_tokens"))
                    .and_then(Value::as_i64)
                    .filter(|&v| v > 0),
            },
            ProviderFormat::Anthropic
            | ProviderFormat::BedrockAnthropic
            | ProviderFormat::VertexAnthropic => {
                let usage = anthropic_usage_view(usage);
                let cache_creation = usage.cache_creation.unwrap_or_default();
                Self {
                    prompt_tokens: usage.input_tokens,
                    completion_tokens: usage.output_tokens,
                    prompt_cached_tokens: usage.cache_read_input_tokens,
                    prompt_cache_creation_tokens: usage.cache_creation_input_tokens,
                    prompt_cache_creation_5m_tokens: cache_creation.ephemeral_5m_input_tokens,
                    prompt_cache_creation_1h_tokens: cache_creation.ephemeral_1h_input_tokens,
                    // Anthropic's input_tokens excludes cache read/creation tokens
                    prompt_tokens_exclude_cache: true,
                    completion_reasoning_tokens: None, // Anthropic doesn't expose thinking tokens separately
                }
            }
            ProviderFormat::Converse => Self {
                prompt_tokens: usage.get("inputTokens").and_then(Value::as_i64),
                completion_tokens: usage.get("outputTokens").and_then(Value::as_i64),
                prompt_cached_tokens: usage.get("cacheReadInputTokens").and_then(Value::as_i64),
                prompt_cache_creation_tokens: usage
                    .get("cacheWriteInputTokens")
                    .and_then(Value::as_i64),
                prompt_cache_creation_5m_tokens: None,
                prompt_cache_creation_1h_tokens: None,
                // Converse's inputTokens excludes cache read/write tokens
                prompt_tokens_exclude_cache: true,
                completion_reasoning_tokens: None, // Bedrock doesn't expose thinking tokens separately
            },
            ProviderFormat::Google => unreachable!("Google usage is handled via typed From trait"),
        }
    }

    /// Prompt tokens following the OpenAI convention of including cache
    /// read/creation tokens. For providers that report prompt tokens
    /// exclusive of the cache buckets (Anthropic, Converse), the cache
    /// tokens are added back. Returns `None` when no prompt-side counts are
    /// present at all.
    pub fn inclusive_prompt_tokens(&self) -> Option<i64> {
        if !self.prompt_tokens_exclude_cache {
            return self.prompt_tokens;
        }
        let prompt_cache_creation_tokens = self.prompt_cache_creation_tokens_for_prompt_math();
        if self.prompt_tokens.is_none()
            && self.prompt_cached_tokens.is_none()
            && prompt_cache_creation_tokens.is_none()
        {
            return None;
        }
        Some(
            self.prompt_tokens.unwrap_or(0)
                + self.prompt_cached_tokens.unwrap_or(0)
                + prompt_cache_creation_tokens.unwrap_or(0),
        )
    }

    fn prompt_cache_creation_tokens_for_prompt_math(&self) -> Option<i64> {
        self.prompt_cache_creation_tokens.or_else(|| {
            if self.prompt_cache_creation_5m_tokens.is_none()
                && self.prompt_cache_creation_1h_tokens.is_none()
            {
                return None;
            }
            Some(
                self.prompt_cache_creation_5m_tokens.unwrap_or(0)
                    + self.prompt_cache_creation_1h_tokens.unwrap_or(0),
            )
        })
    }

    pub fn exclusive_prompt_tokens(&self) -> Option<i64> {
        if self.prompt_tokens_exclude_cache {
            return self.prompt_tokens;
        }
        let prompt_tokens = self.prompt_tokens?;
        Some(
            (prompt_tokens
                - self.prompt_cached_tokens.unwrap_or(0)
                - self
                    .prompt_cache_creation_tokens_for_prompt_math()
                    .unwrap_or(0))
            .max(0),
        )
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
        let inclusive_prompt = self.inclusive_prompt_tokens();
        let prompt = inclusive_prompt.unwrap_or(0);
        let provider_prompt = self.prompt_tokens.unwrap_or(0);
        let completion = self.completion_tokens.unwrap_or(0);

        match provider {
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
                if let Some(p) = self.exclusive_prompt_tokens() {
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
                if self.prompt_cache_creation_5m_tokens.is_some()
                    || self.prompt_cache_creation_1h_tokens.is_some()
                {
                    map.insert(
                        "cache_creation".into(),
                        serde_json::json!({
                            "ephemeral_5m_input_tokens": self
                                .prompt_cache_creation_5m_tokens
                                .unwrap_or(0),
                            "ephemeral_1h_input_tokens": self
                                .prompt_cache_creation_1h_tokens
                                .unwrap_or(0),
                        }),
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
                "inputTokens": provider_prompt,
                "outputTokens": completion
            }),
            ProviderFormat::Google => {
                let mut map = serde_json::Map::new();

                if let Some(p) = inclusive_prompt {
                    map.insert("promptTokenCount".into(), serde_json::json!(p));
                }
                if let Some(c) = self.completion_tokens {
                    map.insert("candidatesTokenCount".into(), serde_json::json!(c));
                }

                if inclusive_prompt.is_some() || self.completion_tokens.is_some() {
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
    use crate::universal::message::AssistantContent;

    #[test]
    fn test_incomplete_finish_reasons() {
        assert!(!FinishReason::Stop.is_incomplete());
        assert!(FinishReason::Length.is_incomplete());
        assert!(!FinishReason::ToolCalls.is_incomplete());
        assert!(FinishReason::ContentFilter.is_incomplete());
        assert!(!FinishReason::Other("done".to_string()).is_incomplete());
    }

    #[test]
    fn test_response_completeness_uses_every_choice() {
        let response = UniversalResponse {
            id: None,
            id_format: None,
            model: None,
            messages: Vec::new(),
            usage: None,
            finish_reason: Some(FinishReason::Stop),
            finish_reasons: vec![FinishReason::Length, FinishReason::Stop],
        };
        assert!(!response.is_complete());

        let response = UniversalResponse {
            id: None,
            id_format: None,
            model: None,
            messages: Vec::new(),
            usage: None,
            finish_reason: Some(FinishReason::Stop),
            finish_reasons: vec![FinishReason::Stop, FinishReason::ToolCalls],
        };
        assert!(response.is_complete());
    }

    #[test]
    fn test_response_content_is_json_validates_every_assistant_message() {
        let response = UniversalResponse {
            id: None,
            id_format: None,
            model: None,
            messages: vec![
                Message::Assistant {
                    content: AssistantContent::String(r#"{"ok":true}"#.to_string()),
                    id: None,
                },
                Message::Assistant {
                    content: AssistantContent::String(r#"{"broken":"#.to_string()),
                    id: None,
                },
            ],
            usage: None,
            finish_reason: Some(FinishReason::Stop),
            finish_reasons: vec![FinishReason::Stop, FinishReason::Stop],
        };
        assert!(!response.content_is_json());

        let response = UniversalResponse {
            id: None,
            id_format: None,
            model: None,
            messages: vec![Message::Assistant {
                content: AssistantContent::Array(vec![
                    crate::universal::message::AssistantContentPart::Text(
                        crate::universal::message::TextContentPart {
                            text: r#"{"ok":true}"#.to_string(),
                            encrypted_content: None,
                            cache_control: None,
                            provider_options: None,
                        },
                    ),
                ]),
                id: None,
            }],
            usage: None,
            finish_reason: Some(FinishReason::Stop),
            finish_reasons: vec![FinishReason::Stop],
        };
        assert!(response.content_is_json());
    }

    #[test]
    fn test_google_escalation_string_maps_to_content_filter() {
        let result = FinishReason::from_provider_string("ESCALATION", ProviderFormat::Google);
        assert_eq!(result, FinishReason::ContentFilter);
    }

    #[test]
    fn test_anthropic_refusal_maps_to_content_filter() {
        for provider in [
            ProviderFormat::Anthropic,
            ProviderFormat::BedrockAnthropic,
            ProviderFormat::VertexAnthropic,
        ] {
            assert_eq!(
                FinishReason::from_provider_string("refusal", provider),
                FinishReason::ContentFilter,
                "expected 'refusal' to map to ContentFilter for {provider:?}"
            );
        }
    }

    #[test]
    fn test_content_filter_roundtrips_as_refusal_for_anthropic() {
        for provider in [
            ProviderFormat::Anthropic,
            ProviderFormat::BedrockAnthropic,
            ProviderFormat::VertexAnthropic,
        ] {
            let wire = FinishReason::ContentFilter.to_provider_string(provider);
            assert_eq!(
                wire, "refusal",
                "ContentFilter should serialize as 'refusal' for {provider:?}"
            );
            let back = FinishReason::from_provider_string(wire, provider);
            assert_eq!(
                back,
                FinishReason::ContentFilter,
                "roundtrip failed for {provider:?}"
            );
        }
    }

    #[test]
    fn test_refusal_in_fromstr_maps_to_content_filter() {
        let result: FinishReason = "refusal".parse().unwrap();
        assert_eq!(result, FinishReason::ContentFilter);
    }

    #[test]
    fn test_google_safety_related_strings_map_to_content_filter() {
        for reason in [
            "SAFETY",
            "RECITATION",
            "OTHER",
            "BLOCKLIST",
            "PROHIBITED_CONTENT",
            "SPII",
            "IMAGE_SAFETY",
            "ESCALATION",
        ] {
            assert_eq!(
                FinishReason::from_provider_string(reason, ProviderFormat::Google),
                FinishReason::ContentFilter,
                "expected {reason} to map to ContentFilter"
            );
        }
    }

    #[test]
    fn test_exclusive_usage_serializes_inclusive_prompt_tokens_for_openai_formats() {
        let usage = UniversalUsage {
            prompt_tokens: Some(10),
            completion_tokens: Some(5),
            prompt_cached_tokens: Some(20),
            prompt_cache_creation_tokens: Some(30),
            prompt_tokens_exclude_cache: true,
            ..Default::default()
        };

        let chat = usage.to_provider_value(ProviderFormat::ChatCompletions);
        assert_eq!(chat["prompt_tokens"], 60);
        assert_eq!(chat["completion_tokens"], 5);
        assert_eq!(chat["total_tokens"], 65);

        let responses = usage.to_provider_value(ProviderFormat::Responses);
        assert_eq!(responses["input_tokens"], 60);
        assert_eq!(responses["output_tokens"], 5);
        assert_eq!(responses["total_tokens"], 65);
    }

    #[test]
    fn test_inclusive_prompt_tokens_uses_split_ttl_when_aggregate_missing() {
        let usage = UniversalUsage {
            prompt_tokens: Some(10),
            prompt_cached_tokens: Some(20),
            prompt_cache_creation_5m_tokens: Some(30),
            prompt_cache_creation_1h_tokens: Some(40),
            prompt_tokens_exclude_cache: true,
            ..Default::default()
        };

        assert_eq!(usage.inclusive_prompt_tokens(), Some(100));
    }

    #[test]
    fn test_exclusive_usage_stays_exclusive_for_anthropic_formats() {
        let usage = UniversalUsage {
            prompt_tokens: Some(10),
            completion_tokens: Some(5),
            prompt_cached_tokens: Some(20),
            prompt_cache_creation_tokens: Some(30),
            prompt_tokens_exclude_cache: true,
            ..Default::default()
        };

        let anthropic = usage.to_provider_value(ProviderFormat::Anthropic);
        assert_eq!(anthropic["input_tokens"], 10);
        assert_eq!(anthropic["output_tokens"], 5);
        assert_eq!(anthropic["cache_read_input_tokens"], 20);
        assert_eq!(anthropic["cache_creation_input_tokens"], 30);
    }

    #[test]
    fn test_inclusive_usage_serializes_exclusive_prompt_tokens_for_anthropic_formats() {
        let usage = UniversalUsage {
            prompt_tokens: Some(60),
            completion_tokens: Some(5),
            prompt_cached_tokens: Some(20),
            prompt_cache_creation_tokens: Some(10),
            prompt_tokens_exclude_cache: false,
            ..Default::default()
        };

        let anthropic = usage.to_provider_value(ProviderFormat::Anthropic);
        assert_eq!(anthropic["input_tokens"], 30);
        assert_eq!(anthropic["output_tokens"], 5);
        assert_eq!(anthropic["cache_read_input_tokens"], 20);
        assert_eq!(anthropic["cache_creation_input_tokens"], 10);
    }

    #[test]
    fn test_anthropic_cache_creation_serializes_both_ttl_buckets() {
        let usage = UniversalUsage {
            prompt_tokens: Some(10),
            prompt_cache_creation_5m_tokens: Some(20),
            prompt_tokens_exclude_cache: true,
            ..Default::default()
        };

        let anthropic = usage.to_provider_value(ProviderFormat::Anthropic);
        assert_eq!(anthropic["cache_creation"]["ephemeral_5m_input_tokens"], 20);
        assert_eq!(anthropic["cache_creation"]["ephemeral_1h_input_tokens"], 0);
    }

    #[test]
    fn test_malformed_anthropic_cache_creation_preserves_token_counts() {
        let usage = crate::serde_json::json!({
            "input_tokens": 10,
            "output_tokens": 5,
            "cache_read_input_tokens": 20,
            "cache_creation_input_tokens": 30,
            "cache_creation": "invalid",
        });

        let usage = UniversalUsage::from_provider_value(&usage, ProviderFormat::Anthropic);
        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(5));
        assert_eq!(usage.prompt_cached_tokens, Some(20));
        assert_eq!(usage.prompt_cache_creation_tokens, Some(30));
        assert_eq!(usage.prompt_cache_creation_5m_tokens, None);
        assert_eq!(usage.prompt_cache_creation_1h_tokens, None);
    }
}
