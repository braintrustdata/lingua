/*!
Typed parameter structs for Bedrock Converse API.

These structs use `#[serde(flatten)]` to automatically capture unknown fields,
eliminating the need for explicit KNOWN_KEYS arrays.
*/

use crate::providers::bedrock::request::{
    BedrockInferenceConfiguration, BedrockMessage, BedrockToolConfiguration,
};
use crate::serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bedrock Converse API request parameters.
///
/// All known fields are explicitly typed. Unknown fields automatically
/// go into `extras` via `#[serde(flatten)]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockParams {
    // === Core fields ===
    pub model_id: Option<String>,
    pub messages: Option<Vec<BedrockMessage>>,

    // === System prompt ===
    pub system: Option<Value>,

    // === Inference configuration ===
    pub inference_config: Option<BedrockInferenceConfiguration>,

    // === Tools and function calling ===
    pub tool_config: Option<BedrockToolConfiguration>,

    // === Guardrails ===
    pub guardrail_config: Option<Value>,

    // === Additional model fields ===
    pub additional_model_request_fields: Option<Value>,
    pub additional_model_response_field_paths: Option<Vec<String>>,

    // === Prompt templates ===
    pub prompt_variables: Option<Value>,

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
    fn test_bedrock_params_known_fields() {
        let json = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [{"role": "user", "content": [{"text": "Hello"}]}],
            "inferenceConfig": {
                "temperature": 0.7,
                "maxTokens": 1024
            }
        });

        let params: BedrockParams = serde_json::from_value(json).unwrap();
        assert_eq!(
            params.model_id,
            Some("anthropic.claude-3-sonnet".to_string())
        );
        assert!(params.inference_config.is_some());
        assert!(params.extras.is_empty());
    }

    #[test]
    fn test_bedrock_params_unknown_fields_go_to_extras() {
        let json = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [],
            "someFutureParam": "value"
        });

        let params: BedrockParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.extras.len(), 1);
        assert_eq!(
            params.extras.get("someFutureParam"),
            Some(&Value::String("value".to_string()))
        );
    }

    #[test]
    fn test_bedrock_roundtrip_preserves_extras() {
        let json = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [],
            "customField": {"nested": "data"}
        });

        let params: BedrockParams = serde_json::from_value(json.clone()).unwrap();
        let back: Value = serde_json::to_value(&params).unwrap();

        // Custom field should be preserved
        assert_eq!(back.get("customField"), json.get("customField"));
    }
}
