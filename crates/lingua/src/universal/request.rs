/*!
Universal request types for cross-provider transformation.

This module provides a canonical representation of LLM requests that can be
converted to/from any provider format.

## Design principles

1. **Round-trip preservation**: Provider-specific fields are stored in
   `provider_extras` keyed by `ProviderFormat`, and restored when converting
   back to the same provider format.

2. **Canonical naming**: Uses consistent field names (e.g., `max_tokens`, `top_p`)
   regardless of what individual providers call them.

3. **Typed configs**: Complex fields like `tool_choice`, `response_format`, `reasoning`,
   and `stop` use typed structs. Only `tools` and `metadata` remain as `Value`.

4. **Provider isolation**: Provider-specific extras are scoped by `ProviderFormat`
   to prevent cross-provider contamination (e.g., OpenAI extras don't bleed into
   Anthropic requests).
*/

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::serde_json::{Map, Value};
use crate::universal::message::Message;
use crate::universal::tools::UniversalTool;

/// Universal request envelope for LLM API calls.
///
/// This type captures the common structure across all provider request formats.
/// Provider-specific fields are stored in `provider_extras`, keyed by the source
/// provider format to prevent cross-provider contamination.
#[derive(Debug, Clone, Serialize)]
pub struct UniversalRequest {
    /// Model identifier (may be None for providers that use endpoint-based model selection)
    pub model: Option<String>,

    /// Conversation messages in universal format
    pub messages: Vec<Message>,

    /// Common request parameters (canonical fields only)
    pub params: UniversalParams,

    /// Provider-specific fields, keyed by the source ProviderFormat.
    ///
    /// When transforming back to the same provider, these extras are merged back.
    /// When transforming to a different provider, they are ignored (no cross-pollination).
    ///
    /// Example: OpenAI Chat extras stay in `provider_extras[ProviderFormat::OpenAI]`
    /// and are only merged back when converting to OpenAI Chat, not to Anthropic.
    #[serde(skip)]
    pub provider_extras: HashMap<ProviderFormat, Map<String, Value>>,
}

/// Common request parameters across providers.
///
/// Uses canonical names - adapters handle mapping to provider-specific names.
/// This struct contains ONLY canonical fields - no extras or provider-specific baggage.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UniversalParams {
    // === Sampling parameters ===
    /// Sampling temperature (0.0 to 2.0 typically)
    pub temperature: Option<f64>,

    /// Nucleus sampling probability
    pub top_p: Option<f64>,

    /// Top-k sampling (not supported by all providers)
    pub top_k: Option<i64>,

    /// Random seed for deterministic generation
    pub seed: Option<i64>,

    /// Presence penalty (-2.0 to 2.0)
    pub presence_penalty: Option<f64>,

    /// Frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: Option<f64>,

    // === Output control ===
    /// Maximum tokens to generate
    pub max_tokens: Option<i64>,

    /// Stop sequences for generation termination.
    ///
    /// All providers accept arrays of strings. OpenAI also accepts a single string,
    /// but we normalize to arrays for simplicity - OpenAI accepts both forms.
    pub stop: Option<Vec<String>>,

    /// Whether to return log probabilities (OpenAI-specific but canonical)
    pub logprobs: Option<bool>,

    /// Number of top logprobs to return (0-20)
    pub top_logprobs: Option<i64>,

    // === Tools and function calling ===
    /// Tool definitions in universal format.
    ///
    /// Tools are normalized to `UniversalTool` which handles the different formats:
    /// - Anthropic: `{"name", "description", "input_schema"}` for custom, `{"type": "bash_20250124"}` for builtins
    /// - OpenAI Chat: `{"type": "function", "function": {...}}`
    /// - OpenAI Responses: `{"type": "function", "name", ...}` or `{"type": "code_interpreter"}`
    pub tools: Option<Vec<UniversalTool>>,

    /// Tool selection strategy configuration.
    ///
    /// Uses canonical fields (`mode`, `tool_name`) for cross-provider conversion.
    pub tool_choice: Option<ToolChoiceConfig>,

    /// Whether tools can be called in parallel
    pub parallel_tool_calls: Option<bool>,

    // === Response format ===
    /// Response format configuration.
    ///
    /// Uses canonical fields (`format_type`, `json_schema`) for cross-provider conversion.
    pub response_format: Option<ResponseFormatConfig>,

    // === Reasoning / Extended thinking ===
    /// Reasoning configuration for extended thinking / chain-of-thought.
    ///
    /// Uses canonical fields (`effort`, `budget_tokens`) for cross-provider conversion.
    /// Skipped when disabled or empty to normalize `{enabled: false}` to `null`.
    #[serde(skip_serializing_if = "reasoning_should_skip")]
    pub reasoning: Option<ReasoningConfig>,

    // === Metadata and identification ===
    /// Request metadata (user tracking, experiment tags, etc.)
    pub metadata: Option<Value>,

    /// Whether to store completion for training/evals (OpenAI-specific but canonical)
    pub store: Option<bool>,

    /// Service tier preference
    pub service_tier: Option<String>,

    // === Streaming ===
    /// Whether to stream the response
    pub stream: Option<bool>,
}

