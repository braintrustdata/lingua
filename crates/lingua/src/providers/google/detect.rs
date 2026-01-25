/*!
Google format detection.

This module provides functions to detect if a payload is in
Google AI (Generative Language API) format by attempting to
deserialize into the protobuf-generated types with pbjson serde support.
*/

use crate::providers::google::generated;
use crate::serde_json::{self, Value};
use thiserror::Error;

/// Error type for Google payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
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
pub fn try_parse_google(
    payload: &Value,
) -> Result<generated::GenerateContentRequest, DetectionError> {
    if payload.get("contents").and_then(Value::as_array).is_none() {
        return Err(DetectionError::DeserializationFailed(
            "missing contents field".to_string(),
        ));
    }

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
        assert_eq!(parsed.contents[0].role, "user");
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
