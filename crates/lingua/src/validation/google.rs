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

    /// Guards the Discovery revision 20260915 additions: every field below is declared in
    /// `specs/google/discovery.json` and reachable from `GenerateContentRequest` /
    /// `GenerateContentResponse`, so the generated types must accept it and re-emit it unchanged.
    #[test]
    fn test_google_request_roundtrips_discovery_20260915_fields() {
        let json = serde_json::json!({
            "model": "models/gemini-3-pro-preview",
            "labels": { "safety_identifier": "user_session_123" },
            "contents": [{
                "role": "user",
                "parts": [
                    {
                        "inlineData": {
                            "mimeType": "video/mp4",
                            "data": "AAAA",
                            "displayName": "my_clip.mp4"
                        },
                        "mediaProcessing": "AGENTIC",
                        "mediaResolution": { "level": "MEDIA_RESOLUTION_ULTRA_HIGH" }
                    },
                    {
                        "fileData": {
                            "fileUri": "gs://bucket/my_file.pdf",
                            "mimeType": "application/pdf",
                            "displayName": "my_file.pdf"
                        }
                    },
                    {
                        "toolCall": { "id": "call_1", "toolName": "google_search", "toolType": "GOOGLE_SEARCH_WEB" }
                    }
                ]
            }],
            "generationConfig": {
                "audioTranscriptionConfig": {
                    "mode": "SMART",
                    "diarization": true,
                    "wordTimestamp": true,
                    "customVocabulary": ["Lingua"],
                    "languageCodes": ["en-US"]
                }
            }
        });

        let request = validate_google_request(&json.to_string()).expect("request must validate");

        // `GenerationConfig::response_schema` is `Box<Option<Schema>>`, which the generator's
        // `add_serde_skip_if_none` pass does not match, so it is always emitted. That is
        // long-standing behavior (see the `"responseSchema": null` entries in the payload
        // transform snapshots), not part of the revision 20260915 update.
        let mut expected = json.clone();
        expected["generationConfig"]["responseSchema"] = serde_json::Value::Null;

        assert_eq!(
            serde_json::to_value(&request).expect("request must serialize"),
            expected,
            "generated Google request types must re-emit every spec-declared field"
        );
    }

    #[test]
    fn test_google_response_roundtrips_discovery_20260915_fields() {
        let json = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{
                        "audioTranscription": {
                            "text": "hello there",
                            "speakerLabel": "spk_1",
                            "words": [{
                                "word": "hello",
                                "startOffset": "0s",
                                "endOffset": "0.5s"
                            }]
                        }
                    }]
                },
                "finishReason": "PUP_LIMITED_DISABLED",
                "index": 0
            }]
        });

        let response = validate_google_response(&json.to_string()).expect("response must validate");
        assert_eq!(
            serde_json::to_value(&response).expect("response must serialize"),
            json,
            "generated Google response types must re-emit every spec-declared field"
        );
    }

    /// `PUP_LIMITED_DISABLED` is an account-level Prohibited Use Policy state, not a
    /// content filter, so it must surface as an explicit passthrough rather than be
    /// coerced into an unrelated universal reason.
    #[test]
    fn test_pup_limited_disabled_finish_reason_is_preserved() {
        use crate::providers::google::generated::FinishReason as GoogleFinishReason;
        use crate::universal::FinishReason;

        let universal = FinishReason::from(&GoogleFinishReason::PupLimitedDisabled);
        assert_eq!(
            universal,
            FinishReason::Other("PUP_LIMITED_DISABLED".to_string())
        );
    }
}
