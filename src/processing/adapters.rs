/*!
Provider adapter trait and registry for unified request transformation.

This module defines the `ProviderAdapter` trait that each provider implements
to handle format detection and request conversion. The adapter pattern consolidates
provider-specific logic into a single interface.

## Adding a new provider

1. Create `adapter.rs` in your provider module
2. Implement `ProviderAdapter` for your adapter struct
3. Register it in `adapters()` with the appropriate feature gate
*/

use crate::capabilities::ProviderFormat;
use crate::processing::transform::TransformError;
use crate::serde_json::{Map, Number, Value};
use crate::universal::{FinishReason, UniversalRequest, UniversalResponse};

/// Trait for provider-specific request and response handling.
///
/// Implementations handle:
/// - Format detection for both requests and responses
/// - Conversion to/from universal request/response format
/// - Provider-specific defaults and finish reason mapping
pub trait ProviderAdapter: Send + Sync {
    // =========================================================================
    // Metadata
    // =========================================================================

    /// Returns the provider format this adapter handles.
    fn format(&self) -> ProviderFormat;

    /// Returns the directory name for this provider's test payloads.
    /// Used by coverage-report to load snapshots from `payloads/snapshots/{dir_name}/`.
    fn directory_name(&self) -> &'static str;

    /// Returns the display name for this provider (used in reports/tables).
    fn display_name(&self) -> &'static str;

    // =========================================================================
    // Request handling
    // =========================================================================

    /// Checks if a payload matches this provider's request format.
    ///
    /// This should delegate to existing detection logic (e.g., `try_parse_*` functions).
    fn detect_request(&self, payload: &Value) -> bool;

    /// Convert a provider-specific payload to universal request format.
    ///
    /// This extracts messages, params, and extras from the provider payload.
    fn request_to_universal(&self, payload: &Value) -> Result<UniversalRequest, TransformError>;

    /// Convert a universal request to provider-specific format.
    ///
    /// This builds a complete request payload in the provider's format.
    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError>;

    /// Apply provider-specific defaults to a universal request.
    ///
    /// This is called after conversion but before building the final payload.
    /// For example, Anthropic requires `max_tokens` to be set.
    fn apply_defaults(&self, req: &mut UniversalRequest);

    // =========================================================================
    // Response handling
    // =========================================================================

    /// Checks if a payload matches this provider's response format.
    fn detect_response(&self, payload: &Value) -> bool;

    /// Convert a provider-specific response to universal format.
    ///
    /// This extracts messages, usage, finish_reason, and extras from the response.
    fn response_to_universal(&self, payload: &Value) -> Result<UniversalResponse, TransformError>;

    /// Convert a universal response to provider-specific format.
    ///
    /// This builds a complete response payload in the provider's format.
    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError>;

    /// Map a universal FinishReason to provider-specific string.
    ///
    /// Each provider uses different strings for finish reasons:
    /// - OpenAI: "stop", "length", "tool_calls", "content_filter"
    /// - Anthropic: "end_turn", "max_tokens", "tool_use"
    /// - Google: "STOP", "MAX_TOKENS"
    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String>;
}

// ============================================================================
// Helper functions for adapter implementations
// ============================================================================

/// Collect fields not in the known keys list into an extras map.
///
/// This preserves provider-specific fields for round-trip transformation.
pub fn collect_extras(payload: &Value, known_keys: &[&str]) -> Map<String, Value> {
    let mut extras = Map::new();
    let Some(obj) = payload.as_object() else {
        return extras;
    };
    for (k, v) in obj {
        if !known_keys.contains(&k.as_str()) {
            extras.insert(k.clone(), v.clone());
        }
    }
    extras
}

/// Insert an optional Value into a map if present.
pub fn insert_opt_value(obj: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(v) = value {
        obj.insert(key.into(), v);
    }
}

/// Insert an optional i64 into a map if present.
pub fn insert_opt_i64(obj: &mut Map<String, Value>, key: &str, value: Option<i64>) {
    if let Some(v) = value {
        obj.insert(key.into(), Value::Number(v.into()));
    }
}

/// Insert an optional f64 into a map if present.
pub fn insert_opt_f64(obj: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    if let Some(v) = value {
        if let Some(n) = Number::from_f64(v) {
            obj.insert(key.into(), Value::Number(n));
        }
    }
}

/// Insert an optional bool into a map if present.
pub fn insert_opt_bool(obj: &mut Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(v) = value {
        obj.insert(key.into(), Value::Bool(v));
    }
}

/// Insert an optional string into a map if present.
pub fn insert_opt_string(obj: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(v) = value {
        obj.insert(key.into(), Value::String(v.to_string()));
    }
}

// ============================================================================
// Adapter registry
// ============================================================================

/// Get all registered adapters in detection priority order.
///
/// Priority order (most specific first):
/// 1. Anthropic (requires max_tokens, has specific structure)
/// 2. OpenAI (most permissive, fallback)
pub fn adapters() -> Vec<Box<dyn ProviderAdapter>> {
    let mut list: Vec<Box<dyn ProviderAdapter>> = Vec::new();

    // Note: Order matters for detection - more specific formats first
    #[cfg(feature = "anthropic")]
    list.push(Box::new(crate::providers::anthropic::AnthropicAdapter));

    #[cfg(feature = "openai")]
    list.push(Box::new(crate::providers::openai::OpenAIAdapter));

    list
}

/// Get the adapter for a specific provider format.
pub fn adapter_for_format(format: ProviderFormat) -> Option<Box<dyn ProviderAdapter>> {
    adapters().into_iter().find(|a| a.format() == format)
}
