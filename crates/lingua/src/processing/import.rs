use crate::import_parse::{try_parsers_in_order, MessageParser};
mod ai_sdk;
mod langchain;
mod pydantic_ai;
mod span;
use crate::processing::import::ai_sdk::try_parse_ai_sdk_for_import;
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
    assistant_content_parts_from_openai_tool_calls, try_parse_openai_for_import,
    try_system_message_from_openai_metadata, ChatCompletionRequestMessageExt,
};
#[cfg(feature = "openai")]
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
pub use span::{import_span, import_span_with_options, ImportedSpan, SpanContext};

pub(crate) fn is_instruction(message: &Message) -> bool {
    matches!(
        message,
        Message::System { .. } | Message::Developer { .. } | Message::AdditionalTools { .. }
    )
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ImportOptions {
    #[serde(default)]
    pub preserve_unsupported: bool,
}

/// Source data retained without interpreting it as a conversational message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaqueItem {
    /// Position in the source array, or None when the payload itself is opaque.
    pub index: Option<usize>,
    pub value: Value,
}

#[derive(Default)]
struct MessageImport {
    options: ImportOptions,
    errors: Vec<String>,
    opaque: Vec<OpaqueItem>,
}

fn is_opaque_item(data: &Value) -> bool {
    #[cfg(feature = "openai")]
    {
        crate::providers::openai::convert::is_opaque_item_for_import(data)
    }
    #[cfg(not(feature = "openai"))]
    {
        let _ = data;
        false
    }
}

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
fn try_converting_to_messages(data: &Value, import: &mut MessageImport) -> Vec<Message> {
    if let Some(messages) = try_parse_ai_sdk_for_import(data) {
        return messages;
    }

    if is_role_message_array(data)
        || (import.options.preserve_unsupported
            && data
                .as_array()
                .is_some_and(|items| items.iter().any(is_opaque_item)))
    {
        return try_parse_mixed_messages_for_import(data, import).unwrap_or_default();
    }

    if let Some(messages) = try_choices_array_parsing(data, import) {
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
                    let nested_messages = try_converting_to_messages(nested, import);
                    if !nested_messages.is_empty() {
                        return nested_messages;
                    }
                }
            }
        }
        return Vec::new();
    }

    if let Some(messages) = try_parse_mixed_messages_for_import(data, import) {
        return messages;
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
            Vec::<ChatCompletionRequestMessageExt>::deserialize(data_to_parse)
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
    if let Some(choices_messages) = try_choices_array_parsing(data_to_parse, import) {
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

fn try_parse_mixed_messages_for_import(
    data: &Value,
    import: &mut MessageImport,
) -> Option<Vec<Message>> {
    let items = data.as_array()?;
    let provider_parsers = provider_parsers_for_import();
    let mut messages = Vec::new();
    let errors_before = import.errors.len();

    for (index, item) in items.iter().enumerate() {
        if import.options.preserve_unsupported && is_opaque_item(item) {
            import.opaque.push(OpaqueItem {
                index: Some(index),
                value: item.clone(),
            });
            continue;
        }
        // Native Anthropic thinking blocks must use the canonical parser so adjacent text
        // metadata survives. Keep the existing parser order for every other role message.
        #[cfg(feature = "anthropic")]
        let mut parsed_messages = try_parse_anthropic_for_import(item)
            .or_else(|| {
                let wrapped_item = Value::Array(vec![item.clone()]);
                try_parse_anthropic_for_import(&wrapped_item)
            })
            .filter(|messages| {
                messages.iter().any(|message| {
                    matches!(
                        message,
                        Message::Assistant {
                            content: AssistantContent::Array(parts),
                            ..
                        } if parts.iter().any(|part| {
                            matches!(part, AssistantContentPart::Reasoning { .. })
                        })
                    )
                })
            });
        #[cfg(not(feature = "anthropic"))]
        let mut parsed_messages = None;

        #[cfg(feature = "openai")]
        if parsed_messages.is_none() {
            parsed_messages =
                try_parse_reasoning_assistant_message(item).map(|message| vec![message]);
        }

        if parsed_messages.is_none() {
            parsed_messages = try_parsers_in_order(item, &provider_parsers).or_else(|| {
                let wrapped_item = Value::Array(vec![item.clone()]);
                try_parsers_in_order(&wrapped_item, &provider_parsers)
            });
        }

        if parsed_messages.is_none() {
            parsed_messages = parse_lenient_message_item(item).map(|message| vec![message]);
        }

        #[cfg(feature = "openai")]
        if parsed_messages.is_none() && import.options.preserve_unsupported {
            parsed_messages = crate::providers::openai::convert::try_parse_responses_with_opaque_metadata_for_import(item);
            if parsed_messages.is_some() {
                import.opaque.push(OpaqueItem {
                    index: Some(index),
                    value: item.clone(),
                });
            }
        }

        let Some(mut parsed_messages) = parsed_messages else {
            import
                .errors
                .push(format!("Unsupported message item at index {index}"));
            if import.options.preserve_unsupported {
                import.opaque.push(OpaqueItem {
                    index: Some(index),
                    value: item.clone(),
                });
            }
            continue;
        };
        messages.append(&mut parsed_messages);
    }

    if import.errors.len() != errors_before && !import.options.preserve_unsupported {
        Some(Vec::new())
    } else if messages.is_empty() {
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
    let items = Vec::<AnthropicOrSystemMessage>::deserialize(data).ok()?;
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

#[cfg(feature = "openai")]
#[derive(Deserialize)]
struct AttachmentImageCompat {
    #[serde(alias = "image")]
    image_url: ImageAttachment,
    #[serde(flatten)]
    part: openai::InputContent,
}

#[cfg(feature = "openai")]
#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
enum ImageAttachment {
    #[serde(rename = "braintrust_attachment")]
    Braintrust {
        key: String,
        filename: String,
        content_type: String,
    },
}

#[cfg(feature = "openai")]
fn try_parse_attachment_image(item: &Value) -> Option<UserContentPart> {
    let AttachmentImageCompat { image_url, part } =
        AttachmentImageCompat::deserialize(item).ok()?;
    if part.input_content_type != openai::InputItemContentListType::InputImage
        || part.prompt_cache_breakpoint.is_some()
    {
        return None;
    }
    let ImageAttachment::Braintrust { content_type, .. } = &image_url;
    let provider_options = part
        .detail
        .map(|detail| crate::universal::message::ProviderOptions {
            options: [("detail".to_string(), serde_json::json!(detail))]
                .into_iter()
                .collect(),
        });
    Some(UserContentPart::Image {
        image: serde_json::to_value(&image_url).ok()?,
        media_type: Some(content_type.clone()),
        provider_options,
    })
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
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(default)]
        signature: Option<String>,
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

#[cfg(feature = "openai")]
#[derive(Debug, Clone, Deserialize)]
struct ReasoningAssistantMessageCompat {
    #[serde(rename = "role")]
    _role: ReasoningAssistantRole,
    content: Vec<LenientAssistantContentPartCompat>,
    #[serde(default)]
    tool_calls: Vec<openai::ToolCall>,
    #[serde(default)]
    reasoning_signature: Option<String>,
}

#[cfg(feature = "openai")]
#[derive(Debug, Clone, Deserialize)]
enum ReasoningAssistantRole {
    #[serde(rename = "assistant")]
    Assistant,
}

#[cfg(feature = "openai")]
fn try_parse_reasoning_assistant_message(item: &Value) -> Option<Message> {
    let ReasoningAssistantMessageCompat {
        _role: _,
        content,
        tool_calls,
        reasoning_signature,
    } = ReasoningAssistantMessageCompat::deserialize(item).ok()?;

    if !content.iter().any(|part| {
        matches!(
            part,
            LenientAssistantContentPartCompat::Reasoning { .. }
                | LenientAssistantContentPartCompat::Thinking { .. }
        )
    }) {
        return None;
    }

    let mut content_parts: Vec<_> = content
        .into_iter()
        .map(parse_lenient_assistant_content_part)
        .collect::<Option<_>>()?;
    for content_part in &mut content_parts {
        if let AssistantContentPart::Reasoning {
            encrypted_content, ..
        } = content_part
        {
            if encrypted_content.is_none() {
                *encrypted_content = reasoning_signature.clone();
            }
        }
    }
    content_parts.extend(assistant_content_parts_from_openai_tool_calls(
        tool_calls,
        reasoning_signature,
    ));

    Some(Message::Assistant {
        content: AssistantContent::Array(content_parts),
        id: None,
    })
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
            custom_tool_call: None,
            caller: None,
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
    match LenientTextContentPartCompat::deserialize(item).ok()? {
        LenientTextContentPartCompat::Text { text } => Some(TextContentPart {
            text,
            encrypted_content: None,
            cache_control: None,
            provider_options: None,
        }),
    }
}

fn parse_tool_call_arguments(value: Option<Value>) -> Option<ToolCallArguments> {
    match value {
        Some(raw) => {
            if let Ok(arguments) = ToolCallArguments::deserialize(&raw) {
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
    let part = LenientAssistantContentPartCompat::deserialize(item).ok()?;
    parse_lenient_assistant_content_part(part)
}

fn parse_lenient_assistant_content_part(
    part: LenientAssistantContentPartCompat,
) -> Option<AssistantContentPart> {
    match part {
        LenientAssistantContentPartCompat::Text { text } => {
            Some(AssistantContentPart::Text(TextContentPart {
                text,
                encrypted_content: None,
                cache_control: None,
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
        LenientAssistantContentPartCompat::Thinking {
            thinking,
            signature,
        } => Some(AssistantContentPart::Reasoning {
            text: thinking,
            encrypted_content: signature,
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
            caller: None,
            encrypted_content,
            provider_options: None,
            status: None,
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
            caller: None,
            provider_options: None,
        }),
    }
}

fn try_parse_lenient_tool_content_part(item: &Value) -> Option<ToolContentPart> {
    match LenientToolContentPartCompat::deserialize(item).ok()? {
        LenientToolContentPartCompat::ToolResult {
            tool_call_id,
            tool_name,
            output,
        } => Some(ToolContentPart::ToolResult(ToolResultContentPart {
            tool_call_id,
            tool_name,
            output,
            custom_tool_call: None,
            caller: None,
            provider_options: None,
        })),
    }
}

/// Parse user/system content from JSON value
fn parse_user_content(value: &Value) -> Option<UserContent> {
    match value {
        Value::String(s) => Some(UserContent::String(s.clone())),
        Value::Array(arr) => {
            let parts: Vec<UserContentPart> =
                arr.iter()
                    .map(|item| {
                        #[cfg(feature = "openai")]
                        {
                            if let Some(image) = try_parse_attachment_image(item) {
                                return Some(image);
                            }
                            if let Some(part) = openai::InputContent::deserialize(item)
                                .ok()
                                .and_then(|part| {
                                    <UserContentPart as TryFromLLM<openai::InputContent>>::try_from(
                                        part,
                                    )
                                    .ok()
                                })
                            {
                                return Some(part);
                            }
                        }
                        try_parse_lenient_text_content_part(item).map(UserContentPart::Text)
                    })
                    .collect::<Option<_>>()?;
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
fn try_choices_array_parsing(data: &Value, import: &mut MessageImport) -> Option<Vec<Message>> {
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
            let nested_messages = try_converting_to_messages(&wrapped, import);
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

struct SpanMessages {
    input: Vec<Message>,
    output: Vec<Message>,
    opaque_input: Vec<OpaqueItem>,
    opaque_output: Vec<OpaqueItem>,
    errors: Vec<String>,
}

/// Import a span's input and output messages, preserving their boundary and parse errors.
///
/// Both structured span imports and message-only imports use this conversion path to
/// convert provider messages into the Lingua format. Best-effort imports also retain
/// unsupported items separately, without treating them as conversational messages.
fn import_span_messages(
    input: Option<Value>,
    output: Option<Value>,
    metadata: Option<&Value>,
    expect_messages: bool,
    options: ImportOptions,
) -> SpanMessages {
    let mut import = MessageImport {
        options,
        ..Default::default()
    };
    let nonempty = |value: &Value| match value {
        Value::Null => false,
        Value::Array(values) => !values.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        _ => true,
    };
    let parse = |value: Value, field: &str, import: &mut MessageImport| {
        if options.preserve_unsupported && is_opaque_item(&value) {
            import.opaque.push(OpaqueItem { index: None, value });
            return Vec::new();
        }
        let errors_before = import.errors.len();
        let messages = try_converting_to_messages(&value, import);
        if messages.is_empty() && import.opaque.is_empty() {
            if expect_messages && import.errors.len() == errors_before {
                import
                    .errors
                    .push(format!("Unsupported {field} message format"));
            }
            if options.preserve_unsupported {
                import.opaque.push(OpaqueItem { index: None, value });
            }
        }
        messages
    };
    let mut input = match input.filter(nonempty) {
        Some(Value::String(text)) => vec![Message::User {
            content: UserContent::String(text),
        }],
        Some(input) => parse(input, "input", &mut import),
        None => Vec::new(),
    };
    let opaque_input = std::mem::take(&mut import.opaque);
    let output = match output.filter(nonempty) {
        Some(Value::String(text)) if !text.is_empty() => vec![Message::Assistant {
            content: AssistantContent::String(text),
            id: None,
        }],
        Some(Value::String(_)) => Vec::new(),
        Some(output) => parse(output, "output", &mut import),
        None => Vec::new(),
    };
    #[cfg(feature = "openai")]
    if let Some(message) = metadata.and_then(try_system_message_from_openai_metadata) {
        if !input
            .iter()
            .any(|message| matches!(message, Message::System { .. }))
        {
            input.insert(0, message);
        }
    }
    SpanMessages {
        input,
        output,
        opaque_input,
        opaque_output: import.opaque,
        errors: import.errors,
    }
}

/// Import messages from a list of spans
///
/// This function processes spans and extracts messages from their input/output fields,
/// attempting to convert them from various provider formats to the lingua format.
/// Recognized messages are retained even when adjacent items are unsupported. Use
/// `import_span_with_options` to also retain unsupported data and diagnostics.
pub fn import_messages_from_spans(spans: Vec<Span>) -> Vec<Message> {
    spans
        .into_iter()
        .flat_map(|span| {
            let messages = import_span_messages(
                span.input,
                span.output,
                span.other.get("metadata"),
                true,
                ImportOptions {
                    preserve_unsupported: true,
                },
            );
            messages.input.into_iter().chain(messages.output)
        })
        .collect()
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
    fn invalid_tool_call_metadata_preserves_valid_hints() {
        let spans: Vec<Span> = serde_json::from_str(include_str!(
            "import/fixtures/invalid-tool-call-metadata.json"
        ))
        .unwrap();
        let imported: Vec<_> = spans
            .into_iter()
            .map(|span| import_span(span).unwrap())
            .collect();
        for span in &imported {
            assert_eq!(span.header.turn.as_deref(), Some("turn"));
            assert_eq!(span.header.model.as_deref(), Some("example-model"));
            assert_eq!(span.errors.len(), 1);
            assert!(span.errors[0].starts_with("Invalid metadata.tool_call_id:"));
        }
        let compaction = imported[0].header.compaction.as_ref().unwrap();
        assert_eq!(compaction.id, "compact");
        assert_eq!(compaction.replaced_message_count, Some(2));
        let tool = imported[1].tool_result.as_ref().unwrap();
        assert!(tool.content.is_none());
        assert_eq!(
            tool.output,
            Some(Value::String("Found a record".to_string()))
        );
    }

    #[test]
    fn imported_spans_preserve_history_and_input_output_boundaries() {
        #[derive(Deserialize)]
        struct Fixture {
            spans: Vec<Span>,
        }
        let fixture: Fixture = serde_json::from_str(include_str!(
            "trajectory/fixtures/responses-tool-cycle.json"
        ))
        .unwrap();
        let imported = fixture
            .spans
            .into_iter()
            .map(import_span)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let final_span = imported
            .iter()
            .find(|span| span.header.id.as_deref() == Some("final"))
            .unwrap();
        assert!(matches!(
            final_span.input.as_slice(),
            [
                Message::AdditionalTools { .. },
                Message::Developer { .. },
                Message::User { .. },
                Message::Assistant { .. },
                Message::Tool { .. }
            ]
        ));
        assert!(matches!(
            final_span.output.as_slice(),
            [Message::Assistant { .. }]
        ));
        assert!(final_span.errors.is_empty());
    }

    fn import_assistant_parts(input: Value) -> Vec<AssistantContentPart> {
        let messages = try_converting_to_messages(&input, &mut MessageImport::default());
        assert_eq!(messages.len(), 1);

        let Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } = messages
            .into_iter()
            .next()
            .expect("expected assistant message")
        else {
            panic!("expected assistant content parts");
        };

        parts
    }

    #[test]
    fn imports_reasoning_and_text_parts_independent_of_object_key_order() {
        let content_first = crate::serde_json::json!([
            {
                "content": [
                    { "text": "internal reasoning", "type": "reasoning" },
                    { "text": "visible answer", "type": "text" }
                ],
                "role": "assistant"
            }
        ]);
        let role_first = crate::serde_json::json!([
            {
                "role": "assistant",
                "content": [
                    { "type": "reasoning", "text": "internal reasoning" },
                    { "type": "text", "text": "visible answer" }
                ]
            }
        ]);

        for input in [content_first, role_first] {
            let parts = import_assistant_parts(input);
            assert!(matches!(
                parts.as_slice(),
                [
                    AssistantContentPart::Reasoning { text, .. },
                    AssistantContentPart::Text(TextContentPart { text: visible, .. }),
                ] if text == "internal reasoning" && visible == "visible answer"
            ));
        }
    }

    #[test]
    fn lenient_import_maps_anthropic_thinking_to_reasoning() {
        let message = parse_lenient_message_item(&crate::serde_json::json!({
            "role": "assistant",
            "content": [
                {
                    "type": "thinking",
                    "thinking": "internal reasoning",
                    "signature": "reasoning-signature"
                },
                { "type": "text", "text": "visible answer" }
            ]
        }))
        .expect("expected assistant message");

        assert!(matches!(
            message,
            Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            } if matches!(
                parts.as_slice(),
                [
                    AssistantContentPart::Reasoning {
                        text,
                        encrypted_content: Some(encrypted_content),
                    },
                    AssistantContentPart::Text(TextContentPart { text: visible, .. }),
                ] if text == "internal reasoning"
                    && encrypted_content == "reasoning-signature"
                    && visible == "visible answer"
            )
        ));
    }

    #[test]
    fn native_anthropic_thinking_preserves_adjacent_text_metadata() {
        let parts = import_assistant_parts(crate::serde_json::json!([
            {
                "role": "assistant",
                "content": [
                    {
                        "type": "thinking",
                        "thinking": "internal reasoning",
                        "signature": "reasoning-signature"
                    },
                    {
                        "type": "text",
                        "text": "visible answer",
                        "cache_control": { "type": "ephemeral", "ttl": "1h" },
                        "citations": { "enabled": true }
                    }
                ]
            }
        ]));

        assert_eq!(
            crate::serde_json::to_value(parts).expect("assistant parts should serialize"),
            crate::serde_json::json!([
                {
                    "type": "reasoning",
                    "text": "internal reasoning",
                    "encrypted_content": "reasoning-signature"
                },
                {
                    "type": "text",
                    "text": "visible answer",
                    "cache_control": { "type": "ephemeral", "ttl": "1h" },
                    "provider_options": {
                        "citations": { "enabled": true }
                    }
                }
            ])
        );
    }

    #[test]
    fn imports_reasoning_only_assistant_content() {
        let parts = import_assistant_parts(crate::serde_json::json!([
            {
                "role": "assistant",
                "reasoning_signature": "reasoning-signature",
                "content": [{ "type": "reasoning", "text": "internal reasoning" }]
            }
        ]));

        assert!(matches!(
            parts.as_slice(),
            [AssistantContentPart::Reasoning {
                text,
                encrypted_content: Some(encrypted_content),
            }] if text == "internal reasoning" && encrypted_content == "reasoning-signature"
        ));
    }

    #[test]
    fn imports_reasoning_with_openai_top_level_tool_calls() {
        let parts = import_assistant_parts(crate::serde_json::json!([
            {
                "tool_calls": [{
                    "id": "call_lookup",
                    "type": "function",
                    "function": {
                        "name": "lookup_weather",
                        "arguments": "{\"city\":\"Paris\"}"
                    }
                }],
                "reasoning_signature": "reasoning-signature",
                "content": [{ "text": "Need the weather", "type": "reasoning" }],
                "role": "assistant"
            }
        ]));

        assert!(matches!(
            parts.as_slice(),
            [
                AssistantContentPart::Reasoning { text, .. },
                AssistantContentPart::ToolCall {
                    tool_call_id,
                    tool_name,
                    arguments,
                    encrypted_content: Some(encrypted_content),
                    ..
                },
            ] if text == "Need the weather"
                && tool_call_id == "call_lookup"
                && tool_name == "lookup_weather"
                && arguments.to_string() == "{\"city\":\"Paris\"}"
                && encrypted_content == "reasoning-signature"
        ));
    }
}
