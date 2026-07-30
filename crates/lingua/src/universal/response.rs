/*!
Universal response types for cross-provider transformation.

This module provides a canonical representation of LLM responses that can be
converted to/from any provider format.
*/

use crate::capabilities::ProviderFormat;
use crate::serde_json::{self, Value};
use crate::universal::defaults::PLACEHOLDER_ID;
use crate::universal::message::{AssistantContent, AssistantContentPart, Message};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

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

    /// Service tier that served the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub served_service_tier: Option<ServedServiceTier>,

    /// Why the model stopped generating
    pub finish_reason: Option<FinishReason>,

    /// Why each choice stopped generating.
    #[serde(skip_serializing)]
    pub finish_reasons: Vec<FinishReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsableResponseInfo {
    pub complete: bool,
    pub content_is_json: bool,
    pub saw_terminal_finish: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseRequirement {
    Any,
    Json,
}

impl ParsableResponseInfo {
    pub fn valid() -> Self {
        Self {
            complete: true,
            content_is_json: true,
            saw_terminal_finish: true,
        }
    }

    pub fn reusable_for_request(self, requirement: ResponseRequirement) -> bool {
        let content_meets_requirement = match requirement {
            ResponseRequirement::Any => true,
            ResponseRequirement::Json => self.content_is_json,
        };

        self.saw_terminal_finish && self.complete && content_meets_requirement
    }
}

/// A provider-independent token modality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenModality {
    Unspecified,
    Text,
    Image,
    Audio,
    Video,
    Document,
}

/// Token count for one modality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModalityTokenCount {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modality: Option<TokenModality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i64>,
}

/// A token subset with an optional modality breakdown.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBreakdown {
    pub total_tokens: Option<i64>,
    pub by_modality: Option<Vec<ModalityTokenCount>>,
}

/// Detailed subsets of inclusive prompt/input usage.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputTokenDetails {
    /// User/request content tokens by modality. Provider-generated tool prompts are separate.
    pub content_by_modality: Option<Vec<ModalityTokenCount>>,
    /// Cache-read tokens, which are included in `UniversalUsage::prompt_tokens`.
    pub cached: Option<TokenBreakdown>,
    /// Cache-write tokens, which are included in `UniversalUsage::prompt_tokens` when reported.
    pub cache_creation: Option<TokenBreakdown>,
    /// Provider-generated tool prompt tokens included in `UniversalUsage::prompt_tokens`.
    pub tool_prompt: Option<TokenBreakdown>,
}

/// Detailed subsets of inclusive completion/output usage.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputTokenDetails {
    /// Returned candidate content tokens by modality. Reasoning tokens are separate.
    pub content_by_modality: Option<Vec<ModalityTokenCount>>,
    /// Reasoning/thinking tokens included in `UniversalUsage::completion_tokens`.
    pub reasoning: Option<TokenBreakdown>,
}

/// Token usage statistics.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UniversalUsage {
    /// Tokens in the prompt/input, including provider-generated tool prompts when reported.
    pub prompt_tokens: Option<i64>,

    /// Tokens in the completion/output
    pub completion_tokens: Option<i64>,

    /// Total tokens. Preserves a provider-reported value or falls back to prompt plus completion.
    pub total_tokens: Option<i64>,

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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub prompt_tokens_exclude_cache: bool,

    /// Reasoning/thinking tokens used in the completion.
    /// `Some(n)` only when `n > 0`; otherwise `None`.
    pub completion_reasoning_tokens: Option<i64>,

    /// Detailed prompt/input token subsets and modality breakdowns.
    pub input_details: Option<InputTokenDetails>,

    /// Detailed completion/output token subsets and modality breakdowns.
    pub output_details: Option<OutputTokenDetails>,
}

/// Service tier reported by the provider that served a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServedServiceTier {
    Auto,
    Default,
    Fast,
    Flex,
    Priority,
    Scale,
    Standard,
    StandardOnly,
    Batch,
    Reserved,
    Unspecified,
}

impl ServedServiceTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Default => "default",
            Self::Fast => "fast",
            Self::Flex => "flex",
            Self::Priority => "priority",
            Self::Scale => "scale",
            Self::Standard => "standard",
            Self::StandardOnly => "standard_only",
            Self::Batch => "batch",
            Self::Reserved => "reserved",
            Self::Unspecified => "unspecified",
        }
    }
}

