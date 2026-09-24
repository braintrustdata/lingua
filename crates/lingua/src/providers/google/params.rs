/*!
Typed parameter structs for Google GenerateContent API.

These structs use `#[serde(flatten)]` to automatically capture unknown fields,
eliminating the need for explicit KNOWN_KEYS arrays.
*/

use crate::providers::google::generated::{
    Content, GenerationConfig, ServiceTier, Tool, ToolConfig,
};
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
    pub tool_config: Option<ToolConfig>,

    // === Caching ===
    pub cached_content: Option<String>,

    // === Service tier ===
    pub service_tier: Option<ServiceTier>,

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

    /// Typed view of the request tagging map added in Discovery revision 20260915.
    #[derive(Debug, Deserialize, PartialEq)]
    struct GoogleLabelsView {
        labels: BTreeMap<String, String>,
    }

    fn expected_labels() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("env".to_string(), "prod".to_string()),
            ("team".to_string(), "search".to_string()),
        ])
    }

    /// `GenerateContentRequest.labels` is deliberately not a named `GoogleParams` field, so the
    /// flattened extras map carries it losslessly. Promoting it to a typed member later must not
    /// silently drop it from the round trip.
    #[test]
    fn test_google_params_captures_labels_in_extras() {
        let json = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "labels": {"team": "search", "env": "prod"}
        });

        let params: GoogleParams = serde_json::from_value(json).unwrap();
        assert!(
            params.extras.contains_key("labels"),
            "labels must fall through serde(flatten) into extras"
        );

        let back: GoogleLabelsView = serde_json::from_value(serde_json::to_value(&params).unwrap())
            .expect("labels must be re-emitted at the top level");
        assert_eq!(back.labels, expected_labels());
    }
}
