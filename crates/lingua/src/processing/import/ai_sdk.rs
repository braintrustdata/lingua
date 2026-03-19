use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct AISDKMessageCompat {
    role: String,
    content: Value,
    #[serde(default, alias = "tool_call_id", alias = "toolCallId")]
    tool_call_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum AISDKContentPartCompat {
    #[serde(rename = "text", alias = "input_text", alias = "output_text")]
    Text { text: String },
    #[serde(rename = "reasoning")]
    Reasoning { text: String },
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(default)]
        signature: Option<String>,
    },
    #[serde(rename = "tool-call", alias = "tool_call", alias = "toolCall")]
    ToolCall {
        #[serde(alias = "toolCallId")]
        tool_call_id: String,
        #[serde(default, alias = "toolName")]
        tool_name: String,
        #[serde(default, alias = "input")]
        input: Option<Value>,
        #[serde(default)]
        args: Option<Value>,
    },
    #[serde(rename = "tool-result", alias = "tool_result", alias = "toolResult")]
    ToolResult {
        #[serde(alias = "toolCallId")]
        tool_call_id: String,
        #[serde(default, alias = "toolName")]
        tool_name: String,
        output: Value,
    },
    #[serde(rename = "image")]
    Image { image: Value },
    #[serde(rename = "file")]
    File {
        data: Value,
        #[serde(default)]
        filename: Option<String>,
        #[serde(default, alias = "mediaType")]
        media_type: Option<String>,
    },
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
        Some(Value::String(text)) => ToolCallArguments::from(text),
        Some(other) => serde_json::to_string(&other)
            .map(ToolCallArguments::Invalid)
            .unwrap_or_else(|_| ToolCallArguments::Invalid(String::new())),
        None => ToolCallArguments::Invalid(String::new()),
    }
}

fn parse_tool_result_output(value: Value) -> Value {
    match value {
        Value::Object(mut map) => {
            let wrapper_type = map
                .get("type")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            if let Some(value) = map.remove("value") {
                if matches!(wrapper_type.as_deref(), Some("json" | "text")) {
                    return parse_tool_result_output(value);
                }
                map.insert("value".to_string(), value);
            }
            Value::Object(map)
        }
        Value::String(text) => serde_json::from_str(&text).unwrap_or(Value::String(text)),
        other => other,
    }
}

fn infer_media_type(data: &Value, explicit: Option<String>) -> String {
    if let Some(media_type) = explicit {
        return media_type;
    }

    data.as_object()
        .and_then(|obj| obj.get("content_type"))
        .and_then(|value| value.as_str())
        .unwrap_or("application/octet-stream")
        .to_string()
}

fn parse_user_content(value: Value) -> Option<UserContent> {
    match value {
        Value::String(text) => Some(UserContent::String(text)),
        Value::Array(parts) => {
            let mut converted_parts = Vec::new();

            for part in parts {
                let parsed = serde_json::from_value::<AISDKContentPartCompat>(part.clone());
                let Ok(parsed) = parsed else {
                    if let Some(text) = value_to_string(&part) {
                        converted_parts.push(UserContentPart::Text(TextContentPart {
                            text,
                            encrypted_content: None,
                            provider_options: None,
                        }));
                    }
                    continue;
                };

                match parsed {
                    AISDKContentPartCompat::Text { text } => {
                        converted_parts.push(UserContentPart::Text(TextContentPart {
                            text,
                            encrypted_content: None,
                            provider_options: None,
                        }));
                    }
                    AISDKContentPartCompat::Image { image } => {
                        converted_parts.push(UserContentPart::Image {
                            image: serde_json::json!({
                                "type": "image_url",
                                "image_url": { "url": image }
                            }),
                            media_type: None,
                            provider_options: None,
                        });
                    }
                    AISDKContentPartCompat::File {
                        data,
                        filename,
                        media_type,
                    } => {
                        let media_type = infer_media_type(&data, media_type);
                        converted_parts.push(UserContentPart::File {
                            data,
                            filename,
                            media_type,
                            provider_options: None,
                        });
                    }
                    AISDKContentPartCompat::Reasoning { .. }
                    | AISDKContentPartCompat::Thinking { .. }
                    | AISDKContentPartCompat::ToolCall { .. }
                    | AISDKContentPartCompat::ToolResult { .. } => {}
                }
            }

            if converted_parts.is_empty() {
                None
            } else if converted_parts.len() == 1 {
                match converted_parts.into_iter().next()? {
                    UserContentPart::Text(text) => Some(UserContent::String(text.text)),
                    part => Some(UserContent::Array(vec![part])),
                }
            } else {
                Some(UserContent::Array(converted_parts))
            }
        }
        other => value_to_string(&other).map(UserContent::String),
    }
}

