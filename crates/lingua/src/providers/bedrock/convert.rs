/*!
Bedrock Converse format conversions.

This module provides TryFromLLM trait implementations for converting between
AWS Bedrock's Converse API format and Lingua's universal message format.
*/

use crate::error::ConvertError;
use crate::import_parse::{
    try_convert_non_empty, try_parse, try_parse_vec_or_single, try_parsers_in_order, MessageParser,
};
use crate::providers::bedrock::request::{
    BedrockContentBlock, BedrockConversationRole, BedrockImageBlock, BedrockImageFormat,
    BedrockImageSource, BedrockMessage, BedrockToolResultBlock, BedrockToolResultContent,
    BedrockToolUseBlock, ConverseRequest,
};
use crate::providers::bedrock::response::{
    BedrockOutputContentBlock, BedrockOutputMessage, ConverseResponse,
};
use crate::serde_json::{self, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};

// ============================================================================
// Bedrock Message -> Universal Message
// ============================================================================

impl TryFromLLM<BedrockMessage> for Message {
    type Error = ConvertError;

    fn try_from(msg: BedrockMessage) -> Result<Self, Self::Error> {
        match msg.role {
            BedrockConversationRole::User => {
                let mut has_tool_results = false;
                let mut tool_results = Vec::new();
                let mut text_parts = Vec::new();

                for block in msg.content {
                    match block {
                        BedrockContentBlock::Text { text } => {
                            text_parts.push(text);
                        }
                        BedrockContentBlock::ToolResult { tool_result } => {
                            has_tool_results = true;
                            let output = tool_result
                                .content
                                .into_iter()
                                .next()
                                .map(|c| match c {
                                    BedrockToolResultContent::Text { text } => Value::String(text),
                                    BedrockToolResultContent::Json { json } => json,
                                    BedrockToolResultContent::Image { .. } => {
                                        Value::String("[image]".to_string())
                                    }
                                })
                                .unwrap_or(Value::Null);

                            tool_results.push(ToolContentPart::ToolResult(ToolResultContentPart {
                                tool_call_id: tool_result.tool_use_id,
                                tool_name: String::new(),
                                output,
                                provider_options: None,
                            }));
                        }
                        BedrockContentBlock::Image { .. } => {
                            // Skip images for now
                        }
                        BedrockContentBlock::ToolUse { .. } => {
                            // Tool use shouldn't be in user messages
                        }
                    }
                }

                if has_tool_results {
                    Ok(Message::Tool {
                        content: tool_results,
                    })
                } else {
                    Ok(Message::User {
                        content: UserContent::String(text_parts.join("")),
                    })
                }
            }
            BedrockConversationRole::Assistant => {
                let mut content_parts = Vec::new();

                for block in msg.content {
                    match block {
                        BedrockContentBlock::Text { text } => {
                            content_parts.push(AssistantContentPart::Text(TextContentPart {
                                text,
                                encrypted_content: None,
                                provider_options: None,
                            }));
                        }
                        BedrockContentBlock::ToolUse { tool_use } => {
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_use.tool_use_id,
                                tool_name: tool_use.name,
                                arguments: ToolCallArguments::from(
                                    serde_json::to_string(&tool_use.input).unwrap_or_default(),
                                ),
                                encrypted_content: None,
                                provider_options: None,
                                provider_executed: None,
                            });
                        }
                        _ => {}
                    }
                }

                if content_parts.is_empty() {
                    content_parts.push(AssistantContentPart::Text(TextContentPart {
                        text: String::new(),
                        encrypted_content: None,
                        provider_options: None,
                    }));
                }

                Ok(Message::Assistant {
                    content: AssistantContent::Array(content_parts),
                    id: None,
                })
            }
        }
    }
}

// ============================================================================
// Universal Message -> Bedrock Message
// ============================================================================

