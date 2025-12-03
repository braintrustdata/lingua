/*!
OpenAI format detection.

This module provides functions to detect if a payload is in
OpenAI chat completion format.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::FormatDetector;
use crate::serde_json::Value;

/// Detector for OpenAI Chat Completions API format.
///
/// OpenAI format is the most permissive and serves as a fallback.
/// It detects payloads with:
/// - `messages` array with `role` and `content`/`tool_calls`
/// - `model` field
#[derive(Debug, Clone, Copy)]
pub struct OpenAIDetector;

impl FormatDetector for OpenAIDetector {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    fn detect(&self, payload: &Value) -> bool {
        is_openai_format(payload)
    }

    fn priority(&self) -> u8 {
        50 // Lowest priority - fallback format
    }
}

/// Check if payload is in OpenAI format.
///
/// This is the most permissive check and serves as a fallback.
///
/// Indicators:
/// - Has "messages" array
/// - Has "model" field
/// - Messages have "role" and "content" fields
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::openai::detect::is_openai_format;
///
/// let openai_payload = json!({
///     "model": "gpt-4",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// assert!(is_openai_format(&openai_payload));
/// ```
pub fn is_openai_format(payload: &Value) -> bool {
    // Must have messages array
    let has_messages = payload
        .get("messages")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    // Should have model field
    let has_model = payload.get("model").is_some();

    // Basic validation: messages have role and content
    if has_messages {
        if let Some(messages) = payload.get("messages").and_then(|v| v.as_array()) {
            let valid_messages = messages.iter().all(|msg| {
                msg.get("role").is_some()
                    && (msg.get("content").is_some() || msg.get("tool_calls").is_some())
            });
            return valid_messages && has_model;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_openai_format_basic() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(is_openai_format(&payload));
    }

    #[test]
    fn test_openai_format_with_system() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Hello"}
            ]
        });
        assert!(is_openai_format(&payload));
    }

    #[test]
    fn test_openai_format_with_tool_calls() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{
                "role": "assistant",
                "tool_calls": [{
                    "id": "call_123",
                    "type": "function",
                    "function": {"name": "get_weather", "arguments": "{}"}
                }]
            }]
        });
        assert!(is_openai_format(&payload));
    }

    #[test]
    fn test_not_openai_format_no_model() {
        let payload = json!({
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_openai_format(&payload));
    }

    #[test]
    fn test_not_openai_format_empty_messages() {
        let payload = json!({
            "model": "gpt-4",
            "messages": []
        });
        assert!(!is_openai_format(&payload));
    }

    #[test]
    fn test_not_openai_format_google() {
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}]
        });
        assert!(!is_openai_format(&payload));
    }
}