fn parse_assistant_parts(parts: Vec<Value>) -> Vec<AssistantContentPart> {
    let mut converted_parts = Vec::new();

    for part in parts {
        let parsed = serde_json::from_value::<AISDKContentPartCompat>(part.clone());
        let Ok(parsed) = parsed else {
            if let Some(text) = value_to_string(&part) {
                if !text.is_empty() {
                    converted_parts.push(AssistantContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        provider_options: None,
                    }));
                }
            }
            continue;
        };

        match parsed {
            AISDKContentPartCompat::Text { text } => {
                if !text.is_empty() {
                    converted_parts.push(AssistantContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        provider_options: None,
                    }));
                }
            }
            AISDKContentPartCompat::Reasoning { text } => {
                if !text.is_empty() {
                    converted_parts.push(AssistantContentPart::Reasoning {
                        text,
                        encrypted_content: None,
                    });
                }
            }
            AISDKContentPartCompat::Thinking { thinking, signature } => {
                if !thinking.is_empty() {
                    converted_parts.push(AssistantContentPart::Reasoning {
                        text: thinking,
                        encrypted_content: signature,
                    });
                }
            }
            AISDKContentPartCompat::ToolCall {
                tool_call_id,
                tool_name,
                input,
                args,
            } => {
                converted_parts.push(AssistantContentPart::ToolCall {
                    tool_call_id,
                    tool_name,
                    arguments: parse_tool_call_arguments(input.or(args)),
                    encrypted_content: None,
                    provider_options: None,
                    provider_executed: None,
                });
            }
            AISDKContentPartCompat::ToolResult {
                tool_call_id,
                tool_name,
                output,
            } => {
                converted_parts.push(AssistantContentPart::ToolResult {
                    tool_call_id,
                    tool_name,
                    output: parse_tool_result_output(output),
                    provider_options: None,
                });
            }
            AISDKContentPartCompat::Image { .. } | AISDKContentPartCompat::File { .. } => {}
        }
    }

    converted_parts
}

fn parse_assistant_content(value: Value) -> Option<AssistantContent> {
    match value {
        Value::String(text) => Some(AssistantContent::String(text)),
        Value::Array(parts) => {
            let converted_parts = parse_assistant_parts(parts);
            if converted_parts.is_empty() {
                None
            } else if converted_parts.len() == 1 {
                match converted_parts.into_iter().next()? {
                    AssistantContentPart::Text(text) => Some(AssistantContent::String(text.text)),
                    part => Some(AssistantContent::Array(vec![part])),
                }
            } else {
                Some(AssistantContent::Array(converted_parts))
            }
        }
        other => value_to_string(&other).map(AssistantContent::String),
    }
}

fn parse_tool_content(value: Value) -> Option<Vec<ToolContentPart>> {
    let Value::Array(parts) = value else {
        return None;
    };

    let mut converted_parts = Vec::new();
    for part in parts {
        let parsed = serde_json::from_value::<AISDKContentPartCompat>(part).ok()?;
        if let AISDKContentPartCompat::ToolResult {
            tool_call_id,
            tool_name,
            output,
        } = parsed
        {
            converted_parts.push(ToolContentPart::ToolResult(ToolResultContentPart {
                tool_call_id,
                tool_name,
                output: parse_tool_result_output(output),
                provider_options: None,
            }));
        }
    }

    if converted_parts.is_empty() {
        None
    } else {
        Some(converted_parts)
    }
}

fn parse_message(message: AISDKMessageCompat) -> Option<Message> {
    match message.role.as_str() {
        "system" => Some(Message::System {
            content: parse_user_content(message.content)?,
        }),
        "user" => Some(Message::User {
            content: parse_user_content(message.content)?,
        }),
        "assistant" => Some(Message::Assistant {
            content: parse_assistant_content(message.content)?,
            id: None,
        }),
        "tool" => {
            let content = parse_tool_content(message.content).or_else(|| {
                let tool_call_id = message.tool_call_id.or(message.id)?;
                Some(vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id,
                    tool_name: message.name.unwrap_or_default(),
                    output: Value::Null,
                    provider_options: None,
                })])
            })?;
            Some(Message::Tool { content })
        }
        _ => None,
    }
}

