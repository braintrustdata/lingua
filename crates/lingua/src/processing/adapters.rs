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

use std::sync::LazyLock;

use crate::capabilities::ProviderFormat;
use crate::processing::transform::TransformError;
use crate::serde_json::{Map, Number, Value};
use crate::universal::{UniversalRequest, UniversalResponse, UniversalStreamChunk};

/// Trait for provider-specific request and response handling.
///
/// Implementations handle:
/// - Format detection for both requests and responses
/// - Conversion to/from universal request/response format
/// - Provider-specific defaults
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
    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError>;

    /// Convert a universal request to provider-specific format.
    ///
    /// This builds a complete request payload in the provider's format.
    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError>;

    // =========================================================================
    // Response handling
    // =========================================================================

    /// Checks if a payload matches this provider's response format.
    fn detect_response(&self, payload: &Value) -> bool;

    /// Convert a provider-specific response to universal format.
    ///
    /// This extracts messages, usage, finish_reason, and extras from the response.
    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError>;

    /// Convert a universal response to provider-specific format.
    ///
    /// This builds a complete response payload in the provider's format.
    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError>;

    /// Apply provider-specific defaults to a universal request.
    ///
    /// This is called after conversion but before building the final payload.
    /// For example, Anthropic requires `max_tokens` to be set.
    ///
    /// Default implementation is a no-op. Override if your provider requires specific defaults.
    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // Default: no-op - override if provider requires specific defaults
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    /// Checks if a payload matches this provider's streaming response format.
    ///
    /// Streaming formats differ from non-streaming responses:
    /// - OpenAI: `object == "chat.completion.chunk"` or has `choices` with `delta`
    /// - Anthropic: has `type` field with streaming event types
    /// - Google: has `candidates` array (same as non-streaming)
    /// - Bedrock: has `messageStart`, `contentBlockDelta`, etc.
    fn detect_stream_response(&self, payload: &Value) -> bool {
        // Default: not implemented
        let _ = payload;
        false
    }

    /// Convert a provider-specific streaming chunk to universal format.
    ///
    /// Returns `Ok(None)` for events that don't produce output (keep-alive, metadata).
    /// Returns `Ok(Some(chunk))` with `chunk.is_keep_alive() == true` for events
    /// that should be acknowledged but not forwarded to clients.
    fn stream_to_universal(
        &self,
        _payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        Err(TransformError::StreamingNotImplemented(
            self.display_name().to_string(),
        ))
    }

    /// Convert a universal streaming chunk to provider-specific format.
    ///
    /// This builds a streaming chunk payload in the provider's format.
    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        let _ = chunk;
        Err(TransformError::StreamingNotImplemented(
            self.display_name().to_string(),
        ))
    }
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

/// Static adapter storage - initialized once, reused for all calls.
#[allow(clippy::vec_init_then_push)] // Can't use vec![] with conditional cfg attributes
static ADAPTERS: LazyLock<Vec<Box<dyn ProviderAdapter>>> = LazyLock::new(|| {
    let mut list: Vec<Box<dyn ProviderAdapter>> = Vec::new();

    // Note: Order matters for detection - more specific formats first
    #[cfg(feature = "openai")]
    list.push(Box::new(crate::providers::openai::ResponsesAdapter));

    #[cfg(feature = "bedrock")]
    list.push(Box::new(crate::providers::bedrock::BedrockAdapter));

    #[cfg(feature = "google")]
    list.push(Box::new(crate::providers::google::GoogleAdapter));

    #[cfg(feature = "anthropic")]
    list.push(Box::new(crate::providers::anthropic::AnthropicAdapter));

    #[cfg(feature = "openai")]
    list.push(Box::new(crate::providers::openai::OpenAIAdapter));

    list
});

/// Get all registered adapters in detection priority order.
///
/// Returns a reference to a static slice - no allocation occurs after first call.
///
/// Priority order (most specific first):
/// 1. Responses API (has unique `input` field)
/// 2. Bedrock (has unique `modelId` field)
/// 3. Google
/// 4. Anthropic
/// 5. OpenAI (most permissive, fallback)
pub fn adapters() -> &'static [Box<dyn ProviderAdapter>] {
    &ADAPTERS
}

/// Get the adapter for a specific provider format.
///
/// Returns a reference to the static adapter instance - no allocation.
pub fn adapter_for_format(format: ProviderFormat) -> Option<&'static dyn ProviderAdapter> {
    ADAPTERS
        .iter()
        .find(|a| a.format() == format)
        .map(|a| a.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "openai")]
    #[test]
    fn test_adapter_caching_is_stable() {
        let a1 = adapter_for_format(ProviderFormat::ChatCompletions).unwrap();
        let a2 = adapter_for_format(ProviderFormat::ChatCompletions).unwrap();
        assert!(std::ptr::eq(a1, a2));
    }
}
