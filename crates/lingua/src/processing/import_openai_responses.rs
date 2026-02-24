use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::{AssistantContent, AssistantContentPart, Message, TextContentPart};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum ResponsesImportItemKind {
    #[serde(rename = "function_call_output")]
    FunctionCallOutput,
    #[serde(rename = "function_call_result")]
    FunctionCallResult,
    #[serde(rename = "custom_tool_call_output")]
    CustomToolCallOutput,
    #[serde(rename = "image_generation_call")]
    ImageGenerationCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ResponsesImportItemType {
    Known(ResponsesImportItemKind),
    Other(String),
}

#[derive(Debug, Deserialize)]
struct ResponsesImportItemKindProbe {
    #[serde(rename = "type")]
    item_type: ResponsesImportItemKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponsesImportCallIdCompatItem {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    item_type: Option<ResponsesImportItemType>,
    #[serde(default, alias = "callId", skip_serializing_if = "Option::is_none")]
    call_id: Option<serde_json::Value>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum ResponsesImportFunctionOutputKind {
    #[serde(rename = "function_call_output")]
    FunctionCallOutput,
    #[serde(rename = "function_call_result")]
    FunctionCallResult,
    #[serde(rename = "custom_tool_call_output")]
    CustomToolCallOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponsesImportFunctionOutputCompatItem {
    #[serde(rename = "type")]
    item_type: ResponsesImportFunctionOutputKind,
    #[serde(default, alias = "callId", skip_serializing_if = "Option::is_none")]
    call_id: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_json")]
    output: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponsesImportImageGenerationCompatItem {
    #[serde(rename = "type")]
    item_type: ResponsesImportItemKind,
    #[serde(default, alias = "callId", skip_serializing_if = "Option::is_none")]
    call_id: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_optional_string_or_json")]
    result: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

fn deserialize_optional_string_or_json<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        serde_json::Value::String(s) => Ok(Some(s)),
        other => Ok(Some(other.to_string())),
    }
}

fn normalize_responses_import_item_value(
    original: &serde_json::Value,
) -> Option<serde_json::Value> {
    let original = original.clone();
    let item_kind = serde_json::from_value::<ResponsesImportItemKindProbe>(original.clone())
        .ok()
        .map(|probe| probe.item_type);

    let normalized = match item_kind {
        Some(
            ResponsesImportItemKind::FunctionCallOutput
            | ResponsesImportItemKind::FunctionCallResult,
        )
        | Some(ResponsesImportItemKind::CustomToolCallOutput) => {
            let mut compat =
                serde_json::from_value::<ResponsesImportFunctionOutputCompatItem>(original.clone())
                    .ok()?;
            if matches!(
                compat.item_type,
                ResponsesImportFunctionOutputKind::FunctionCallResult
            ) {
                compat.item_type = ResponsesImportFunctionOutputKind::FunctionCallOutput;
            }
            serde_json::to_value(compat).ok()?
        }
        Some(ResponsesImportItemKind::ImageGenerationCall) => {
            let compat = serde_json::from_value::<ResponsesImportImageGenerationCompatItem>(
                original.clone(),
            )
            .ok()?;
            serde_json::to_value(compat).ok()?
        }
        _ => {
            let compat =
                serde_json::from_value::<ResponsesImportCallIdCompatItem>(original.clone()).ok()?;
            serde_json::to_value(compat).ok()?
        }
    };

    if normalized == original {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_responses_import_items(data: &serde_json::Value) -> Option<serde_json::Value> {
    let arr = serde_json::from_value::<Vec<serde_json::Value>>(data.clone()).ok()?;
    let mut changed = false;
    let mut normalized = Vec::with_capacity(arr.len());

    for item in arr {
        if let Some(normalized_item) = normalize_responses_import_item_value(&item) {
            changed = true;
            normalized.push(normalized_item);
        } else {
            normalized.push(item);
        }
    }

    if changed {
        Some(serde_json::Value::Array(normalized))
    } else {
        None
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

fn try_from_responses_items_candidate(candidate: &serde_json::Value) -> Option<Vec<Message>> {
    let wrapped;
    let candidate = if candidate.is_object() {
        wrapped = serde_json::Value::Array(vec![candidate.clone()]);
        &wrapped
    } else {
        candidate
    };

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

    None
}

pub(crate) fn try_import_openai_responses(data: &serde_json::Value) -> Option<Vec<Message>> {
    if let Some(messages) = try_from_responses_items_candidate(data) {
        return Some(messages);
    }

    let normalized = normalize_responses_import_items(data)?;
    try_from_responses_items_candidate(&normalized)
}