// =============================================================================
// UniversalParams Helper Methods
// =============================================================================

impl UniversalParams {
    /// Get tool_choice for a provider.
    pub fn tool_choice_for(&self, provider: ProviderFormat) -> Option<Value> {
        let config = self.tool_choice.clone().unwrap_or_default();
        config
            .to_provider(provider, self.parallel_tool_calls)
            .ok()
            .flatten()
    }

    /// Get reasoning config for a provider.
    ///
    /// This helper reduces boilerplate in adapters by handling the common pattern:
    /// ```ignore
    /// req.params.reasoning.as_ref()
    ///     .and_then(|r| r.to_provider(provider, max_tokens).ok())
    ///     .flatten()
    /// ```
    pub fn reasoning_for(&self, provider: ProviderFormat) -> Option<Value> {
        self.reasoning
            .as_ref()
            .and_then(|r| r.to_provider(provider, self.max_tokens).ok())
            .flatten()
    }

    /// Get response_format for a provider.
    ///
    /// This helper reduces boilerplate in adapters by handling the common pattern:
    /// ```ignore
    /// req.params.response_format.as_ref()
    ///     .and_then(|rf| rf.to_provider(provider).ok())
    ///     .flatten()
    /// ```
    pub fn response_format_for(&self, provider: ProviderFormat) -> Option<Value> {
        self.response_format
            .as_ref()
            .and_then(|rf| rf.to_provider(provider).ok())
            .flatten()
    }
}

// =============================================================================
// Reasoning Configuration
// =============================================================================

/// Configuration for extended thinking / reasoning capabilities.
///
/// Uses `budget_tokens` as the canonical field for cross-provider conversion.
/// When converting TO a provider, values are converted at the adapter boundary.
/// OpenAI's `reasoning_effort` levels are converted to/from budget_tokens using heuristics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// Whether reasoning/thinking is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Token budget for thinking (canonical field).
    /// All providers' reasoning configurations are normalized to this field.
    /// OpenAI effort levels are converted to budget_tokens at adapter boundaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<i64>,

    /// Summary mode for reasoning output.
    /// Maps to OpenAI Responses API's `reasoning.summary` field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<SummaryMode>,
}

impl ReasoningConfig {
    /// Returns true if this config represents "no reasoning" (disabled or empty).
    /// Used for skip_serializing_if to normalize disabled configs to null.
    pub fn is_effectively_disabled(&self) -> bool {
        // Explicitly disabled
        if self.enabled == Some(false) {
            return true;
        }
        // Empty config (no meaningful fields set)
        self.enabled.is_none() && self.budget_tokens.is_none() && self.summary.is_none()
    }
}

/// Helper for serde skip_serializing_if on Option<ReasoningConfig>.
/// Returns true if the reasoning config should be skipped during serialization.
fn reasoning_should_skip(reasoning: &Option<ReasoningConfig>) -> bool {
    match reasoning {
        None => true,
        Some(config) => config.is_effectively_disabled(),
    }
}

/// Reasoning effort level (portable across providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl FromStr for ReasoningEffort {
    type Err = ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(ConvertError::InvalidEnumValue {
                type_name: "ReasoningEffort",
                value: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ReasoningEffort {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Summary mode for reasoning output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SummaryMode {
    /// No summary included in response.
    None,
    /// Provider decides whether to include summary.
    Auto,
    /// Detailed summary included in response.
    Detailed,
}

impl SummaryMode {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Auto => "auto",
            Self::Detailed => "detailed",
        }
    }
}

impl FromStr for SummaryMode {
    type Err = ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "auto" => Ok(Self::Auto),
            "detailed" => Ok(Self::Detailed),
            _ => Err(ConvertError::InvalidEnumValue {
                type_name: "SummaryMode",
                value: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for SummaryMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for SummaryMode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

// =============================================================================
// Tool Choice Configuration
// =============================================================================

/// Tool selection strategy configuration.
///
/// Uses canonical fields (`mode`, `tool_name`) for cross-provider conversion.
///
/// Provider mapping:
/// - OpenAI Chat: `"auto"` | `"none"` | `"required"` | `{ type: "function", function: { name } }`
/// - OpenAI Responses: `"auto"` | `{ type: "function", name }`
/// - Anthropic: `{ type: "auto" | "any" | "none" | "tool", name?, disable_parallel_tool_use? }`
#[derive(Debug, Clone, Default, Serialize)]
pub struct ToolChoiceConfig {
    /// Selection mode - the semantic intent of the tool choice
    pub mode: Option<ToolChoiceMode>,

    /// Specific tool name (when mode = Tool)
    pub tool_name: Option<String>,

    /// Whether to disable parallel tool calls.
    /// Maps to Anthropic's `disable_parallel_tool_use` field.
    /// For OpenAI, this is handled via the separate `parallel_tool_calls` param.
    pub disable_parallel: Option<bool>,
}

/// Tool selection mode (portable across providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ToolChoiceMode {
    /// Provider decides whether to use tools
    Auto,
    /// No tools allowed
    None,
    /// Must use a tool (OpenAI "required" / Anthropic "any")
    Required,
    /// Specific tool required (use `tool_name` field)
    Tool,
}

impl ToolChoiceMode {
    /// Returns the string representation (OpenAI format).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::None => "none",
            Self::Required => "required",
            Self::Tool => "function",
        }
    }

    /// Convert to Anthropic format string.
    pub fn as_anthropic_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::None => "none",
            Self::Required => "any",
            Self::Tool => "tool",
        }
    }
}

