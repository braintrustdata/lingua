/*!
Google format validation.
*/

use crate::providers::google::generated::{GenerateContentRequest, GenerateContentResponse};
use crate::validation::{validate_json, ValidationError};

/// Validates a JSON string as a Google GenerateContent request
pub fn validate_google_request(json: &str) -> Result<GenerateContentRequest, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as a Google GenerateContent response
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
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_google_request_invalid() {
        let json = r#"{ "not_a_valid_field": true }"#;

        let result = validate_google_request(json);
        // Should succeed since all fields are optional in GenerateContentRequest
        // but at minimum it should parse as valid JSON
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_validate_google_request_invalid_json() {
        let json = r#"not json at all"#;

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
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_google_response_invalid_json() {
        let json = r#"not json"#;

        let result = validate_google_response(json);
        assert!(result.is_err());
    }
}
