/*!
OpenAI import helpers for processing traces.

This module is used by `processing/import.rs` to detect and import OpenAI-shaped
message payloads from spans.

What this module does:
- Detects OpenAI Responses API items via generated enum types.
- Imports canonical OpenAI chat/responses payloads through strict typed converters.
- Handles non-canonical trace shapes with narrow OpenAI-specific fallbacks.
- Merges adjacent reasoning-only assistant messages with the following assistant message.

What this module does not do:
- It does not own generic cross-provider import orchestration.
- It does not replace strict provider conversion logic in `providers/openai/convert.rs`.
*/

use crate::providers::openai::convert::ChatCompletionRequestMessageExt;
use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::convert::TryFromLLM;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};

pub(crate) fn is_openai_responses_item(obj: &serde_json::Map<String, Value>) -> bool {
    let Some(item_type) = obj.get("type").cloned() else {
        return false;
    };

    serde_json::from_value::<openai::InputItemType>(item_type.clone()).is_ok()
        || serde_json::from_value::<openai::OutputItemType>(item_type).is_ok()
}

pub(crate) fn try_import_openai_messages(data: &Value) -> Option<Vec<Message>> {
    let wrapped;
    let candidate = if data.is_object() {
        wrapped = Value::Array(vec![data.clone()]);
        &wrapped
    } else {
        data
    };

    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<ChatCompletionRequestMessageExt>>(candidate.clone())
    {
        if let Ok(messages) =
            <Vec<Message> as TryFromLLM<Vec<ChatCompletionRequestMessageExt>>>::try_from(
                provider_messages,
            )
        {
            if !messages.is_empty() {
                return Some(messages);
            }
        }
    }

    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<openai::InputItem>>(candidate.clone())
    {
        if let Ok(messages) =
            <Vec<Message> as TryFromLLM<Vec<openai::InputItem>>>::try_from(provider_messages)
        {
            if !messages.is_empty() {
                return Some(merge_adjacent_reasoning_assistant_messages(messages));
            }
        }
    }

    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<openai::OutputItem>>(candidate.clone())
    {
        if let Ok(messages) =
            <Vec<Message> as TryFromLLM<Vec<openai::OutputItem>>>::try_from(provider_messages)
        {
            if !messages.is_empty() {
                return Some(merge_adjacent_reasoning_assistant_messages(messages));
            }
        }
    }

    let lenient_messages = try_lenient_openai_non_role_parsing(candidate)?;
    if lenient_messages.is_empty() {
        None
    } else {
        Some(merge_adjacent_reasoning_assistant_messages(
            lenient_messages,
        ))
    }
}

fn merge_adjacent_reasoning_assistant_messages(messages: Vec<Message>) -> Vec<Message> {
    let mut merged: Vec<Message> = Vec::with_capacity(messages.len());

    for message in messages {
        let can_merge = match (merged.last(), &message) {
            (
                Some(Message::Assistant {
                    content: AssistantContent::Array(prev_parts),
                    ..
                }),
                Message::Assistant { .. },
            ) => {
                !prev_parts.is_empty()
                    && prev_parts
                        .iter()
                        .all(|part| matches!(part, AssistantContentPart::Reasoning { .. }))
            }
            _ => false,
        };

        if !can_merge {
            merged.push(message);
            continue;
        }

        let previous = merged.pop().expect("previous message should exist");
        let Message::Assistant {
            content: AssistantContent::Array(reasoning_parts),
            id: reasoning_id,
        } = previous
        else {
            unreachable!("checked previous assistant reasoning message above");
        };

        let Message::Assistant {
            content: next_content,
            id: next_id,
        } = message
        else {
            unreachable!("checked current assistant message above");
        };

        let mut combined_parts = reasoning_parts;
        match next_content {
            AssistantContent::Array(parts) => combined_parts.extend(parts),
            AssistantContent::String(text) => {
                combined_parts.push(AssistantContentPart::Text(TextContentPart {
                    text,
                    encrypted_content: None,
                    provider_options: None,
                }))
            }
        }

        merged.push(Message::Assistant {
            content: AssistantContent::Array(combined_parts),
            id: next_id.or(reasoning_id),
        });
    }

    merged
}

