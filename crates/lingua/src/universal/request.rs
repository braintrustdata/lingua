/*!
Universal request types for cross-provider transformation.

This module provides a canonical representation of LLM requests that can be
converted to/from any provider format.

## Design principles

1. **Round-trip preservation**: Any field not mapped to a canonical field goes
   into `extras` and is restored when converting back to the source format.

2. **Canonical naming**: Uses consistent field names (e.g., `max_tokens`, `top_p`)
   regardless of what individual providers call them.

3. **Minimal typing for complex fields**: Fields like `tools`, `tool_choice`, and
   `response_format` are kept as `Value` since they vary significantly across providers.
*/

use crate::serde_json::{Map, Value};
use crate::universal::message::Message;

/// Universal request envelope for LLM API calls.
///
/// This type captures the common structure across all provider request formats.
/// Provider-specific fields that don't map to canonical params go into `extras`.
#[derive(Debug, Clone)]
pub struct UniversalRequest {
    /// Model identifier (may be None for providers that use endpoint-based model selection)
    pub model: Option<String>,

    /// Conversation messages in universal format
    pub messages: Vec<Message>,

    /// Common request parameters
    pub params: UniversalParams,

    /// Provider-specific fields not captured in params
    pub extras: Map<String, Value>,
}

/// Common request parameters across providers.
///
/// Uses canonical names - adapters handle mapping to provider-specific names.
#[derive(Debug, Clone, Default)]
pub struct UniversalParams {
    /// Sampling temperature (0.0 to 2.0 typically)
    pub temperature: Option<f64>,

    /// Nucleus sampling probability
    pub top_p: Option<f64>,

    /// Top-k sampling (not supported by all providers)
    pub top_k: Option<i64>,

    /// Maximum tokens to generate
    pub max_tokens: Option<i64>,

    /// Stop sequences (kept as Value due to union type in OpenAI)
    pub stop: Option<Value>,

    /// Tool definitions (schema varies by provider)
    pub tools: Option<Value>,

    /// Tool selection strategy (varies by provider)
    pub tool_choice: Option<Value>,

    /// Output format specification (varies by provider)
    pub response_format: Option<Value>,

    /// Random seed for deterministic generation
    pub seed: Option<i64>,

    /// Presence penalty (-2.0 to 2.0)
    pub presence_penalty: Option<f64>,

    /// Frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: Option<f64>,

    /// Whether to stream the response
    pub stream: Option<bool>,
}
