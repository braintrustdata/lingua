/*!
Google format detection.

This module provides functions to detect if a payload is in
Google AI (Generative Language API) format by attempting to
deserialize into the generated types.
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
    fn test_try_parse_google_accepts_numeric_schema_int64_fields() {
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "tools": [{
                "functionDeclarations": [{
                    "name": "demo",
                    "description": "demo tool",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "minLength": 1,
                                "maxLength": 9
                            },
                            "tags": {
                                "type": "array",
                                "items": {"type": "string"},
                                "minItems": 1,
                                "maxItems": 3
                            }
                        }
                    }
                }]
            }]
        });

        assert!(try_parse_google(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_google_accepts_string_schema_int64_fields() {
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "tools": [{
                "functionDeclarations": [{
                    "name": "demo",
                    "description": "demo tool",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "minLength": "1"
                            }
                        }
                    }
                }]
            }]
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
        let contents = parsed.contents.unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role.as_deref(), Some("user"));
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

    #[test]
    fn test_part_media_resolution_level_ultra_high_parses() {
        // Part-level mediaResolution is the object-shaped `MediaResolution` schema, whose `Level`
        // enum carries ULTRA_HIGH. Discovery moved this schema under a version-package-prefixed
        // id; the wire contract must be unchanged by that retype.
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{
                    "text": "Describe this",
                    "mediaResolution": {"level": "MEDIA_RESOLUTION_ULTRA_HIGH"}
                }]
            }]
        });

        let parsed = try_parse_google(&payload).expect("part mediaResolution must parse");
        let contents = parsed.contents.clone().expect("contents");
        let parts = contents[0].parts.clone().expect("parts");
        let media_resolution = parts[0]
            .media_resolution
            .clone()
            .expect("mediaResolution present");
        assert_eq!(
            media_resolution.level,
            Some(generated::Level::MediaResolutionUltraHigh)
        );

        let reserialized = serde_json::to_value(&parsed).expect("re-serialize");
        assert_eq!(reserialized, payload);
    }

    #[test]
    fn test_generation_config_media_resolution_is_flat_string() {
        // generationConfig.mediaResolution is a flat enum, not the object-shaped Part-level type.
        // Naming MediaResolutionEnum here makes a future generated rename a compile-time failure.
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "generationConfig": {"mediaResolution": "MEDIA_RESOLUTION_MEDIUM"}
        });

        let parsed = try_parse_google(&payload).expect("flat mediaResolution must parse");
        let generation_config = parsed.generation_config.clone().expect("generationConfig");
        let media_resolution: Option<generated::MediaResolutionEnum> =
            generation_config.media_resolution;
        assert_eq!(
            media_resolution,
            Some(generated::MediaResolutionEnum::MediaResolutionMedium)
        );

        let reserialized = serde_json::to_value(&parsed).expect("re-serialize");
        assert_eq!(
            reserialized["generationConfig"]["mediaResolution"],
            json!("MEDIA_RESOLUTION_MEDIUM")
        );

        // The Part-level object shape is not accepted here.
        let object_shaped = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "generationConfig": {"mediaResolution": {"level": "MEDIA_RESOLUTION_MEDIUM"}}
        });
        assert!(try_parse_google(&object_shaped).is_err());
    }

    #[test]
    fn test_generation_config_media_resolution_rejects_ultra_high() {
        // Intentional asymmetry: the generationConfig enum has four values while the Part-level
        // Level enum has five. ULTRA_HIGH is only valid on a Part.
        let payload = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "generationConfig": {"mediaResolution": "MEDIA_RESOLUTION_ULTRA_HIGH"}
        });

        assert!(try_parse_google(&payload).is_err());
    }
}
