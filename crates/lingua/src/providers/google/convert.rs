/*!
Google format conversions.

This module provides TryFromLLM trait implementations for converting between
Google's GenerateContent API format and Lingua's universal message format.
*/

use crate::error::ConvertError;
use crate::providers::google::detect::{
    GoogleBlob, GoogleContent, GoogleFunctionCall, GoogleFunctionResponse,
    GoogleGenerateContentRequest, GooglePart,
};
use crate::serde_json::{self, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::defaults::DEFAULT_MIME_TYPE;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};

// ============================================================================
// Google Content -> Universal Message
// ============================================================================

impl TryFromLLM<GoogleContent> for Message {
    type Error = ConvertError;

    fn try_from(content: GoogleContent) -> Result<Self, Self::Error> {
        let role = content.role.as_deref().unwrap_or("user");

        // Collect text parts
        let text_parts: Vec<String> = content
            .parts
            .iter()
            .filter_map(|part| part.text.clone())
            .collect();
        let text = text_parts.join("");

        // Check for function calls/responses
        let function_calls: Vec<_> = content
            .parts
            .iter()
            .filter_map(|part| part.function_call.as_ref())
            .collect();

        let function_responses: Vec<_> = content
            .parts
            .iter()
            .filter_map(|part| part.function_response.as_ref())
            .collect();

        if !function_calls.is_empty() {
            // Model message with function calls
            let mut parts = Vec::new();

            if !text.is_empty() {
                parts.push(AssistantContentPart::Text(TextContentPart {
                    text,
                    provider_options: None,
                }));
            }

            for fc in function_calls {
                parts.push(AssistantContentPart::ToolCall {
                    tool_call_id: fc.name.clone(), // Google uses name as ID
                    tool_name: fc.name.clone(),
                    arguments: ToolCallArguments::from(
                        serde_json::to_string(&fc.args).unwrap_or_default(),
                    ),
                    provider_options: None,
                    provider_executed: None,
                });
            }

            Ok(Message::Assistant {
                content: AssistantContent::Array(parts),
                id: None,
            })
        } else if !function_responses.is_empty() {
            // User message with function responses (tool results)
            let tool_parts: Vec<ToolContentPart> = function_responses
                .iter()
                .map(|fr| {
                    ToolContentPart::ToolResult(ToolResultContentPart {
                        tool_call_id: fr.name.clone(),
                        tool_name: fr.name.clone(),
                        output: fr.response.clone(),
                        provider_options: None,
                    })
                })
                .collect();

            Ok(Message::Tool {
                content: tool_parts,
            })
        } else {
            // Regular text message
            match role {
                "user" => Ok(Message::User {
                    content: UserContent::String(text),
                }),
                "model" => Ok(Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::Text(
                        TextContentPart {
                            text,
                            provider_options: None,
                        },
                    )]),
                    id: None,
                }),
                _ => {
                    // Treat unknown roles as user
                    Ok(Message::User {
                        content: UserContent::String(text),
                    })
                }
            }
        }
    }
}

// ============================================================================
// Universal Message -> Google Content
// ============================================================================