fn parse_message_sequence(value: &Value) -> Option<Vec<Message>> {
    let items = value.as_array()?;
    let messages: Option<Vec<Message>> = items
        .iter()
        .cloned()
        .map(|item| serde_json::from_value::<AISDKMessageCompat>(item).ok())
        .map(|item| item.and_then(parse_message))
        .collect();
    let messages = messages?;

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

fn has_input_wrapper_shape(data: &Value) -> bool {
    let Value::Object(obj) = data else {
        return false;
    };

    obj.contains_key("prompt") || obj.contains_key("system") || obj.contains_key("messages")
}

fn try_parse_prompt_value(value: &Value) -> Option<Vec<Message>> {
    match value {
        Value::String(text) => Some(vec![Message::User {
            content: UserContent::String(text.clone()),
        }]),
        Value::Array(_) => parse_message_sequence(value),
        _ => None,
    }
}

fn try_parse_ai_sdk_input(data: &Value) -> Option<Vec<Message>> {
    if !has_input_wrapper_shape(data) {
        return None;
    }

    let obj = data.as_object()?;
    let mut messages = Vec::new();

    if let Some(system_value) = obj.get("system") {
        let content = parse_user_content(system_value.clone())?;
        messages.push(Message::System { content });
    }

    if let Some(message_value) = obj.get("messages") {
        if let Some(parsed) = parse_message_sequence(message_value) {
            messages.extend(parsed);
        }
    }

    if let Some(prompt_value) = obj.get("prompt") {
        if let Some(parsed) = try_parse_prompt_value(prompt_value) {
            messages.extend(parsed);
        }
    }

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

fn has_ai_sdk_output_signal(obj: &serde_json::Map<String, Value>) -> bool {
    if obj.contains_key("steps")
        || obj.contains_key("responseMessages")
        || obj.contains_key("toolCalls")
        || obj.contains_key("toolResults")
        || obj.contains_key("finishReason")
        || obj.contains_key("warnings")
        || obj.contains_key("providerMetadata")
        || obj.contains_key("experimental_providerMetadata")
        || obj.contains_key("includeRawChunks")
        || obj.contains_key("reasoning")
    {
        return true;
    }

    if obj.contains_key("object") && (obj.contains_key("response") || obj.contains_key("usage")) {
        return true;
    }

    obj.get("response")
        .and_then(|value| value.as_object())
        .map(|response| {
            response.contains_key("messages")
                || response.contains_key("headers")
                || response.contains_key("body")
                || response.contains_key("modelId")
                || response.contains_key("timestamp")
        })
        .unwrap_or(false)
}

fn build_assistant_message_from_fields(obj: &serde_json::Map<String, Value>) -> Option<Message> {
    if let Some(content) = obj.get("content") {
        let content = parse_assistant_content(content.clone())?;
        return Some(Message::Assistant { content, id: None });
    }

    let mut parts = Vec::new();

    if let Some(reasoning) = obj.get("reasoning").and_then(value_to_string) {
        if !reasoning.is_empty() {
            parts.push(AssistantContentPart::Reasoning {
                text: reasoning,
                encrypted_content: None,
            });
        }
    }

    if let Some(text) = obj.get("text").and_then(value_to_string) {
        if !text.is_empty() {
            parts.push(AssistantContentPart::Text(TextContentPart {
                text,
                encrypted_content: None,
                provider_options: None,
            }));
        }
    }

    if let Some(tool_calls_value) = obj.get("toolCalls").and_then(|value| value.as_array()) {
        parts.extend(parse_assistant_parts(tool_calls_value.clone()));
    }

    if parts.is_empty() {
        if let Some(object_value) = obj.get("object") {
            let text = serde_json::to_string(object_value).ok()?;
            return Some(Message::Assistant {
                content: AssistantContent::String(text),
                id: None,
            });
        }
        return None;
    }

    let content = if parts.len() == 1 {
        match parts.pop() {
            Some(AssistantContentPart::Text(text)) => AssistantContent::String(text.text),
            Some(part) => AssistantContent::Array(vec![part]),
            None => return None,
        }
    } else {
        AssistantContent::Array(parts)
    };

    Some(Message::Assistant { content, id: None })
}

fn parse_step_message(step: &Value) -> Option<Message> {
    let obj = step.as_object()?;

    if let Some(content) = obj.get("content") {
        let Value::Array(parts) = content else {
            return None;
        };
        let assistant_parts: Vec<AssistantContentPart> = parts
            .iter()
            .filter_map(|part| {
                let parsed = serde_json::from_value::<AISDKContentPartCompat>(part.clone()).ok()?;
                match parsed {
                    AISDKContentPartCompat::Text { text } => {
                        if text.is_empty() {
                            None
                        } else {
                            Some(AssistantContentPart::Text(TextContentPart {
                                text,
                                encrypted_content: None,
                                provider_options: None,
                            }))
                        }
                    }
                    AISDKContentPartCompat::Reasoning { text } => {
                        if text.is_empty() {
                            None
                        } else {
                            Some(AssistantContentPart::Reasoning {
                                text,
                                encrypted_content: None,
                            })
                        }
                    }
                    AISDKContentPartCompat::Thinking { thinking, signature } => {
                        if thinking.is_empty() {
                            None
                        } else {
                            Some(AssistantContentPart::Reasoning {
                                text: thinking,
                                encrypted_content: signature,
                            })
                        }
                    }
                    AISDKContentPartCompat::ToolCall {
                        tool_call_id,
                        tool_name,
                        input,
                        args,
                    } => Some(AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments: parse_tool_call_arguments(input.or(args)),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: None,
                    }),
                    AISDKContentPartCompat::ToolResult { .. }
                    | AISDKContentPartCompat::Image { .. }
                    | AISDKContentPartCompat::File { .. } => None,
                }
            })
            .collect();

        if !assistant_parts.is_empty() {
            let content = if assistant_parts.len() == 1 {
                match assistant_parts.into_iter().next()? {
                    AssistantContentPart::Text(text) => AssistantContent::String(text.text),
                    part => AssistantContent::Array(vec![part]),
                }
            } else {
                AssistantContent::Array(assistant_parts)
            };
            return Some(Message::Assistant { content, id: None });
        }
    }

    let response_messages = obj
        .get("response")
        .and_then(|response| response.as_object())
        .and_then(|response| response.get("messages"))?;
    let parsed = parse_message_sequence(response_messages)?;
    parsed.into_iter().find(|message| matches!(message, Message::Assistant { .. }))
}

fn try_parse_ai_sdk_output(data: &Value) -> Option<Vec<Message>> {
    let obj = data.as_object()?;

    if !has_ai_sdk_output_signal(obj) {
        return None;
    }

    if let Some(steps) = obj.get("steps").and_then(|value| value.as_array()) {
        let messages: Vec<Message> = steps.iter().filter_map(parse_step_message).collect();
        if !messages.is_empty() {
            return Some(messages);
        }
    }

    if let Some(response_messages) = obj.get("responseMessages") {
        if let Some(messages) = parse_message_sequence(response_messages) {
            return Some(messages);
        }
    }

    if let Some(response_messages) = obj
        .get("response")
        .and_then(|response| response.as_object())
        .and_then(|response| response.get("messages"))
    {
        if let Some(messages) = parse_message_sequence(response_messages) {
            return Some(messages);
        }
    }

    build_assistant_message_from_fields(obj).map(|message| vec![message])
}

pub(crate) fn try_parse_ai_sdk_for_import(data: &Value) -> Option<Vec<Message>> {
    if let Some(messages) = try_parse_ai_sdk_input(data) {
        return Some(messages);
    }

    try_parse_ai_sdk_output(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prompt_and_steps_wrapper() {
        let input = serde_json::json!({
            "prompt": "What is the capital of France?"
        });
        let output = serde_json::json!({
            "steps": [
                {
                    "content": [
                        { "type": "reasoning", "text": "" },
                        { "type": "text", "text": "The capital of France is Paris." }
                    ]
                }
            ]
        });

        let input_messages = try_parse_ai_sdk_for_import(&input).expect("should parse input");
        let output_messages = try_parse_ai_sdk_for_import(&output).expect("should parse output");

        assert_eq!(input_messages.len(), 1);
        assert_eq!(output_messages.len(), 1);
    }

    #[test]
    fn parses_provider_level_output_with_reasoning() {
        let output = serde_json::json!({
            "reasoning": "2 plus 2 equals 4",
            "text": "The answer is 4.",
            "toolCalls": []
        });

        let messages = try_parse_ai_sdk_for_import(&output).expect("should parse output");
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn parses_message_array_with_attachments() {
        let input = serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image",
                            "image": {
                                "content_type": "image/png",
                                "key": "file_123",
                                "type": "braintrust_attachment"
                            }
                        },
                        {
                            "type": "text",
                            "text": "What color is this image?"
                        }
                    ]
                }
            ]
        });

        let messages = try_parse_ai_sdk_for_import(&input).expect("should parse input");
        assert_eq!(messages.len(), 1);
    }
}
