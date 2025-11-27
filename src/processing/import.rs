use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::convert::TryFromLLM;
use crate::universal::Message;
use crate::universal::{AssistantContent, TextContentPart, UserContent, UserContentPart};
use serde::{Deserialize, Serialize};

/// Represents a minimal span structure with input/output fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(flatten)]
    pub other: serde_json::Map<String, Value>,
}

/// Cheap check to see if a value looks like it might contain messages
/// Returns early to avoid expensive deserialization attempts on non-message data
fn has_message_structure(data: &Value) -> bool {
    match data {
        // Check if it's an array where first element has "role" field
        Value::Array(arr) => {
            if arr.is_empty() {
                return false;
            }
            if let Some(Value::Object(first)) = arr.first() {
                return first.contains_key("role");
            }
            false
        }
        // Check if it's an object with "role" field (single message)
        Value::Object(obj) => obj.contains_key("role"),
        _ => false,
    }
}

/// Try to convert a value to lingua messages by attempting multiple format conversions
fn try_converting_to_messages(data: &Value) -> Vec<Message> {
    // Early bailout: if data doesn't have message structure, skip expensive deserializations
    if !has_message_structure(data) {
        // Still try nested object search (for wrapped messages like {messages: [...]})
        if let Value::Object(obj) = data {
            for key in [
                "messages", "input", "output", "choices", "result", "response",
            ] {
                if let Some(nested) = obj.get(key) {
                    let nested_messages = try_converting_to_messages(nested);
                    if !nested_messages.is_empty() {
                        return nested_messages;
                    }
                }
            }
        }
        return Vec::new();
    }

    // Try Chat Completions format (most common)
    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<openai::ChatCompletionRequestMessage>>(data.clone())
    {
        if let Ok(messages) = <Vec<Message> as TryFromLLM<
            Vec<openai::ChatCompletionRequestMessage>,
        >>::try_from(provider_messages)
        {
            if !messages.is_empty() {
                return messages;
            }
        }
    }

    // Try Responses API format
    if let Ok(provider_messages) = serde_json::from_value::<Vec<openai::InputItem>>(data.clone()) {
        if let Ok(messages) =
            <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(provider_messages)
        {
            if !messages.is_empty() {
                return messages;
            }
        }
    }

    // Try Anthropic format
    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<anthropic::InputMessage>>(data.clone())
    {
        if let Ok(messages) =
            <Vec<Message> as TryFromLLM<Vec<anthropic::InputMessage>>>::try_from(provider_messages)
        {
            if !messages.is_empty() {
                return messages;
            }
        }
    }

    // Try lenient parsing for non-standard message formats
    if let Some(lenient_messages) = try_lenient_message_parsing(data) {
        if !lenient_messages.is_empty() {
            return lenient_messages;
        }
    }

    Vec::new()
}

/// Lenient message parser for messages that don't match strict provider schemas
///
/// This parser looks for basic message structure: { "role": "...", "content": "..." }
/// without requiring strict schema validation. This helps capture messages from:
/// - Custom LLM wrappers
/// - Logging that doesn't perfectly match provider formats
/// - Messages with extra/missing fields
fn try_lenient_message_parsing(data: &Value) -> Option<Vec<Message>> {
    // Only process arrays
    let arr = data.as_array()?;
    let mut messages = Vec::new();

    for item in arr {
        let obj = item.as_object()?;

        // Must have a "role" field
        let role_str = obj.get("role")?.as_str()?;

        // Must have a "content" field
        let content_value = obj.get("content")?;

        // Create message based on role
        let message = match role_str {
            "user" => {
                let content = parse_user_content(content_value)?;
                Message::User { content }
            }
            "system" => {
                let content = parse_user_content(content_value)?;
                Message::System { content }
            }
            "assistant" => {
                let content = parse_assistant_content(content_value)?;
                Message::Assistant { content, id: None }
            }
            _ => continue, // Skip unknown roles (including "tool" for now)
        };

        messages.push(message);
    }

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

/// Parse user/system content from JSON value
fn parse_user_content(value: &Value) -> Option<UserContent> {
    match value {
        Value::String(s) => Some(UserContent::String(s.clone())),
        Value::Array(arr) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(Value::String(text_type)) = obj.get("type") {
                        if text_type == "text" {
                            if let Some(Value::String(text)) = obj.get("text") {
                                parts.push(UserContentPart::Text(TextContentPart {
                                    text: text.clone(),
                                    provider_options: None,
                                }));
                            }
                        }
                    }
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(UserContent::Array(parts))
            }
        }
        _ => None,
    }
}

/// Parse assistant content from JSON value
fn parse_assistant_content(value: &Value) -> Option<AssistantContent> {
    match value {
        Value::String(s) => Some(AssistantContent::String(s.clone())),
        Value::Array(arr) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(Value::String(text_type)) = obj.get("type") {
                        if text_type == "text" {
                            if let Some(Value::String(text)) = obj.get("text") {
                                parts.push(crate::universal::AssistantContentPart::Text(
                                    TextContentPart {
                                        text: text.clone(),
                                        provider_options: None,
                                    },
                                ));
                            }
                        }
                    }
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(AssistantContent::Array(parts))
            }
        }
        _ => None,
    }
}

/// Import messages from a list of spans
///
/// This function processes spans and extracts messages from their input/output fields,
/// attempting to convert them from various provider formats to the lingua format.
pub fn import_messages_from_spans(spans: Vec<Span>) -> Vec<Message> {
    let mut messages = Vec::new();

    for span in spans {
        // Try to extract messages from input
        if let Some(input) = &span.input {
            let input_messages = try_converting_to_messages(input);
            messages.extend(input_messages);
        }

        // Try to extract messages from output
        if let Some(output) = &span.output {
            let output_messages = try_converting_to_messages(output);
            messages.extend(output_messages);
        }
    }

    messages
}

/// Import and deduplicate messages from spans in a single operation
pub fn import_and_deduplicate_messages(spans: Vec<Span>) -> Vec<Message> {
    let messages = import_messages_from_spans(spans);
    super::dedup::deduplicate_messages(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_empty_spans() {
        let spans = vec![];
        let messages = import_messages_from_spans(spans);
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_import_spans_with_no_messages() {
        let span = Span {
            input: Some(serde_json::json!({"random": "data"})),
            output: None,
            other: serde_json::Map::new(),
        };
        let messages = import_messages_from_spans(vec![span]);
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_import_spans_with_chat_completion_messages() {
        let span = Span {
            input: Some(serde_json::json!([
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there"}
            ])),
            output: None,
            other: serde_json::Map::new(),
        };
        let messages = import_messages_from_spans(vec![span]);
        assert_eq!(messages.len(), 2);
    }
}
