//! Request extraction utilities.
//!
//! This module provides helper functions for extracting key fields from request bodies
//! without requiring full transformation. Useful for routing and pre-processing.

use std::borrow::Cow;

/// Hints extracted from request body for routing decisions.
#[derive(Debug, Clone, Default)]
pub struct RequestHints {
    pub model: Option<String>,
    pub stream: bool,
}

/// Extract routing hints from request body.
///
/// Checks common field names across providers:
/// - `model` (OpenAI, Anthropic, Mistral, Google)
/// - `modelId` (Bedrock)
/// - `stream` (most providers)
///
/// Returns `None` if the body is invalid JSON.
pub fn extract_request_hints(body: &[u8]) -> Option<RequestHints> {
    #[derive(serde::Deserialize)]
    struct Hints<'a> {
        #[serde(borrow)]
        model: Option<Cow<'a, str>>,
        #[serde(alias = "modelId", borrow)]
        model_id: Option<Cow<'a, str>>,
        stream: Option<bool>,
    }

    let hints: Hints = crate::serde_json::from_slice(body).ok()?;
    Some(RequestHints {
        model: hints.model.or(hints.model_id).map(|s| s.into_owned()),
        stream: hints.stream.unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_hints_openai() {
        let body = br#"{"model": "gpt-4", "messages": [], "stream": true}"#;
        let hints = extract_request_hints(body).unwrap();
        assert_eq!(hints.model, Some("gpt-4".to_string()));
        assert!(hints.stream);
    }

    #[test]
    fn extract_hints_bedrock_model_id() {
        let body = br#"{"modelId": "anthropic.claude-3", "messages": []}"#;
        let hints = extract_request_hints(body).unwrap();
        assert_eq!(hints.model, Some("anthropic.claude-3".to_string()));
        assert!(!hints.stream);
    }

    #[test]
    fn extract_hints_stream_defaults_to_false() {
        let body = br#"{"model": "gpt-4"}"#;
        let hints = extract_request_hints(body).unwrap();
        assert!(!hints.stream);
    }

    #[test]
    fn extract_hints_returns_none_for_invalid_json() {
        let body = b"not json";
        assert!(extract_request_hints(body).is_none());
    }
}
