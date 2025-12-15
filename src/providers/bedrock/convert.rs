//! Conversions between Bedrock Converse API format and universal Message format.
//!
//! This module provides conversions for AWS Bedrock's Converse API responses
//! to the universal lingua Message format.

use crate::error::ConvertError;
use crate::serde_json::{self, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
};

use super::response::{BedrockOutputContentBlock, ConverseResponse};

/// Convert a Bedrock ConverseResponse to universal Messages.
impl TryFromLLM<ConverseResponse> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(response: ConverseResponse) -> Result<Self, Self::Error> {
        let content_parts = response
            .output
            .message
            .content
            .into_iter()
            .filter_map(|block| convert_bedrock_block_to_assistant_content(block).ok())
            .collect::<Vec<_>>();

        let content = if content_parts.is_empty() {
            AssistantContent::String(String::new())
        } else if content_parts.len() == 1 {
            // If there's only one text part, simplify to string
            match &content_parts[0] {
                AssistantContentPart::Text(text_part) => {
                    AssistantContent::String(text_part.text.clone())
                }
                _ => AssistantContent::Array(content_parts),
            }
        } else {
            AssistantContent::Array(content_parts)
        };

        Ok(vec![Message::Assistant { content, id: None }])
    }
}

/// Convert a Bedrock content block to an AssistantContentPart.
fn convert_bedrock_block_to_assistant_content(
    block: BedrockOutputContentBlock,
) -> Result<AssistantContentPart, ConvertError> {
    match block {
        BedrockOutputContentBlock::Text { text } => {
            Ok(AssistantContentPart::Text(TextContentPart {
                text,
                provider_options: None,
            }))
        }
        BedrockOutputContentBlock::ToolUse { tool_use } => {
            // Convert input to ToolCallArguments
            let arguments = match tool_use.input {
                Value::Object(map) => ToolCallArguments::Valid(map),
                other => ToolCallArguments::Invalid(other.to_string()),
            };

            Ok(AssistantContentPart::ToolCall {
                tool_call_id: tool_use.tool_use_id,
                tool_name: tool_use.name,
                arguments,
                provider_options: None,
                provider_executed: None,
            })
        }
    }
}

/// Convert a Bedrock Converse response JSON value to universal Messages.
///
/// This handles the response format from AWS Bedrock's Converse API when
/// working with raw JSON values.
pub fn bedrock_response_to_messages(response: &Value) -> Result<Vec<Message>, ConvertError> {
    let mut content_parts: Vec<AssistantContentPart> = Vec::new();

    // Try to parse as ConverseResponse first
    if let Some(output) = response.get("output") {
        if let Some(message) = output.get("message") {
            if let Some(content) = message.get("content").and_then(Value::as_array) {
                for block in content {
                    if let Some(part) = convert_json_block_to_assistant_content(block)? {
                        content_parts.push(part);
                    }
                }
            }
        }
    }

    // Fallback: check for outputText (older format)
    if content_parts.is_empty() {
        if let Some(output_text) = response.get("outputText").and_then(Value::as_str) {
            content_parts.push(AssistantContentPart::Text(TextContentPart {
                text: output_text.to_string(),
                provider_options: None,
            }));
        }
    }

    // Build the message
    let content = if content_parts.is_empty() {
        AssistantContent::String(String::new())
    } else if content_parts.len() == 1 {
        match &content_parts[0] {
            AssistantContentPart::Text(text_part) => {
                AssistantContent::String(text_part.text.clone())
            }
            _ => AssistantContent::Array(content_parts),
        }
    } else {
        AssistantContent::Array(content_parts)
    };

    Ok(vec![Message::Assistant { content, id: None }])
}

/// Convert a JSON content block to an AssistantContentPart.
fn convert_json_block_to_assistant_content(
    block: &Value,
) -> Result<Option<AssistantContentPart>, ConvertError> {
    // Check for text content
    if let Some(text) = block.get("text").and_then(Value::as_str) {
        return Ok(Some(AssistantContentPart::Text(TextContentPart {
            text: text.to_string(),
            provider_options: None,
        })));
    }

    // Check for toolUse content
    if let Some(tool_use) = block.get("toolUse") {
        let tool_use_id = tool_use
            .get("toolUseId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let name = tool_use
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let input = tool_use
            .get("input")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        let arguments = match input {
            Value::Object(map) => ToolCallArguments::Valid(map),
            other => ToolCallArguments::Invalid(other.to_string()),
        };

        return Ok(Some(AssistantContentPart::ToolCall {
            tool_call_id: tool_use_id,
            tool_name: name,
            arguments,
            provider_options: None,
            provider_executed: None,
        }));
    }

    // Unknown block type - skip
    Ok(None)
}

/// Wrapper struct for Bedrock response conversion from JSON.
pub struct BedrockResponseJson(pub Value);

impl TryFromLLM<BedrockResponseJson> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(response: BedrockResponseJson) -> Result<Self, Self::Error> {
        bedrock_response_to_messages(&response.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_simple_text_response() {
        let response = json!({
            "output": {
                "message": {
                    "role": "assistant",
                    "content": [{
                        "text": "Hello from Bedrock!"
                    }]
                }
            },
            "stopReason": "end_turn",
            "usage": {
                "inputTokens": 10,
                "outputTokens": 5,
                "totalTokens": 15
            }
        });

        let messages = bedrock_response_to_messages(&response).unwrap();
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Message::Assistant { content, .. } => match content {
                AssistantContent::String(text) => assert_eq!(text, "Hello from Bedrock!"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_tool_use_response() {
        let response = json!({
            "output": {
                "message": {
                    "role": "assistant",
                    "content": [{
                        "toolUse": {
                            "toolUseId": "tool_123",
                            "name": "get_weather",
                            "input": {
                                "location": "Seattle"
                            }
                        }
                    }]
                }
            },
            "stopReason": "tool_use",
            "usage": {
                "inputTokens": 10,
                "outputTokens": 5,
                "totalTokens": 15
            }
        });

        let messages = bedrock_response_to_messages(&response).unwrap();
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
    fn test_mixed_content_response() {
        let response = json!({
            "output": {
                "message": {
                    "role": "assistant",
                    "content": [
                        { "text": "I'll check the weather for you." },
                        {
                            "toolUse": {
                                "toolUseId": "tool_456",
                                "name": "get_weather",
                                "input": { "location": "NYC" }
                            }
                        }
                    ]
                }
            },
            "stopReason": "tool_use",
            "usage": {
                "inputTokens": 15,
                "outputTokens": 10,
                "totalTokens": 25
            }
        });

        let messages = bedrock_response_to_messages(&response).unwrap();
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 2);
                    match &parts[0] {
                        AssistantContentPart::Text(text_part) => {
                            assert_eq!(text_part.text, "I'll check the weather for you.");
                        }
                        _ => panic!("Expected text"),
                    }
                    match &parts[1] {
                        AssistantContentPart::ToolCall { tool_name, .. } => {
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
    fn test_legacy_output_text_response() {
        let response = json!({
            "outputText": "This is a legacy format response."
        });

        let messages = bedrock_response_to_messages(&response).unwrap();
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Message::Assistant { content, .. } => match content {
                AssistantContent::String(text) => {
                    assert_eq!(text, "This is a legacy format response.");
                }
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }
}