/// Reason why the model stopped generating.
///
/// Normalized across provider-specific values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FinishReason {
    /// Normal completion (OpenAI: "stop", Anthropic: "end_turn", Google: "STOP")
    Stop,

    /// Hit token or context limit (OpenAI: "length", Anthropic: "max_tokens")
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
            "length"
            | "max_tokens"
            | "max_output_tokens"
            | "model_context_window_exceeded"
            | "incomplete" => FinishReason::Length,
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
    /// - Anthropic: "end_turn", "stop_sequence", "max_tokens", "model_context_window_exceeded", "tool_use"
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
                "max_tokens" | "model_context_window_exceeded",
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
            || matches!(self, Self::Other(reason) if ["queued", "in_progress", "failed", "cancelled"].contains(&reason.as_ref()))
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
        let contents = self.assistant_texts();
        !contents.is_empty()
            && contents
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
                Message::Assistant { content, .. } => assistant_content_text(content),
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

    pub fn parsable_info(&self) -> ParsableResponseInfo {
        ParsableResponseInfo {
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

fn assistant_content_text(content: &AssistantContent) -> Option<String> {
    match content {
        AssistantContent::String(text) => Some(text.clone()),
        AssistantContent::Array(parts) => {
            let text: String = parts
                .iter()
                .filter_map(|part| match part {
                    AssistantContentPart::Text(text_part) => Some(&text_part.text),
                    _ => None,
                })
                .map(String::as_str)
                .collect();
            (!text.is_empty()).then_some(text)
        }
    }
}

fn message_text(message: &Message) -> Option<String> {
    match message {
        Message::Assistant { content, .. } => assistant_content_text(content),
        Message::System { .. }
        | Message::Developer { .. }
        | Message::User { .. }
        | Message::Tool { .. }
        | Message::AdditionalTools { .. } => None,
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
    output_tokens_details: Option<TypedUsageDetails<AnthropicOutputTokenDetailsView>>,
}

#[derive(Default, Deserialize)]
struct AnthropicCacheCreationView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    ephemeral_5m_input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    ephemeral_1h_input_tokens: Option<i64>,
}

#[derive(Default, Deserialize)]
struct AnthropicOutputTokenDetailsView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    thinking_tokens: Option<i64>,
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

#[derive(Debug, Default, Deserialize)]
struct OpenAIChatUsageDetailsView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    cached_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    reasoning_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    audio_tokens: Option<i64>,
}

/// A typed optional usage-details object or its explicitly modeled malformed value.
/// Malformed details do not invalidate independent aggregate counters.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TypedUsageDetails<T> {
    Valid(T),
    Malformed(Value),
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIChatUsageView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    prompt_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    completion_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    total_tokens: Option<i64>,
    prompt_tokens_details: Option<TypedUsageDetails<OpenAIChatUsageDetailsView>>,
    completion_tokens_details: Option<TypedUsageDetails<OpenAIChatUsageDetailsView>>,
}

fn openai_chat_usage_view(usage: &Value) -> OpenAIChatUsageView {
    serde_json::from_value::<OpenAIChatUsageView>(usage.clone()).unwrap_or_default()
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIResponsesUsageDetailsView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    cached_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    cache_write_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    reasoning_tokens: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIResponsesUsageView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    output_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    total_tokens: Option<i64>,
    input_tokens_details: Option<TypedUsageDetails<OpenAIResponsesUsageDetailsView>>,
    output_tokens_details: Option<TypedUsageDetails<OpenAIResponsesUsageDetailsView>>,
}

