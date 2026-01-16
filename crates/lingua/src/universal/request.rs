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

3. **Typed configs with lossless round-trip**: Complex fields like `tool_choice`,
   `response_format`, and `stop` use typed structs with a `raw` field for lossless
   preservation. Only `tools` and `metadata` remain as `Value`.

4. **Provider isolation**: Provider-specific extras are scoped by `ProviderFormat`
   to prevent cross-provider contamination (e.g., OpenAI extras don't bleed into
   Anthropic requests).
*/

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::capabilities::ProviderFormat;
use crate::serde_json::{Map, Value};
use crate::universal::message::Message;

// =============================================================================
// Error Types
// =============================================================================

/// Error type for enum parsing from strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEnumError {
    /// The type name that failed to parse.
    pub type_name: &'static str,
    /// The invalid input value.
    pub value: String,
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {} value: '{}'", self.type_name, self.value)
    }
}

impl std::error::Error for ParseEnumError {}

/// Universal request envelope for LLM API calls.
///
/// This type captures the common structure across all provider request formats.
/// Provider-specific fields are stored in `provider_extras`, keyed by the source
/// provider format to prevent cross-provider contamination.
#[derive(Debug, Clone)]
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
    pub provider_extras: HashMap<ProviderFormat, Map<String, Value>>,
}

/// Common request parameters across providers.
///
/// Uses canonical names - adapters handle mapping to provider-specific names.
/// This struct contains ONLY canonical fields - no extras or provider-specific baggage.
#[derive(Debug, Clone, Default)]
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

    /// Stop sequences configuration.
    ///
    /// This is a typed struct that supports both:
    /// - **Lossless round-trip**: Original provider value stored in `raw`
    /// - **Cross-provider conversion**: Normalized `sequences` array enables semantic translation
    pub stop: Option<StopConfig>,

    /// Whether to return log probabilities (OpenAI-specific but canonical)
    pub logprobs: Option<bool>,

    /// Number of top logprobs to return (0-20)
    pub top_logprobs: Option<i64>,

    // === Tools and function calling ===

    /// Tool definitions (schema varies by provider)
    pub tools: Option<Value>,

    /// Tool selection strategy configuration.
    ///
    /// This is a typed struct that supports both:
    /// - **Lossless round-trip**: Original provider value stored in `raw`
    /// - **Cross-provider conversion**: Canonical fields enable semantic translation
    pub tool_choice: Option<ToolChoiceConfig>,

    /// Whether tools can be called in parallel
    pub parallel_tool_calls: Option<bool>,

    // === Response format ===

    /// Response format configuration.
    ///
    /// This is a typed struct that supports both:
    /// - **Lossless round-trip**: Original provider value stored in `raw`
    /// - **Cross-provider conversion**: Canonical fields enable semantic translation
    pub response_format: Option<ResponseFormatConfig>,

    // === Reasoning / Extended thinking ===

    /// Reasoning configuration for extended thinking / chain-of-thought.
    ///
    /// This is a typed struct that supports both:
    /// - **Lossless round-trip**: Original provider value stored in `raw`
    /// - **Cross-provider conversion**: Canonical fields enable semantic translation
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
// Reasoning Configuration
// =============================================================================

/// Configuration for extended thinking / reasoning capabilities.
///
/// Supports two usage modes:
/// 1. **Same-provider round-trip**: Use `raw` field for exact preservation
/// 2. **Cross-provider conversion**: Use canonical fields (`effort`, `budget_tokens`)
///
/// When converting TO a provider:
/// - If `provider_extras` has the target provider's reasoning field → use it (lossless)
/// - Otherwise, derive from canonical fields using heuristics
#[derive(Debug, Clone, Default)]
pub struct ReasoningConfig {
    /// Whether reasoning/thinking is enabled.
    pub enabled: Option<bool>,

    /// Effort level (portable across providers that support effort-based control).
    /// Maps to OpenAI's `reasoning_effort` and can be converted to Anthropic's `budget_tokens`.
    pub effort: Option<ReasoningEffort>,

    /// Token budget for thinking (Anthropic's native format).
    /// Can be derived from `effort` using heuristics when not explicitly set.
    pub budget_tokens: Option<i64>,