impl TryFromLLM<Message> for BedrockMessage {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let (role, content) = match message {
            Message::System { content } | Message::Developer { content } => {
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
                    BedrockConversationRole::User,
                    vec![BedrockContentBlock::Text { text }],
                )
            }
            Message::User { content } => {
                let blocks = match content {
                    UserContent::String(s) => vec![BedrockContentBlock::Text { text: s }],
                    UserContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            UserContentPart::Text(t) => {
                                Some(BedrockContentBlock::Text { text: t.text })
                            }
                            UserContentPart::Image {
                                image, media_type, ..
                            } => {
                                if let Value::String(data) = image {
                                    let format = media_type
                                        .as_deref()
                                        .and_then(|mt| mt.strip_prefix("image/"))
                                        .map(|f| match f {
                                            "png" => BedrockImageFormat::Png,
                                            "gif" => BedrockImageFormat::Gif,
                                            "webp" => BedrockImageFormat::Webp,
                                            _ => BedrockImageFormat::Jpeg,
                                        })
                                        .unwrap_or(BedrockImageFormat::Jpeg);
                                    Some(BedrockContentBlock::Image {
                                        image: BedrockImageBlock {
                                            format,
                                            source: BedrockImageSource { bytes: data },
                                        },
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect(),
                };
                (BedrockConversationRole::User, blocks)
            }
            Message::Assistant { content, .. } => {
                let blocks = match content {
                    AssistantContent::String(s) => vec![BedrockContentBlock::Text { text: s }],
                    AssistantContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            AssistantContentPart::Text(t) => {
                                Some(BedrockContentBlock::Text { text: t.text })
                            }
                            AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                ..
                            } => {
                                let input: Value = match arguments {
                                    ToolCallArguments::Valid(map) => serde_json::to_value(map)
                                        .unwrap_or(Value::Object(Default::default())),
                                    ToolCallArguments::Invalid(s) => serde_json::from_str(&s)
                                        .unwrap_or(Value::Object(Default::default())),
                                };
                                Some(BedrockContentBlock::ToolUse {
                                    tool_use: BedrockToolUseBlock {
                                        tool_use_id: tool_call_id,
                                        name: tool_name,
                                        input,
                                    },
                                })
                            }
                            _ => None,
                        })
                        .collect(),
                };
                (BedrockConversationRole::Assistant, blocks)
            }
            Message::Tool { content } => {
                let blocks: Vec<BedrockContentBlock> = content
                    .into_iter()
                    .map(|part| {
                        let ToolContentPart::ToolResult(result) = part;
                        let content_text = match result.output {
                            Value::String(s) => s,
                            other => serde_json::to_string(&other).unwrap_or_default(),
                        };
                        BedrockContentBlock::ToolResult {
                            tool_result: BedrockToolResultBlock {
                                tool_use_id: result.tool_call_id,
                                content: vec![BedrockToolResultContent::Text {
                                    text: content_text,
                                }],
                                status: None,
                            },
                        }
                    })
                    .collect();
                (BedrockConversationRole::User, blocks)
            }
        };

        Ok(BedrockMessage { role, content })
    }
}

// ============================================================================
// Convenience functions using trait implementations
// ============================================================================

/// Convert Bedrock ConverseRequest to universal messages.
pub fn bedrock_to_universal(request: &ConverseRequest) -> Result<Vec<Message>, ConvertError> {
    <Vec<Message> as TryFromLLM<Vec<BedrockMessage>>>::try_from(request.messages.clone())
}

/// Convert universal messages to Bedrock messages.
pub fn universal_to_bedrock_messages(
    messages: &[Message],
) -> Result<Vec<BedrockMessage>, ConvertError> {
    <Vec<BedrockMessage> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
}

/// Convert universal messages to Bedrock Converse format as JSON Value.
///
/// This serializes the converted BedrockMessage structs to JSON for use
/// in contexts where a Value is needed (e.g., when building full requests).
pub fn universal_to_bedrock(messages: &[Message]) -> Result<Value, ConvertError> {
    let bedrock_messages = universal_to_bedrock_messages(messages)?;
    serde_json::to_value(bedrock_messages).map_err(|e| ConvertError::JsonSerializationFailed {
        field: "messages".to_string(),
        error: e.to_string(),
    })
}

fn try_messages_from_bedrock_messages(messages: Vec<BedrockMessage>) -> Option<Vec<Message>> {
    try_convert_non_empty(messages)
}

fn try_message_from_bedrock_output_message(message: BedrockOutputMessage) -> Option<Vec<Message>> {
    if message.role != "assistant" {
        return None;
    }

    let message = <Message as TryFromLLM<BedrockOutputMessage>>::try_from(message).ok()?;
    Some(vec![message])
}

fn try_messages_from_bedrock_output_messages(
    output_messages: Vec<BedrockOutputMessage>,
) -> Option<Vec<Message>> {
    if output_messages
        .iter()
        .any(|message| message.role != "assistant")
    {
        return None;
    }

    try_convert_non_empty(output_messages)
}