fn openai_responses_usage_view(usage: &Value) -> OpenAIResponsesUsageView {
    serde_json::from_value::<OpenAIResponsesUsageView>(usage.clone()).unwrap_or_default()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConverseUsageView {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    output_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    total_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    cache_read_input_tokens: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    cache_write_input_tokens: Option<i64>,
}

fn converse_usage_view(usage: &Value) -> ConverseUsageView {
    serde_json::from_value::<ConverseUsageView>(usage.clone()).unwrap_or_default()
}

fn sum_usage_counts(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.saturating_add(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn usage_token_breakdown(
    total_tokens: Option<i64>,
    by_modality: Option<Vec<ModalityTokenCount>>,
) -> Option<TokenBreakdown> {
    if total_tokens.is_none() && by_modality.is_none() {
        return None;
    }
    Some(TokenBreakdown {
        total_tokens,
        by_modality,
    })
}

fn single_modality_count(
    modality: TokenModality,
    token_count: Option<i64>,
) -> Option<Vec<ModalityTokenCount>> {
    token_count
        .filter(|&token_count| token_count > 0)
        .map(|token_count| {
            vec![ModalityTokenCount {
                modality: Some(modality),
                token_count: Some(token_count),
            }]
        })
}

fn optional_input_details(details: InputTokenDetails) -> Option<InputTokenDetails> {
    (details != InputTokenDetails::default()).then_some(details)
}

fn optional_output_details(details: OutputTokenDetails) -> Option<OutputTokenDetails> {
    (details != OutputTokenDetails::default()).then_some(details)
}

fn modality_token_total(
    details: Option<&[ModalityTokenCount]>,
    modality: TokenModality,
) -> Option<i64> {
    let mut total = None;
    for detail in details.unwrap_or_default() {
        if detail.modality.as_ref() != Some(&modality) {
            continue;
        }
        if let Some(token_count) = detail.token_count {
            total = Some(total.unwrap_or(0_i64).saturating_add(token_count));
        }
    }
    total
}

fn google_modality_name(modality: &TokenModality) -> &'static str {
    match modality {
        TokenModality::Unspecified => "MODALITY_UNSPECIFIED",
        TokenModality::Text => "TEXT",
        TokenModality::Image => "IMAGE",
        TokenModality::Audio => "AUDIO",
        TokenModality::Video => "VIDEO",
        TokenModality::Document => "DOCUMENT",
    }
}

fn google_modality_token_counts_value(details: Option<&[ModalityTokenCount]>) -> Option<Value> {
    details.map(|details| {
        Value::Array(
            details
                .iter()
                .map(|detail| {
                    let mut map = serde_json::Map::new();
                    if let Some(modality) = detail.modality.as_ref() {
                        map.insert(
                            "modality".into(),
                            Value::String(google_modality_name(modality).into()),
                        );
                    }
                    if let Some(token_count) = detail.token_count {
                        map.insert("tokenCount".into(), serde_json::json!(token_count));
                    }
                    Value::Object(map)
                })
                .collect(),
        )
    })
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
                let usage = openai_chat_usage_view(usage);
                let prompt_details = match usage.prompt_tokens_details {
                    Some(TypedUsageDetails::Valid(details)) => details,
                    Some(TypedUsageDetails::Malformed(value)) => {
                        drop(value);
                        OpenAIChatUsageDetailsView::default()
                    }
                    None => OpenAIChatUsageDetailsView::default(),
                };
                let completion_details = match usage.completion_tokens_details {
                    Some(TypedUsageDetails::Valid(details)) => details,
                    Some(TypedUsageDetails::Malformed(value)) => {
                        drop(value);
                        OpenAIChatUsageDetailsView::default()
                    }
                    None => OpenAIChatUsageDetailsView::default(),
                };
                let reasoning_tokens = completion_details.reasoning_tokens.filter(|&v| v > 0);
                let total_tokens = usage
                    .total_tokens
                    .or_else(|| sum_usage_counts(usage.prompt_tokens, usage.completion_tokens));
                Self {
                    prompt_tokens: usage.prompt_tokens,
                    completion_tokens: usage.completion_tokens,
                    total_tokens,
                    prompt_cached_tokens: prompt_details.cached_tokens,
                    prompt_cache_creation_tokens: None,
                    prompt_cache_creation_5m_tokens: None,
                    prompt_cache_creation_1h_tokens: None,
                    // OpenAI's prompt_tokens already includes cached tokens
                    prompt_tokens_exclude_cache: false,
                    completion_reasoning_tokens: reasoning_tokens,
                    input_details: optional_input_details(InputTokenDetails {
                        content_by_modality: single_modality_count(
                            TokenModality::Audio,
                            prompt_details.audio_tokens,
                        ),
                        cached: usage_token_breakdown(prompt_details.cached_tokens, None),
                        cache_creation: None,
                        tool_prompt: None,
                    }),
                    output_details: optional_output_details(OutputTokenDetails {
                        content_by_modality: single_modality_count(
                            TokenModality::Audio,
                            completion_details.audio_tokens,
                        ),
                        reasoning: usage_token_breakdown(reasoning_tokens, None),
                    }),
                }
            }
            ProviderFormat::Responses => {
                let usage = openai_responses_usage_view(usage);
                let input_details = match usage.input_tokens_details {
                    Some(TypedUsageDetails::Valid(details)) => details,
                    Some(TypedUsageDetails::Malformed(value)) => {
                        drop(value);
                        OpenAIResponsesUsageDetailsView::default()
                    }
                    None => OpenAIResponsesUsageDetailsView::default(),
                };
                let output_details = match usage.output_tokens_details {
                    Some(TypedUsageDetails::Valid(details)) => details,
                    Some(TypedUsageDetails::Malformed(value)) => {
                        drop(value);
                        OpenAIResponsesUsageDetailsView::default()
                    }
                    None => OpenAIResponsesUsageDetailsView::default(),
                };
                let reasoning_tokens = output_details.reasoning_tokens.filter(|&v| v > 0);
                let total_tokens = usage
                    .total_tokens
                    .or_else(|| sum_usage_counts(usage.input_tokens, usage.output_tokens));
                Self {
                    prompt_tokens: usage.input_tokens,
                    completion_tokens: usage.output_tokens,
                    total_tokens,
                    prompt_cached_tokens: input_details.cached_tokens,
                    prompt_cache_creation_tokens: input_details.cache_write_tokens,
                    prompt_cache_creation_5m_tokens: None,
                    prompt_cache_creation_1h_tokens: None,
                    // OpenAI's input_tokens already includes cached tokens
                    prompt_tokens_exclude_cache: false,
                    // Treat 0 as None: 0 reasoning tokens means "no reasoning" = semantically None
                    completion_reasoning_tokens: reasoning_tokens,
                    input_details: optional_input_details(InputTokenDetails {
                        content_by_modality: None,
                        cached: usage_token_breakdown(input_details.cached_tokens, None),
                        cache_creation: usage_token_breakdown(
                            input_details.cache_write_tokens,
                            None,
                        ),
                        tool_prompt: None,
                    }),
                    output_details: optional_output_details(OutputTokenDetails {
                        content_by_modality: None,
                        reasoning: usage_token_breakdown(reasoning_tokens, None),
                    }),
                }
            }
            ProviderFormat::Anthropic
            | ProviderFormat::BedrockAnthropic
            | ProviderFormat::VertexAnthropic => {
                let usage = anthropic_usage_view(usage);
                let cache_creation = usage.cache_creation.unwrap_or_default();
                let cache_creation_tokens = usage.cache_creation_input_tokens.or_else(|| {
                    sum_usage_counts(
                        cache_creation.ephemeral_5m_input_tokens,
                        cache_creation.ephemeral_1h_input_tokens,
                    )
                });
                let inclusive_prompt = sum_usage_counts(
                    sum_usage_counts(usage.input_tokens, usage.cache_read_input_tokens),
                    cache_creation_tokens,
                );
                let reasoning_tokens = match usage.output_tokens_details {
                    Some(TypedUsageDetails::Valid(details)) => details.thinking_tokens,
                    Some(TypedUsageDetails::Malformed(value)) => {
                        drop(value);
                        None
                    }
                    None => None,
                }
                .filter(|&tokens| tokens > 0);
                Self {
                    prompt_tokens: usage.input_tokens,
                    completion_tokens: usage.output_tokens,
                    total_tokens: sum_usage_counts(inclusive_prompt, usage.output_tokens),
                    prompt_cached_tokens: usage.cache_read_input_tokens,
                    prompt_cache_creation_tokens: cache_creation_tokens,
                    prompt_cache_creation_5m_tokens: cache_creation.ephemeral_5m_input_tokens,
                    prompt_cache_creation_1h_tokens: cache_creation.ephemeral_1h_input_tokens,
                    // Anthropic's input_tokens excludes cache read/creation tokens
                    prompt_tokens_exclude_cache: true,
                    completion_reasoning_tokens: reasoning_tokens,
                    input_details: optional_input_details(InputTokenDetails {
                        content_by_modality: None,
                        cached: usage_token_breakdown(usage.cache_read_input_tokens, None),
                        cache_creation: usage_token_breakdown(cache_creation_tokens, None),
                        tool_prompt: None,
                    }),
                    output_details: optional_output_details(OutputTokenDetails {
                        content_by_modality: None,
                        reasoning: usage_token_breakdown(reasoning_tokens, None),
                    }),
                }
            }
            ProviderFormat::Converse => {
                let usage = converse_usage_view(usage);
                let inclusive_prompt = sum_usage_counts(
                    sum_usage_counts(usage.input_tokens, usage.cache_read_input_tokens),
                    usage.cache_write_input_tokens,
                );
                Self {
                    prompt_tokens: usage.input_tokens,
                    completion_tokens: usage.output_tokens,
                    total_tokens: usage
                        .total_tokens
                        .or_else(|| sum_usage_counts(inclusive_prompt, usage.output_tokens)),
                    prompt_cached_tokens: usage.cache_read_input_tokens,
                    prompt_cache_creation_tokens: usage.cache_write_input_tokens,
                    prompt_cache_creation_5m_tokens: None,
                    prompt_cache_creation_1h_tokens: None,
                    // Converse's inputTokens excludes cache read/write tokens
                    prompt_tokens_exclude_cache: true,
                    completion_reasoning_tokens: None,
                    input_details: optional_input_details(InputTokenDetails {
                        content_by_modality: None,
                        cached: usage_token_breakdown(usage.cache_read_input_tokens, None),
                        cache_creation: usage_token_breakdown(usage.cache_write_input_tokens, None),
                        tool_prompt: None,
                    }),
                    output_details: None,
                }
            }
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
        let prompt_cached_tokens = self.prompt_cached_tokens_for_prompt_math();
        let prompt_cache_creation_tokens = self.prompt_cache_creation_tokens_for_prompt_math();
        if self.prompt_tokens.is_none()
            && prompt_cached_tokens.is_none()
            && prompt_cache_creation_tokens.is_none()
        {
            return None;
        }
        Some(
            self.prompt_tokens.unwrap_or(0)
                + prompt_cached_tokens.unwrap_or(0)
                + prompt_cache_creation_tokens.unwrap_or(0),
        )
    }

    fn prompt_cached_tokens_for_prompt_math(&self) -> Option<i64> {
        self.input_details
            .as_ref()
            .and_then(|details| details.cached.as_ref())
            .and_then(|details| details.total_tokens)
            .or(self.prompt_cached_tokens)
    }

    fn prompt_cache_creation_tokens_for_prompt_math(&self) -> Option<i64> {
        self.input_details
            .as_ref()
            .and_then(|details| details.cache_creation.as_ref())
            .and_then(|details| details.total_tokens)
            .or(self.prompt_cache_creation_tokens)
            .or_else(|| {
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
                - self.prompt_cached_tokens_for_prompt_math().unwrap_or(0)
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
        let completion = self.completion_tokens.unwrap_or(0);
        let total = self.total_tokens.unwrap_or(prompt + completion);
        let input_details = self.input_details.as_ref();
        let output_details = self.output_details.as_ref();
        let cached_tokens = self.prompt_cached_tokens_for_prompt_math();
        let cache_creation_tokens = self.prompt_cache_creation_tokens_for_prompt_math();
        let reasoning_tokens = output_details
            .and_then(|details| details.reasoning.as_ref())
            .and_then(|details| details.total_tokens)
            .or(self.completion_reasoning_tokens);

        match provider {
            // OpenAI, Mistral, and Unknown use OpenAI format
            ProviderFormat::ChatCompletions | ProviderFormat::Mistral | ProviderFormat::Unknown => {
                let mut map = serde_json::Map::new();
                map.insert("prompt_tokens".into(), serde_json::json!(prompt));
                map.insert("completion_tokens".into(), serde_json::json!(completion));
                map.insert("total_tokens".into(), serde_json::json!(total));

                let prompt_audio_tokens = modality_token_total(
                    input_details.and_then(|details| details.content_by_modality.as_deref()),
                    TokenModality::Audio,
                );
                let mut prompt_details = serde_json::Map::new();
                if let Some(cached_tokens) = cached_tokens {
                    prompt_details.insert("cached_tokens".into(), serde_json::json!(cached_tokens));
                }
                if let Some(audio_tokens) = prompt_audio_tokens {
                    prompt_details.insert("audio_tokens".into(), serde_json::json!(audio_tokens));
                }
                if !prompt_details.is_empty() {
                    map.insert(
                        "prompt_tokens_details".into(),
                        Value::Object(prompt_details),
                    );
                }

                let completion_audio_tokens = modality_token_total(
                    output_details.and_then(|details| details.content_by_modality.as_deref()),
                    TokenModality::Audio,
                );
                let mut completion_details = serde_json::Map::new();
                if let Some(reasoning_tokens) = reasoning_tokens {
                    completion_details.insert(
                        "reasoning_tokens".into(),
                        serde_json::json!(reasoning_tokens),
                    );
                }
                if let Some(audio_tokens) = completion_audio_tokens {
                    completion_details
                        .insert("audio_tokens".into(), serde_json::json!(audio_tokens));
                }
                if !completion_details.is_empty() {
                    map.insert(
                        "completion_tokens_details".into(),
                        Value::Object(completion_details),
                    );
                }

                Value::Object(map)
            }
            ProviderFormat::Responses => {
                let mut map = serde_json::Map::new();
                map.insert("input_tokens".into(), serde_json::json!(prompt));
                map.insert("output_tokens".into(), serde_json::json!(completion));
                map.insert("total_tokens".into(), serde_json::json!(total));

                let cached = cached_tokens.unwrap_or(0);
                let mut input_details = serde_json::Map::new();
                input_details.insert("cached_tokens".into(), serde_json::json!(cached));
                if let Some(cache_write) = cache_creation_tokens {
                    input_details
                        .insert("cache_write_tokens".into(), serde_json::json!(cache_write));
                }
                map.insert("input_tokens_details".into(), Value::Object(input_details));

                let reasoning = reasoning_tokens.unwrap_or(0);
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

                if let Some(cache_creation) = cache_creation_tokens {
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

                if let Some(cache_read) = cached_tokens {
                    map.insert(
                        "cache_read_input_tokens".into(),
                        serde_json::json!(cache_read),
                    );
                }
                if let Some(reasoning_tokens) = reasoning_tokens {
                    map.insert(
                        "output_tokens_details".into(),
                        serde_json::json!({ "thinking_tokens": reasoning_tokens }),
                    );
                }

                Value::Object(map)
            }
            ProviderFormat::Converse => {
                let mut map = serde_json::Map::new();
                map.insert(
                    "inputTokens".into(),
                    serde_json::json!(self.exclusive_prompt_tokens().unwrap_or(0)),
                );
                map.insert("outputTokens".into(), serde_json::json!(completion));
                map.insert("totalTokens".into(), serde_json::json!(total));
                if let Some(cache_read) = cached_tokens {
                    map.insert("cacheReadInputTokens".into(), serde_json::json!(cache_read));
                }
                if let Some(cache_write) = cache_creation_tokens {
                    map.insert(
                        "cacheWriteInputTokens".into(),
                        serde_json::json!(cache_write),
                    );
                }
                Value::Object(map)
            }
            ProviderFormat::Google => {
                let mut map = serde_json::Map::new();
                let tool_prompt = input_details.and_then(|details| details.tool_prompt.as_ref());
                let tool_prompt_tokens = tool_prompt
                    .and_then(|details| details.total_tokens)
                    .unwrap_or(0);

                if let Some(prompt) = inclusive_prompt {
                    map.insert(
                        "promptTokenCount".into(),
                        serde_json::json!(prompt.saturating_sub(tool_prompt_tokens).max(0)),
                    );
                }
                if let Some(completion) = self.completion_tokens {
                    map.insert(
                        "candidatesTokenCount".into(),
                        serde_json::json!(completion
                            .saturating_sub(reasoning_tokens.unwrap_or(0))
                            .max(0)),
                    );
                }

                if self.total_tokens.is_some()
                    || inclusive_prompt.is_some()
                    || self.completion_tokens.is_some()
                {
                    map.insert("totalTokenCount".into(), serde_json::json!(total));
                }

                if let Some(cached_tokens) = cached_tokens {
                    map.insert(
                        "cachedContentTokenCount".into(),
                        serde_json::json!(cached_tokens),
                    );
                }
                if let Some(cache_details) = input_details
                    .and_then(|details| details.cached.as_ref())
                    .and_then(|details| {
                        google_modality_token_counts_value(details.by_modality.as_deref())
                    })
                {
                    map.insert("cacheTokensDetails".into(), cache_details);
                }
                if let Some(reasoning_tokens) = reasoning_tokens {
                    map.insert(
                        "thoughtsTokenCount".into(),
                        serde_json::json!(reasoning_tokens),
                    );
                }
                if let Some(prompt_details) = google_modality_token_counts_value(
                    input_details.and_then(|details| details.content_by_modality.as_deref()),
                ) {
                    map.insert("promptTokensDetails".into(), prompt_details);
                }
                if let Some(candidate_details) = google_modality_token_counts_value(
                    output_details.and_then(|details| details.content_by_modality.as_deref()),
                ) {
                    map.insert("candidatesTokensDetails".into(), candidate_details);
                }
                if let Some(tool_prompt_tokens) =
                    tool_prompt.and_then(|details| details.total_tokens)
                {
                    map.insert(
                        "toolUsePromptTokenCount".into(),
                        serde_json::json!(tool_prompt_tokens),
                    );
                }
                if let Some(tool_prompt_details) = tool_prompt.and_then(|details| {
                    google_modality_token_counts_value(details.by_modality.as_deref())
                }) {
                    map.insert("toolUsePromptTokensDetails".into(), tool_prompt_details);
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
        assert!(FinishReason::Other("queued".to_string()).is_incomplete());
        assert!(FinishReason::Other("in_progress".to_string()).is_incomplete());
        assert!(FinishReason::Other("failed".to_string()).is_incomplete());
        assert!(FinishReason::Other("cancelled".to_string()).is_incomplete());
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
            served_service_tier: None,
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
            served_service_tier: None,
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
            served_service_tier: None,
            finish_reason: Some(FinishReason::Stop),
            finish_reasons: vec![FinishReason::Stop, FinishReason::Stop],
        };
        assert!(!response.content_is_json());

        let response = UniversalResponse {
            id: None,
            id_format: None,
            model: None,
            messages: Vec::new(),
            usage: None,
            served_service_tier: None,
            finish_reason: Some(FinishReason::Stop),
            finish_reasons: vec![FinishReason::Stop],
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
            served_service_tier: None,
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
    fn test_anthropic_context_window_exceeded_maps_to_length() {
        let parsed: FinishReason = "model_context_window_exceeded".parse().unwrap();
        assert_eq!(parsed, FinishReason::Length);

        for provider in [
            ProviderFormat::Anthropic,
            ProviderFormat::BedrockAnthropic,
            ProviderFormat::VertexAnthropic,
            ProviderFormat::Converse,
        ] {
            assert_eq!(
                FinishReason::from_provider_string("model_context_window_exceeded", provider),
                FinishReason::Length,
                "expected context-window stop to map to Length for {provider:?}"
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
        assert_eq!(responses["input_tokens_details"]["cache_write_tokens"], 30);
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
    fn test_responses_cache_write_tokens_uses_split_ttl_when_aggregate_missing() {
        let usage = UniversalUsage {
            prompt_tokens: Some(10),
            prompt_cached_tokens: Some(20),
            prompt_cache_creation_5m_tokens: Some(30),
            prompt_cache_creation_1h_tokens: Some(40),
            prompt_tokens_exclude_cache: true,
            ..Default::default()
        };

        let responses = usage.to_provider_value(ProviderFormat::Responses);

        assert_eq!(responses["input_tokens"], 100);
        assert_eq!(responses["input_tokens_details"]["cache_write_tokens"], 70);
    }

    #[test]
    fn test_converse_serializes_exclusive_prompt_tokens_with_cache_buckets() {
        let usage = UniversalUsage {
            prompt_tokens: Some(100),
            completion_tokens: Some(25),
            prompt_cached_tokens: Some(40),
            prompt_cache_creation_tokens: Some(15),
            prompt_tokens_exclude_cache: false,
            ..Default::default()
        };

        let converse = usage.to_provider_value(ProviderFormat::Converse);
        assert_eq!(converse["inputTokens"], 45);
        assert_eq!(converse["outputTokens"], 25);
        assert_eq!(converse["totalTokens"], 125);
        assert_eq!(converse["cacheReadInputTokens"], 40);
        assert_eq!(converse["cacheWriteInputTokens"], 15);

        let roundtrip = UniversalUsage::from_provider_value(&converse, ProviderFormat::Converse);
        assert_eq!(roundtrip.prompt_tokens, Some(45));
        assert!(roundtrip.prompt_tokens_exclude_cache);
        assert_eq!(roundtrip.inclusive_prompt_tokens(), Some(100));
        assert_eq!(roundtrip.total_tokens, Some(125));
    }

    #[test]
    fn test_converse_preserves_already_exclusive_prompt_tokens() {
        let usage = UniversalUsage {
            prompt_tokens: Some(45),
            completion_tokens: Some(25),
            prompt_cached_tokens: Some(40),
            prompt_cache_creation_tokens: Some(15),
            prompt_tokens_exclude_cache: true,
            ..Default::default()
        };

        let converse = usage.to_provider_value(ProviderFormat::Converse);
        assert_eq!(converse["inputTokens"], 45);
        assert_eq!(converse["cacheReadInputTokens"], 40);
        assert_eq!(converse["cacheWriteInputTokens"], 15);
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

    #[test]
    fn test_google_provider_value_preserves_nested_usage_details() {
        let usage = UniversalUsage {
            prompt_tokens: Some(12),
            completion_tokens: Some(8),
            total_tokens: Some(20),
            input_details: Some(InputTokenDetails {
                content_by_modality: Some(vec![ModalityTokenCount {
                    modality: Some(TokenModality::Text),
                    token_count: Some(10),
                }]),
                cached: Some(TokenBreakdown {
                    total_tokens: Some(4),
                    by_modality: Some(vec![ModalityTokenCount {
                        modality: Some(TokenModality::Text),
                        token_count: Some(4),
                    }]),
                }),
                cache_creation: None,
                tool_prompt: Some(TokenBreakdown {
                    total_tokens: Some(2),
                    by_modality: Some(vec![ModalityTokenCount {
                        modality: Some(TokenModality::Text),
                        token_count: Some(2),
                    }]),
                }),
            }),
            output_details: Some(OutputTokenDetails {
                content_by_modality: Some(vec![ModalityTokenCount {
                    modality: Some(TokenModality::Audio),
                    token_count: Some(5),
                }]),
                reasoning: Some(TokenBreakdown {
                    total_tokens: Some(3),
                    by_modality: None,
                }),
            }),
            ..Default::default()
        };

        let google = usage.to_provider_value(ProviderFormat::Google);

        assert_eq!(
            google,
            crate::serde_json::json!({
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 20,
                "cachedContentTokenCount": 4,
                "cacheTokensDetails": [{ "modality": "TEXT", "tokenCount": 4 }],
                "thoughtsTokenCount": 3,
                "promptTokensDetails": [{ "modality": "TEXT", "tokenCount": 10 }],
                "candidatesTokensDetails": [{ "modality": "AUDIO", "tokenCount": 5 }],
                "toolUsePromptTokenCount": 2,
                "toolUsePromptTokensDetails": [{ "modality": "TEXT", "tokenCount": 2 }]
            })
        );
    }

    #[test]
    fn test_openai_chat_malformed_counter_preserves_valid_usage() {
        let provider_usage = crate::serde_json::json!({
            "prompt_tokens": 10,
            "completion_tokens": "invalid",
            "total_tokens": 18,
            "prompt_tokens_details": { "cached_tokens": 4 }
        });

        for provider in [ProviderFormat::ChatCompletions, ProviderFormat::Mistral] {
            let usage = UniversalUsage::from_provider_value(&provider_usage, provider);

            assert_eq!(usage.prompt_tokens, Some(10));
            assert_eq!(usage.completion_tokens, None);
            assert_eq!(usage.total_tokens, Some(18));
            assert_eq!(usage.prompt_cached_tokens, Some(4));
        }
    }

    #[test]
    fn test_openai_chat_malformed_optional_details_preserve_valid_usage() {
        let provider_usage = crate::serde_json::json!({
            "prompt_tokens": 10,
            "completion_tokens": 8,
            "total_tokens": 18,
            "prompt_tokens_details": "invalid",
            "completion_tokens_details": { "reasoning_tokens": 2 }
        });

        let usage =
            UniversalUsage::from_provider_value(&provider_usage, ProviderFormat::ChatCompletions);

        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(8));
        assert_eq!(usage.total_tokens, Some(18));
        assert_eq!(usage.input_details, None);
        assert_eq!(usage.completion_reasoning_tokens, Some(2));
    }

    #[test]
    fn test_openai_chat_usage_details_roundtrip() {
        let provider_usage = crate::serde_json::json!({
            "prompt_tokens": 10,
            "completion_tokens": 8,
            "total_tokens": 18,
            "prompt_tokens_details": {
                "cached_tokens": 4,
                "audio_tokens": 3
            },
            "completion_tokens_details": {
                "reasoning_tokens": 2,
                "audio_tokens": 1
            }
        });

        let usage =
            UniversalUsage::from_provider_value(&provider_usage, ProviderFormat::ChatCompletions);
        assert_eq!(usage.total_tokens, Some(18));
        assert_eq!(
            usage
                .input_details
                .as_ref()
                .and_then(|details| details.cached.as_ref())
                .and_then(|details| details.total_tokens),
            Some(4)
        );
        assert_eq!(
            usage
                .output_details
                .as_ref()
                .and_then(|details| details.reasoning.as_ref())
                .and_then(|details| details.total_tokens),
            Some(2)
        );

        let roundtrip = usage.to_provider_value(ProviderFormat::ChatCompletions);
        assert_eq!(roundtrip, provider_usage);
    }

    #[test]
    fn test_anthropic_malformed_output_details_preserve_valid_usage() {
        let provider_usage = crate::serde_json::json!({
            "input_tokens": 10,
            "output_tokens": 5,
            "cache_read_input_tokens": 20,
            "output_tokens_details": "invalid"
        });

        let usage = UniversalUsage::from_provider_value(&provider_usage, ProviderFormat::Anthropic);

        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(5));
        assert_eq!(usage.prompt_cached_tokens, Some(20));
        assert_eq!(usage.completion_reasoning_tokens, None);
    }

    #[test]
    fn test_anthropic_usage_details_include_cache_reasoning_and_total() {
        let provider_usage = crate::serde_json::json!({
            "input_tokens": 10,
            "output_tokens": 5,
            "cache_read_input_tokens": 20,
            "cache_creation_input_tokens": 30,
            "output_tokens_details": { "thinking_tokens": 2 }
        });

        let usage = UniversalUsage::from_provider_value(&provider_usage, ProviderFormat::Anthropic);
        assert_eq!(usage.total_tokens, Some(65));
        assert_eq!(
            usage
                .input_details
                .as_ref()
                .and_then(|details| details.cache_creation.as_ref())
                .and_then(|details| details.total_tokens),
            Some(30)
        );
        assert_eq!(
            usage
                .output_details
                .as_ref()
                .and_then(|details| details.reasoning.as_ref())
                .and_then(|details| details.total_tokens),
            Some(2)
        );

        let roundtrip = usage.to_provider_value(ProviderFormat::Anthropic);
        assert_eq!(roundtrip, provider_usage);
    }

    #[test]
    fn test_openai_responses_malformed_counter_preserves_valid_usage() {
        let provider_usage = crate::serde_json::json!({
            "input_tokens": "invalid",
            "output_tokens": 25,
            "total_tokens": 125,
            "output_tokens_details": { "reasoning_tokens": 5 }
        });

        let usage = UniversalUsage::from_provider_value(&provider_usage, ProviderFormat::Responses);

        assert_eq!(usage.prompt_tokens, None);
        assert_eq!(usage.completion_tokens, Some(25));
        assert_eq!(usage.total_tokens, Some(125));
        assert_eq!(usage.completion_reasoning_tokens, Some(5));
    }

    #[test]
    fn test_converse_malformed_counter_preserves_valid_usage() {
        let provider_usage = crate::serde_json::json!({
            "inputTokens": 100,
            "outputTokens": "invalid",
            "totalTokens": 125,
            "cacheReadInputTokens": 40
        });

        let usage = UniversalUsage::from_provider_value(&provider_usage, ProviderFormat::Converse);

        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, None);
        assert_eq!(usage.total_tokens, Some(125));
        assert_eq!(usage.prompt_cached_tokens, Some(40));
    }

    #[test]
    fn test_openai_responses_malformed_input_details_preserve_valid_usage() {
        let provider_usage = crate::serde_json::json!({
            "input_tokens": 100,
            "output_tokens": 25,
            "total_tokens": 125,
            "input_tokens_details": "invalid",
            "output_tokens_details": { "reasoning_tokens": 5 }
        });

        let usage = UniversalUsage::from_provider_value(&provider_usage, ProviderFormat::Responses);

        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, Some(25));
        assert_eq!(usage.total_tokens, Some(125));
        assert_eq!(usage.input_details, None);
        assert_eq!(usage.completion_reasoning_tokens, Some(5));
    }

    #[test]
    fn test_openai_responses_cache_write_tokens() {
        let usage = crate::serde_json::json!({
            "input_tokens": 100,
            "output_tokens": 25,
            "input_tokens_details": {
                "cached_tokens": 40,
                "cache_write_tokens": 15
            },
            "output_tokens_details": {
                "reasoning_tokens": 5
            }
        });

        let usage = UniversalUsage::from_provider_value(&usage, ProviderFormat::Responses);

        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, Some(25));
        assert_eq!(usage.prompt_cached_tokens, Some(40));
        assert_eq!(usage.prompt_cache_creation_tokens, Some(15));
        assert_eq!(usage.completion_reasoning_tokens, Some(5));
        assert_eq!(usage.total_tokens, Some(125));
        assert_eq!(
            usage
                .input_details
                .as_ref()
                .and_then(|details| details.cache_creation.as_ref())
                .and_then(|details| details.total_tokens),
            Some(15)
        );
        assert_eq!(
            usage
                .output_details
                .as_ref()
                .and_then(|details| details.reasoning.as_ref())
                .and_then(|details| details.total_tokens),
            Some(5)
        );

        let responses = usage.to_provider_value(ProviderFormat::Responses);
        assert_eq!(responses["input_tokens_details"]["cached_tokens"], 40);
        assert_eq!(responses["input_tokens_details"]["cache_write_tokens"], 15);
    }
}
