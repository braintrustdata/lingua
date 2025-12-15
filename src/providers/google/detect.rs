/*!
Google format detection.

This module provides functions to detect if a payload is in
Google AI (Generative Language API) format.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::FormatDetector;
use crate::serde_json::Value;

/// Detector for Google AI / Gemini format.
///
/// Google's GenerateContent API has distinctive features:
/// - `contents` array instead of `messages`
/// - Content items have `parts` array
/// - Role can be `model` instead of `assistant`
/// - `generationConfig` for generation parameters
#[derive(Debug, Clone, Copy)]
pub struct GoogleDetector;

impl FormatDetector for GoogleDetector {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Google
    }

    fn detect(&self, payload: &Value) -> bool {
        is_google_format(payload)
    }

    fn priority(&self) -> u8 {
        90 // High priority - very distinctive structure
    }
}

/// Check if payload is in Google format.
///
/// Indicators:
/// - Has "contents" array (not "messages")
/// - Content items have "parts" array
/// - Role can be "model" (Anthropic uses "assistant")
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::google::detect::is_google_format;
///
/// let google_payload = json!({
///     "contents": [{
///         "role": "user",
///         "parts": [{"text": "Hello"}]
///     }]
/// });
///
/// assert!(is_google_format(&google_payload));
/// ```
pub fn is_google_format(payload: &Value) -> bool {
    // Primary indicator: "contents" array with "parts" structure
    if let Some(contents) = payload.get("contents").and_then(|v| v.as_array()) {
        if !contents.is_empty() {
            // Check if first content has "parts" field
            if contents[0].get("parts").is_some() {
                return true;
            }
        }
    }

    // Secondary indicator: generationConfig
    if payload.get("generationConfig").is_some() {
        return true;
    }

    // Check for role "model" which is Google-specific
    if let Some(contents) = payload.get("contents").and_then(|v| v.as_array()) {
        for content in contents {
            if content.get("role").and_then(|v| v.as_str()) == Some("model") {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    // Note: Google types are protobuf-generated (prost) without serde support,
    // so we use raw JSON for tests. The protobuf types (GenerateContentRequest, Content, Part)
    // cannot be easily serialized to JSON. This is a known limitation documented in mod.rs.

    #[test]
    fn test_google_format_with_contents_and_parts() {
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }]
        });
        assert!(is_google_format(&payload));
    }

    #[test]
    fn test_google_format_with_generation_config() {
        let payload = json!({
            "contents": [{"role": "user"}],
            "generationConfig": {
                "temperature": 0.7
            }
        });
        assert!(is_google_format(&payload));
    }

    #[test]
    fn test_google_format_with_model_role() {
        let payload = json!({
            "contents": [
                {"role": "user", "parts": [{"text": "Hello"}]},
                {"role": "model", "parts": [{"text": "Hi there!"}]}
            ]
        });
        assert!(is_google_format(&payload));
    }

    #[test]
    fn test_not_google_format_openai() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_google_format(&payload));
    }

    #[test]
    fn test_not_google_format_empty_contents() {
        let payload = json!({
            "contents": []
        });
        assert!(!is_google_format(&payload));
    }
}
