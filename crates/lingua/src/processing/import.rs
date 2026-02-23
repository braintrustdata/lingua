use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::convert::ChatCompletionRequestMessageExt;
use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::convert::TryFromLLM;
use crate::universal::Message;
use crate::universal::{
    AssistantContent, AssistantContentPart, TextContentPart, ToolCallArguments, ToolContent,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
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

fn is_openai_responses_item_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "message"
            | "reasoning"
            | "function_call"
            | "function_call_output"
            | "function_call_result"
            | "web_search_call"
            | "file_search_call"
            | "computer_call"
            | "image_generation_call"
            | "code_interpreter_call"
            | "local_shell_call"
            | "mcp_call"
            | "mcp_list_tools"
            | "mcp_approval_request"
            | "custom_tool_call"
            | "custom_tool_call_output"
    )
}

fn normalize_openai_responses_items(data: &Value) -> Option<Value> {
    let arr = data.as_array()?;
    let mut changed = false;
    let mut normalized = Vec::with_capacity(arr.len());

    for item in arr {
        let Some(obj) = item.as_object() else {
            normalized.push(item.clone());
            continue;
        };

        let mut map = obj.clone();

        if map.contains_key("callId") && !map.contains_key("call_id") {
            if let Some(call_id) = map.get("callId").cloned() {
                map.insert("call_id".to_string(), call_id);
                changed = true;
            }
        }

        if let Some(Value::String(item_type)) = map.get("type") {
            if item_type == "function_call_result" {
                map.insert(
                    "type".to_string(),
                    Value::String("function_call_output".to_string()),
                );
                changed = true;
            }
        }

        let item_type = map
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_string);

        let is_function_output = matches!(
            item_type.as_deref(),
            Some("function_call_output") | Some("custom_tool_call_output")
        );
        if is_function_output {
            if let Some(output_value) = map.get("output") {
                if !output_value.is_string() && !output_value.is_null() {
                    let output_str = serde_json::to_string(output_value).ok()?;
                    map.insert("output".to_string(), Value::String(output_str));
                    changed = true;
                }
            }
        }

        // Responses image generation results may be structured attachment objects in traces,
        // but the typed schema expects `result` as a string.
        if matches!(item_type.as_deref(), Some("image_generation_call")) {
            if let Some(result_value) = map.get("result") {
                if !result_value.is_string() && !result_value.is_null() {
                    let result_str = serde_json::to_string(result_value).ok()?;
                    map.insert("result".to_string(), Value::String(result_str));
                    changed = true;
                }
            }
        }

        normalized.push(Value::Object(map));
    }

    if changed {
        Some(Value::Array(normalized))
    } else {
        None
    }
}

