use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum LangChainInputCompat {
    NestedMessages(Vec<Vec<LangChainMessageCompat>>),
    MessagesWrapper {
        messages: Vec<LangChainMessageCompat>,
    },
    Messages(Vec<LangChainMessageCompat>),
    Single(LangChainMessageCompat),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum LangChainOutputCompat {
    LlmResult(LangChainLlmResultCompat),
    MessagesWrapper {
        messages: Vec<LangChainMessageCompat>,
    },
    Single(LangChainMessageCompat),
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainLlmResultCompat {
    generations: Vec<Vec<LangChainGenerationCompat>>,
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainGenerationCompat {
    message: Option<LangChainMessageCompat>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum LangChainMessageCompat {
    Python(LangChainPythonMessageCompat),
    Js(LangChainJsMessageCompat),
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainPythonMessageCompat {
    #[serde(rename = "type")]
    message_type: String,
    content: Value,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    tool_calls: Vec<LangChainToolCallCompat>,
    #[serde(default)]
    additional_kwargs: LangChainAdditionalKwargsCompat,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct LangChainAdditionalKwargsCompat {
    #[serde(default)]
    tool_calls: Vec<LangChainToolCallCompat>,
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainJsMessageCompat {
    lc: Value,
    id: Vec<String>,
    kwargs: LangChainJsKwargsCompat,
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainJsKwargsCompat {
    content: Value,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    tool_calls: Vec<LangChainToolCallCompat>,
    #[serde(default)]
    additional_kwargs: LangChainAdditionalKwargsCompat,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum LangChainToolCallCompat {
    LangChain(LangChainNativeToolCallCompat),
    OpenAi(OpenAiToolCallCompat),
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainNativeToolCallCompat {
    id: String,
    name: String,
    #[serde(default)]
    args: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiToolCallCompat {
    id: String,
    function: OpenAiFunctionCompat,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiFunctionCompat {
    name: String,
    arguments: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum LangChainContentPartCompat {
    #[serde(rename = "text", alias = "input_text", alias = "output_text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: LangChainImageUrlCompat },
    #[serde(rename = "image")]
    Image { source: Value },
    #[serde(rename = "file")]
    File { file: LangChainFilePartCompat },
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainImageUrlCompat {
    url: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainFilePartCompat {
    file_data: Value,
    #[serde(default)]
    filename: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LangChainAttachmentCompat {
    #[serde(default, alias = "contentType")]
    content_type: Option<String>,
}

#[derive(Debug, Clone)]
enum LangChainRole {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

#[derive(Debug, Clone)]
struct NormalizedLangChainMessage {
    role: LangChainRole,
    content: Value,
    name: Option<String>,
    id: Option<String>,
    tool_call_id: Option<String>,
    tool_calls: Vec<LangChainToolCallCompat>,
}

fn normalize_role(message_type: &str) -> Option<LangChainRole> {
    let lower = message_type.to_ascii_lowercase();
    if lower.contains("human") {
        return Some(LangChainRole::User);
    }
    if lower.contains("ai") || lower.contains("assistant") {
        return Some(LangChainRole::Assistant);
    }
    if lower.contains("system") {
        return Some(LangChainRole::System);
    }
    if lower.contains("tool") {
        return Some(LangChainRole::Tool);
    }
    if lower.contains("function") {
        return Some(LangChainRole::Function);
    }
    None
}

fn normalize_langchain_message(
    message: LangChainMessageCompat,
) -> Option<NormalizedLangChainMessage> {
    match message {
        LangChainMessageCompat::Python(python) => {
            let role = normalize_role(&python.message_type)?;
            let mut tool_calls = python.tool_calls;
            tool_calls.extend(python.additional_kwargs.tool_calls);
            Some(NormalizedLangChainMessage {
                role,
                content: python.content,
                name: python.name,
                id: python.id,
                tool_call_id: python.tool_call_id,
                tool_calls,
            })
        }
        LangChainMessageCompat::Js(js) => {
            if js.id.is_empty() || js.lc.is_null() {
                return None;
            }
            let message_type = js.id.last()?;
            let role = normalize_role(message_type)?;
            let mut tool_calls = js.kwargs.tool_calls;
            tool_calls.extend(js.kwargs.additional_kwargs.tool_calls);
            Some(NormalizedLangChainMessage {
                role,
                content: js.kwargs.content,
                name: js.kwargs.name,
                id: js.kwargs.id,
                tool_call_id: js.kwargs.tool_call_id,
                tool_calls,
            })
        }
    }
}

fn value_to_string(value: Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(&value).ok(),
        Value::Null => None,
    }
}

fn parse_user_content(value: Value) -> Option<UserContent> {
    match value {
        Value::String(text) => Some(UserContent::String(text)),
        Value::Array(parts) => {
            let mut converted_parts = Vec::new();
            for part in parts {
                let parsed = serde_json::from_value::<LangChainContentPartCompat>(part);
                let Ok(parsed) = parsed else {
                    continue;
                };
                match parsed {
                    LangChainContentPartCompat::Text { text } => {
                        converted_parts.push(UserContentPart::Text(TextContentPart {
                            text,
                            encrypted_content: None,
                            provider_options: None,
                        }))
                    }
                    LangChainContentPartCompat::ImageUrl { image_url } => {
                        converted_parts.push(UserContentPart::Image {
                            image: serde_json::json!({
                                "type": "image_url",
                                "image_url": { "url": image_url.url }
                            }),
                            media_type: None,
                            provider_options: None,
                        })
                    }
                    LangChainContentPartCompat::Image { source } => {
                        converted_parts.push(UserContentPart::Image {
                            image: serde_json::json!({
                                "type": "image_url",
                                "image_url": { "url": source }
                            }),
                            media_type: None,
                            provider_options: None,
                        })
                    }
                    LangChainContentPartCompat::File { file } => {
                        let media_type = serde_json::from_value::<LangChainAttachmentCompat>(
                            file.file_data.clone(),
                        )
                        .ok()
                        .and_then(|attachment| attachment.content_type)
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                        converted_parts.push(UserContentPart::File {
                            data: file.file_data,
                            filename: file.filename,
                            media_type,
                            provider_options: None,
                        });
                    }
                }
            }
            if converted_parts.is_empty() {
                None
            } else {
                Some(UserContent::Array(converted_parts))
            }
        }
        other => value_to_string(other).map(UserContent::String),
    }
}

fn parse_tool_call_arguments(value: Option<Value>) -> ToolCallArguments {
    match value {
        Some(Value::Object(map)) => ToolCallArguments::Valid(map),
        Some(Value::String(text)) => ToolCallArguments::Invalid(text),
        Some(other) => match serde_json::to_string(&other) {
            Ok(text) => ToolCallArguments::Invalid(text),
            Err(_) => ToolCallArguments::Invalid(String::new()),
        },
        None => ToolCallArguments::Invalid(String::new()),
    }
}

fn convert_tool_call(tool_call: LangChainToolCallCompat) -> AssistantContentPart {
    match tool_call {
        LangChainToolCallCompat::LangChain(native) => AssistantContentPart::ToolCall {
            tool_call_id: native.id,
            tool_name: native.name,
            arguments: parse_tool_call_arguments(native.args),
            encrypted_content: None,
            provider_options: None,
            provider_executed: None,
        },
        LangChainToolCallCompat::OpenAi(openai) => AssistantContentPart::ToolCall {
            tool_call_id: openai.id,
            tool_name: openai.function.name,
            arguments: parse_tool_call_arguments(Some(openai.function.arguments)),
            encrypted_content: None,
            provider_options: None,
            provider_executed: None,
        },
    }
}

fn parse_assistant_content(
    content: Value,
    tool_calls: Vec<LangChainToolCallCompat>,
) -> Option<AssistantContent> {
    if tool_calls.is_empty() {
        return match content {
            Value::String(text) => Some(AssistantContent::String(text)),
            Value::Array(values) => {
                let mut parts = Vec::new();
                for value in values {
                    let parsed =
                        serde_json::from_value::<LangChainContentPartCompat>(value).ok()?;
                    if let LangChainContentPartCompat::Text { text } = parsed {
                        parts.push(AssistantContentPart::Text(TextContentPart {
                            text,
                            encrypted_content: None,
                            provider_options: None,
                        }));
                    }
                }
                if parts.is_empty() {
                    None
                } else {
                    Some(AssistantContent::Array(parts))
                }
            }
            other => value_to_string(other).map(AssistantContent::String),
        };
    }

    let mut parts = Vec::new();

    match content {
        Value::String(text) => {
            if !text.is_empty() {
                parts.push(AssistantContentPart::Text(TextContentPart {
                    text,
                    encrypted_content: None,
                    provider_options: None,
                }));
            }
        }
        Value::Array(values) => {
            for value in values {
                let parsed = serde_json::from_value::<LangChainContentPartCompat>(value).ok()?;
                if let LangChainContentPartCompat::Text { text } = parsed {
                    parts.push(AssistantContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        provider_options: None,
                    }));
                }
            }
        }
        other => {
            if let Some(text) = value_to_string(other) {
                if !text.is_empty() {
                    parts.push(AssistantContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        provider_options: None,
                    }));
                }
            }
        }
    }

    for tool_call in tool_calls {
        parts.push(convert_tool_call(tool_call));
    }

    if parts.is_empty() {
        None
    } else {
        Some(AssistantContent::Array(parts))
    }
}

fn parse_tool_message_content(
    content: Value,
    name: Option<String>,
    tool_call_id: Option<String>,
    id: Option<String>,
) -> Option<Message> {
    let tool_call_id = tool_call_id.or(id)?;
    let tool_name = name.unwrap_or_default();
    let output = match content {
        Value::String(text) => match serde_json::from_str::<Value>(&text) {
            Ok(parsed) => parsed,
            Err(_) => Value::String(text),
        },
        other => other,
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

fn convert_message(normalized: NormalizedLangChainMessage) -> Option<Message> {
    match normalized.role {
        LangChainRole::System => Some(Message::System {
            content: parse_user_content(normalized.content)?,
        }),
        LangChainRole::User => Some(Message::User {
            content: parse_user_content(normalized.content)?,
        }),
        LangChainRole::Assistant | LangChainRole::Function => Some(Message::Assistant {
            content: parse_assistant_content(normalized.content, normalized.tool_calls)?,
            id: None,
        }),
        LangChainRole::Tool => parse_tool_message_content(
            normalized.content,
            normalized.name,
            normalized.tool_call_id,
            normalized.id,
        ),
    }
}

fn try_parse_messages(messages: Vec<LangChainMessageCompat>) -> Option<Vec<Message>> {
    let mut converted = Vec::new();
    for message in messages {
        let normalized = normalize_langchain_message(message)?;
        let converted_message = convert_message(normalized)?;
        converted.push(converted_message);
    }
    if converted.is_empty() {
        None
    } else {
        Some(converted)
    }
}

fn try_parse_input_shape(data: &Value) -> Option<Vec<Message>> {
    let input = serde_json::from_value::<LangChainInputCompat>(data.clone()).ok()?;
    match input {
        LangChainInputCompat::NestedMessages(nested) => {
            let first = nested.into_iter().next()?;
            try_parse_messages(first)
        }
        LangChainInputCompat::MessagesWrapper { messages } => try_parse_messages(messages),
        LangChainInputCompat::Messages(messages) => try_parse_messages(messages),
        LangChainInputCompat::Single(message) => try_parse_messages(vec![message]),
    }
}

fn try_parse_output_shape(data: &Value) -> Option<Vec<Message>> {
    let output = serde_json::from_value::<LangChainOutputCompat>(data.clone()).ok()?;
    match output {
        LangChainOutputCompat::LlmResult(result) => {
            let first_batch = result.generations.into_iter().next()?;
            let messages: Option<Vec<LangChainMessageCompat>> = first_batch
                .into_iter()
                .map(|generation| generation.message)
                .collect();
            try_parse_messages(messages?)
        }
        LangChainOutputCompat::MessagesWrapper { messages } => try_parse_messages(messages),
        LangChainOutputCompat::Single(message) => try_parse_messages(vec![message]),
    }
}

pub(crate) fn try_parse_langchain_for_import(data: &Value) -> Option<Vec<Message>> {
    if let Some(messages) = try_parse_input_shape(data) {
        return Some(messages);
    }
    try_parse_output_shape(data)
}
