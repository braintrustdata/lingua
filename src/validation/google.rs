/*!
Google format validation.

Note: Google types are generated from protobuf and don't have serde support by default.
Validation for Google types is not yet implemented.
*/

use crate::validation::ValidationError;

/// Google protobuf types don't have serde support
pub fn validate_google_request(_json: &str) -> Result<(), ValidationError> {
    Err(ValidationError::DeserializationFailed(
        "Google protobuf types don't support JSON validation yet".to_string(),
    ))
}

/// Google protobuf types don't have serde support
pub fn validate_google_response(_json: &str) -> Result<(), ValidationError> {
    Err(ValidationError::DeserializationFailed(
        "Google protobuf types don't support JSON validation yet".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_google_request_not_supported() {
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
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("protobuf"));
    }

    #[test]
    fn test_validate_google_request_invalid() {
        let json = r#"{
            "model": "gemini-pro"
        }"#; // missing contents

        let result = validate_google_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_google_response_not_supported() {
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
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("protobuf"));
    }
}
