//! Request extraction utilities.
//!
//! This module provides helper functions for extracting key fields from request bodies
//! without requiring full transformation. Useful for routing and pre-processing.

use crate::capabilities::ProviderFormat;
use crate::processing::adapters;
use crate::serde_json::Value;
use std::borrow::Cow;

/// Metadata extracted from request for routing decisions.
#[derive(Debug, Clone)]
pub struct RequestMeta {
    pub model: Option<String>,
    pub stream: Option<bool>,
}

/// Extract routing metadata from request body.
///
/// Only parses what's needed (model + stream flag) without full deserialization.
/// Handles provider-specific differences:
/// - OpenAI/Anthropic/Mistral/Google: `body.model`
/// - Bedrock: `body.modelId`
///
/// Returns `None` if the body is invalid JSON.
pub fn extract_request_meta(body: &[u8]) -> Option<RequestMeta> {
    #[derive(serde::Deserialize)]
    struct MetaHints<'a> {
        #[serde(borrow)]
        model: Option<Cow<'a, str>>,
        #[serde(alias = "modelId", borrow)]
        model_id: Option<Cow<'a, str>>,
        stream: Option<bool>,
    }

    let hints: MetaHints = crate::serde_json::from_slice(body).ok()?;
    Some(RequestMeta {
        model: hints.model.or(hints.model_id).map(|s| s.into_owned()),
        stream: hints.stream,
    })
}

/// Detect provider format from request body using adapter detection.
///
/// Uses the existing adapter infrastructure to detect the format.
/// Returns `ProviderFormat::Unknown` if the body is invalid JSON or no adapter matches.
pub fn detect_format_from_body(body: &[u8]) -> ProviderFormat {
    let payload: Value = match crate::serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return ProviderFormat::Unknown,
    };

    // Reuse existing adapter detection logic
    for adapter in adapters() {
        if adapter.detect_request(&payload) {
            return adapter.format();
        }
    }
    ProviderFormat::Unknown
}

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
            if let Ok(universal) = adapter.request_to_universal(payload.clone()) {
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

/// Extract model from a JSON payload directly (without parsing bytes).
///
/// Same as `extract_model_from_body` but takes a pre-parsed JSON value.
pub fn extract_model_from_value(payload: &Value) -> Option<String> {
    // Try provider-specific detection and extraction
    for adapter in adapters() {
        if adapter.detect_request(payload) {
            if let Ok(universal) = adapter.request_to_universal(payload.clone()) {
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

    // Tests for extract_request_meta
    #[test]
    fn extract_meta_openai() {
        let body = br#"{"model": "gpt-4", "messages": [], "stream": true}"#;
        let meta = extract_request_meta(body).unwrap();
        assert_eq!(meta.model, Some("gpt-4".to_string()));
        assert_eq!(meta.stream, Some(true));
    }

    #[test]
    fn extract_meta_bedrock_model_id() {
        let body = br#"{"modelId": "anthropic.claude-3", "messages": []}"#;
        let meta = extract_request_meta(body).unwrap();
        assert_eq!(meta.model, Some("anthropic.claude-3".to_string()));
        assert_eq!(meta.stream, None);
    }

    #[test]
    fn extract_meta_returns_none_for_invalid_json() {
        let body = b"not json";
        assert!(extract_request_meta(body).is_none());
    }

    // Tests for detect_format_from_body
    #[test]
    fn detect_format_openai() {
        let body = br#"{"model": "gpt-4", "messages": [{"role": "user", "content": "hi"}]}"#;
        assert_eq!(detect_format_from_body(body), ProviderFormat::OpenAI);
    }

    #[test]
    fn detect_format_anthropic() {
        let body = br#"{"model": "claude-3", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 100}"#;
        assert_eq!(detect_format_from_body(body), ProviderFormat::Anthropic);
    }

    #[test]
    fn detect_format_google() {
        let body = br#"{"contents": [{"role": "user", "parts": [{"text": "hi"}]}]}"#;
        assert_eq!(detect_format_from_body(body), ProviderFormat::Google);
    }

    #[test]
    fn detect_format_invalid_json() {
        let body = b"not json";
        assert_eq!(detect_format_from_body(body), ProviderFormat::Unknown);
    }
}
