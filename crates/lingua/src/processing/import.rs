use crate::import_parse::try_parsers_in_order;
use crate::import_value::Field;
pub use crate::import_value::{ImportValue, InputKind};
mod ai_sdk;
mod audio;
#[cfg(feature = "bedrock")]
mod bedrock;
pub use ai_sdk::try_import_ai_sdk_prompt_message;
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
use crate::providers::google::import::{
    TraceToolCalls, try_parse_adk_for_import, try_parse_google_for_import,
};
#[cfg(feature = "openai")]
use crate::providers::openai::convert::{
    ChatCompletionRequestMessageExt, assistant_content_parts_from_openai_tool_calls,
    try_parse_openai_for_import, try_parse_responses_with_opaque_metadata_for_import,
    try_system_message_from_openai_metadata,
};
#[cfg(feature = "openai")]
use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::Message;
use crate::universal::convert::TryFromLLM;
pub use crate::universal::trajectory::OpaqueItem;
use crate::universal::trajectory::OpaqueItemKind;
use crate::universal::{
    AssistantContent, AssistantContentPart, TextContentPart, ToolCallArguments, ToolContent,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
pub use audio::{VoiceContext, VoicePayload};
use serde::{Deserialize, Serialize};
pub use span::{
    ImportedSpan, SpanContext, TRAJECTORY_METADATA_FIELDS, import_span, import_span_with_options,
};

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

#[derive(Default)]
struct MessageImport {
    options: ImportOptions,
    errors: Vec<String>,
    warnings: Vec<String>,
    opaque: Vec<OpaqueItem>,
    discard_opaque: bool,
    #[cfg(feature = "google")]
    google_tools: TraceToolCalls,
}

impl MessageImport {
    #[cfg(feature = "google")]
    fn finish_google_import(
        &mut self,
        result: Result<Vec<Message>, crate::error::ConvertError>,
    ) -> Vec<Message> {
        result.unwrap_or_else(|error| {
            self.errors.push(error.to_string());
            Vec::new()
        })
    }

    fn try_message_batch(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Option<Vec<Message>>,
    ) -> Option<Vec<Message>> {
        #[cfg(feature = "google")]
        let previous_tools = self.google_tools.clone();

        let messages = parse(self);
        #[cfg(feature = "google")]
        if messages.as_ref().is_none_or(Vec::is_empty) {
            self.google_tools = previous_tools;
        }
        messages
    }
}

fn opaque_item_kind(data: &impl ImportValue) -> Option<OpaqueItemKind> {
    #[cfg(feature = "openai")]
    {
        crate::providers::openai::convert::opaque_item_kind_for_import(data)
    }
    #[cfg(not(feature = "openai"))]
    {
        let _ = data;
        None
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
fn try_converting_to_messages<V: ImportValue>(
    data: &V,
    import: &mut MessageImport,
) -> Vec<Message> {
    if matches!(
        data.kind(),
        InputKind::Null | InputKind::Other | InputKind::String
    ) {
        return Vec::new();
    }

    // ADK and the AI SDK both log finishReason and content. Prefer the typed
    // ADK envelope before interpreting its content object as AI SDK JSON text.
    #[cfg(feature = "google")]
    if let Some(result) = try_parse_adk_for_import(data, &mut import.google_tools) {
        return import.finish_google_import(result);
    }

    if let Some(messages) = try_parse_ai_sdk_for_import(data) {
        return messages;
    }

    if is_role_message_array(data)
        || (import.options.preserve_unsupported
            && data
                .array()
                .is_some_and(|mut items| items.any(|item| opaque_item_kind(&item).is_some())))
    {
        return try_parse_mixed_messages_for_import(data, import).unwrap_or_default();
    }

    if let Some(messages) = try_choices_array_parsing(data, import) {
        return messages;
    }

    if let Some(messages) = try_parse_provider_messages_for_import(data, import) {
        return messages;
    }

    if let Some(messages) = try_parse_pydantic_ai_for_import(data) {
        return messages;
    }

    if let Some(messages) = try_parse_langchain_for_import(data) {
        return messages;
    }

    let has_message_structure = if let Some(mut items) = data.array() {
        items.any(|item| {
            item.kind() == InputKind::Object
                && item.parse::<LegacyMessageCompat>().is_some_and(|item| {
                    item.role.is_present()
                        || item.message.as_ref().is_some_and(|message| {
                            message
                                .parse::<LegacyMessageCompat>()
                                .is_some_and(|message| message.role.is_present())
                        })
                })
        })
    } else {
        data.kind() == InputKind::Object
            && data
                .parse::<LegacyMessageCompat>()
                .is_some_and(|item| item.role.is_present())
    };

    if !has_message_structure {
        let wrapped = (data.kind() == InputKind::Object)
            .then(|| data.parse::<NestedMessagesCompat>())
            .flatten();
        if let Some(wrapped) = wrapped {
            for nested in [
                wrapped.messages,
                wrapped.prompt,
                wrapped.input,
                wrapped.output,
                wrapped.choices,
                wrapped.result,
                wrapped.response,
            ]
            .into_iter()
            .flatten()
            {
                let messages = try_converting_to_messages(&nested, import);
                if !messages.is_empty() {
                    return messages;
                }
            }
        }
        return Vec::new();
    }

    let materialized = data.to_value();
    let data = materialized.as_ref();

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
            && let Ok(messages) = <Vec<Message> as TryFromLLM<
                Vec<ChatCompletionRequestMessageExt>,
            >>::try_from(provider_messages)
            && !messages.is_empty()
        {
            return messages;
        }
    }

    // Try Anthropic format (including role-based system/developer messages).
    #[cfg(feature = "anthropic")]
    {
        if let Some(anthropic_messages) = try_anthropic_or_system_messages(data_to_parse)
            && !anthropic_messages.is_empty()
        {
            return anthropic_messages;
        }
    }

    // Try lenient parsing for non-standard message formats
    if let Some(lenient_messages) = try_lenient_message_parsing(data_to_parse)
        && !lenient_messages.is_empty()
    {
        return lenient_messages;
    }

    // Try parsing as choices array (Chat Completions response format)
    // This handles [{"finish_reason": "stop", "message": {"role": "assistant", ...}}]
    if let Some(choices_messages) = try_choices_array_parsing(data_to_parse, import)
        && !choices_messages.is_empty()
    {
        return choices_messages;
    }

    Vec::new()
}

#[derive(Deserialize)]
struct RoleMessageCompat {
    #[serde(rename = "role")]
    _role: String,
}

#[derive(Deserialize)]
struct LegacyMessageCompat {
    #[serde(default)]
    role: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    message: Field<Value>,
}

#[derive(Deserialize)]
struct NestedMessagesCompat {
    messages: Option<Value>,
    prompt: Option<Value>,
    input: Option<Value>,
    output: Option<Value>,
    choices: Option<Value>,
    result: Option<Value>,
    response: Option<Value>,
}

fn is_role_message_array(data: &impl ImportValue) -> bool {
    data.array().is_some_and(|mut items| {
        items.len() > 0
            && items.all(|item| {
                item.kind() == InputKind::Object && item.parse::<RoleMessageCompat>().is_some()
            })
    })
}

fn try_parse_mixed_messages_for_import(
    data: &impl ImportValue,
    import: &mut MessageImport,
) -> Option<Vec<Message>> {
    let items = data.array()?;
    import.try_message_batch(|import| {
        let mut messages = Vec::new();
        let errors_before = import.errors.len();

        for (index, item) in items.enumerate() {
            messages.extend(import_mixed_message_item(&item, index, import));
        }

        if import.errors.len() != errors_before && !import.options.preserve_unsupported {
            Some(Vec::new())
        } else {
            crate::import_parse::non_empty_messages(messages)
        }
    })
}

/// Imports one role-bearing item using the mixed role-array parser.
///
/// Concatenating per-item results matches sequence import through
/// [`import_messages_from_values`] or [`import_messages_from_spans`] when every
/// array item has a string `role`. This uses `preserve_unsupported: true` behavior:
/// unsupported items yield no messages while supported items are retained.
/// With [`import_span_with_options`] and `preserve_unsupported: false`, one
/// unsupported item rejects the whole array instead.
pub fn import_role_message_from_value(data: &impl ImportValue) -> Vec<Message> {
    let mut import = MessageImport {
        options: ImportOptions {
            preserve_unsupported: true,
        },
        discard_opaque: true,
        ..Default::default()
    };
    import_mixed_message_item(data, 0, &mut import)
}

fn import_mixed_message_item<V: ImportValue>(
    item: &V,
    index: usize,
    import: &mut MessageImport,
) -> Vec<Message> {
    let errors_before = import.errors.len();
    let kind = if import.options.preserve_unsupported {
        opaque_item_kind(item)
    } else {
        None
    };
    if kind.is_some() {
        if !import.discard_opaque {
            import.opaque.push(OpaqueItem {
                index: Some(index),
                value: item.to_value().into_owned(),
                kind,
                is_metadata: false,
            });
        }
        return Vec::new();
    }
    // Native Anthropic thinking blocks must use the canonical parser so adjacent text
    // metadata survives. Keep the existing parser order for every other role message.
    #[cfg(feature = "anthropic")]
    let mut parsed_messages = try_parse_anthropic_for_import(item).filter(|messages| {
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
        parsed_messages = try_parse_reasoning_assistant_message(item).map(|message| vec![message]);
    }

    if parsed_messages.is_none() {
        parsed_messages = try_parse_provider_messages_for_import(item, import).or_else(|| {
            let wrapped_item = Value::Array(vec![item.to_value().into_owned()]);
            try_parse_provider_messages_for_import(&wrapped_item, import)
        });
    }

    if parsed_messages.is_none() {
        parsed_messages = parse_lenient_message_item(item).map(|message| vec![message]);
    }

    #[cfg(feature = "openai")]
    if parsed_messages.is_none()
        && import.options.preserve_unsupported
        && let Some((messages, warning)) = try_parse_responses_with_opaque_metadata_for_import(item)
    {
        parsed_messages = Some(messages);
        import
            .warnings
            .push(format!("Message item {index}: {warning}"));
        if !import.discard_opaque {
            import.opaque.push(OpaqueItem {
                index: Some(index),
                value: item.to_value().into_owned(),
                kind: None,
                is_metadata: true,
            });
        }
    }

    if parsed_messages.is_none() {
        import
            .errors
            .push(format!("Unsupported message item at index {index}"));
    }
    if import.errors.len() > errors_before
        && import.options.preserve_unsupported
        && !import.discard_opaque
    {
        import.opaque.push(OpaqueItem {
            index: Some(index),
            value: item.to_value().into_owned(),
            kind: None,
            is_metadata: false,
        });
    }
    parsed_messages.unwrap_or_default()
}

fn try_parse_provider_messages_for_import<V: ImportValue>(
    data: &V,
    _import: &mut MessageImport,
) -> Option<Vec<Message>> {
    let messages = try_parsers_in_order(
        data,
        &[
            #[cfg(feature = "openai")]
            try_parse_openai_for_import,
            #[cfg(feature = "anthropic")]
            try_parse_anthropic_for_import,
        ],
    );
    #[cfg(feature = "google")]
    let messages = messages.or_else(|| {
        let result = try_parse_google_for_import(data, &mut _import.google_tools)?;
        Some(_import.finish_google_import(result))
    });
    #[cfg(feature = "bedrock")]
    let messages = messages.or_else(|| {
        try_parsers_in_order(
            data,
            &[
                bedrock::try_parse_bedrock_trace_for_import,
                try_parse_bedrock_for_import,
            ],
        )
    });
    messages
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
#[serde(untagged)]
enum AttachmentImageCompat {
    Responses {
        #[serde(alias = "image")]
        image_url: ImageAttachment,
        #[serde(flatten)]
        part: openai::InputContent,
    },
    ChatCompletions {
        image_url: AttachmentImageUrlCompat,
        #[serde(flatten)]
        part: openai::ChatCompletionRequestMessageContentPart,
    },
}

#[cfg(feature = "openai")]
#[derive(Deserialize)]
struct AttachmentImageUrlCompat {
    url: ImageAttachment,
    detail: Option<openai::Detail>,
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
    let (image_url, detail) = match AttachmentImageCompat::deserialize(item).ok()? {
        AttachmentImageCompat::Responses { image_url, part }
            if part.input_content_type == openai::InputItemContentListType::InputImage
                && part.prompt_cache_breakpoint.is_none() =>
        {
            (
                image_url,
                part.detail.map(|detail| serde_json::json!(detail)),
            )
        }
        AttachmentImageCompat::ChatCompletions { image_url, part }
            if part.content_part_type == openai::PurpleType::ImageUrl
                && part.prompt_cache_breakpoint.is_none() =>
        {
            (
                image_url.url,
                image_url.detail.map(|detail| serde_json::json!(detail)),
            )
        }
        _ => return None,
    };
    let ImageAttachment::Braintrust { content_type, .. } = &image_url;
    let provider_options = detail.map(|detail| crate::universal::message::ProviderOptions {
        options: [("detail".to_string(), detail)].into_iter().collect(),
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
fn try_parse_reasoning_assistant_message(item: &impl ImportValue) -> Option<Message> {
    let ReasoningAssistantMessageCompat {
        _role: _,
        content,
        tool_calls,
        reasoning_signature,
    } = item.parse::<ReasoningAssistantMessageCompat>()?;

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
            && encrypted_content.is_none()
        {
            *encrypted_content = reasoning_signature.clone();
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

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum LenientMessageRole {
    User,
    System,
    Developer,
    Assistant,
    Tool,
}

#[derive(Deserialize)]
struct LenientMessageCompat {
    role: LenientMessageRole,
    content: Value,
}

fn parse_lenient_message_item(item: &impl ImportValue) -> Option<Message> {
    let parsed = item.parse::<LenientMessageCompat>()?;
    let content_value = &parsed.content;

    match parsed.role {
        LenientMessageRole::User => Some(Message::User {
            content: parse_user_content(content_value)?,
        }),
        LenientMessageRole::System => Some(Message::System {
            content: parse_user_content(content_value)?,
        }),
        LenientMessageRole::Developer => Some(Message::Developer {
            content: parse_user_content(content_value)?,
        }),
        LenientMessageRole::Assistant => Some(Message::Assistant {
            content: parse_assistant_content(content_value)?,
            id: None,
        }),
        LenientMessageRole::Tool => parse_lenient_tool_message(item, content_value),
    }
}

fn parse_lenient_tool_message(item: &impl ImportValue, content_value: &Value) -> Option<Message> {
    if let Some(content) = parse_tool_content(content_value) {
        return Some(Message::Tool { content });
    }

    let parsed = item.parse::<LenientToolMessageCompat>()?;
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
            if parts.is_empty() { None } else { Some(parts) }
        }
        _ => None,
    }
}

/// Parse choices array from Chat Completions response format
///
/// This handles the output format: [{"finish_reason": "stop", "message": {"role": "assistant", ...}}]
/// Extracts messages from the "message" field of each choice object.
#[derive(Deserialize)]
struct ChoiceMessageCompat {
    #[serde(default)]
    message: Field<Value>,
    #[serde(default)]
    finish_reason: Field<serde::de::IgnoredAny>,
}

fn try_choices_array_parsing(
    data: &impl ImportValue,
    import: &mut MessageImport,
) -> Option<Vec<Message>> {
    if data.kind() != InputKind::Array {
        return None;
    }
    if !data.array()?.all(|item| item.kind() == InputKind::Object) {
        return None;
    }
    let choices = data.parse::<Vec<ChoiceMessageCompat>>()?;
    import.try_message_batch(|import| {
        let mut messages = Vec::new();
        for choice in choices {
            if !choice.message.is_present() && !choice.finish_reason.is_present() {
                return None;
            }
            if let Field::Present(message) = choice.message {
                let wrapped = Value::Array(vec![message]);
                let nested = try_converting_to_messages(&wrapped, import);
                if nested.is_empty() {
                    return None;
                }
                messages.extend(nested);
            }
        }
        crate::import_parse::non_empty_messages(messages)
    })
}

struct SpanMessages {
    input: Vec<Message>,
    output: Vec<Message>,
    opaque_input: Vec<OpaqueItem>,
    opaque_output: Vec<OpaqueItem>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn try_system_message_from_metadata(metadata: &impl ImportValue) -> Option<Message> {
    #[cfg(feature = "openai")]
    if let Some(message) = try_system_message_from_openai_metadata(metadata) {
        return Some(message);
    }
    #[cfg(feature = "bedrock")]
    if let Some(message) = bedrock::try_system_message_from_strands_metadata(metadata) {
        return Some(message);
    }
    let _ = metadata;
    None
}

/// Import a span's input and output messages, preserving their boundary and parse errors.
///
/// Both structured span imports and message-only imports use this conversion path to
/// convert provider messages into the Lingua format. Best-effort imports also retain
/// unsupported items separately, without treating them as conversational messages,
/// and report warnings when imported messages omit provider metadata.
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
        let kind = if options.preserve_unsupported {
            opaque_item_kind(&value)
        } else {
            None
        };
        if kind.is_some() {
            import.opaque.push(OpaqueItem {
                index: None,
                value,
                kind,
                is_metadata: false,
            });
            return Vec::new();
        }
        let errors_before = import.errors.len();
        let warnings_before = import.warnings.len();
        let messages = try_converting_to_messages(&value, import);
        if messages.is_empty() && import.opaque.is_empty() {
            if expect_messages && import.errors.len() == errors_before {
                import
                    .errors
                    .push(format!("Unsupported {field} message format"));
            }
            if options.preserve_unsupported {
                import.opaque.push(OpaqueItem {
                    index: None,
                    value,
                    kind: None,
                    is_metadata: false,
                });
            }
        }
        if options.preserve_unsupported && !import.opaque.is_empty() {
            import.warnings.extend(import.errors.drain(errors_before..));
        }
        for warning in &mut import.warnings[warnings_before..] {
            *warning = format!("{field}: {warning}");
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
    if let Some(message) = metadata.and_then(try_system_message_from_metadata)
        && !input
            .iter()
            .any(|message| matches!(message, Message::System { .. }))
    {
        input.insert(0, message);
    }
    SpanMessages {
        input,
        output,
        opaque_input,
        opaque_output: import.opaque,
        errors: import.errors,
        warnings: import.warnings,
    }
}

/// Import owned messages from caller-provided values, discarding unsupported payloads.
///
/// Metadata can be omitted with `None`. Use [`import_messages_from_values_with_metadata`]
/// to deserialize metadata through a caller-provided [`ImportValue`] implementation.
pub fn import_messages_from_values<I: ImportValue, O: ImportValue>(
    input: &I,
    output: &O,
    metadata: Option<&Value>,
) -> Vec<Message> {
    import_messages_from_values_with_metadata(input, output, metadata)
}

/// Import owned messages and metadata through caller-provided deserializers.
///
/// Unrecognized values, including strings that fail deserialization, are omitted.
/// Returned messages do not borrow from the inputs; their storage can be released
/// after import. Unsupported payloads and parse diagnostics are discarded.
pub fn import_messages_from_values_with_metadata<I: ImportValue, O: ImportValue, M: ImportValue>(
    input: &I,
    output: &O,
    metadata: Option<&M>,
) -> Vec<Message> {
    let mut import = MessageImport {
        options: ImportOptions {
            preserve_unsupported: true,
        },
        discard_opaque: true,
        ..Default::default()
    };
    fn parse(value: &impl ImportValue, import: &mut MessageImport) -> Vec<Message> {
        if value.array().is_some_and(|items| items.len() == 0) || opaque_item_kind(value).is_some()
        {
            return Vec::new();
        }
        try_converting_to_messages(value, import)
    }
    let mut input = if input.kind() == InputKind::String {
        match input.parse::<String>() {
            Some(text) => vec![Message::User {
                content: UserContent::String(text),
            }],
            None => Vec::new(),
        }
    } else {
        parse(input, &mut import)
    };
    if let Some(message) = metadata.and_then(try_system_message_from_metadata)
        && !input
            .iter()
            .any(|message| matches!(message, Message::System { .. }))
    {
        input.insert(0, message);
    }
    let output = if output.kind() == InputKind::String {
        match output.parse::<String>() {
            Some(text) if !text.is_empty() => vec![Message::Assistant {
                content: AssistantContent::String(text),
                id: None,
            }],
            _ => Vec::new(),
        }
    } else {
        parse(output, &mut import)
    };
    input.extend(output);
    input
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

    // Copied calls deduplicate even when Gemini arguments have reordered JSON keys.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_tool_call_history_deduplicates_against_original_output() {
        fn tool_id(message: &Message) -> &str {
            match message {
                Message::Assistant {
                    content: AssistantContent::Array(parts),
                    ..
                } => match &parts[..] {
                    [AssistantContentPart::ToolCall { tool_call_id, .. }] => tool_call_id,
                    _ => panic!("expected a single tool call"),
                },
                Message::Tool { content } => match &content[..] {
                    [ToolContentPart::ToolResult(result)] => &result.tool_call_id,
                    _ => panic!("expected a single tool result"),
                },
                _ => panic!("expected a tool call or result"),
            }
        }

        let spans: Vec<Span> = serde_json::from_str(
            r#"[
                {
                    "output": {
                        "candidates": [{"content": {"parts": [
                            {"functionCall": {"name": "clock", "args": {
                                "zone": "UTC", "options": {"hour12": false, "precision": "minutes"}
                            }}}
                        ]}}]
                    }
                },
                {
                    "input": {"contents": [
                        {"role": "model", "parts": [{"functionCall": {"name": "weather"}}]},
                        {"role": "model", "parts": [
                            {"functionCall": {"name": "clock", "args": {
                                "options": {"precision": "minutes", "hour12": false}, "zone": "UTC"
                            }}}
                        ]},
                        {"role": "model", "parts": [{"functionCall": {"name": "weather"}}]},
                        [
                            {"functionResponse": {"name": "weather"}},
                            "Summarize the weather.",
                            {"functionResponse": {"name": "weather"}},
                            "And check the weather again.",
                            {"functionResponse": {"name": "clock"}}
                        ]
                    ]},
                    "output": {
                        "candidates": [{"content": {"parts": [
                            {"functionCall": {"name": "clock", "args": {
                                "zone": "UTC", "options": {"hour12": false, "precision": "minutes"}
                            }}}
                        ]}}]
                    }
                },
                {
                    "input": {"contents": [
                        {"role": "model", "parts": [
                            {"functionCall": {"name": "clock", "args": {
                                "options": {"hour12": false, "precision": "minutes"}, "zone": "UTC"
                            }}}
                        ]},
                        {"role": "user", "parts": [{"functionResponse": {"name": "clock"}}]},
                        {"role": "model", "parts": [
                            {"functionCall": {"name": "clock", "args": {
                                "zone": "UTC", "options": {"precision": "minutes", "hour12": false}
                            }}}
                        ]}
                    ]}
                }
            ]"#,
        )
        .unwrap();

        let borrowed_messages: Vec<_> = spans
            .iter()
            .flat_map(|span| {
                import_messages_from_values(
                    span.input.as_ref().unwrap_or(&Value::Null),
                    span.output.as_ref().unwrap_or(&Value::Null),
                    None,
                )
            })
            .collect();
        let messages = import_messages_from_spans(spans);
        assert_eq!(
            serde_json::to_value(borrowed_messages).unwrap(),
            serde_json::to_value(&messages).unwrap()
        );
        let [
            original,
            weather,
            copied,
            repeated_weather,
            weather_result,
            _,
            repeated_weather_result,
            _,
            clock_result,
            new_clock,
            _,
            _,
            copied_new_clock,
        ] = &messages[..]
        else {
            panic!("expected all calls and results with intervening text");
        };
        assert_eq!(tool_id(weather), tool_id(weather_result));
        assert_eq!(tool_id(copied), tool_id(clock_result));
        assert_eq!(tool_id(repeated_weather), tool_id(repeated_weather_result));
        assert_ne!(tool_id(weather), tool_id(copied));
        assert_ne!(tool_id(weather), tool_id(repeated_weather));
        assert_eq!(tool_id(original), tool_id(clock_result));
        assert_ne!(tool_id(new_clock), tool_id(clock_result));
        assert_eq!(tool_id(new_clock), tool_id(copied_new_clock));
        assert_eq!(
            super::super::dedup::deduplicate_messages_iter(messages.iter()).len(),
            9
        );
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_keeps_tool_state_after_rejecting_message_array() {
        let candidate = crate::serde_json::json!({
            "role": "user",
            "content": {"role": "model", "parts": [{"functionCall": {"name": "weather"}}]}
        });
        let unsupported = crate::serde_json::json!({
            "role": "model",
            "parts": [{"audioTranscription": {"text": "Unsupported"}}]
        });
        let output = crate::serde_json::json!({
            "candidates": [{"content": {"role": "model", "parts": [
                {"functionCall": {"name": "weather"}}
            ]}}]
        });
        let import_with_input = |input| {
            import_span(Span {
                input,
                output: Some(output.clone()),
                other: Default::default(),
            })
            .unwrap()
        };
        let clean = import_with_input(None);
        let expected_output = serde_json::to_value(&clean.output).unwrap();
        let mixed = crate::serde_json::json!([candidate, unsupported]);
        for input in [
            mixed.clone(),
            crate::serde_json::json!([{"message": candidate}, {"message": unsupported}]),
            crate::serde_json::json!([
                {"role": "model", "parts": [{"functionCall": {"name": "weather"}}]},
                unsupported
            ]),
        ] {
            let imported = import_with_input(Some(input));
            assert!(imported.input.is_empty());
            assert!(!imported.errors.is_empty());
            assert_eq!(
                serde_json::to_value(imported.output).unwrap(),
                expected_output
            );
        }

        let supported = import_with_input(Some(crate::serde_json::json!([candidate])));
        let preserved = import_span_with_options(
            Span {
                input: Some(mixed),
                output: Some(output),
                other: Default::default(),
            },
            ImportOptions {
                preserve_unsupported: true,
            },
        )
        .unwrap();
        assert_eq!(preserved.input.len(), 1);
        assert_eq!(preserved.opaque_input.len(), 1);
        assert_eq!(
            serde_json::to_value(preserved.output).unwrap(),
            serde_json::to_value(supported.output).unwrap()
        );
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_keeps_tool_state_after_a_failed_candidate_conversion() {
        let failing = crate::serde_json::json!({
            "candidates": [{"content": {"role": "model", "parts": [
                {"functionCall": {"name": "weather"}},
                {"audioTranscription": {"text": "Unsupported"}}
            ]}}]
        });
        let success = crate::serde_json::json!({
            "candidates": [{"content": {"role": "model", "parts": [
                {"functionCall": {"name": "weather"}}
            ]}}]
        });
        let history = crate::serde_json::json!({
            "contents": [{"role": "model", "parts": [{"functionCall": {"name": "weather"}}]}]
        });
        for (input, output, clean_input) in [
            (Some(failing.clone()), success.clone(), None),
            (
                Some(history.clone()),
                crate::serde_json::json!({"result": failing, "response": success}),
                Some(history),
            ),
        ] {
            let imported = import_span(Span {
                input,
                output: Some(output),
                other: Default::default(),
            })
            .unwrap();
            let clean = import_span(Span {
                input: clean_input,
                output: Some(success.clone()),
                other: Default::default(),
            })
            .unwrap();
            let [message] = imported.output.as_slice() else {
                panic!("expected a tool call from the valid response");
            };
            assert!(imported.errors.iter().any(|error| {
                error.contains("Unsupported mapping") && error.contains("audioTranscription")
            }));
            assert_eq!(
                serde_json::to_value(message).unwrap(),
                serde_json::to_value(&clean.output[0]).unwrap()
            );
        }
    }

    // Unfinished ADK chunks and standalone completion signals cannot be replayed as messages.
    #[cfg(feature = "google")]
    #[test]
    fn adk_import_reports_unsupported_lifecycle_events() {
        let content = crate::serde_json::json!({
            "role": "model",
            "parts": [{"functionCall": {"name": "weather"}}]
        });
        for (output, field) in [
            (
                crate::serde_json::json!({"content": content, "partial": true}),
                "partial",
            ),
            (
                crate::serde_json::json!({"content": content, "interrupted": true}),
                "interrupted",
            ),
            (
                crate::serde_json::json!({"content": content, "turnComplete": false}),
                "turnComplete",
            ),
            (
                crate::serde_json::json!({
                    "Content": {"parts": [{"text": "Par"}]},
                    "Partial": false,
                    "TurnComplete": false
                }),
                "turnComplete",
            ),
            (
                crate::serde_json::json!({"turnComplete": true}),
                "turnComplete",
            ),
            (
                crate::serde_json::json!({"content": {}, "turnComplete": true}),
                "turnComplete",
            ),
            (
                crate::serde_json::json!({"turn_complete": true}),
                "turnComplete",
            ),
            (
                crate::serde_json::json!({"TurnComplete": true}),
                "turnComplete",
            ),
            (
                crate::serde_json::json!({
                    "content": content,
                    "turnComplete": true,
                    "interactionStatus": "IN_PROGRESS"
                }),
                "interactionStatus IN_PROGRESS",
            ),
            (
                crate::serde_json::json!({"interaction_status": "IN_PROGRESS"}),
                "interactionStatus IN_PROGRESS",
            ),
            (
                crate::serde_json::json!({
                    "Content": {"parts": [{"text": "Par"}]},
                    "TurnComplete": true,
                    "InteractionStatus": "FUTURE_STATUS"
                }),
                "interactionStatus",
            ),
        ] {
            let imported = import_span(Span {
                output: Some(output),
                input: None,
                other: Default::default(),
            })
            .unwrap();
            assert!(
                imported.output.is_empty(),
                "unexpected message from ADK {field}"
            );
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains(field)
                })
            );
        }
    }

    // An unspecified finish reason alone cannot create a completed message.
    #[cfg(feature = "google")]
    #[test]
    fn google_import_rejects_unspecified_finish_reason_without_completion() {
        for output in [
            crate::serde_json::json!({
                "content": {"parts": [{"text": "Par"}]},
                "finishReason": "FINISH_REASON_UNSPECIFIED"
            }),
            crate::serde_json::json!({
                "candidates": [{"finishReason": "FINISH_REASON_UNSPECIFIED"}]
            }),
        ] {
            let imported = import_span(Span {
                input: None,
                output: Some(output),
                other: Default::default(),
            })
            .unwrap();
            assert!(imported.output.is_empty());
            assert!(imported.errors.iter().any(|error| {
                error.contains("Unsupported mapping") && error.contains("FINISH_REASON_UNSPECIFIED")
            }));
        }
    }

    // Continuation state cannot be carried by a replayable assistant message.
    #[cfg(feature = "google")]
    #[test]
    fn google_import_rejects_candidate_continuation_state() {
        let content = crate::serde_json::json!({"parts": [{"text": "Checking your order."}]});
        for (output, field) in [
            (
                crate::serde_json::json!({
                    "content": content,
                    "finishReason": "CONTINUATION",
                    "continuationToken": "opaque-token"
                }),
                "continuationToken",
            ),
            (
                crate::serde_json::json!({
                    "Content": content,
                    "FinishReason": "CONTINUATION"
                }),
                "finishReason CONTINUATION",
            ),
            (
                crate::serde_json::json!({"candidates": [{
                    "content": content,
                    "continuation_token": "opaque-token"
                }]}),
                "continuationToken",
            ),
        ] {
            let imported = import_span(Span {
                input: None,
                output: Some(output),
                other: Default::default(),
            })
            .unwrap();
            assert!(imported.output.is_empty());
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains(field)
                })
            );
        }
    }

    // An explicit completion signal or terminal finish reason can establish a completed ADK turn.
    #[cfg(feature = "google")]
    #[test]
    fn adk_import_keeps_completed_lifecycle_content() {
        let parts = crate::serde_json::json!([{"text": "Meal is ready."}]);
        for output in [
            crate::serde_json::json!({
                "content": {"parts": parts},
                "partial": false
            }),
            crate::serde_json::json!({
                "Content": {"parts": parts},
                "Partial": false
            }),
            crate::serde_json::json!({
                "content": {"parts": parts},
                "partial": false,
                "interrupted": false,
                "turn_complete": true,
                "interaction_status": "IDLE"
            }),
            crate::serde_json::json!({
                "content": {"parts": parts},
                "finishReason": "FINISH_REASON_UNSPECIFIED",
                "turnComplete": true
            }),
            crate::serde_json::json!({
                "Content": {"role": "model", "parts": parts},
                "FinishReason": "STOP",
                "TurnComplete": false
            }),
        ] {
            let imported = import_span(Span {
                output: Some(output),
                input: None,
                other: Default::default(),
            })
            .unwrap();
            assert!(matches!(
                imported.output.as_slice(),
                [Message::Assistant { content: AssistantContent::Array(parts), .. }]
                    if matches!(parts.as_slice(), [AssistantContentPart::Text(text)] if text.text == "Meal is ready.")
            ));
            assert!(imported.errors.is_empty());
        }
    }

    // A terminal reason paired with an explicit ADK completion can end an empty assistant turn.
    #[cfg(feature = "google")]
    #[test]
    fn adk_import_keeps_completed_contentless_turns() {
        for output in [
            crate::serde_json::json!({"finishReason": "STOP", "turnComplete": true}),
            crate::serde_json::json!({"FinishReason": "STOP", "TurnComplete": true}),
            crate::serde_json::json!({"content": {}, "finishReason": "STOP", "turnComplete": true}),
            crate::serde_json::json!({"Content": {}, "FinishReason": "STOP", "TurnComplete": true}),
        ] {
            let imported = import_span(Span {
                input: None,
                output: Some(output),
                other: Default::default(),
            })
            .unwrap();
            match imported.output.as_slice() {
                [
                    Message::Assistant {
                        content: AssistantContent::String(text),
                        ..
                    },
                ] => {
                    assert!(text.is_empty())
                }
                [
                    Message::Assistant {
                        content: AssistantContent::Array(parts),
                        ..
                    },
                ] => {
                    assert!(parts.is_empty())
                }
                messages => panic!("expected an empty ADK assistant turn, got {messages:?}"),
            }
            assert!(imported.errors.is_empty());
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn adk_error_only_output_imports_an_empty_assistant_turn() {
        for output in [
            crate::serde_json::json!({"errorCode": "PERMISSION_DENIED"}),
            crate::serde_json::json!({"errorCode": "PERMISSION_DENIED", "errorMessage": "Denied"}),
            crate::serde_json::json!({"error_code": "RESOURCE_EXHAUSTED", "error_message": "Quota"}),
            crate::serde_json::json!({"ErrorCode": "UNKNOWN_ERROR", "ErrorMessage": "Unknown"}),
        ] {
            let span = Span {
                input: None,
                output: Some(output),
                other: Default::default(),
            };
            let imported = import_span(span).unwrap();
            assert!(matches!(
                imported.output.as_slice(),
                [Message::Assistant {
                    content: AssistantContent::String(text),
                    ..
                }] if text.is_empty()
            ));
            assert!(imported.errors.is_empty());
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_preserves_unsupported_role_array_item() {
        let unsupported = crate::serde_json::json!({
            "role": "model",
            "parts": [{"text": "Unrepresentable", "partMetadata": {"stream": "a"}}]
        });
        let span = Span {
            input: Some(crate::serde_json::json!([
                {"role": "user", "content": "Keep this message."},
                unsupported
            ])),
            output: None,
            other: Default::default(),
        };
        let strict = import_span(span.clone()).unwrap();
        assert!(strict.input.is_empty());
        assert!(strict.opaque_input.is_empty());
        assert!(strict.errors.iter().any(|error| {
            error.contains("Unsupported mapping") && error.contains("partMetadata")
        }));

        let preserved = import_span_with_options(
            span,
            ImportOptions {
                preserve_unsupported: true,
            },
        )
        .unwrap();
        assert!(matches!(
            preserved.input.as_slice(),
            [Message::User { content: UserContent::String(text) }] if text == "Keep this message."
        ));
        assert!(matches!(
            preserved.opaque_input.as_slice(),
            [OpaqueItem { index: Some(1), value, kind: None, is_metadata: false }]
                if value == &unsupported
        ));
        assert!(preserved.errors.is_empty());
        assert!(preserved.warnings.iter().any(|warning| {
            warning.contains("Unsupported mapping") && warning.contains("partMetadata")
        }));
    }

    #[cfg(feature = "google")]
    #[test]
    fn adk_unsupported_part_keeps_the_google_mapping_error() {
        let span: Span = serde_json::from_str(
            r#"{
                "output": {
                    "content": {
                        "role": "model",
                        "parts": [{"toolCall": {"tool_type": "GOOGLE_MAPS"}}]
                    },
                    "finishReason": "STOP"
                }
            }"#,
        )
        .unwrap();

        let imported = import_span(span).unwrap();
        assert!(imported.output.is_empty());
        assert!(
            imported
                .errors
                .iter()
                .any(|error| error.contains("Unsupported mapping") && error.contains("toolCall"))
        );
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_keeps_direct_video_uri_with_metadata() {
        let span: Span = serde_json::from_value(crate::serde_json::json!({
            "input": {"contents": [{
                "uri": "https://generativelanguage.googleapis.com/v1beta/files/video",
                "mime_type": "video/mp4",
                "video_metadata": {},
                "mediaResolution": {"level": "MEDIA_RESOLUTION_UNSPECIFIED"}
            }]}
        }))
        .unwrap();

        let imported = import_span(span).unwrap();
        let [
            Message::User {
                content: UserContent::Array(parts),
            },
        ] = imported.input.as_slice()
        else {
            panic!("expected one user message with a file");
        };
        assert!(matches!(
            parts.as_slice(),
            [UserContentPart::File { data: Value::String(uri), media_type, .. }]
                if uri.ends_with("/files/video") && media_type == "video/mp4"
        ));
        assert!(imported.errors.is_empty());
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_media_resolution_token_budgets() {
        for token_field in ["num_tokens", "numTokens"] {
            let span: Span = serde_json::from_value(crate::serde_json::json!({
                "input": {"contents": [{"role": "user", "parts": [{
                    "inline_data": {"data": "aW1hZ2U=", "mime_type": "image/png"},
                    "media_resolution": {(token_field): 512}
                }]}]}
            }))
            .unwrap();

            let imported = import_span(span).unwrap();
            assert!(imported.input.is_empty(), "{token_field}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains("mediaResolution")
                }),
                "{token_field}"
            );
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_video_preprocessing() {
        for (field, value) in [
            ("start_offset", crate::serde_json::json!("1s")),
            ("endOffset", crate::serde_json::json!("2s")),
            ("fps", crate::serde_json::json!(2.0)),
        ] {
            let span: Span = serde_json::from_value(crate::serde_json::json!({
                "input": {"contents": [{
                    "uri": "https://generativelanguage.googleapis.com/v1beta/files/video",
                    "mime_type": "video/mp4",
                    "video_metadata": {(field): value}
                }]}
            }))
            .unwrap();
            let imported = import_span(span).unwrap();
            assert!(imported.input.is_empty(), "{field}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains("videoMetadata")
                }),
                "{field}"
            );
        }
    }

    // Signed media can't be replayed because Lingua file parts have no signature field.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_rejects_media_with_thought_signature() {
        for part in [
            crate::serde_json::json!({
                "inline_data": {"mime_type": "image/png", "data": "AA=="},
                "thought_signature": "c2ln"
            }),
            crate::serde_json::json!({
                "fileData": {"mimeType": "image/png", "fileUri": "https://example.com/image.png"},
                "thoughtSignature": "c2ln"
            }),
        ] {
            let imported = import_span(Span {
                output: Some(crate::serde_json::json!({
                    "candidates": [{"content": {"role": "model", "parts": [part]}}]
                })),
                input: None,
                other: Default::default(),
            })
            .unwrap();
            assert!(imported.output.is_empty());
            assert!(imported.errors.iter().any(|error| {
                error.contains("Unsupported mapping") && error.contains("thoughtSignature")
            }));
        }
    }

    // SDK media preprocessing cannot be omitted without changing what the model sees.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_rejects_sdk_media_preprocessing() {
        for (field, error_field, metadata, file) in [
            (
                "document_metadata",
                "documentMetadata",
                crate::serde_json::json!({"start_page": 2}),
                crate::serde_json::json!({"mime_type": "application/pdf", "data": "AA=="}),
            ),
            (
                "spreadsheetMetadata",
                "spreadsheetMetadata",
                crate::serde_json::json!({"visibleCells": "A1:D10"}),
                crate::serde_json::json!({"mime_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "data": "AA=="}),
            ),
            (
                "audio_metadata",
                "audioMetadata",
                crate::serde_json::json!({"start_offset": "1s"}),
                crate::serde_json::json!({"mime_type": "audio/wav", "data": "AA=="}),
            ),
        ] {
            let imported = import_span(Span {
                input: Some(crate::serde_json::json!({"contents": [{
                    "inline_data": file,
                    (field): metadata
                }]})),
                output: None,
                other: Default::default(),
            })
            .unwrap();
            assert!(imported.input.is_empty(), "{field}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains(error_field)
                }),
                "{field}"
            );
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_trace_part_fields() {
        for (part, field) in [
            (
                crate::serde_json::json!({
                    "toolResponse": {"id": "x", "toolType": "GOOGLE_MAPS", "response": {"ok": true}}
                }),
                "toolResponse",
            ),
            (
                crate::serde_json::json!({
                    "inlineData": {"mimeType": "image/png", "data": "AA=="},
                    "media_resolution": {"level": "MEDIA_RESOLUTION_HIGH"}
                }),
                "mediaResolution",
            ),
            (
                crate::serde_json::json!({"text": "source", "part_metadata": {"stream": "a"}}),
                "partMetadata",
            ),
        ] {
            let span: Span = serde_json::from_value(crate::serde_json::json!({
                "input": {"contents": [{"role": "user", "parts": [part]}]}
            }))
            .unwrap();
            let imported = import_span(span).unwrap();
            assert!(imported.input.is_empty(), "{field}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains(field)
                }),
                "{field}"
            );
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_code_execution() {
        for (part, field) in [
            (
                crate::serde_json::json!({
                    "executable_code": {"language": "PYTHON", "code": "print(1 + 1)"}
                }),
                "executableCode",
            ),
            (
                crate::serde_json::json!({
                    "code_execution_result": {"outcome": "OUTCOME_OK", "output": "2"}
                }),
                "codeExecutionResult",
            ),
        ] {
            let span: Span = serde_json::from_value(crate::serde_json::json!({
                "output": {"candidates": [{"content": {"role": "model", "parts": [
                    {"text": "Computing the result."},
                    part
                ]}}]}
            }))
            .unwrap();
            let imported = import_span(span).unwrap();
            assert!(imported.output.is_empty(), "{field}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains(field)
                }),
                "{field}"
            );
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_matches_idless_calls_and_results_in_direct_content() {
        let call = crate::serde_json::json!({
            "role": "model",
            "parts": [{"function_call": {"name": "weather", "args": {"city": "Paris"}}}]
        });
        let result = crate::serde_json::json!({
            "role": "tool",
            "parts": [{"function_response": {"name": "weather", "response": {"degrees": 20}}}]
        });
        let imported = import_span(Span {
            input: Some(crate::serde_json::json!([call, call])),
            output: Some(crate::serde_json::json!([result, result])),
            other: Default::default(),
        })
        .unwrap();

        assert!(imported.errors.is_empty());
        assert_eq!(imported.input.len(), 2);
        assert_eq!(imported.output.len(), 2);
        let call_ids: Vec<_> = imported
            .input
            .iter()
            .zip(&imported.output)
            .map(|(call, result)| {
                let (
                    Message::Assistant {
                        content: AssistantContent::Array(call),
                        ..
                    },
                    Message::Tool { content: result },
                ) = (call, result)
                else {
                    panic!("expected an assistant call and tool result");
                };
                let [AssistantContentPart::ToolCall { tool_call_id, .. }] = call.as_slice() else {
                    panic!("expected one tool call");
                };
                let [ToolContentPart::ToolResult(result)] = result.as_slice() else {
                    panic!("expected one tool result");
                };
                assert_eq!(tool_call_id, &result.tool_call_id);
                tool_call_id
            })
            .collect();
        assert_ne!(call_ids[0], call_ids[1]);
    }

    // A blocked Gemini prompt has an assistant turn even when no candidate was generated.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_preserves_prompt_blocked_response() {
        for output in [
            crate::serde_json::json!({
                "candidates": [],
                "promptFeedback": {"blockReason": "SAFETY"}
            }),
            crate::serde_json::json!({
                "prompt_feedback": {"block_reason": "PROHIBITED_CONTENT"}
            }),
            crate::serde_json::json!({
                "Candidates": null,
                "PromptFeedback": {"BlockReason": "BLOCKLIST"}
            }),
        ] {
            let imported = import_span(Span {
                input: None,
                output: Some(output),
                other: Default::default(),
            })
            .unwrap();
            assert!(matches!(
                imported.output.as_slice(),
                [Message::Assistant {
                    content: AssistantContent::Array(parts),
                    id: None,
                }] if parts.is_empty()
            ));
            assert!(imported.errors.is_empty(), "{:?}", imported.errors);
        }

        let unblocked = import_span(Span {
            input: None,
            output: Some(crate::serde_json::json!({
                "candidates": [],
                "promptFeedback": {"blockReason": "BLOCK_REASON_UNSPECIFIED"}
            })),
            other: Default::default(),
        })
        .unwrap();
        assert!(unblocked.output.is_empty());
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_accepts_pascal_case_response_wrapper() {
        let output = crate::serde_json::json!({
            "Candidates": [{
                "Content": {"parts": [{"text": "Go candidate reply."}]},
                "FinishReason": "STOP"
            }]
        });
        let imported = import_span(Span {
            input: None,
            output: Some(output),
            other: Default::default(),
        })
        .unwrap();

        let [
            Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            },
        ] = imported.output.as_slice()
        else {
            panic!("expected a Gemini assistant message");
        };
        assert!(matches!(
            parts.as_slice(),
            [AssistantContentPart::Text(text)] if text.text == "Go candidate reply."
        ));
        assert!(imported.errors.is_empty());
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_accepts_standalone_tool_content() {
        let tool_content = crate::serde_json::json!({
            "role": "tool",
            "parts": [{"function_response": {
                "id": "weather_call",
                "name": "weather",
                "response": {"degrees": 20}
            }}]
        });
        for input in [
            tool_content.clone(),
            crate::serde_json::json!([tool_content]),
        ] {
            let imported = import_span(Span {
                input: Some(input),
                output: None,
                other: Default::default(),
            })
            .unwrap();
            let [Message::Tool { content }] = imported.input.as_slice() else {
                panic!("expected a standalone Google tool result");
            };
            let [ToolContentPart::ToolResult(result)] = content.as_slice() else {
                panic!("expected one tool result");
            };
            assert_eq!(result.tool_call_id, "weather_call");
            assert_eq!(result.tool_name, "weather");
            assert_eq!(result.output, crate::serde_json::json!({"degrees": 20}));
            assert!(imported.errors.is_empty());
        }
    }

    // Direct Google user content must keep the instruction and pair its result with the original call.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_preserves_text_after_a_standalone_tool_result() {
        let input = crate::serde_json::json!({
            "role": "model",
            "parts": [{"functionCall": {"name": "weather"}}]
        });
        let output = crate::serde_json::json!({
            "role": "user",
            "parts": [
                {"functionResponse": {"name": "weather", "response": {"degrees": 20}}},
                {"text": "Summarize."}
            ]
        });
        for output in [output.clone(), crate::serde_json::json!([output])] {
            let imported = import_span(Span {
                input: Some(input.clone()),
                output: Some(output),
                other: Default::default(),
            })
            .unwrap();
            let [
                Message::Assistant {
                    content: AssistantContent::Array(calls),
                    ..
                },
            ] = imported.input.as_slice()
            else {
                panic!("expected a Google assistant tool call");
            };
            let [AssistantContentPart::ToolCall { tool_call_id, .. }] = calls.as_slice() else {
                panic!("expected one tool call");
            };
            let [
                Message::Tool { content },
                Message::User {
                    content: UserContent::String(text),
                },
            ] = imported.output.as_slice()
            else {
                panic!("expected the tool result followed by the user instruction");
            };
            let [ToolContentPart::ToolResult(result)] = content.as_slice() else {
                panic!("expected one tool result");
            };
            assert_eq!(&result.tool_call_id, tool_call_id);
            assert_eq!(text, "Summarize.");
            assert!(imported.errors.is_empty());
        }
    }

    // An explicit tool role cannot carry text beside a result in a Lingua tool message.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_rejects_mixed_tool_role_content() {
        let imported = import_span(Span {
            output: Some(crate::serde_json::json!({
                "role": "tool",
                "parts": [
                    {"functionResponse": {"name": "weather", "response": {"degrees": 20}}},
                    {"text": "Forecast from the airport."}
                ]
            })),
            input: None,
            other: Default::default(),
        })
        .unwrap();
        assert!(imported.output.is_empty());
        assert!(
            imported.errors.iter().any(|error| {
                error.contains("Unsupported mapping") && error.contains("tool role")
            })
        );
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_rejects_function_calls_in_nested_user_parts() {
        let span: Span = serde_json::from_value(crate::serde_json::json!({
            "input": {"contents": [
                "Before the user turn.",
                [
                    "Use this forecast.",
                    {"function_call": {"name": "weather", "args": {"city": "Paris"}}}
                ]
            ]}
        }))
        .unwrap();

        let imported = import_span(span).unwrap();
        assert!(imported.input.is_empty());
        assert!(imported.errors.iter().any(|error| {
            error.contains("Unsupported mapping")
                && error.contains("user")
                && error.contains("functionCall")
        }));
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_keeps_text_between_explicit_content_tool_results() {
        let span: Span = serde_json::from_value(crate::serde_json::json!({
            "input": {"contents": [
                {"role": "model", "parts": [
                    {"function_call": {"name": "weather"}},
                    {"function_call": {"name": "clock"}}
                ]},
                {"role": "user", "parts": [
                    {"function_response": {"name": "weather", "response": {"degrees": 20}}},
                    {"text": "Summarize."},
                    {"function_response": {"name": "clock", "response": {"hour": 12}}},
                    {"text": "Include the time."}
                ]}
            ]}
        }))
        .unwrap();

        let imported = import_span(span).unwrap();
        let [
            Message::Assistant {
                content: AssistantContent::Array(calls),
                ..
            },
            Message::Tool { content: weather },
            Message::User {
                content: UserContent::String(summarize),
            },
            Message::Tool { content: clock },
            Message::User {
                content: UserContent::String(include_time),
            },
        ] = imported.input.as_slice()
        else {
            panic!("expected tool results and text in their original order");
        };
        assert_eq!(summarize, "Summarize.");
        assert_eq!(include_time, "Include the time.");
        for (call, result) in calls.iter().zip([weather, clock]) {
            let AssistantContentPart::ToolCall {
                tool_call_id,
                tool_name,
                ..
            } = call
            else {
                panic!("expected a tool call");
            };
            let [ToolContentPart::ToolResult(result)] = result.as_slice() else {
                panic!("expected a tool result");
            };
            assert_eq!(tool_call_id, &result.tool_call_id);
            assert_eq!(tool_name, &result.tool_name);
        }
        assert_eq!(calls.len(), 2);
        assert!(imported.errors.is_empty());
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_request_part() {
        let span: Span = serde_json::from_value(crate::serde_json::json!({
            "input": {"contents": [{"role": "user", "parts": [
                {"toolCall": {"tool_type": "GOOGLE_MAPS"}}
            ]}]}
        }))
        .unwrap();
        let imported = import_span(span).unwrap();
        assert!(imported.input.is_empty());
        assert!(
            imported.errors.iter().any(|error| {
                error.contains("Unsupported mapping") && error.contains("toolCall")
            })
        );
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_candidate_part() {
        let span: Span = serde_json::from_value(crate::serde_json::json!({
            "output": {"candidates": [
                {"content": {"role": "model", "parts": [{"text": "A plain candidate"}]}},
                {"content": {"role": "model", "parts": [
                    {"toolCall": {"tool_type": "GOOGLE_MAPS"}}
                ]}}
            ]}
        }))
        .unwrap();
        let imported = import_span(span).unwrap();
        assert!(imported.output.is_empty());
        assert!(
            imported.errors.iter().any(|error| {
                error.contains("Unsupported mapping") && error.contains("toolCall")
            })
        );
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_direct_content_and_candidates() {
        let content = crate::serde_json::json!({
            "role": "user",
            "parts": [{"toolCall": {"tool_type": "GOOGLE_MAPS"}}]
        });
        let candidate = crate::serde_json::json!({"content": content.clone()});
        for input in [
            content,
            candidate.clone(),
            crate::serde_json::json!([
                {"content": {"role": "model", "parts": [{"text": "Plain candidate"}]}},
                candidate
            ]),
        ] {
            let imported = import_span(Span {
                input: Some(input),
                output: None,
                other: Default::default(),
            })
            .unwrap();
            assert!(imported.input.is_empty());
            assert!(imported.errors.iter().any(|error| {
                error.contains("Unsupported mapping") && error.contains("toolCall")
            }));
        }
    }

    // An unrecognized part must not become an empty message in a trace.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_rejects_unrecognized_content_parts() {
        let content = crate::serde_json::json!({
            "role": "model",
            "parts": [{"unknownPayload": {"value": "opaque"}}]
        });
        for input in [
            content.clone(),
            crate::serde_json::json!({"content": content}),
            crate::serde_json::json!({"candidates": [{"content": content}]}),
            crate::serde_json::json!({"contents": [content]}),
        ] {
            let imported = import_span(Span {
                input: Some(input),
                output: None,
                other: Default::default(),
            })
            .unwrap();
            assert!(imported.input.is_empty());
            assert!(
                imported
                    .errors
                    .iter()
                    .any(|error| { error.contains("unrecognized Google content part") })
            );
        }
    }

    // Streaming call state can contain unfinished arguments and cannot be replayed as a completed call.
    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_rejects_continuing_function_calls() {
        for (field, value, error_field) in [
            (
                "will_continue",
                crate::serde_json::json!(true),
                "willContinue",
            ),
            (
                "willContinue",
                crate::serde_json::json!(true),
                "willContinue",
            ),
            (
                "partial_args",
                crate::serde_json::json!([{"json_path": "$.city", "string_value": "Par"}]),
                "partialArgs",
            ),
            (
                "partialArgs",
                crate::serde_json::json!([{"jsonPath": "$.city", "stringValue": "Par"}]),
                "partialArgs",
            ),
        ] {
            let imported = import_span(Span {
                input: None,
                output: Some(crate::serde_json::json!({"candidates": [{"content": {
                    "role": "model",
                    "parts": [{"function_call": {"name": "weather", (field): value}}]
                }}]})),
                other: Default::default(),
            })
            .unwrap();
            assert!(imported.output.is_empty(), "{field}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains(error_field)
                }),
                "{field}"
            );
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_rejects_continuing_function_responses() {
        for continuation in ["will_continue", "willContinue"] {
            let span: Span = serde_json::from_value(crate::serde_json::json!({
                "input": {"contents": [{"role": "user", "parts": [{
                    "function_response": {"name": "work", "response": {}, (continuation): true}
                }]}]}
            }))
            .unwrap();
            let imported = import_span(span).unwrap();
            assert!(imported.input.is_empty(), "{continuation}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping") && error.contains("willContinue")
                }),
                "{continuation}"
            );
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_keeps_completed_function_responses() {
        for continuation in ["will_continue", "willContinue"] {
            let span: Span = serde_json::from_value(crate::serde_json::json!({
                "input": {"contents": [
                    {"text": "Run the tool.", "partMetadata": {}},
                    {"role": "model", "parts": [{"functionCall": {
                        "name": "work",
                        "id": "call_a",
                        "will_continue": false,
                        "partialArgs": []
                    }}]},
                    {"role": "user", "parts": [{
                        "function_response": {
                            "name": "work",
                            "id": "call_a",
                            "response": {"value": 1},
                            "parts": [],
                            "scheduling": "SCHEDULING_UNSPECIFIED",
                            (continuation): false
                        }
                    }]}
                ]}
            }))
            .unwrap();

            let imported = import_span(span).unwrap();
            let [
                Message::User { .. },
                Message::Assistant { .. },
                Message::Tool { content },
            ] = imported.input.as_slice()
            else {
                panic!("expected a prompt with a completed tool response: {continuation}");
            };
            let [ToolContentPart::ToolResult(result)] = content.as_slice() else {
                panic!("expected one tool result: {continuation}");
            };
            assert_eq!(result.tool_call_id, "call_a");
            assert_eq!(result.output, crate::serde_json::json!({"value": 1}));
            assert!(imported.errors.is_empty(), "{continuation}");
        }
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_function_response_media() {
        let span: Span = serde_json::from_value(crate::serde_json::json!({
            "input": {"contents": [{"role": "user", "parts": [{
                "functionResponse": {
                    "name": "render",
                    "willContinue": false,
                    "response": {"image": {"$ref": "chart.png"}},
                    "parts": [{"inlineData": {"mimeType": "image/png", "data": "AA=="}}]
                }
            }]}]}
        }))
        .unwrap();

        let imported = import_span(span).unwrap();
        assert!(imported.input.is_empty());
        assert!(imported.errors.iter().any(|error| {
            error.contains("Unsupported mapping") && error.contains("functionResponse.parts")
        }));
    }

    #[cfg(feature = "google")]
    #[test]
    fn gemini_import_reports_unsupported_function_response_scheduling() {
        for scheduling in ["SILENT", "INTERRUPT", "WHEN_IDLE"] {
            let span: Span = serde_json::from_value(crate::serde_json::json!({
                "input": {"contents": [{"role": "user", "parts": [{
                    "function_response": {
                        "name": "update",
                        "response": {},
                        "will_continue": false,
                        "scheduling": scheduling
                    }
                }]}]}
            }))
            .unwrap();
            let imported = import_span(span).unwrap();
            assert!(imported.input.is_empty(), "{scheduling}");
            assert!(
                imported.errors.iter().any(|error| {
                    error.contains("Unsupported mapping")
                        && error.contains("functionResponse.scheduling")
                }),
                "{scheduling}"
            );
        }
    }

    #[test]
    fn imported_span_accepts_integral_float_metrics() {
        let span: Span = serde_json::from_str(
            r#"{
                "created": "2026-10-27T18:25:56.948Z",
                "metrics": {
                    "start": 1793125556.941876,
                    "end": 1793125562.4392064,
                    "prompt_tokens": 46696.0,
                    "completion_tokens": 286.0,
                    "prompt_cached_tokens": 46030.0,
                    "prompt_cache_creation_tokens": 663.0,
                    "completion_reasoning_tokens": 77.0,
                    "tokens": 46982.0
                },
                "span_attributes": {"type": "llm", "name": "LLM"},
                "input": [{"role": "user", "content": "Hello"}],
                "output": [{"role": "assistant", "content": "Hi"}]
            }"#,
        )
        .unwrap();
        let imported = import_span(span).unwrap();
        assert_eq!(
            imported.header.start.unwrap().timestamp_micros(),
            1793125556941876
        );
        assert_eq!(
            imported.header.end.unwrap().timestamp_micros(),
            1793125562439206
        );
        let usage = imported.usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(46696));
        assert_eq!(usage.completion_tokens, Some(286));
        assert_eq!(usage.prompt_cached_tokens, Some(46030));
        assert_eq!(usage.prompt_cache_creation_tokens, Some(663));
        assert_eq!(usage.completion_reasoning_tokens, Some(77));
        assert_eq!(usage.total_tokens, Some(46982));
        assert_eq!(imported.input.len(), 1);
        assert_eq!(imported.output.len(), 1);
        assert!(imported.errors.is_empty());
    }

    #[test]
    fn imported_span_rejects_fractional_token_counts() {
        for field in ["tokens", "prompt_tokens", "completion_tokens"] {
            let span: Span =
                serde_json::from_str(&format!("{{\"metrics\":{{\"{field}\":1.5}}}}")).unwrap();
            let error = import_span(span).unwrap_err();
            assert!(
                error.starts_with("Invalid span fields:"),
                "{field}: {error}"
            );
            assert!(error.contains("expected an integer"), "{field}: {error}");
        }
    }

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