fn try_parse_bedrock_message_for_import(data: &Value) -> Option<Vec<Message>> {
    let messages = try_parse_vec_or_single::<BedrockMessage>(data)?;
    try_messages_from_bedrock_messages(messages)
}

fn try_parse_bedrock_request_for_import(data: &Value) -> Option<Vec<Message>> {
    let request = try_parse::<ConverseRequest>(data)?;
    try_messages_from_bedrock_messages(request.messages)
}

fn try_parse_bedrock_output_message_for_import(data: &Value) -> Option<Vec<Message>> {
    let output_messages = try_parse_vec_or_single::<BedrockOutputMessage>(data)?;
    if output_messages.len() == 1 {
        return try_message_from_bedrock_output_message(output_messages.into_iter().next()?);
    }
    try_messages_from_bedrock_output_messages(output_messages)
}

fn try_parse_bedrock_response_for_import(data: &Value) -> Option<Vec<Message>> {
    let response = try_parse::<ConverseResponse>(data)?;
    try_message_from_bedrock_output_message(response.output.message)
}

pub(crate) fn try_parse_bedrock_for_import(data: &Value) -> Option<Vec<Message>> {
    const PARSERS: &[MessageParser] = &[
        try_parse_bedrock_message_for_import,
        try_parse_bedrock_request_for_import,
        try_parse_bedrock_output_message_for_import,
        try_parse_bedrock_response_for_import,
    ];
    try_parsers_in_order(data, PARSERS)
}

// ============================================================================
// BedrockOutputMessage (Response) -> Universal Message
// ============================================================================

impl TryFromLLM<BedrockOutputMessage> for Message {
    type Error = ConvertError;

    fn try_from(msg: BedrockOutputMessage) -> Result<Self, Self::Error> {
        // Output messages are always from the assistant
        let mut content_parts = Vec::new();

        for block in msg.content {
            match block {
                BedrockOutputContentBlock::Text { text } => {
                    content_parts.push(AssistantContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        provider_options: None,
                    }));
                }
                BedrockOutputContentBlock::ToolUse { tool_use } => {
                    content_parts.push(AssistantContentPart::ToolCall {
                        tool_call_id: tool_use.tool_use_id,
                        tool_name: tool_use.name,
                        arguments: ToolCallArguments::from(
                            serde_json::to_string(&tool_use.input).unwrap_or_default(),
                        ),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: None,
                    });
                }
            }
        }

        if content_parts.is_empty() {
            content_parts.push(AssistantContentPart::Text(TextContentPart {
                text: String::new(),
                encrypted_content: None,
                provider_options: None,
            }));
        }

        Ok(Message::Assistant {
            content: AssistantContent::Array(content_parts),
            id: None,
        })
    }
}

// ============================================================================
// Universal Message -> BedrockOutputMessage (Response)
// ============================================================================

