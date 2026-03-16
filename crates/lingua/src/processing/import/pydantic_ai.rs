use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIWrapperCompat {
    #[serde(default)]
    user_prompt: Option<Value>,
    #[serde(default)]
    message_history: Vec<PydanticAIMessageCompat>,
    #[serde(default)]
    system_prompt: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIMessagesWrapperCompat {
    messages: Vec<PydanticAIMessageLikeCompat>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum PydanticAIMessageLikeCompat {
    Message(PydanticAIMessageCompat),
    MessageParts(PydanticAIMessagePartsCompat),
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIMessageCompat {
    kind: PydanticAIMessageKindCompat,
    parts: Vec<PydanticAIPartCompat>,
    #[serde(default)]
    instructions: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIMessagePartsCompat {
    parts: Vec<PydanticAIPartCompat>,
    #[serde(default)]
    instructions: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PydanticAIMessageKindCompat {
    Request,
    Response,
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIPartCompat {
    part_kind: String,
    #[serde(default)]
    content: Option<Value>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    args: Option<Value>,
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIOutputWrapperCompat {
    response: PydanticAIOutputMessageCompat,
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIOutputMessageCompat {
    parts: Vec<PydanticAIPartCompat>,
    #[serde(default)]
    instructions: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIBinaryContentCompat {
    #[serde(rename = "type")]
    item_type: String,
    attachment: PydanticAIAttachmentCompat,
}

#[derive(Debug, Clone, Deserialize)]
struct PydanticAIAttachmentCompat {
    content_type: String,
    #[serde(default)]
    filename: Option<String>,
    key: String,
    #[serde(rename = "type")]
    attachment_type: String,
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).ok(),
        Value::Null => None,
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

fn parse_tool_result_output(value: Option<Value>) -> Value {
    match value {
        Some(Value::String(text)) => match serde_json::from_str::<Value>(&text) {
            Ok(parsed) => parsed,
            Err(_) => Value::String(text),
        },
        Some(other) => other,
        None => Value::Null,
    }
}

fn parse_user_content(value: Value) -> Option<UserContent> {
    match value {
        Value::String(text) => Some(UserContent::String(text)),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                match item {
                    Value::String(text) => parts.push(UserContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        provider_options: None,
                    })),
                    other => {
                        if let Ok(binary) =
                            serde_json::from_value::<PydanticAIBinaryContentCompat>(other.clone())
                        {
                            if binary.item_type == "binary" {
                                let attachment_value = serde_json::json!({
                                    "content_type": binary.attachment.content_type,
                                    "filename": binary.attachment.filename,
                                    "key": binary.attachment.key,
                                    "type": binary.attachment.attachment_type,
                                });
                                if binary.attachment.content_type.starts_with("image/") {
                                    parts.push(UserContentPart::Image {
                                        image: serde_json::json!({
                                            "type": "image_url",
                                            "image_url": { "url": attachment_value }
                                        }),
                                        media_type: None,
                                        provider_options: None,
                                    });
                                } else {
                                    parts.push(UserContentPart::File {
                                        data: attachment_value,
                                        filename: binary.attachment.filename,
                                        media_type: binary.attachment.content_type,
                                        provider_options: None,
                                    });
                                }
                                continue;
                            }
                        }

                        if let Some(text) = value_to_string(&other) {
                            parts.push(UserContentPart::Text(TextContentPart {
                                text,
                                encrypted_content: None,
                                provider_options: None,
                            }));
                        }
                    }
                }
            }

            if parts.is_empty() {
                None
            } else if parts.len() == 1 {
                match parts.into_iter().next()? {
                    UserContentPart::Text(text) => Some(UserContent::String(text.text)),
                    part => Some(UserContent::Array(vec![part])),
                }
            } else {
                Some(UserContent::Array(parts))
            }
        }
        other => value_to_string(&other).map(UserContent::String),
    }
}

fn assistant_text_part(value: Option<Value>) -> Option<AssistantContentPart> {
    let text = value_to_string(&value?)?;
    Some(AssistantContentPart::Text(TextContentPart {
        text,
        encrypted_content: None,
        provider_options: None,
    }))
}

fn tool_call_part(part: PydanticAIPartCompat) -> Option<AssistantContentPart> {
    Some(AssistantContentPart::ToolCall {
        tool_call_id: part.tool_call_id?,
        tool_name: part.tool_name.unwrap_or_default(),
        arguments: parse_tool_call_arguments(part.args),
        encrypted_content: None,
        provider_options: None,
        provider_executed: None,
    })
}

fn flush_assistant_parts(
    messages: &mut Vec<Message>,
    assistant_parts: &mut Vec<AssistantContentPart>,
) {
    if assistant_parts.is_empty() {
        return;
    }

    let content = if assistant_parts.len() == 1 {
        match assistant_parts.pop() {
            Some(AssistantContentPart::Text(text)) => AssistantContent::String(text.text),
            Some(part) => AssistantContent::Array(vec![part]),
            None => return,
        }
    } else {
        AssistantContent::Array(std::mem::take(assistant_parts))
    };

    messages.push(Message::Assistant { content, id: None });
}

fn convert_message_parts(
    kind: PydanticAIMessageKindCompat,
    instructions: Option<String>,
    parts: Vec<PydanticAIPartCompat>,
) -> Option<Vec<Message>> {
    let mut messages = Vec::new();
    let mut assistant_parts = Vec::new();

    if let Some(instructions) = instructions {
        messages.push(Message::System {
            content: UserContent::String(instructions),
        });
    }

    for part in parts {
        match part.part_kind.as_str() {
            "system-prompt" => {
                flush_assistant_parts(&mut messages, &mut assistant_parts);
                let content = parse_user_content(part.content?)?;
                messages.push(Message::System { content });
            }
            "user-prompt" | "retry-prompt" => {
                flush_assistant_parts(&mut messages, &mut assistant_parts);
                let content = parse_user_content(part.content?)?;
                messages.push(Message::User { content });
            }
            "text" => match kind {
                PydanticAIMessageKindCompat::Request => {
                    flush_assistant_parts(&mut messages, &mut assistant_parts);
                    let content = parse_user_content(part.content?)?;
                    messages.push(Message::User { content });
                }
                PydanticAIMessageKindCompat::Response => {
                    if let Some(text) = assistant_text_part(part.content) {
                        assistant_parts.push(text);
                    }
                }
            },
            "prefill" => {
                if let Some(text) = assistant_text_part(part.content) {
                    assistant_parts.push(text);
                }
            }
            "thinking" => {
                let text = value_to_string(&part.content?)?;
                assistant_parts.push(AssistantContentPart::Reasoning {
                    text,
                    encrypted_content: part.id,
                });
            }
            "tool-call" => {
                if let Some(tool_call) = tool_call_part(part) {
                    assistant_parts.push(tool_call);
                }
            }
            "tool-return" => {
                flush_assistant_parts(&mut messages, &mut assistant_parts);
                messages.push(Message::Tool {
                    content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                        tool_call_id: part.tool_call_id?,
                        tool_name: part.tool_name.unwrap_or_default(),
                        output: parse_tool_result_output(part.content),
                        provider_options: None,
                    })],
                });
            }
            _ => {}
        }
    }

    flush_assistant_parts(&mut messages, &mut assistant_parts);

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

fn try_parse_wrapper_input(data: &Value) -> Option<Vec<Message>> {
    if !matches!(data, Value::Object(_)) {
        return None;
    }

    let wrapper = serde_json::from_value::<PydanticAIWrapperCompat>(data.clone()).ok()?;
    if wrapper.user_prompt.is_none()
        && wrapper.system_prompt.is_none()
        && wrapper.message_history.is_empty()
    {
        return None;
    }

    let mut messages = Vec::new();

    if let Some(system_prompt) = wrapper.system_prompt {
        messages.push(Message::System {
            content: UserContent::String(system_prompt),
        });
    }

    for message in wrapper.message_history {
        let converted = convert_message_parts(message.kind, message.instructions, message.parts)?;
        messages.extend(converted);
    }

    if let Some(user_prompt) = wrapper.user_prompt {
        messages.push(Message::User {
            content: parse_user_content(user_prompt)?,
        });
    }

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

fn try_parse_internal_input(data: &Value) -> Option<Vec<Message>> {
    if let Ok(wrapper) = serde_json::from_value::<PydanticAIMessagesWrapperCompat>(data.clone()) {
        return try_parse_message_sequence(wrapper.messages);
    }

    if !matches!(data, Value::Array(items) if !items.is_empty()
        && items
            .iter()
            .all(|item| matches!(item, Value::Object(obj) if obj.contains_key("parts"))))
    {
        return None;
    }

    let messages = serde_json::from_value::<Vec<PydanticAIMessageLikeCompat>>(data.clone()).ok()?;
    try_parse_message_sequence(messages)
}

fn try_parse_message_sequence(messages: Vec<PydanticAIMessageLikeCompat>) -> Option<Vec<Message>> {
    let mut converted = Vec::new();

    for message in messages {
        let parsed = match message {
            PydanticAIMessageLikeCompat::Message(message) => {
                convert_message_parts(message.kind, message.instructions, message.parts)?
            }
            PydanticAIMessageLikeCompat::MessageParts(message) => convert_message_parts(
                PydanticAIMessageKindCompat::Request,
                message.instructions,
                message.parts,
            )?,
        };
        converted.extend(parsed);
    }

    if converted.is_empty() {
        None
    } else {
        Some(converted)
    }
}

fn try_parse_output(data: &Value) -> Option<Vec<Message>> {
    let Value::Object(obj) = data else {
        return None;
    };

    if obj.contains_key("response") {
        let wrapper = serde_json::from_value::<PydanticAIOutputWrapperCompat>(data.clone()).ok()?;
        return convert_message_parts(
            PydanticAIMessageKindCompat::Response,
            wrapper.response.instructions,
            wrapper.response.parts,
        );
    }

    if !obj.contains_key("parts") {
        return None;
    }

    let direct = serde_json::from_value::<PydanticAIOutputMessageCompat>(data.clone()).ok()?;
    convert_message_parts(
        PydanticAIMessageKindCompat::Response,
        direct.instructions,
        direct.parts,
    )
}

pub(crate) fn try_parse_pydantic_ai_for_import(data: &Value) -> Option<Vec<Message>> {
    if matches!(data, Value::Array(items) if items.iter().any(|item| matches!(item, Value::Array(_))))
    {
        return None;
    }

    if let Some(messages) = try_parse_wrapper_input(data) {
        return Some(messages);
    }

    if let Some(messages) = try_parse_internal_input(data) {
        return Some(messages);
    }

    try_parse_output(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_plain_text_item_arrays() {
        let value = crate::serde_json::json!([
            {
                "content": "<lang primary=\"en-US\"/><summary>...",
                "type": "text"
            }
        ]);

        assert!(try_parse_pydantic_ai_for_import(&value).is_none());
    }
}
