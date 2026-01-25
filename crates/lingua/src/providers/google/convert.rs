/*!
Google format conversions.

This module provides TryFromLLM trait implementations for converting between
Google's GenerateContent API format and Lingua's universal message format.
*/

use crate::error::ConvertError;
use crate::providers::google::generated::{
    part, Blob as GoogleBlob, Content as GoogleContent, FunctionCall as GoogleFunctionCall,
    FunctionResponse as GoogleFunctionResponse, GenerateContentRequest, Part as GooglePart,
};
use crate::serde_json::{self, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::defaults::DEFAULT_MIME_TYPE;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use crate::util::media::parse_base64_data_url;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pbjson_types::Struct;

// ============================================================================
// Google Content -> Universal Message
// ============================================================================

fn part_from_data(data: part::Data) -> GooglePart {
    GooglePart {
        thought: false,
        thought_signature: Vec::new(),
        part_metadata: None,
        data: Some(data),
        metadata: None,
    }
}

fn json_to_struct(value: &Value) -> Result<Struct, ConvertError> {
    match value {
        Value::Object(_) => serde_json::from_value(value.clone()).map_err(|e| {
            ConvertError::ContentConversionFailed {
                reason: format!("Failed to convert JSON object to Struct: {e}"),
            }
        }),
        Value::Null => Ok(Struct {
            fields: Default::default(),
        }),
        _ => Err(ConvertError::ContentConversionFailed {
            reason: "Google function args/response must be a JSON object".to_string(),
        }),
    }
}

fn struct_to_json(value: &Struct) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

impl TryFromLLM<GoogleContent> for Message {
    type Error = ConvertError;

    fn try_from(content: GoogleContent) -> Result<Self, Self::Error> {
        let role = if content.role.is_empty() {
            "user"
        } else {
            content.role.as_str()
        };

        // Collect text parts
        let mut text = String::new();
        let mut function_calls = Vec::new();
        let mut function_responses = Vec::new();

        for part in &content.parts {
            if let Some(data) = &part.data {
                match data {
                    part::Data::Text(t) => text.push_str(t),
                    part::Data::FunctionCall(fc) => function_calls.push(fc.clone()),
                    part::Data::FunctionResponse(fr) => function_responses.push(fr.clone()),
                    _ => {}
                }
            }
        }

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
                let args_value = fc.args.as_ref().map(struct_to_json).unwrap_or(Value::Null);
                parts.push(AssistantContentPart::ToolCall {
                    tool_call_id: if fc.id.is_empty() {
                        fc.name.clone()
                    } else {
                        fc.id.clone()
                    },
                    tool_name: fc.name.clone(),
                    arguments: ToolCallArguments::from(
                        serde_json::to_string(&args_value).unwrap_or_default(),
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
                    let output = fr
                        .response
                        .as_ref()
                        .map(struct_to_json)
                        .unwrap_or(Value::Null);
                    ToolContentPart::ToolResult(ToolResultContentPart {
                        tool_call_id: if fr.id.is_empty() {
                            fr.name.clone()
                        } else {
                            fr.id.clone()
                        },
                        tool_name: fr.name.clone(),
                        output,
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
                    vec![part_from_data(part::Data::Text(text))],
                )
            }
            Message::User { content } => {
                let parts = match content {
                    UserContent::String(s) => vec![part_from_data(part::Data::Text(s))],
                    UserContent::Array(parts) => {
                        let mut converted = Vec::new();
                        for part in parts {
                            match part {
                                UserContentPart::Text(t) => {
                                    converted.push(part_from_data(part::Data::Text(t.text)));
                                }
                                UserContentPart::Image {
                                    image: Value::String(data),
                                    media_type,
                                    ..
                                } => {
                                    let mut inferred_media_type = None;
                                    let base64_data =
                                        if let Some(block) = parse_base64_data_url(&data) {
                                            inferred_media_type = Some(block.media_type);
                                            block.data
                                        } else {
                                            data
                                        };

                                    let mime_type = media_type
                                        .or(inferred_media_type)
                                        .unwrap_or_else(|| DEFAULT_MIME_TYPE.to_string());
                                    let bytes =
                                        STANDARD.decode(base64_data.as_bytes()).map_err(|e| {
                                            ConvertError::ContentConversionFailed {
                                                reason: format!("Invalid base64 inline image: {e}"),
                                            }
                                        })?;

                                    converted.push(part_from_data(part::Data::InlineData(
                                        GoogleBlob {
                                            mime_type,
                                            data: bytes,
                                        },
                                    )));
                                }
                                _ => {}
                            }
                        }
                        converted
                    }
                };
                ("user".to_string(), parts)
            }
            Message::Assistant { content, .. } => {
                let parts = match content {
                    AssistantContent::String(s) => vec![part_from_data(part::Data::Text(s))],
                    AssistantContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            AssistantContentPart::Text(t) => {
                                Some(part_from_data(part::Data::Text(t.text)))
                            }
                            AssistantContentPart::ToolCall {
                                tool_name,
                                arguments,
                                ..
                            } => {
                                let value = match arguments {
                                    ToolCallArguments::Valid(map) => Some(Value::Object(map)),
                                    ToolCallArguments::Invalid(s) => serde_json::from_str(&s).ok(),
                                };
                                let args = value.and_then(|v| match v {
                                    Value::Object(_) => json_to_struct(&v).ok(),
                                    _ => None,
                                });

                                Some(part_from_data(part::Data::FunctionCall(
                                    GoogleFunctionCall {
                                        id: String::new(),
                                        name: tool_name,
                                        args,
                                    },
                                )))
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
                        let response = match &result.output {
                            Value::Null => None,
                            Value::Object(_) => json_to_struct(&result.output).ok(),
                            other => {
                                let mut wrapped = serde_json::Map::new();
                                wrapped.insert("output".to_string(), other.clone());
                                json_to_struct(&Value::Object(wrapped)).ok()
                            }
                        };

                        part_from_data(part::Data::FunctionResponse(GoogleFunctionResponse {
                            id: result.tool_call_id,
                            name: result.tool_name,
                            response,
                            parts: Vec::new(),
                            will_continue: false,
                            scheduling: None,
                        }))
                    })
                    .collect();
                ("user".to_string(), parts)
            }
        };

        Ok(GoogleContent { role, parts })
    }
}

// ============================================================================
// Convenience functions using trait implementations
// ============================================================================

/// Convert Google GenerateContentRequest to universal messages.
pub fn google_to_universal(request: &GenerateContentRequest) -> Result<Vec<Message>, ConvertError> {
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
            role: "user".to_string(),
            parts: vec![part_from_data(part::Data::Text("Hello".to_string()))],
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
            role: "model".to_string(),
            parts: vec![part_from_data(part::Data::Text("Hi there!".to_string()))],
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
            role: "model".to_string(),
            parts: vec![part_from_data(part::Data::FunctionCall(
                GoogleFunctionCall {
                    id: String::new(),
                    name: "get_weather".to_string(),
                    args: Some(json_to_struct(&json!({"location": "SF"})).unwrap()),
                },
            ))],
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
        assert_eq!(content.role, "user");
        assert_eq!(content.parts.len(), 1);
        match &content.parts[0].data {
            Some(part::Data::Text(t)) => assert_eq!(t, "Hello"),
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn test_message_to_google_content_assistant() {
        let message = Message::Assistant {
            content: AssistantContent::String("Hi there!".to_string()),
            id: None,
        };

        let content = <GoogleContent as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(content.role, "model");
        assert_eq!(content.parts.len(), 1);
        match &content.parts[0].data {
            Some(part::Data::Text(t)) => assert_eq!(t, "Hi there!"),
            _ => panic!("Expected text part"),
        }
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
        assert_eq!(content.role, "model");
        assert_eq!(content.parts.len(), 1);
        match &content.parts[0].data {
            Some(part::Data::FunctionCall(fc)) => assert_eq!(fc.name, "get_weather"),
            _ => panic!("Expected function call part"),
        }
    }

    #[test]
    fn test_google_to_universal_simple() {
        let request = GenerateContentRequest {
            model: String::new(),
            system_instruction: None,
            contents: vec![GoogleContent {
                role: "user".to_string(),
                parts: vec![part_from_data(part::Data::Text("Hello".to_string()))],
            }],
            tools: Vec::new(),
            tool_config: None,
            safety_settings: Vec::new(),
            generation_config: None,
            cached_content: None,
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