/// Cheap check to see if a value looks like it might contain messages
/// Returns early to avoid expensive deserialization attempts on non-message data
fn has_message_structure(data: &Value) -> bool {
    match data {
        // Check if it's an array where ANY element has "role" field or is a choice object
        Value::Array(arr) => {
            if arr.is_empty() {
                return false;
            }
            // Check if ANY element in the array looks like a message (not just the first)
            // This handles mixed-type arrays from Responses API
            for item in arr {
                if let Value::Object(obj) = item {
                    // Direct message format: has "role" field
                    if obj.contains_key("role") {
                        return true;
                    }
                    // Chat completions response choices format: has "message" field with role inside
                    if let Some(Value::Object(msg)) = obj.get("message") {
                        if msg.contains_key("role") {
                            return true;
                        }
                    }
                    // OpenAI Responses items can be tool calls/results/reasoning without a role field
                    if let Some(item_type) = obj.get("type").and_then(Value::as_str) {
                        if is_openai_responses_item_type(item_type) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        // Check if it's an object with "role" field (single message)
        Value::Object(obj) => {
            obj.contains_key("role")
                || obj
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(is_openai_responses_item_type)
        }
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
                "messages", "prompt", "input", "output", "choices", "result", "response",
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

    // If data is a single message object (not an array), wrap it in an array for parsing
    let wrapped;
    let data_to_parse = if let Value::Object(obj) = data {
        if obj.contains_key("role") {
            wrapped = Value::Array(vec![data.clone()]);
            &wrapped
        } else {
            data
        }
    } else {
        data
    };

    // Try Chat Completions format (most common)
    // Use extended type to capture reasoning field from vLLM/OpenRouter convention
    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<ChatCompletionRequestMessageExt>>(data_to_parse.clone())
    {
        if let Ok(messages) =
            <Vec<Message> as TryFromLLM<Vec<ChatCompletionRequestMessageExt>>>::try_from(
                provider_messages,
            )
        {
            if !messages.is_empty() {
                return messages;
            }
        }
    }

    let normalized_openai = normalize_openai_responses_items(data_to_parse);

    // Try Responses API format
    for candidate in std::iter::once(data_to_parse).chain(normalized_openai.as_ref()) {
        if let Ok(provider_messages) =
            serde_json::from_value::<Vec<openai::InputItem>>(candidate.clone())
        {
            if let Ok(messages) =
                <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(provider_messages)
            {
                if !messages.is_empty() {
                    return messages;
                }
            }
        }
    }

    // Try Responses API output format
    for candidate in std::iter::once(data_to_parse).chain(normalized_openai.as_ref()) {
        if let Ok(provider_messages) =
            serde_json::from_value::<Vec<openai::OutputItem>>(candidate.clone())
        {
            if let Ok(messages) =
                <Vec<Message> as TryFromLLM<Vec<openai::OutputItem>>>::try_from(provider_messages)
            {
                if !messages.is_empty() {
                    return messages;
                }
            }
        }
    }

    // Try Anthropic format
    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<anthropic::InputMessage>>(data_to_parse.clone())
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
    if let Some(lenient_messages) = try_lenient_message_parsing(data_to_parse) {
        if !lenient_messages.is_empty() {
            return lenient_messages;
        }
    }

    // Try parsing as choices array (Chat Completions response format)
    // This handles [{"finish_reason": "stop", "message": {"role": "assistant", ...}}]
    if let Some(choices_messages) = try_choices_array_parsing(data_to_parse) {
        if !choices_messages.is_empty() {
            return choices_messages;
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
fn parse_lenient_message_item(item: &Value) -> Option<Message> {
    let obj = item.as_object()?;
    let role_str = obj.get("role")?.as_str()?;
    let content_value = obj.get("content")?;

    match role_str {
        "user" => Some(Message::User {
            content: parse_user_content(content_value)?,
        }),
        "system" => Some(Message::System {
            content: parse_user_content(content_value)?,
        }),
        "developer" => Some(Message::Developer {
            content: parse_user_content(content_value)?,
        }),
        "assistant" => Some(Message::Assistant {
            content: parse_assistant_content(content_value)?,
            id: None,
        }),
        "tool" => Some(Message::Tool {
            content: parse_tool_content(content_value)?,
        }),
        _ => None,
    }
}

fn try_lenient_message_parsing(data: &Value) -> Option<Vec<Message>> {
    let arr = data.as_array()?;
    let mut messages = Vec::new();

    for item in arr {
        if let Some(message) = parse_lenient_message_item(item) {
            messages.push(message);
        }
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
                        if matches!(text_type.as_str(), "text" | "input_text" | "output_text") {
                            if let Some(Value::String(text)) = obj.get("text") {
                                parts.push(UserContentPart::Text(TextContentPart {
                                    text: text.clone(),
                                    encrypted_content: None,
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
                        if matches!(text_type.as_str(), "text" | "output_text" | "input_text") {
                            if let Some(Value::String(text)) = obj.get("text") {
                                parts.push(crate::universal::AssistantContentPart::Text(
                                    TextContentPart {
                                        text: text.clone(),
                                        encrypted_content: None,
                                        provider_options: None,
                                    },
                                ));
                            }
                        } else if text_type == "tool-call" {
                            let tool_call_id = obj.get("toolCallId")?.as_str()?.to_string();
                            let tool_name = obj
                                .get("toolName")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let arguments = match obj.get("input") {
                                Some(Value::Object(map)) => ToolCallArguments::Valid(map.clone()),
                                Some(Value::String(s)) => ToolCallArguments::Invalid(s.clone()),
                                Some(other) => ToolCallArguments::Invalid(
                                    serde_json::to_string(other).unwrap_or_default(),
                                ),
                                None => ToolCallArguments::Invalid(String::new()),
                            };
                            parts.push(AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                encrypted_content: None,
                                provider_options: None,
                                provider_executed: None,
                            });
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

fn parse_tool_content(value: &Value) -> Option<ToolContent> {
    match value {
        Value::Array(arr) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(Value::String(text_type)) = obj.get("type") {
                        if text_type == "tool-result" {
                            let tool_call_id = obj.get("toolCallId")?.as_str()?.to_string();
                            let tool_name = obj
                                .get("toolName")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let output = obj.get("output").cloned().unwrap_or(Value::Null);
                            parts.push(ToolContentPart::ToolResult(ToolResultContentPart {
                                tool_call_id,
                                tool_name,
                                output,
                                provider_options: None,
                            }));
                        }
                    }
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts)
            }
        }
        _ => None,
    }
}

/// Parse choices array from Chat Completions response format
///
/// This handles the output format: [{"finish_reason": "stop", "message": {"role": "assistant", ...}}]
/// Extracts messages from the "message" field of each choice object.
fn try_choices_array_parsing(data: &Value) -> Option<Vec<Message>> {
    let arr = data.as_array()?;
    let mut messages = Vec::new();

    for item in arr {
        let obj = item.as_object()?;

        // Check if this looks like a choice object (has "message" or "finish_reason")
        // Note: has_message_structure only checks the first element, so we need to validate
        // each element here to ensure the entire array is a valid choices array
        if !obj.contains_key("message") && !obj.contains_key("finish_reason") {
            return None; // Not a choices array
        }

        // Extract the message from the choice
        if let Some(message_value) = obj.get("message") {
            // The message is a single object, wrap in array for try_converting_to_messages
            let wrapped = Value::Array(vec![message_value.clone()]);
            let nested_messages = try_converting_to_messages(&wrapped);
            if nested_messages.is_empty() {
                // If element has "message" but we couldn't parse it, this is malformed
                return None;
            } else {
                messages.extend(nested_messages);
            }
        }
    }

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

/// Import messages from a list of spans
///
/// This function processes spans and extracts messages from their input/output fields,
/// attempting to convert them from various provider formats to the lingua format.
pub fn import_messages_from_spans(spans: Vec<Span>) -> Vec<Message> {
    let mut messages = Vec::new();

    for span in spans {
        // Preserve raw string prompts as user messages (common in Responses API traces).
        if let Some(Value::String(input_text)) = &span.input {
            messages.push(Message::User {
                content: UserContent::String(input_text.clone()),
            });
        } else if let Some(input) = &span.input {
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
