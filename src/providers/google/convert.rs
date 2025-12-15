//! Conversions between Google AI format and universal Message format.
//!
//! This module provides conversions for Google's GenerateContent API responses
//! to the universal lingua Message format.

use crate::error::ConvertError;
use crate::serde_json::{self, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
};

/// Convert a Google GenerateContentResponse JSON value to universal Messages.
///
/// This handles the response format from Google's GenerateContent API.
pub fn google_response_to_messages(response: &Value) -> Result<Vec<Message>, ConvertError> {
    let mut messages = Vec::new();

    // Get candidates array from response
    let candidates = response
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| ConvertError::MissingRequiredField {
            field: "candidates".to_string(),
        })?;

    for candidate in candidates {
        if let Some(content) = candidate.get("content") {
            if let Some(message) = convert_google_content_to_message(content)? {
                messages.push(message);
            }
        }
    }

    // If no messages were extracted, return a default empty assistant message
    if messages.is_empty() {
        messages.push(Message::Assistant {
            content: AssistantContent::String(String::new()),
            id: None,
        });
    }

    Ok(messages)
}

/// Convert a single Google Content object to a universal Message.
fn convert_google_content_to_message(content: &Value) -> Result<Option<Message>, ConvertError> {
    let role = content
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("model");

    let parts = content.get("parts").and_then(Value::as_array);

    match role {
        "model" => {
            let mut content_parts: Vec<AssistantContentPart> = Vec::new();

            if let Some(parts_array) = parts {
                for part in parts_array {
                    if let Some(content_part) = convert_google_part_to_assistant_content(part)? {
                        content_parts.push(content_part);
                    }
                }
            }

            if content_parts.is_empty() {
                Ok(Some(Message::Assistant {
                    content: AssistantContent::String(String::new()),
                    id: None,
                }))
            } else if content_parts.len() == 1 {
                // If there's only one text part, simplify to string
                match &content_parts[0] {
                    AssistantContentPart::Text(text_part) => Ok(Some(Message::Assistant {
                        content: AssistantContent::String(text_part.text.clone()),
                        id: None,
                    })),
                    _ => Ok(Some(Message::Assistant {
                        content: AssistantContent::Array(content_parts),
                        id: None,
                    })),
                }
            } else {
                Ok(Some(Message::Assistant {
                    content: AssistantContent::Array(content_parts),
                    id: None,
                }))
            }
        }
        _ => {
            // Skip non-model roles in response
            Ok(None)
        }
    }
}

/// Convert a Google Part to an AssistantContentPart.
fn convert_google_part_to_assistant_content(
    part: &Value,
) -> Result<Option<AssistantContentPart>, ConvertError> {
    // Check for text content
    if let Some(text) = part.get("text").and_then(Value::as_str) {
        // Check if this is a "thought" part (reasoning)
        let is_thought = part.get("thought").and_then(Value::as_bool).unwrap_or(false);

        if is_thought {
            return Ok(Some(AssistantContentPart::Reasoning {
                text: text.to_string(),
                encrypted_content: None,
            }));
        }

        return Ok(Some(AssistantContentPart::Text(TextContentPart {
            text: text.to_string(),
            provider_options: None,
        })));
    }

    // Check for function call
    if let Some(function_call) = part.get("functionCall") {
        let name = function_call
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let id = function_call
            .get("id")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("call_{}", uuid_v4_simple()));

        let args = function_call
            .get("args")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        // Convert args to ToolCallArguments
        let arguments = match args {
            Value::Object(map) => {
                // Convert serde_json::Map to the format expected by ToolCallArguments
                let converted: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .map(|(k, v)| (k, v))
                    .collect();
                ToolCallArguments::Valid(converted)
            }
            _ => ToolCallArguments::Invalid(args.to_string()),
        };

        return Ok(Some(AssistantContentPart::ToolCall {
            tool_call_id: id,
            tool_name: name,
            arguments,
            provider_options: None,
            provider_executed: None,
        }));
    }

    // Check for executable code (code interpreter)
    if let Some(executable_code) = part.get("executableCode") {
        let code = executable_code
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let language = executable_code
            .get("language")
            .and_then(Value::as_str)
            .unwrap_or("PYTHON");

        // Represent executable code as text with code block
        return Ok(Some(AssistantContentPart::Text(TextContentPart {
            text: format!("```{}\n{}\n```", language.to_lowercase(), code),
            provider_options: None,
        })));
    }

    // Check for code execution result
    if let Some(code_result) = part.get("codeExecutionResult") {
        let output = code_result
            .get("output")
            .and_then(Value::as_str)
            .unwrap_or_default();

        // Represent code execution result as text
        return Ok(Some(AssistantContentPart::Text(TextContentPart {
            text: format!("Code output:\n{}", output),
            provider_options: None,
        })));
    }

    // Unknown part type - skip
    Ok(None)
}

/// Generate a simple UUID-like string for tool call IDs.
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", timestamp)
}

/// Wrapper struct for Google response conversion from JSON.
pub struct GoogleResponseJson(pub Value);

impl TryFromLLM<GoogleResponseJson> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(response: GoogleResponseJson) -> Result<Self, Self::Error> {
        google_response_to_messages(&response.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_simple_text_response() {
        let response = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "Hello, world!"
                    }]
                }
            }]
        });

        let messages = google_response_to_messages(&response).unwrap();
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Message::Assistant { content, .. } => match content {
                AssistantContent::String(text) => assert_eq!(text, "Hello, world!"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_function_call_response() {
        let response = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{
                        "functionCall": {
                            "name": "get_weather",
                            "id": "call_123",
                            "args": {
                                "location": "NYC"
                            }
                        }
                    }]
                }
            }]
        });

        let messages = google_response_to_messages(&response).unwrap();
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
                            assert_eq!(tool_call_id, "call_123");
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
    fn test_thought_response() {
        let response = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "Let me think about this...",
                        "thought": true
                    }, {
                        "text": "The answer is 42."
                    }]
                }
            }]
        });

        let messages = google_response_to_messages(&response).unwrap();
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 2);
                    match &parts[0] {
                        AssistantContentPart::Reasoning { text, .. } => {
                            assert_eq!(text, "Let me think about this...");
                        }
                        _ => panic!("Expected reasoning"),
                    }
                    match &parts[1] {
                        AssistantContentPart::Text(text_part) => {
                            assert_eq!(text_part.text, "The answer is 42.");
                        }
                        _ => panic!("Expected text"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }
}

