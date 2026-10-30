use crate::import_value::{Field, ImportValue, InputKind};
use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct AISDKInputCompat {
    #[serde(default)]
    system: Field<Value>,
    #[serde(default)]
    messages: Field<Value>,
    #[serde(default)]
    prompt: Field<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AISDKOutputCompat {
    #[serde(default)]
    content: Field<Value>,
    #[serde(default)]
    reasoning: Field<Value>,
    #[serde(default)]
    text: Field<Value>,
    #[serde(default)]
    object: Field<Value>,
    #[serde(default)]
    tool_calls: Field<Value>,
    #[serde(default)]
    response_messages: Field<Value>,
    #[serde(default)]
    response: Field<Value>,
    #[serde(default)]
    steps: Field<Value>,
    #[serde(default)]
    tool_results: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    finish_reason: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    warnings: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    provider_metadata: Field<serde::de::IgnoredAny>,
    #[serde(default, rename = "experimental_providerMetadata")]
    experimental_provider_metadata: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    include_raw_chunks: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    usage: Field<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AISDKResponseCompat {
    #[serde(default)]
    messages: Field<Value>,
    #[serde(default)]
    headers: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    body: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    model_id: Field<serde::de::IgnoredAny>,
    #[serde(default)]
    timestamp: Field<serde::de::IgnoredAny>,
}

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
                let parsed = AISDKContentPartCompat::deserialize(&part);
                let Ok(parsed) = parsed else {
                    if let Some(text) = value_to_string(&part) {
                        converted_parts.push(UserContentPart::Text(TextContentPart {
                            text,
                            encrypted_content: None,
                            cache_control: None,
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
                            cache_control: None,
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
        let parsed = AISDKContentPartCompat::deserialize(&part);
        let Ok(parsed) = parsed else {
            if let Some(text) = value_to_string(&part) {
                if !text.is_empty() {
                    converted_parts.push(AssistantContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        cache_control: None,
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
                        cache_control: None,
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
            AISDKContentPartCompat::Thinking {
                thinking,
                signature,
            } => {
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
                    caller: None,
                    encrypted_content: None,
                    provider_options: None,
                    status: None,
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
                    caller: None,
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
                custom_tool_call: None,
                caller: None,
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
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                })])
            })?;
            Some(Message::Tool { content })
        }
        _ => None,
    }
}

/// Imports one AI SDK prompt item using the strict AI SDK parser.
///
/// To match sequence import, callers must reject the entire sequence if it is
/// empty or any item returns `None`; keeping only successful items would accept
/// partial history that sequence import rejects.
pub fn try_import_ai_sdk_prompt_message(value: &impl ImportValue) -> Option<Message> {
    value.parse::<AISDKMessageCompat>().and_then(parse_message)
}

fn parse_message_sequence(value: &Value) -> Option<Vec<Message>> {
    let items = value.as_array()?;
    let messages: Option<Vec<Message>> =
        items.iter().map(try_import_ai_sdk_prompt_message).collect();
    crate::import_parse::non_empty_messages(messages?)
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

fn try_parse_ai_sdk_input(data: &impl ImportValue) -> Option<Vec<Message>> {
    if data.kind() != InputKind::Object {
        return None;
    }
    let input = data.parse::<AISDKInputCompat>()?;
    let mut messages = Vec::new();
    if let Field::Present(system) = input.system {
        messages.push(Message::System {
            content: parse_user_content(system)?,
        });
    }
    if let Field::Present(value) = input.messages {
        if let Some(parsed) = parse_message_sequence(&value) {
            messages.extend(parsed);
        }
    }
    if let Field::Present(prompt) = input.prompt {
        if let Some(parsed) = try_parse_prompt_value(&prompt) {
            messages.extend(parsed);
        }
    }
    crate::import_parse::non_empty_messages(messages)
}

fn has_ai_sdk_output_signal(output: &AISDKOutputCompat) -> bool {
    if output.steps.is_present()
        || output.response_messages.is_present()
        || output.tool_calls.is_present()
        || output.tool_results.is_present()
        || output.finish_reason.is_present()
        || output.warnings.is_present()
        || output.provider_metadata.is_present()
        || output.experimental_provider_metadata.is_present()
        || output.include_raw_chunks.is_present()
        || output.reasoning.is_present()
    {
        return true;
    }
    if output.object.is_present() && (output.response.is_present() || output.usage.is_present()) {
        return true;
    }
    output
        .response
        .as_ref()
        .and_then(|response| response.parse::<AISDKResponseCompat>())
        .is_some_and(|response| {
            response.messages.is_present()
                || response.headers.is_present()
                || response.body.is_present()
                || response.model_id.is_present()
                || response.timestamp.is_present()
        })
}

fn build_assistant_message_from_fields(output: &AISDKOutputCompat) -> Option<Message> {
    if let Some(content) = output.content.as_ref() {
        let content = parse_assistant_content(content.clone())?;
        return Some(Message::Assistant { content, id: None });
    }

    let mut parts = Vec::new();

    if let Some(reasoning) = output.reasoning.as_ref().and_then(value_to_string) {
        if !reasoning.is_empty() {
            parts.push(AssistantContentPart::Reasoning {
                text: reasoning,
                encrypted_content: None,
            });
        }
    }

    if let Some(text) = output.text.as_ref().and_then(value_to_string) {
        if !text.is_empty() {
            parts.push(AssistantContentPart::Text(TextContentPart {
                text,
                encrypted_content: None,
                cache_control: None,
                provider_options: None,
            }));
        }
    }

    if let Some(tool_calls_value) = output
        .tool_calls
        .as_ref()
        .and_then(|value| value.as_array())
    {
        parts.extend(parse_assistant_parts(tool_calls_value.clone()));
    }

    if parts.is_empty() {
        if let Some(object_value) = output.object.as_ref() {
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
                let parsed = AISDKContentPartCompat::deserialize(part).ok()?;
                match parsed {
                    AISDKContentPartCompat::Text { text } => {
                        if text.is_empty() {
                            None
                        } else {
                            Some(AssistantContentPart::Text(TextContentPart {
                                text,
                                encrypted_content: None,
                                cache_control: None,
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
                    AISDKContentPartCompat::Thinking {
                        thinking,
                        signature,
                    } => {
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
                        caller: None,
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
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
    parsed
        .into_iter()
        .find(|message| matches!(message, Message::Assistant { .. }))
}

fn try_parse_ai_sdk_output(data: &impl ImportValue) -> Option<Vec<Message>> {
    if data.kind() != InputKind::Object {
        return None;
    }
    let output = data.parse::<AISDKOutputCompat>()?;
    if !has_ai_sdk_output_signal(&output) {
        return None;
    }
    if let Some(value) = output.response_messages.as_ref() {
        if let Some(messages) = parse_message_sequence(value) {
            return Some(messages);
        }
    }
    if let Some(response) = output
        .response
        .as_ref()
        .and_then(|value| value.parse::<AISDKResponseCompat>())
    {
        if let Some(value) = response.messages.as_ref() {
            if let Some(messages) = parse_message_sequence(value) {
                return Some(messages);
            }
        }
    }
    if let Some(steps) = output.steps.as_ref().and_then(Value::as_array) {
        let messages: Vec<_> = steps.iter().filter_map(parse_step_message).collect();
        if !messages.is_empty() {
            return Some(messages);
        }
    }
    build_assistant_message_from_fields(&output).map(|message| vec![message])
}

pub(crate) fn try_parse_ai_sdk_for_import(data: &impl ImportValue) -> Option<Vec<Message>> {
    try_parse_ai_sdk_input(data).or_else(|| try_parse_ai_sdk_output(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prompt_and_steps_wrapper() {
        let input = crate::serde_json::json!({
            "prompt": "What is the capital of France?"
        });
        let output = crate::serde_json::json!({
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
        let output = crate::serde_json::json!({
            "reasoning": "2 plus 2 equals 4",
            "text": "The answer is 4.",
            "toolCalls": []
        });

        let messages = try_parse_ai_sdk_for_import(&output).expect("should parse output");
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn parses_message_array_with_attachments() {
        let input = crate::serde_json::json!({
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

    #[test]
    fn prefers_response_messages_over_steps_when_tool_results_are_present() {
        let output = crate::serde_json::json!({
            "finishReason": "stop",
            "response": {
                "messages": [
                    {
                        "role": "assistant",
                        "content": [
                            {
                                "type": "tool-call",
                                "toolCallId": "call_storeA",
                                "toolName": "get_store_price",
                                "input": {
                                    "item": "laptop",
                                    "store": "StoreA"
                                }
                            }
                        ]
                    },
                    {
                        "role": "tool",
                        "content": [
                            {
                                "type": "tool-result",
                                "toolCallId": "call_storeA",
                                "toolName": "get_store_price",
                                "output": {
                                    "type": "text",
                                    "value": "{\"price\":999}"
                                }
                            }
                        ]
                    },
                    {
                        "role": "assistant",
                        "content": [
                            {
                                "type": "text",
                                "text": "StoreA costs $999."
                            }
                        ]
                    }
                ]
            },
            "steps": [
                {
                    "content": [
                        {
                            "type": "tool-call",
                            "toolCallId": "call_storeA",
                            "toolName": "get_store_price",
                            "input": {
                                "item": "laptop",
                                "store": "StoreA"
                            }
                        },
                        {
                            "type": "tool-result",
                            "toolCallId": "call_storeA",
                            "toolName": "get_store_price",
                            "output": "{\"price\":999}"
                        }
                    ]
                },
                {
                    "content": [
                        {
                            "type": "text",
                            "text": "StoreA costs $999."
                        }
                    ]
                }
            ]
        });

        let messages = try_parse_ai_sdk_for_import(&output).expect("should parse output");

        assert_eq!(messages.len(), 3);
        assert!(matches!(messages[0], Message::Assistant { .. }));
        assert!(matches!(messages[1], Message::Tool { .. }));
        assert!(matches!(messages[2], Message::Assistant { .. }));
    }
}