    /// Summary mode for reasoning output.
    /// Maps to OpenAI Responses API's `reasoning.summary` field.
    pub summary: Option<SummaryMode>,

    /// Original provider-specific value for lossless round-trip.
    /// This stores the exact JSON that was received, enabling perfect reconstruction.
    pub raw: Option<Value>,
}

/// Reasoning effort level (portable across providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(ParseEnumError {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "auto" => Ok(Self::Auto),
            "detailed" => Ok(Self::Detailed),
            _ => Err(ParseEnumError {
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
/// Supports two usage modes:
/// 1. **Same-provider round-trip**: Use `raw` field for exact preservation
/// 2. **Cross-provider conversion**: Use canonical fields (`mode`, `tool_name`)
///
/// Provider mapping:
/// - OpenAI Chat: `"auto"` | `"none"` | `"required"` | `{ type: "function", function: { name } }`
/// - OpenAI Responses: `"auto"` | `{ type: "function", name }`
/// - Anthropic: `{ type: "auto" | "any" | "none" | "tool", name?, disable_parallel_tool_use? }`
#[derive(Debug, Clone, Default)]
pub struct ToolChoiceConfig {
    /// Selection mode - the semantic intent of the tool choice
    pub mode: Option<ToolChoiceMode>,

    /// Specific tool name (when mode = Tool)
    pub tool_name: Option<String>,

    /// Whether to disable parallel tool calls.
    /// Maps to Anthropic's `disable_parallel_tool_use` field.
    /// For OpenAI, this is handled via the separate `parallel_tool_calls` param.
    pub disable_parallel: Option<bool>,

    /// Original provider-specific value for lossless round-trip.
    /// This stores the exact JSON that was received, enabling perfect reconstruction.
    pub raw: Option<Value>,
}

/// Tool selection mode (portable across providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "none" => Ok(Self::None),
            "required" | "any" => Ok(Self::Required),
            "tool" | "function" => Ok(Self::Tool),
            _ => Err(ParseEnumError {
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
/// Supports two usage modes:
/// 1. **Same-provider round-trip**: Use `raw` field for exact preservation
/// 2. **Cross-provider conversion**: Use canonical fields (`format_type`, `json_schema`)
///
/// Provider mapping:
/// - OpenAI Chat: `{ type: "text" | "json_object" | "json_schema", json_schema? }`
/// - OpenAI Responses: nested under `text.format`
/// - Google: `response_mime_type` + `response_schema`
/// - Anthropic: Not supported
#[derive(Debug, Clone, Default)]
pub struct ResponseFormatConfig {
    /// Output format type
    pub format_type: Option<ResponseFormatType>,

    /// JSON schema configuration (when format_type = JsonSchema)
    pub json_schema: Option<JsonSchemaConfig>,

    /// Original provider-specific value for lossless round-trip.
    /// This stores the exact JSON that was received, enabling perfect reconstruction.
    pub raw: Option<Value>,
}

/// Response format type (portable across providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json_object" => Ok(Self::JsonObject),
            "json_schema" => Ok(Self::JsonSchema),
            _ => Err(ParseEnumError {
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
#[derive(Debug, Clone)]
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
// Stop Configuration
// =============================================================================

/// Stop sequences configuration.
///
/// Supports two usage modes:
/// 1. **Same-provider round-trip**: Use `raw` field for exact preservation
/// 2. **Cross-provider conversion**: Use normalized `sequences` array
///
/// Provider mapping:
/// - OpenAI: `stop: string | string[]` (allows single string or array)
/// - Anthropic: `stop_sequences: string[]`
/// - Google: `generationConfig.stop_sequences: string[]`
/// - Bedrock: `inferenceConfig.stopSequences: string[]`
#[derive(Debug, Clone, Default)]
pub struct StopConfig {
    /// Normalized stop sequences (always an array).
    /// Single string inputs are converted to single-element arrays.
    pub sequences: Vec<String>,

    /// Original provider-specific value for lossless round-trip.
    /// This stores the exact JSON that was received, enabling perfect reconstruction.
    pub raw: Option<Value>,
}