impl FromStr for ToolChoiceMode {
    type Err = ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "none" => Ok(Self::None),
            "required" | "any" => Ok(Self::Required),
            "tool" | "function" => Ok(Self::Tool),
            _ => Err(ConvertError::InvalidEnumValue {
                type_name: "ToolChoiceMode",
                value: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for ToolChoiceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ToolChoiceMode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

// =============================================================================
// Response Format Configuration
// =============================================================================

/// Response format configuration for structured output.
///
/// Provider mapping:
/// - OpenAI Chat: `{ type: "text" | "json_object" | "json_schema", json_schema? }`
/// - OpenAI Responses: nested under `text.format`
/// - Google: `response_mime_type` + `response_schema`
/// - Anthropic: `{ type: "json_schema", schema, name?, strict?, description? }`
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResponseFormatConfig {
    /// Output format type
    pub format_type: Option<ResponseFormatType>,

    /// JSON schema configuration (when format_type = JsonSchema)
    pub json_schema: Option<JsonSchemaConfig>,
}

/// Response format type (portable across providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ResponseFormatType {
    /// Plain text output (default)
    Text,
    /// JSON object output (unstructured)
    JsonObject,
    /// JSON output conforming to a schema
    JsonSchema,
}

impl ResponseFormatType {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::JsonObject => "json_object",
            Self::JsonSchema => "json_schema",
        }
    }
}

impl FromStr for ResponseFormatType {
    type Err = ConvertError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json_object" => Ok(Self::JsonObject),
            "json_schema" => Ok(Self::JsonSchema),
            _ => Err(ConvertError::InvalidEnumValue {
                type_name: "ResponseFormatType",
                value: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for ResponseFormatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ResponseFormatType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// JSON schema configuration for structured output.
#[derive(Debug, Clone, Serialize)]
pub struct JsonSchemaConfig {
    /// Schema name (required by OpenAI)
    pub name: String,

    /// The JSON schema definition
    pub schema: Value,

    /// Whether to enable strict schema validation
    pub strict: Option<bool>,

    /// Human-readable description of the schema
    pub description: Option<String>,
}

// =============================================================================
// Stop Sequences Helper
// =============================================================================

/// Parse stop sequences from a JSON value.
///
/// Handles:
/// - `"single_string"` → `vec!["single_string"]`
/// - `["arr", "of", "strings"]` → `vec!["arr", "of", "strings"]`
/// - Other types → `None`
pub fn parse_stop_sequences(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(arr) => {
            let sequences: Vec<String> = arr
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect();
            if sequences.is_empty() {
                None
            } else {
                Some(sequences)
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_parse_stop_sequences_single_string() {
        let value = json!("stop");
        assert_eq!(parse_stop_sequences(&value), Some(vec!["stop".to_string()]));
    }

    #[test]
    fn test_parse_stop_sequences_array_of_strings() {
        let value = json!(["stop1", "stop2"]);
        assert_eq!(
            parse_stop_sequences(&value),
            Some(vec!["stop1".to_string(), "stop2".to_string()])
        );
    }

    #[test]
    fn test_parse_stop_sequences_empty_array() {
        let value = json!([]);
        assert_eq!(parse_stop_sequences(&value), None);
    }

    #[test]
    fn test_parse_stop_sequences_array_with_non_strings() {
        let value = json!([1, 2, 3]);
        assert_eq!(parse_stop_sequences(&value), None);
    }

    #[test]
    fn test_parse_stop_sequences_mixed_array() {
        let value = json!(["stop", 1, "end"]);
        assert_eq!(
            parse_stop_sequences(&value),
            Some(vec!["stop".to_string(), "end".to_string()])
        );
    }

    #[test]
    fn test_parse_stop_sequences_null() {
        let value = json!(null);
        assert_eq!(parse_stop_sequences(&value), None);
    }

    #[test]
    fn test_parse_stop_sequences_number() {
        let value = json!(42);
        assert_eq!(parse_stop_sequences(&value), None);
    }

    #[test]
    fn test_parse_stop_sequences_object() {
        let value = json!({});
        assert_eq!(parse_stop_sequences(&value), None);
    }

    #[test]
    fn test_parse_stop_sequences_boolean() {
        let value = json!(true);
        assert_eq!(parse_stop_sequences(&value), None);
    }
}