impl TryFromLLM<Message> for BedrockOutputMessage {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::Assistant { content, .. } => {
                let blocks = match content {
                    AssistantContent::String(s) => {
                        vec![BedrockOutputContentBlock::Text { text: s }]
                    }
                    AssistantContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            AssistantContentPart::Text(t) => {
                                Some(BedrockOutputContentBlock::Text { text: t.text })
                            }
                            AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                ..
                            } => {
                                use crate::providers::bedrock::response::BedrockOutputToolUse;
                                let input: Value = match arguments {
                                    ToolCallArguments::Valid(map) => serde_json::to_value(map)
                                        .unwrap_or(Value::Object(Default::default())),
                                    ToolCallArguments::Invalid(s) => serde_json::from_str(&s)
                                        .unwrap_or(Value::Object(Default::default())),
                                };
                                Some(BedrockOutputContentBlock::ToolUse {
                                    tool_use: BedrockOutputToolUse {
                                        tool_use_id: tool_call_id,
                                        name: tool_name,
                                        input,
                                    },
                                })
                            }
                            _ => None,
                        })
                        .collect(),
                };
                Ok(BedrockOutputMessage {
                    role: "assistant".to_string(),
                    content: blocks,
                })
            }
            _ => Err(ConvertError::UnsupportedInputType {
                type_info: "Only Assistant messages can be converted to BedrockOutputMessage"
                    .to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_bedrock_message_to_universal_user() {
        let msg = BedrockMessage {
            role: BedrockConversationRole::User,
            content: vec![BedrockContentBlock::Text {
                text: "Hello".to_string(),
            }],
        };

        let message = <Message as TryFromLLM<BedrockMessage>>::try_from(msg).unwrap();
        match message {
            Message::User { content } => match content {
                UserContent::String(s) => assert_eq!(s, "Hello"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_bedrock_message_to_universal_assistant() {
        let msg = BedrockMessage {
            role: BedrockConversationRole::Assistant,
            content: vec![BedrockContentBlock::Text {
                text: "Hi there!".to_string(),
            }],
        };

        let message = <Message as TryFromLLM<BedrockMessage>>::try_from(msg).unwrap();
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
    fn test_bedrock_message_to_universal_tool_use() {
        let msg = BedrockMessage {
            role: BedrockConversationRole::Assistant,
            content: vec![BedrockContentBlock::ToolUse {
                tool_use: BedrockToolUseBlock {
                    tool_use_id: "tool_123".to_string(),
                    name: "get_weather".to_string(),
                    input: json!({"location": "SF"}),
                },
            }],
        };

        let message = <Message as TryFromLLM<BedrockMessage>>::try_from(msg).unwrap();
        match message {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::ToolCall {
                            tool_call_id,
                            tool_name,
                            ..
                        } => {
                            assert_eq!(tool_call_id, "tool_123");
                            assert_eq!(tool_name, "get_weather");
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
    fn test_message_to_bedrock_user() {
        let message = Message::User {
            content: UserContent::String("Hello".to_string()),
        };

        let msg = <BedrockMessage as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(msg.role, BedrockConversationRole::User);
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            BedrockContentBlock::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_message_to_bedrock_assistant() {
        let message = Message::Assistant {
            content: AssistantContent::String("Hi there!".to_string()),
            id: None,
        };

        let msg = <BedrockMessage as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(msg.role, BedrockConversationRole::Assistant);
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            BedrockContentBlock::Text { text } => assert_eq!(text, "Hi there!"),
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_message_to_bedrock_tool_call() {
        let message = Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id: "tool_123".to_string(),
                tool_name: "get_weather".to_string(),
                arguments: ToolCallArguments::from(r#"{"location":"SF"}"#.to_string()),
                encrypted_content: None,
                provider_options: None,
                provider_executed: None,
            }]),
            id: None,
        };

        let msg = <BedrockMessage as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(msg.role, BedrockConversationRole::Assistant);
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            BedrockContentBlock::ToolUse { tool_use } => {
                assert_eq!(tool_use.tool_use_id, "tool_123");
                assert_eq!(tool_use.name, "get_weather");
            }
            _ => panic!("Expected tool use block"),
        }
    }

    #[test]
    fn test_bedrock_to_universal_simple() {
        let request = ConverseRequest {
            model_id: "test-model".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::User,
                content: vec![BedrockContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            }],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let messages = bedrock_to_universal(&request).unwrap();
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
    fn test_bedrock_to_universal_with_tool_use() {
        let request = ConverseRequest {
            model_id: "test-model".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::Assistant,
                content: vec![BedrockContentBlock::ToolUse {
                    tool_use: BedrockToolUseBlock {
                        tool_use_id: "tool_123".to_string(),
                        name: "get_weather".to_string(),
                        input: json!({"location": "SF"}),
                    },
                }],
            }],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let messages = bedrock_to_universal(&request).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::ToolCall {
                            tool_call_id,
                            tool_name,
                            ..
                        } => {
                            assert_eq!(tool_call_id, "tool_123");
                            assert_eq!(tool_name, "get_weather");
                        }
                        _ => panic!("Expected tool call"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_universal_to_bedrock_simple() {
        let messages = vec![Message::User {
            content: UserContent::String("Hello".to_string()),
        }];

        let result = universal_to_bedrock(&messages).unwrap();
        let expected = json!([{
            "role": "user",
            "content": [{"text": "Hello"}]
        }]);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_universal_to_bedrock_with_tool_call() {
        let messages = vec![Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id: "tool_123".to_string(),
                tool_name: "get_weather".to_string(),
                arguments: ToolCallArguments::from(r#"{"location":"SF"}"#.to_string()),
                encrypted_content: None,
                provider_options: None,
                provider_executed: None,
            }]),
            id: None,
        }];

        let result = universal_to_bedrock(&messages).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["role"], "assistant");
        assert!(arr[0]["content"][0].get("toolUse").is_some());
    }
}
