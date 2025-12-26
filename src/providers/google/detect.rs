/*!
Google format detection.

This module provides functions to detect if a payload is in
Google AI (Generative Language API) format by attempting to
deserialize into lightweight serde structs that mirror the
protobuf types.

Note: Google's official types are protobuf-generated (prost) without
serde support. We use custom serde structs for validation.
*/

use crate::serde_json::{self, Value};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for Google payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Lightweight serde struct for Google GenerateContent request validation.
///
/// This mirrors the structure of Google's protobuf GenerateContentRequest
/// but uses serde for JSON deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleGenerateContentRequest {
    /// The content of the conversation
    pub contents: Vec<GoogleContent>,

    /// Generation configuration (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GoogleGenerationConfig>,

    /// System instruction (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GoogleContent>,

    /// Safety settings (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<GoogleSafetySetting>>,

    /// Tools for function calling (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GoogleTool>>,
}

/// Content in a Google conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleContent {
    /// Role: "user" or "model"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Content parts
    #[serde(default)]
    pub parts: Vec<GooglePart>,
}

/// A part of content (text, image, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePart {
    /// Text content
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Inline data (images, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<GoogleBlob>,

    /// Function call
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_call: Option<GoogleFunctionCall>,

    /// Function response
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_response: Option<GoogleFunctionResponse>,
}

/// Inline binary data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleBlob {
    pub mime_type: String,
    pub data: String,
}

/// Function call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleFunctionCall {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
}

/// Response to a function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleFunctionResponse {
    pub name: String,
    pub response: Value,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleGenerationConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Safety setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSafetySetting {
    pub category: String,
    pub threshold: String,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTool {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<GoogleFunctionDeclaration>>,
}

/// Function declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleFunctionDeclaration {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// Attempt to parse a JSON Value as Google GenerateContentRequest.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Google format.
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::google::detect::try_parse_google;
///
/// let google_payload = json!({
///     "contents": [{
///         "role": "user",
///         "parts": [{"text": "Hello"}]
///     }]
/// });
///
/// assert!(try_parse_google(&google_payload).is_ok());
/// ```
pub fn try_parse_google(payload: &Value) -> Result<GoogleGenerateContentRequest, DetectionError> {
    serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_try_parse_google_with_contents_and_parts() {
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }]
        });
        assert!(try_parse_google(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_google_with_generation_config() {
        let payload = json!({
            "contents": [{"parts": [{"text": "Hello"}]}],
            "generationConfig": {
                "temperature": 0.7
            }
        });
        assert!(try_parse_google(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_google_with_model_role() {
        let payload = json!({
            "contents": [
                {"role": "user", "parts": [{"text": "Hello"}]},
                {"role": "model", "parts": [{"text": "Hi there!"}]}
            ]
        });
        assert!(try_parse_google(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_google_fails_for_openai() {
        // OpenAI uses "messages" not "contents" - should fail struct deserialization
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(try_parse_google(&payload).is_err());
    }

    #[test]
    fn test_try_parse_google_empty_contents() {
        // Empty contents array is technically valid but unusual
        let payload = json!({
            "contents": []
        });
        // Empty contents array may pass validation depending on struct definition
        let _ = try_parse_google(&payload);
    }

    #[test]
    fn test_try_parse_google_success() {
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }]
        });

        let result = try_parse_google(&payload);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.contents.len(), 1);
        assert_eq!(parsed.contents[0].role.as_deref(), Some("user"));
    }

    #[test]
    fn test_try_parse_google_with_function_call() {
        let payload = json!({
            "contents": [{
                "role": "model",
                "parts": [{
                    "functionCall": {
                        "name": "get_weather",
                        "args": {"location": "SF"}
                    }
                }]
            }]
        });

        let result = try_parse_google(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_parse_google_fails_without_contents() {
        // Missing contents - required field
        let payload = json!({
            "generationConfig": {"temperature": 0.7}
        });

        let result = try_parse_google(&payload);
        assert!(result.is_err());
    }
}
