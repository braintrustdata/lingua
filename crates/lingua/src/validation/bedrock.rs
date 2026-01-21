/*!
Bedrock format validation.
*/

use crate::providers::bedrock::request::ConverseRequest;
use crate::providers::bedrock::response::ConverseResponse;
use crate::validation::{validate_json, ValidationError};

/// Validates a JSON string as a Bedrock Converse request
pub fn validate_bedrock_request(json: &str) -> Result<ConverseRequest, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as a Bedrock Converse response
pub fn validate_bedrock_response(json: &str) -> Result<ConverseResponse, ValidationError> {
    validate_json(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_bedrock_request_minimal() {
        // Uses camelCase keys to match Bedrock API format
        let json = r#"{
            "modelId": "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "text": "Hello"
                        }
                    ]
                }
            ]
        }"#;

        let result = validate_bedrock_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_bedrock_request_invalid() {
        let json = r#"{
            "modelId": "anthropic.claude-3-5-sonnet-20241022-v2:0"
        }"#; // missing messages

        let result = validate_bedrock_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_bedrock_response_minimal() {
        // Uses camelCase keys to match Bedrock API format
        let json = r#"{
            "output": {
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "text": "Hello!"
                        }
                    ]
                }
            },
            "stopReason": "end_turn",
            "usage": {
                "inputTokens": 10,
                "outputTokens": 20,
                "totalTokens": 30
            },
            "metrics": {
                "latencyMs": 1000
            }
        }"#;

        let result = validate_bedrock_response(json);
        assert!(result.is_ok());
    }
}
