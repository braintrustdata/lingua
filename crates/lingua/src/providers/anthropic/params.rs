/*!
Typed parameter structs for Anthropic Messages API.

These structs use `#[serde(flatten)]` to automatically capture unknown fields,
eliminating the need for explicit KNOWN_KEYS arrays.
*/

use crate::providers::anthropic::generated::{InputMessage, Thinking};
use crate::serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Anthropic Messages API request parameters.
///
/// All known fields are explicitly typed. Unknown fields automatically
/// go into `extras` via `#[serde(flatten)]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnthropicParams {
    // === Core fields ===
    pub model: Option<String>,
    pub messages: Option<Vec<InputMessage>>,

    // === System prompt (can be string or array with cache_control) ===
    pub system: Option<Value>,

    // === Required output control ===
    pub max_tokens: Option<i64>,

    // === Sampling parameters ===
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i64>,
    pub stop_sequences: Option<Value>,

    // === Streaming ===
    pub stream: Option<bool>,

    // === Tools and function calling ===
    pub tools: Option<Value>,
    pub tool_choice: Option<Value>,

    // === Extended thinking ===
    pub thinking: Option<Thinking>,

    // === Structured outputs (beta: structured-outputs-2025-11-13) ===
    /// Output format for structured JSON responses.
    /// Structure: `{ type: "json_schema", schema: {...} }`
    pub output_format: Option<Value>,

    // === Metadata and identification ===
    pub metadata: Option<Value>,
    pub service_tier: Option<String>,

    /// Unknown fields - automatically captured by serde flatten.
    /// These are provider-specific fields not in the canonical set.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json;
    use crate::serde_json::json;

    #[test]
    fn test_anthropic_params_known_fields() {
        let json = json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 1024,
            "temperature": 0.7,
            "top_k": 40
        });

        let params: AnthropicParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.model, Some("claude-sonnet-4-20250514".to_string()));
        assert_eq!(params.max_tokens, Some(1024));
        assert_eq!(params.temperature, Some(0.7));
        assert_eq!(params.top_k, Some(40));
        assert!(params.extras.is_empty());
    }

    #[test]
    fn test_anthropic_params_with_thinking() {
        use crate::providers::anthropic::generated::ThinkingType;

        let json = json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [],
            "max_tokens": 16000,
            "thinking": {
                "type": "enabled",
                "budget_tokens": 10000
            }
        });

        let params: AnthropicParams = serde_json::from_value(json).unwrap();
        assert!(params.thinking.is_some());
        let thinking = params.thinking.unwrap();
        assert_eq!(thinking.thinking_type, ThinkingType::Enabled);
        assert_eq!(thinking.budget_tokens, Some(10000));
    }

    #[test]
    fn test_anthropic_params_with_system_cache_control() {
        let json = json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [],
            "max_tokens": 1024,
            "system": [
                {
                    "type": "text",
                    "text": "Be helpful.",
                    "cache_control": {"type": "ephemeral", "ttl": "5m"}
                }
            ]
        });

        let params: AnthropicParams = serde_json::from_value(json).unwrap();
        assert!(params.system.is_some());
        assert!(params.extras.is_empty());
    }

    #[test]
    fn test_anthropic_params_unknown_fields_go_to_extras() {
        let json = json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [],
            "max_tokens": 1024,
            "some_future_param": "value"
        });

        let params: AnthropicParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.extras.len(), 1);
        assert_eq!(
            params.extras.get("some_future_param"),
            Some(&Value::String("value".to_string()))
        );
    }

    #[test]
    fn test_anthropic_roundtrip_preserves_extras() {
        let json = json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [],
            "max_tokens": 1024,
            "custom_field": {"nested": "data"}
        });

        let params: AnthropicParams = serde_json::from_value(json.clone()).unwrap();
        let back: Value = serde_json::to_value(&params).unwrap();

        // Custom field should be preserved
        assert_eq!(back.get("custom_field"), json.get("custom_field"));
    }
}