impl TryFromLLM<Message> for GoogleContent {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let (role, parts) = match message {
            Message::System { content } => {
                let text = match content {
                    UserContent::String(s) => format!("System: {}", s),
                    UserContent::Array(parts) => {
                        let texts: Vec<String> = parts
                            .into_iter()
                            .filter_map(|p| match p {
                                UserContentPart::Text(t) => Some(t.text),
                                _ => None,
                            })
                            .collect();
                        format!("System: {}", texts.join(""))
                    }
                };
                (
                    "user".to_string(),
                    vec![GooglePart {
                        text: Some(text),
                        inline_data: None,
                        function_call: None,
                        function_response: None,
                    }],
                )
            }
            Message::User { content } => {
                let parts = match content {
                    UserContent::String(s) => vec![GooglePart {
                        text: Some(s),
                        inline_data: None,
                        function_call: None,
                        function_response: None,
                    }],
                    UserContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            UserContentPart::Text(t) => Some(GooglePart {
                                text: Some(t.text),
                                inline_data: None,
                                function_call: None,
                                function_response: None,
                            }),
                            UserContentPart::Image {
                                image, media_type, ..
                            } => {
                                if let Value::String(data) = image {
                                    Some(GooglePart {
                                        text: None,
                                        inline_data: Some(GoogleBlob {
                                            mime_type: media_type
                                                .unwrap_or_else(|| DEFAULT_MIME_TYPE.to_string()),
                                            data,
                                        }),
                                        function_call: None,
                                        function_response: None,
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect(),
                };
                ("user".to_string(), parts)
            }
            Message::Assistant { content, .. } => {
                let parts = match content {
                    AssistantContent::String(s) => vec![GooglePart {
                        text: Some(s),
                        inline_data: None,
                        function_call: None,
                        function_response: None,
                    }],
                    AssistantContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            AssistantContentPart::Text(t) => Some(GooglePart {
                                text: Some(t.text),
                                inline_data: None,
                                function_call: None,
                                function_response: None,
                            }),
                            AssistantContentPart::ToolCall {
                                tool_name,
                                arguments,
                                ..
                            } => {
                                let args: Option<Value> = match arguments {
                                    ToolCallArguments::Valid(map) => serde_json::to_value(map).ok(),
                                    ToolCallArguments::Invalid(s) => serde_json::from_str(&s).ok(),
                                };
                                Some(GooglePart {
                                    text: None,
                                    inline_data: None,
                                    function_call: Some(GoogleFunctionCall {
                                        name: tool_name,
                                        args,
                                    }),
                                    function_response: None,
                                })
                            }
                            _ => None,
                        })
                        .collect(),
                };
                ("model".to_string(), parts)
            }
            Message::Tool { content } => {
                let parts: Vec<GooglePart> = content
                    .into_iter()
                    .map(|part| {
                        let ToolContentPart::ToolResult(result) = part;
                        GooglePart {
                            text: None,
                            inline_data: None,
                            function_call: None,
                            function_response: Some(GoogleFunctionResponse {
                                name: result.tool_name,
                                response: result.output,
                            }),
                        }
                    })
                    .collect();
                ("user".to_string(), parts)
            }
        };

        Ok(GoogleContent {
            role: Some(role),
            parts,
        })
    }
}

// ============================================================================
// Convenience functions using trait implementations
// ============================================================================

/// Convert Google GenerateContentRequest to universal messages.
pub fn google_to_universal(
    request: &GoogleGenerateContentRequest,
) -> Result<Vec<Message>, ConvertError> {
    <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(request.contents.clone())
}

/// Convert universal messages to Google contents.
pub fn universal_to_google_contents(
    messages: &[Message],
) -> Result<Vec<GoogleContent>, ConvertError> {
    <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
}

/// Convert universal messages to Google GenerateContent format as JSON Value.
///
/// This serializes the converted GoogleContent structs to JSON for use
/// in contexts where a Value is needed (e.g., when building full requests).
pub fn universal_to_google(messages: &[Message]) -> Result<Value, ConvertError> {
    let contents = universal_to_google_contents(messages)?;
    serde_json::to_value(contents).map_err(|e| ConvertError::JsonSerializationFailed {
        field: "contents".to_string(),
        error: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_google_content_to_message_user() {
        let content = GoogleContent {
            role: Some("user".to_string()),
            parts: vec![GooglePart {
                text: Some("Hello".to_string()),
                inline_data: None,
                function_call: None,
                function_response: None,
            }],
        };

        let message = <Message as TryFromLLM<GoogleContent>>::try_from(content).unwrap();
        match message {
            Message::User { content } => match content {
                UserContent::String(s) => assert_eq!(s, "Hello"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_google_content_to_message_model() {
        let content = GoogleContent {
            role: Some("model".to_string()),
            parts: vec![GooglePart {
                text: Some("Hi there!".to_string()),
                inline_data: None,
                function_call: None,
                function_response: None,
            }],
        };

        let message = <Message as TryFromLLM<GoogleContent>>::try_from(content).unwrap();
        match message {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::Text(t) => assert_eq!(t.text, "Hi there!"),
                        _ => panic!("Expected text part"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_google_content_to_message_function_call() {
        let content = GoogleContent {
            role: Some("model".to_string()),
            parts: vec![GooglePart {
                text: None,
                inline_data: None,
                function_call: Some(GoogleFunctionCall {
                    name: "get_weather".to_string(),
                    args: Some(json!({"location": "SF"})),
                }),
                function_response: None,
            }],
        };

        let message = <Message as TryFromLLM<GoogleContent>>::try_from(content).unwrap();
        match message {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::ToolCall {
                            tool_name,
                            tool_call_id,
                            ..
                        } => {
                            assert_eq!(tool_name, "get_weather");
                            assert_eq!(tool_call_id, "get_weather");
                        }
                        _ => panic!("Expected tool call part"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_message_to_google_content_user() {
        let message = Message::User {
            content: UserContent::String("Hello".to_string()),
        };

        let content = <GoogleContent as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(content.role.as_deref(), Some("user"));
        assert_eq!(content.parts.len(), 1);
        assert_eq!(content.parts[0].text.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_message_to_google_content_assistant() {
        let message = Message::Assistant {
            content: AssistantContent::String("Hi there!".to_string()),
            id: None,
        };

        let content = <GoogleContent as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(content.role.as_deref(), Some("model"));
        assert_eq!(content.parts.len(), 1);
        assert_eq!(content.parts[0].text.as_deref(), Some("Hi there!"));
    }

    #[test]
    fn test_message_to_google_content_tool_call() {
        let message = Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id: "call_123".to_string(),
                tool_name: "get_weather".to_string(),
                arguments: ToolCallArguments::from(r#"{"location":"SF"}"#.to_string()),
                provider_options: None,
                provider_executed: None,
            }]),
            id: None,
        };

        let content = <GoogleContent as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(content.role.as_deref(), Some("model"));
        assert_eq!(content.parts.len(), 1);
        assert!(content.parts[0].function_call.is_some());
        let fc = content.parts[0].function_call.as_ref().unwrap();
        assert_eq!(fc.name, "get_weather");
    }

    #[test]
    fn test_google_to_universal_simple() {
        let request = GoogleGenerateContentRequest {
            contents: vec![GoogleContent {
                role: Some("user".to_string()),
                parts: vec![GooglePart {
                    text: Some("Hello".to_string()),
                    inline_data: None,
                    function_call: None,
                    function_response: None,
                }],
            }],
            generation_config: None,
            system_instruction: None,
            safety_settings: None,
            tools: None,
        };

        let messages = google_to_universal(&request).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::User { content } => match content {
                UserContent::String(s) => assert_eq!(s, "Hello"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_universal_to_google_simple() {
        let messages = vec![Message::User {
            content: UserContent::String("Hello".to_string()),
        }];

        let result = universal_to_google(&messages).unwrap();
        let expected = json!([{
            "role": "user",
            "parts": [{"text": "Hello"}]
        }]);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_universal_to_google_with_assistant() {
        let messages = vec![
            Message::User {
                content: UserContent::String("Hello".to_string()),
            },
            Message::Assistant {
                content: AssistantContent::String("Hi there!".to_string()),
                id: None,
            },
        ];

        let result = universal_to_google(&messages).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["role"], "user");
        assert_eq!(arr[1]["role"], "model");
    }
}
