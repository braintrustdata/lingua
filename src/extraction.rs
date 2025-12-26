//! Request extraction utilities.
//!
//! This module provides helper functions for extracting key fields from request bodies
//! without requiring full transformation. Useful for routing and pre-processing.

use crate::processing::adapters;
use crate::serde_json::Value;

/// Extract model from request body, handling different provider formats.
///
/// This uses lingua's adapter detection to find the correct format, then extracts
/// the model field. Handles provider-specific differences:
/// - OpenAI/Anthropic/Mistral: `body.model`
/// - Bedrock: `body.modelId`
/// - Google: `body.model` (may also be in URL path, not handled here)
///
/// Returns `None` if the body is invalid JSON or no model field is found.
pub fn extract_model_from_body(body: &[u8]) -> Option<String> {
    let payload: Value = crate::serde_json::from_slice(body).ok()?;

    // Try provider-specific detection and extraction
    for adapter in adapters() {
        if adapter.detect_request(&payload) {
            if let Ok(universal) = adapter.request_to_universal(&payload) {
                return universal.model;
            }
        }
    }

    // Fallback: try common field names directly
    payload
        .get("model")
        .or_else(|| payload.get("modelId"))
        .and_then(Value::as_str)
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_openai_model() {
        let body = br#"{"model": "gpt-4", "messages": [{"role": "user", "content": "hi"}]}"#;
        assert_eq!(extract_model_from_body(body), Some("gpt-4".to_string()));
    }

    #[test]
    fn extracts_anthropic_model() {
        let body = br#"{"model": "claude-3-5-sonnet-20241022", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 100}"#;
        assert_eq!(
            extract_model_from_body(body),
            Some("claude-3-5-sonnet-20241022".to_string())
        );
    }

    #[test]
    fn extracts_bedrock_model_id() {
        let body = br#"{"modelId": "anthropic.claude-3-5-sonnet-20241022-v2:0", "messages": [{"role": "user", "content": [{"text": "hi"}]}]}"#;
        assert_eq!(
            extract_model_from_body(body),
            Some("anthropic.claude-3-5-sonnet-20241022-v2:0".to_string())
        );
    }

    #[test]
    fn returns_none_for_invalid_json() {
        let body = b"not json";
        assert_eq!(extract_model_from_body(body), None);
    }

    #[test]
    fn returns_none_for_missing_model() {
        let body = br#"{"messages": []}"#;
        assert_eq!(extract_model_from_body(body), None);
    }
}
