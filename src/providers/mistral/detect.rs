/*!
Mistral format detection.

This module provides functions to detect if a payload is in
Mistral AI format.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::FormatDetector;
use crate::serde_json::Value;

/// Detector for Mistral AI format.
///
/// Mistral uses an OpenAI-compatible format with some distinctive features:
/// - `safe_prompt` field for content filtering
/// - Model names starting with `mistral-`, `codestral-`, `pixtral-`, or `ministral-`
#[derive(Debug, Clone, Copy)]
pub struct MistralDetector;

impl FormatDetector for MistralDetector {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Mistral
    }

    fn detect(&self, payload: &Value) -> bool {
        is_mistral_format(payload)
    }

    fn priority(&self) -> u8 {
        70 // Check before OpenAI since Mistral is OpenAI-compatible
    }
}

/// Check if payload is in Mistral format.
///
/// Indicators:
/// - Has `safe_prompt` field (Mistral-specific)
/// - Model name starts with `mistral-`, `codestral-`, `pixtral-`, or `ministral-`
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::mistral::detect::is_mistral_format;
///
/// let mistral_payload = json!({
///     "model": "mistral-large-latest",
///     "messages": [{"role": "user", "content": "Hello"}],
///     "safe_prompt": true
/// });
///
/// assert!(is_mistral_format(&mistral_payload));
/// ```
/// Known Mistral model prefixes (lowercase for case-insensitive matching).
const MISTRAL_MODEL_PREFIXES: &[&str] = &["mistral-", "codestral-", "pixtral-", "ministral-"];

pub fn is_mistral_format(payload: &Value) -> bool {
    // Check for Mistral-specific fields
    if payload.get("safe_prompt").is_some() {
        return true;
    }

    // Check model name for Mistral patterns
    if let Some(model) = payload.get("model").and_then(|v| v.as_str()) {
        let model_lower = model.to_ascii_lowercase();
        if MISTRAL_MODEL_PREFIXES
            .iter()
            .any(|prefix| model_lower.starts_with(prefix))
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_mistral_format_with_safe_prompt() {
        let payload = json!({
            "model": "some-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "safe_prompt": true
        });
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_with_model_prefix() {
        let payload = json!({
            "model": "mistral-large-latest",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_codestral() {
        let payload = json!({
            "model": "codestral-latest",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_pixtral() {
        let payload = json!({
            "model": "pixtral-12b-2024-09-11",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_ministral() {
        let payload = json!({
            "model": "ministral-8b-latest",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_not_mistral_format_openai() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_mistral_format(&payload));
    }
}
