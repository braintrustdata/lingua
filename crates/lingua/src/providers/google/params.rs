/*!
Typed parameter structs for Google GenerateContent API.

These structs use `#[serde(flatten)]` to automatically capture unknown fields,
eliminating the need for explicit KNOWN_KEYS arrays.
*/

use crate::providers::google::generated::{Content, GenerationConfig, Tool};
use crate::serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Google GenerateContent API request parameters.
///
/// All known fields are explicitly typed. Unknown fields automatically
/// go into `extras` via `#[serde(flatten)]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleParams {
    // === Core fields ===
    pub model: Option<String>,
    pub contents: Option<Vec<Content>>,

    // === System prompt ===
    pub system_instruction: Option<Value>,

    // === Generation configuration ===
    pub generation_config: Option<GenerationConfig>,

    // === Safety settings ===
    pub safety_settings: Option<Value>,

    // === Tools and function calling ===
    pub tools: Option<Vec<Tool>>,
    pub tool_config: Option<Value>,

    // === Caching ===
    pub cached_content: Option<String>,

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
    fn test_google_params_known_fields() {
        let json = json!({
            "model": "gemini-pro",
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 1024
            }
        });

        let params: GoogleParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.model, Some("gemini-pro".to_string()));
        assert!(params.generation_config.is_some());
        assert!(params.extras.is_empty());
    }

    #[test]
    fn test_google_params_unknown_fields_go_to_extras() {
        let json = json!({
            "contents": [{"parts": [{"text": "Hello"}]}],
            "someFutureParam": "value"
        });

        let params: GoogleParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.extras.len(), 1);
        assert_eq!(
            params.extras.get("someFutureParam"),
            Some(&Value::String("value".to_string()))
        );
    }

    #[test]
    fn test_google_roundtrip_preserves_extras() {
        let json = json!({
            "contents": [],
            "customField": {"nested": "data"}
        });

        let params: GoogleParams = serde_json::from_value(json.clone()).unwrap();
        let back: Value = serde_json::to_value(&params).unwrap();

        // Custom field should be preserved
        assert_eq!(back.get("customField"), json.get("customField"));
    }
}
