/*!
Mistral format detection.

This module provides functions to detect if a payload is in
Mistral AI format by attempting to deserialize into the struct
and checking for Mistral-specific indicators.

Mistral uses an OpenAI-compatible format, so we first validate
that the payload is valid OpenAI format, then check for
Mistral-specific features.
*/

use crate::serde_json::{self, Value};
use serde::Deserialize;
use thiserror::Error;

/// Error type for Mistral payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Known Mistral model prefixes (lowercase for case-insensitive matching).
const MISTRAL_MODEL_PREFIXES: &[&str] = &["mistral-", "codestral-", "pixtral-", "ministral-"];

/// Extended Mistral request that includes OpenAI fields plus Mistral-specific fields.
///
/// We use this struct to validate Mistral payloads, which are OpenAI-compatible
/// but may include additional Mistral-specific fields like `safe_prompt`.
#[derive(Debug, Clone, Deserialize)]
pub struct MistralChatRequest {
    /// Model name (required)
    pub model: String,

    /// Messages array (required)
    pub messages: Vec<Value>,

    /// Mistral-specific: safe prompt setting
    #[serde(default)]
    pub safe_prompt: Option<bool>,

    /// Temperature (optional)
    #[serde(default)]
    pub temperature: Option<f64>,

    /// Top P (optional)
    #[serde(default)]
    pub top_p: Option<f64>,

    /// Max tokens (optional)
    #[serde(default)]
    pub max_tokens: Option<i64>,

    /// Stream (optional)
    #[serde(default)]
    pub stream: Option<bool>,

    /// Random seed (optional)
    #[serde(default)]
    pub random_seed: Option<i64>,
}

/// Detect if a payload is in Mistral format.
///
/// Detection is performed by:
/// 1. Checking for Mistral-specific indicators (safe_prompt, model name)
/// 2. Attempting to deserialize the payload as a valid chat request
///
/// Mistral uses an OpenAI-compatible format with some distinctive features:
/// - `safe_prompt` field for content filtering
/// - Model names starting with `mistral-`, `codestral-`, `pixtral-`, or `ministral-`
pub fn detect_mistral(payload: &Value) -> bool {
    has_mistral_indicators(payload) && try_parse_mistral(payload).is_ok()
}

/// Check if payload has Mistral-specific indicators.
///
/// This checks for:
/// - `safe_prompt` field (Mistral-specific)
/// - Model names starting with Mistral prefixes
fn has_mistral_indicators(payload: &Value) -> bool {
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

/// Attempt to parse a JSON Value as Mistral chat request.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Mistral format.
///
/// Note: For full Mistral format detection, also check `has_mistral_indicators()`
/// or use `MistralDetector::detect()` which combines both checks.
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::mistral::detect::try_parse_mistral;
///
/// let mistral_payload = json!({
///     "model": "mistral-large-latest",
///     "messages": [{"role": "user", "content": "Hello"}],
///     "safe_prompt": true
/// });
///
/// assert!(try_parse_mistral(&mistral_payload).is_ok());
/// ```
pub fn try_parse_mistral(payload: &Value) -> Result<MistralChatRequest, DetectionError> {
    serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_detect_with_safe_prompt() {
        let payload = json!({
            "model": "some-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "safe_prompt": true
        });
        assert!(detect_mistral(&payload));
    }

    #[test]
    fn test_detect_with_model_prefix() {
        let payload = json!({
            "model": "mistral-large-latest",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(detect_mistral(&payload));
    }

    #[test]
    fn test_detect_codestral() {
        let payload = json!({
            "model": "codestral-latest",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(detect_mistral(&payload));
    }

    #[test]
    fn test_detect_pixtral() {
        let payload = json!({
            "model": "pixtral-12b-2024-09-11",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(detect_mistral(&payload));
    }

    #[test]
    fn test_detect_ministral() {
        let payload = json!({
            "model": "ministral-8b-latest",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(detect_mistral(&payload));
    }

    #[test]
    fn test_detect_rejects_openai() {
        // OpenAI model name without Mistral indicators
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!detect_mistral(&payload));
    }

    #[test]
    fn test_try_parse_mistral_success() {
        let payload = json!({
            "model": "mistral-large-latest",
            "messages": [{"role": "user", "content": "Hello"}],
            "safe_prompt": true
        });

        let result = try_parse_mistral(&payload);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.model, "mistral-large-latest");
        assert_eq!(parsed.safe_prompt, Some(true));
    }

    #[test]
    fn test_try_parse_mistral_fails_without_messages() {
        // Missing messages - required field
        let payload = json!({
            "model": "mistral-large-latest",
            "safe_prompt": true
        });

        let result = try_parse_mistral(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_uses_struct_validation() {
        // Valid Mistral format with model prefix
        let valid = json!({
            "model": "mistral-large-latest",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(detect_mistral(&valid));

        // Valid Mistral format with safe_prompt
        let valid_safe = json!({
            "model": "any-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "safe_prompt": true
        });
        assert!(detect_mistral(&valid_safe));

        // Not Mistral - no indicators
        let invalid = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!detect_mistral(&invalid));
    }

    #[test]
    fn test_mistral_indicators_required() {
        // Even with valid structure, needs Mistral indicators
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!has_mistral_indicators(&payload));
        assert!(!detect_mistral(&payload));
    }
}
