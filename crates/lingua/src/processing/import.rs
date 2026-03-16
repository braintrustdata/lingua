use crate::import_parse::{try_parsers_in_order, MessageParser};
mod langchain;
mod pydantic_ai;
use crate::processing::import::langchain::try_parse_langchain_for_import;
use crate::processing::import::pydantic_ai::try_parse_pydantic_ai_for_import;
#[cfg(feature = "anthropic")]
use crate::providers::anthropic::convert::try_parse_anthropic_for_import;
#[cfg(feature = "anthropic")]
use crate::providers::anthropic::generated as anthropic;
#[cfg(feature = "bedrock")]
use crate::providers::bedrock::convert::try_parse_bedrock_for_import;
#[cfg(feature = "google")]
use crate::providers::google::convert::try_parse_google_for_import;
#[cfg(feature = "openai")]
use crate::providers::openai::convert::{
    try_parse_openai_for_import, try_system_message_from_openai_metadata,
    ChatCompletionRequestMessageExt,
};
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

/// Try to convert a value to lingua messages by attempting multiple format conversions
fn try_converting_to_messages(data: &Value) -> Vec<Message> {
    if is_role_message_array(data) {
        return try_parse_mixed_role_messages_for_import(data).unwrap_or_default();
    }

    if let Some(messages) = try_choices_array_parsing(data) {
        return messages;
    }

    if let Some(messages) = try_parse_provider_messages_for_import(data) {
        return messages;
    }

    if let Some(messages) = try_parse_pydantic_ai_for_import(data) {
        return messages;
    }

    if let Some(messages) = try_parse_langchain_for_import(data) {
        return messages;
    }

    // Cheap check to see if a value looks like it might contain messages.
    // Returns early to avoid expensive deserialization attempts on non-message data.
    let has_message_structure = match data {
        // Check if it's an array where any element has "role" or nested "message.role".
        Value::Array(arr) => arr.iter().any(|item| match item {
            Value::Object(obj) => {
                if obj.contains_key("role") {
                    return true;
                }
                if let Some(Value::Object(msg)) = obj.get("message") {
                    if msg.contains_key("role") {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }),
        // Check if it's an object with "role" field (single message)
        Value::Object(obj) => obj.contains_key("role"),
        _ => false,
    };

    // Early bailout: if data doesn't have message structure, skip expensive deserializations
    if !has_message_structure {
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
    #[cfg(feature = "openai")]
    {
        if let Ok(provider_messages) =
            serde_json::from_value::<Vec<ChatCompletionRequestMessageExt>>(data_to_parse.clone())
        {
            if let Ok(messages) = <Vec<Message> as TryFromLLM<
                Vec<ChatCompletionRequestMessageExt>,
            >>::try_from(provider_messages)
            {
                if !messages.is_empty() {
                    return messages;
                }
            }
        }
    }

    // Try Anthropic format (including role-based system/developer messages).
    #[cfg(feature = "anthropic")]
    {
        if let Some(anthropic_messages) = try_anthropic_or_system_messages(data_to_parse) {
            if !anthropic_messages.is_empty() {
                return anthropic_messages;
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

fn is_role_message_array(data: &Value) -> bool {
    let Value::Array(items) = data else {
        return false;
    };

    !items.is_empty()
        && items.iter().all(|item| match item {
            Value::Object(obj) => matches!(obj.get("role"), Some(Value::String(_))),
            _ => false,
        })
}

fn provider_parsers_for_import() -> Vec<MessageParser> {
    vec![
        #[cfg(feature = "openai")]
        try_parse_openai_for_import,
        #[cfg(feature = "anthropic")]
        try_parse_anthropic_for_import,
        #[cfg(feature = "google")]
        try_parse_google_for_import,
        #[cfg(feature = "bedrock")]
        try_parse_bedrock_for_import,
    ]
}

fn try_parse_mixed_role_messages_for_import(data: &Value) -> Option<Vec<Message>> {
    let items = data.as_array()?;
    let provider_parsers = provider_parsers_for_import();
    let mut messages = Vec::new();

    for item in items {
        let mut parsed_messages = try_parsers_in_order(item, &provider_parsers).or_else(|| {
            let wrapped_item = Value::Array(vec![item.clone()]);
            try_parsers_in_order(&wrapped_item, &provider_parsers)
        });

        if parsed_messages.is_none() {
            parsed_messages = parse_lenient_message_item(item).map(|message| vec![message]);
        }

        if let Some(mut parsed_messages) = parsed_messages {
            messages.append(&mut parsed_messages);
        }
    }

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

fn try_parse_provider_messages_for_import(data: &Value) -> Option<Vec<Message>> {
    let provider_parsers = provider_parsers_for_import();

    try_parsers_in_order(data, &provider_parsers)
}

#[cfg(feature = "anthropic")]
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum AnthropicOrSystemMessage {
    Anthropic(anthropic::InputMessage),
    SystemOrDeveloper(SystemOrDeveloperMessage),
}

#[cfg(feature = "anthropic")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemOrDeveloperMessage {
    role: SystemOrDeveloperRole,
    content: Value,
}

#[cfg(feature = "anthropic")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SystemOrDeveloperRole {
    System,
    Developer,
}

#[cfg(feature = "anthropic")]
fn try_parse_anthropic_or_system_message(item: AnthropicOrSystemMessage) -> Option<Message> {
    match item {
        AnthropicOrSystemMessage::Anthropic(provider_message) => {
            <Message as TryFromLLM<anthropic::InputMessage>>::try_from(provider_message).ok()
        }
        AnthropicOrSystemMessage::SystemOrDeveloper(system_or_developer) => {
            let value = serde_json::to_value(system_or_developer).ok()?;
            parse_lenient_message_item(&value)
        }
    }
}

#[cfg(feature = "anthropic")]
fn try_anthropic_or_system_messages(data: &Value) -> Option<Vec<Message>> {
    let items: Vec<AnthropicOrSystemMessage> = serde_json::from_value(data.clone()).ok()?;
    if items.is_empty() {
        return None;
    }

    let messages: Option<Vec<Message>> = items
        .into_iter()
        .map(try_parse_anthropic_or_system_message)
        .collect();
    let messages = messages?;

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

/// Lenient message parser for messages that don't match strict provider schemas
///
/// This parser looks for basic message structure: { "role": "...", "content": "..." }
/// without requiring strict schema validation. This helps capture messages from:
/// - Custom LLM wrappers
/// - Logging that doesn't perfectly match provider formats
/// - Messages with extra/missing fields
#[derive(Debug, Clone, Deserialize)]
struct LenientToolMessageCompat {
    #[serde(default, alias = "tool_call_id", alias = "toolCallId")]
    tool_call_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum LenientTextContentPartCompat {
    #[serde(rename = "text", alias = "input_text", alias = "output_text")]
    Text { text: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum LenientAssistantContentPartCompat {
    #[serde(rename = "text", alias = "input_text", alias = "output_text")]
    Text { text: String },
    #[serde(rename = "reasoning")]
    Reasoning {
        text: String,
        #[serde(default)]
        encrypted_content: Option<String>,
    },
    #[serde(rename = "tool_call", alias = "tool-call", alias = "toolCall")]
    ToolCall {
        #[serde(alias = "toolCallId")]
        tool_call_id: String,
        #[serde(default, alias = "toolName")]
        tool_name: String,
        #[serde(default, alias = "input")]
        arguments: Option<Value>,
        #[serde(default)]
        encrypted_content: Option<String>,
        #[serde(default, alias = "providerExecuted")]
        provider_executed: Option<bool>,
    },
    #[serde(rename = "tool_result", alias = "tool-result", alias = "toolResult")]
    ToolResult {
        #[serde(alias = "toolCallId")]
        tool_call_id: String,
        #[serde(default, alias = "toolName")]
        tool_name: String,
        #[serde(default)]
        output: Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum LenientToolContentPartCompat {
    #[serde(rename = "tool_result", alias = "tool-result", alias = "toolResult")]
    ToolResult {
        #[serde(alias = "toolCallId")]
        tool_call_id: String,
        #[serde(default, alias = "toolName")]
        tool_name: String,
        #[serde(default)]
        output: Value,
    },
}

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
        "tool" => parse_lenient_tool_message(item, content_value),
        _ => None,
    }
}

fn parse_lenient_tool_message(item: &Value, content_value: &Value) -> Option<Message> {
    if let Some(content) = parse_tool_content(content_value) {
        return Some(Message::Tool { content });
    }

    let parsed = LenientToolMessageCompat::deserialize(item).ok()?;
    let tool_call_id = parsed.tool_call_id?;
    let tool_name = parsed.name.unwrap_or_default();

    let output = match content_value {
        Value::String(text) => match serde_json::from_str::<Value>(text) {
            Ok(parsed) => parsed,
            Err(_) => Value::String(text.clone()),
        },
        other => other.clone(),
    };

    Some(Message::Tool {
        content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
            tool_call_id,
            tool_name,
            output,
            provider_options: None,
        })],
    })
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

fn try_parse_lenient_text_content_part(item: &Value) -> Option<TextContentPart> {
    match serde_json::from_value::<LenientTextContentPartCompat>(item.clone()).ok()? {
        LenientTextContentPartCompat::Text { text } => Some(TextContentPart {
            text,
            encrypted_content: None,
            provider_options: None,
        }),
    }
}

fn parse_tool_call_arguments(value: Option<Value>) -> Option<ToolCallArguments> {
    match value {
        Some(raw) => {
            if let Ok(arguments) = serde_json::from_value::<ToolCallArguments>(raw.clone()) {
                return Some(arguments);
            }

            match raw {
                Value::Object(map) => Some(ToolCallArguments::Valid(map)),
                Value::String(text) => Some(ToolCallArguments::Invalid(text)),
                other => serde_json::to_string(&other)
                    .ok()
                    .map(ToolCallArguments::Invalid),
            }
        }
        None => Some(ToolCallArguments::Invalid(String::new())),
    }
}

fn try_parse_lenient_assistant_content_part(item: &Value) -> Option<AssistantContentPart> {
    match serde_json::from_value::<LenientAssistantContentPartCompat>(item.clone()).ok()? {
        LenientAssistantContentPartCompat::Text { text } => {
            Some(AssistantContentPart::Text(TextContentPart {
                text,
                encrypted_content: None,
                provider_options: None,
            }))
        }
        LenientAssistantContentPartCompat::Reasoning {
            text,
            encrypted_content,
        } => Some(AssistantContentPart::Reasoning {
            text,
            encrypted_content,
        }),
        LenientAssistantContentPartCompat::ToolCall {
            tool_call_id,
            tool_name,
            arguments,
            encrypted_content,
            provider_executed,
        } => Some(AssistantContentPart::ToolCall {
            tool_call_id,
            tool_name,
            arguments: parse_tool_call_arguments(arguments)?,
            encrypted_content,
            provider_options: None,
            provider_executed,
        }),
        LenientAssistantContentPartCompat::ToolResult {
            tool_call_id,
            tool_name,
            output,
        } => Some(AssistantContentPart::ToolResult {
            tool_call_id,
            tool_name,
            output,
            provider_options: None,
        }),
    }
}

fn try_parse_lenient_tool_content_part(item: &Value) -> Option<ToolContentPart> {
    match serde_json::from_value::<LenientToolContentPartCompat>(item.clone()).ok()? {
        LenientToolContentPartCompat::ToolResult {
            tool_call_id,
            tool_name,
            output,
        } => Some(ToolContentPart::ToolResult(ToolResultContentPart {
            tool_call_id,
            tool_name,
            output,
            provider_options: None,
        })),
    }
}

/// Parse user/system content from JSON value
fn parse_user_content(value: &Value) -> Option<UserContent> {
    match value {
        Value::String(s) => Some(UserContent::String(s.clone())),
        Value::Array(arr) => {
            let parts: Vec<UserContentPart> = arr
                .iter()
                .filter_map(try_parse_lenient_text_content_part)
                .map(UserContentPart::Text)
                .collect();
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
            let parts: Vec<AssistantContentPart> = arr
                .iter()
                .filter_map(try_parse_lenient_assistant_content_part)
                .collect();
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
            let parts: Vec<ToolContentPart> = arr
                .iter()
                .filter_map(try_parse_lenient_tool_content_part)
                .collect();
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

        // Check if this looks like a choice object (has "message" or "finish_reason").
        // We still validate each element here to ensure the entire array is a valid choices array.
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
        let mut span_messages = Vec::new();

        // Try to extract messages from input
        if let Some(Value::String(input_text)) = &span.input {
            span_messages.push(Message::User {
                content: UserContent::String(input_text.clone()),
            });
        } else if let Some(input) = &span.input {
            let input_messages = try_converting_to_messages(input);
            span_messages.extend(input_messages);
        }

        #[cfg(feature = "openai")]
        if let Some(metadata) = span.other.get("metadata") {
            if let Some(system_message) = try_system_message_from_openai_metadata(metadata) {
                let has_system_message = span_messages
                    .iter()
                    .any(|message| matches!(message, Message::System { .. }));
                if !has_system_message {
                    span_messages.insert(0, system_message);
                }
            }
        }

        messages.extend(span_messages);

        // Try to extract messages from output
        if let Some(Value::String(output_text)) = &span.output {
            if !output_text.is_empty() {
                messages.push(Message::Assistant {
                    content: AssistantContent::String(output_text.clone()),
                    id: None,
                });
            }
        } else if let Some(output) = &span.output {
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