fn try_lenient_openai_non_role_parsing(data: &Value) -> Option<Vec<Message>> {
    let arr = data.as_array()?;
    let mut messages = Vec::new();

    for item in arr {
        if let Some(message) = parse_lenient_openai_message_item(item) {
            messages.push(message);
            continue;
        }

        if let Some(message) = parse_lenient_openai_non_role_item(item) {
            messages.push(message);
        }
    }

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

fn parse_lenient_openai_message_item(item: &Value) -> Option<Message> {
    let obj = item.as_object()?;
    if !obj.contains_key("role") {
        return None;
    }
    if !is_openai_role_message_candidate(obj) {
        return None;
    }

    let wrapped = Value::Array(vec![item.clone()]);
    if let Ok(provider_messages) =
        serde_json::from_value::<Vec<ChatCompletionRequestMessageExt>>(wrapped)
    {
        if let Ok(mut messages) = <Vec<Message> as TryFromLLM<
            Vec<ChatCompletionRequestMessageExt>,
        >>::try_from(provider_messages)
        {
            if !messages.is_empty() {
                return Some(messages.remove(0));
            }
        }
    }

    let role = obj.get("role")?.as_str()?;
    let content = obj.get("content")?;
    match role {
        "user" => Some(Message::User {
            content: parse_openai_user_content(content)?,
        }),
        "system" => Some(Message::System {
            content: parse_openai_user_content(content)?,
        }),
        "developer" => Some(Message::Developer {
            content: parse_openai_user_content(content)?,
        }),
        "assistant" => Some(Message::Assistant {
            content: parse_openai_assistant_content(content)?,
            id: obj.get("id").and_then(Value::as_str).map(str::to_string),
        }),
        _ => None,
    }
}

fn is_openai_role_message_candidate(obj: &serde_json::Map<String, Value>) -> bool {
    let input_item_type = obj
        .get("type")
        .and_then(|v| serde_json::from_value::<openai::InputItemType>(v.clone()).ok());
    let output_item_type = obj
        .get("type")
        .and_then(|v| serde_json::from_value::<openai::OutputItemType>(v.clone()).ok());

    if matches!(input_item_type, Some(openai::InputItemType::Message))
        || matches!(output_item_type, Some(openai::OutputItemType::Message))
    {
        return true;
    }

    let Some(parts) = obj.get("content").and_then(Value::as_array) else {
        return false;
    };

    parts.iter().filter_map(Value::as_object).any(|part| {
        matches!(
            part.get("type").and_then(Value::as_str),
            Some("input_text" | "output_text" | "input_image")
        )
    })
}

fn parse_lenient_openai_non_role_item(item: &Value) -> Option<Message> {
    let obj = item.as_object()?;
    let input_item_type = obj
        .get("type")
        .and_then(|v| serde_json::from_value::<openai::InputItemType>(v.clone()).ok());
    let output_item_type = obj
        .get("type")
        .and_then(|v| serde_json::from_value::<openai::OutputItemType>(v.clone()).ok());

    let parse_tool_call = || {
        let tool_call_id = tool_call_id_from_obj(obj).unwrap_or_default();
        let tool_name = tool_name_from_obj(obj);
        let arguments = parse_tool_arguments(obj.get("arguments"));
        Some(Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id,
                tool_name,
                arguments,
                encrypted_content: None,
                provider_options: None,
                provider_executed: None,
            }]),
            id: obj.get("id").and_then(Value::as_str).map(str::to_string),
        })
    };

    let parse_tool_result = |output: Value, default_name: &str| {
        let tool_call_id = tool_call_id_from_obj(obj).unwrap_or_default();
        let tool_name = tool_name_from_obj(obj);
        Some(Message::Tool {
            content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                tool_call_id,
                tool_name: if tool_name.is_empty() {
                    default_name.to_string()
                } else {
                    tool_name
                },
                output,
                provider_options: None,
            })],
        })
    };

    let parse_reasoning = || {
        let text = reasoning_text_from_obj(obj)?;
        Some(Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                text,
                encrypted_content: None,
            }]),
            id: obj.get("id").and_then(Value::as_str).map(str::to_string),
        })
    };

    if let Some(input_item_type) = input_item_type {
        match input_item_type {
            openai::InputItemType::FunctionCall => return parse_tool_call(),
            openai::InputItemType::FunctionCallOutput
            | openai::InputItemType::CustomToolCallOutput => {
                return parse_tool_result(
                    obj.get("output").cloned().unwrap_or(Value::Null),
                    "tool",
                );
            }
            openai::InputItemType::ImageGenerationCall => {
                return Some(Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::ToolResult {
                        tool_call_id: tool_call_id_from_obj(obj).unwrap_or_default(),
                        tool_name: "image_generation_call".to_string(),
                        output: obj.get("result").cloned().unwrap_or(Value::Null),
                        provider_options: None,
                    }]),
                    id: obj.get("id").and_then(Value::as_str).map(str::to_string),
                });
            }
            openai::InputItemType::WebSearchCall => {
                return parse_tool_result(
                    obj.get("action").cloned().unwrap_or(Value::Null),
                    "web_search",
                );
            }
            openai::InputItemType::Reasoning => return parse_reasoning(),
            _ => {}
        }
    }

    if let Some(output_item_type) = output_item_type {
        match output_item_type {
            openai::OutputItemType::FunctionCall => return parse_tool_call(),
            openai::OutputItemType::ImageGenerationCall => {
                return Some(Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::ToolResult {
                        tool_call_id: tool_call_id_from_obj(obj).unwrap_or_default(),
                        tool_name: "image_generation_call".to_string(),
                        output: obj.get("result").cloned().unwrap_or(Value::Null),
                        provider_options: None,
                    }]),
                    id: obj.get("id").and_then(Value::as_str).map(str::to_string),
                });
            }
            openai::OutputItemType::WebSearchCall => {
                return parse_tool_result(
                    obj.get("action").cloned().unwrap_or(Value::Null),
                    "web_search",
                );
            }
            openai::OutputItemType::Reasoning => return parse_reasoning(),
            _ => {}
        }
    }

    if obj.get("type").and_then(Value::as_str) == Some("function_call_result") {
        return parse_tool_result(obj.get("output").cloned().unwrap_or(Value::Null), "tool");
    }

    None
}

