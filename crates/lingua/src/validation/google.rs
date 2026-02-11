/*!
Google format validation.

Uses the generated pbjson serde implementations for GenerateContentRequest
and GenerateContentResponse to validate JSON payloads.
*/

use crate::providers::google::generated::{GenerateContentRequest, GenerateContentResponse};
use crate::validation::{validate_json, ValidationError};

/// Validates a JSON string as a Google GenerateContentRequest.
pub fn validate_google_request(json: &str) -> Result<GenerateContentRequest, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as a Google GenerateContentResponse.
pub fn validate_google_response(json: &str) -> Result<GenerateContentResponse, ValidationError> {
    validate_json(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_google_request_minimal() {
        let json = r#"{
            "contents": [
                {
                    "role": "user",
                    "parts": [
                        {
                            "text": "Hello"
                        }
                    ]
                }
            ]
        }"#;

        let result = validate_google_request(json);
        assert!(result.is_ok(), "Expected OK, got: {:?}", result.err());
    }

    #[test]
    fn test_validate_google_request_with_model_and_config() {
        let json = r#"{
            "model": "gemini-2.5-flash",
            "contents": [
                {
                    "role": "user",
                    "parts": [
                        {
                            "text": "Hello"
                        }
                    ]
                }
            ],
            "generationConfig": {
                "maxOutputTokens": 100
            }
        }"#;

        let result = validate_google_request(json);
        assert!(result.is_ok(), "Expected OK, got: {:?}", result.err());
    }

    #[test]
    fn test_validate_google_request_with_system_instruction() {
        let json = r#"{
            "contents": [
                {
                    "role": "user",
                    "parts": [{"text": "What is your role?"}]
                }
            ],
            "systemInstruction": {
                "parts": [{"text": "You are a helpful assistant."}]
            }
        }"#;

        let result = validate_google_request(json);
        assert!(result.is_ok(), "Expected OK, got: {:?}", result.err());
    }

    #[test]
    fn test_validate_google_request_invalid_json() {
        let json = r#"not valid json"#;
        let result = validate_google_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_google_response_minimal() {
        let json = r#"{
            "candidates": [
                {
                    "content": {
                        "role": "model",
                        "parts": [
                            {
                                "text": "Hello!"
                            }
                        ]
                    },
                    "finishReason": "STOP",
                    "index": 0
                }
            ],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 10,
                "totalTokenCount": 15
            }
        }"#;

        let result = validate_google_response(json);
        assert!(result.is_ok(), "Expected OK, got: {:?}", result.err());
    }

    #[test]
    fn test_validate_google_response_invalid_json() {
        let json = r#"not valid json"#;
        let result = validate_google_response(json);
        assert!(result.is_err());
    }
}
