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
    use crate::providers::openai::generated::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContent,
        ChatCompletionRequestMessageRole, CreateChatCompletionRequestClass,
    };
    use crate::serde_json::{self, json};

    fn create_openai_request(model: &str) -> CreateChatCompletionRequestClass {
        CreateChatCompletionRequestClass {
            model: model.to_string(),
            messages: vec![ChatCompletionRequestMessage {
                role: ChatCompletionRequestMessageRole::User,
                content: Some(ChatCompletionRequestMessageContent::String(
                    "Hello".to_string(),
                )),
                name: None,
                audio: None,
                function_call: None,
                refusal: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            metadata: None,
            prompt_cache_key: None,
            safety_identifier: None,
            service_tier: None,
            temperature: None,
            top_logprobs: None,
            top_p: None,
            user: None,
            audio: None,
            frequency_penalty: None,
            function_call: None,
            functions: None,
            logit_bias: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            modalities: None,
            n: None,
            parallel_tool_calls: None,
            prediction: None,
            presence_penalty: None,
            reasoning_effort: None,
            response_format: None,
            seed: None,
            stop: None,
            store: None,
            stream: None,
            stream_options: None,
            tool_choice: None,
            tools: None,
            verbosity: None,
            web_search_options: None,
        }
    }

    #[test]
    fn test_mistral_format_with_safe_prompt() {
        // safe_prompt is a Mistral-specific field not in OpenAI types,
        // so we construct the JSON manually and add the field
        let request = create_openai_request("some-model");
        let mut payload = serde_json::to_value(&request).unwrap();
        payload["safe_prompt"] = json!(true);
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_with_model_prefix() {
        let request = create_openai_request("mistral-large-latest");
        let payload = serde_json::to_value(&request).unwrap();
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_codestral() {
        let request = create_openai_request("codestral-latest");
        let payload = serde_json::to_value(&request).unwrap();
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_pixtral() {
        let request = create_openai_request("pixtral-12b-2024-09-11");
        let payload = serde_json::to_value(&request).unwrap();
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_mistral_format_ministral() {
        let request = create_openai_request("ministral-8b-latest");
        let payload = serde_json::to_value(&request).unwrap();
        assert!(is_mistral_format(&payload));
    }

    #[test]
    fn test_not_mistral_format_openai() {
        let request = create_openai_request("gpt-4");
        let payload = serde_json::to_value(&request).unwrap();
        assert!(!is_mistral_format(&payload));
    }
}