fn tool_call_id_from_obj(obj: &serde_json::Map<String, Value>) -> Option<String> {
    obj.get("call_id")
        .and_then(Value::as_str)
        .or_else(|| obj.get("callId").and_then(Value::as_str))
        .or_else(|| obj.get("id").and_then(Value::as_str))
        .map(str::to_string)
}

fn tool_name_from_obj(obj: &serde_json::Map<String, Value>) -> String {
    obj.get("name")
        .and_then(Value::as_str)
        .or_else(|| obj.get("toolName").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

fn parse_tool_arguments(arguments: Option<&Value>) -> ToolCallArguments {
    match arguments {
        Some(Value::Object(map)) => ToolCallArguments::Valid(map.clone()),
        Some(Value::String(s)) => ToolCallArguments::from(s.clone()),
        Some(other) => match serde_json::to_string(other) {
            Ok(s) => ToolCallArguments::Invalid(s),
            Err(_) => ToolCallArguments::Invalid(String::new()),
        },
        None => ToolCallArguments::Invalid(String::new()),
    }
}

fn reasoning_text_from_obj(obj: &serde_json::Map<String, Value>) -> Option<String> {
    let summary = obj.get("summary")?.as_array()?;
    let texts: Vec<&str> = summary
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect();

    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

fn parse_openai_user_content(value: &Value) -> Option<UserContent> {
    match value {
        Value::String(s) => Some(UserContent::String(s.clone())),
        Value::Array(arr) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(Value::String(content_type)) = obj.get("type") {
                        if matches!(content_type.as_str(), "text" | "input_text" | "output_text") {
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

fn parse_openai_assistant_content(value: &Value) -> Option<AssistantContent> {
    match value {
        Value::String(s) => Some(AssistantContent::String(s.clone())),
        Value::Array(arr) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(Value::String(content_type)) = obj.get("type") {
                        if matches!(content_type.as_str(), "text" | "input_text" | "output_text") {
                            if let Some(Value::String(text)) = obj.get("text") {
                                parts.push(AssistantContentPart::Text(TextContentPart {
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
                Some(AssistantContent::Array(parts))
            }
        }
        _ => None,
    }
}
