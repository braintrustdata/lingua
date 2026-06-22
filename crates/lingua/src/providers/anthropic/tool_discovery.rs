use crate::error::ConvertError;
use crate::processing::transform::TransformError;
use crate::providers::anthropic::generated;
#[cfg(feature = "openai")]
use crate::providers::openai::{
    generated::NamespaceToolParam, tool_parsing::parse_openai_responses_tools_array,
};
use crate::serde_json::{self, Value};
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, ProviderOptions, ToolContentPart,
};
use crate::universal::tools::{
    BuiltinToolProvider, ToolAvailability, UniversalTool, UniversalToolType,
};
use crate::universal::{ToolDiscoveryResultContentPart, ToolDiscoveryResultItem};

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct ToolDiscoveryInputView {
    query: Option<String>,
}

pub(super) fn is_tool_search_name(name: &str) -> bool {
    matches!(
        name,
        "tool_search_tool_regex" | "tool_search_tool_bm25" | "tool_search"
    )
}

pub(super) fn tool_search_name(name: &str) -> String {
    match name {
        "tool_search" => "tool_search_tool_regex".to_string(),
        _ => name.to_string(),
    }
}

pub(super) fn tool_search_call_id(tool_call_id: &str) -> String {
    if tool_call_id.starts_with("srvtoolu_") {
        tool_call_id.to_string()
    } else {
        format!("srvtoolu_{tool_call_id}")
    }
}

pub(super) fn input_map(
    arguments: Option<Value>,
    query: Option<String>,
) -> Result<Option<serde_json::Map<String, Value>>, ConvertError> {
    match arguments {
        Some(Value::Object(map)) => Ok(Some(map)),
        Some(_) => Err(ConvertError::UnsupportedMapping {
            from: "non-object ToolDiscoveryCall arguments".to_string(),
            to: "Anthropic tool_search input",
        }),
        None if query.is_some() => Err(ConvertError::UnsupportedMapping {
            from: "query-only ToolDiscoveryCall".to_string(),
            to: "Anthropic tool_search input",
        }),
        None => Ok(None),
    }
}

pub(super) fn query(
    input: &Option<serde_json::Map<String, Value>>,
) -> Result<Option<String>, ConvertError> {
    let Some(input) = input else {
        return Ok(None);
    };
    let view: ToolDiscoveryInputView = serde_json::from_value(Value::Object(input.clone()))
        .map_err(|e| ConvertError::JsonSerializationFailed {
            field: "tool_search.input".to_string(),
            error: e.to_string(),
        })?;
    Ok(view.query)
}

pub(super) fn arguments(input: Option<serde_json::Map<String, Value>>) -> Option<Value> {
    input.map(Value::Object)
}

pub(super) fn result_from_input_content(
    tool_call_id: String,
    discovery_tool_name: String,
    content: Option<generated::InputContentBlockContent>,
) -> Result<ToolDiscoveryResultContentPart, ConvertError> {
    let tools = match content {
        Some(generated::InputContentBlockContent::RequestWebSearchToolResultError(result)) => {
            if result.request_web_search_tool_result_error_type
                == generated::RequestWebSearchToolResultErrorType::ToolSearchToolResultError
            {
                return Err(ConvertError::UnsupportedMapping {
                    from: "Anthropic tool_search_tool_result_error".to_string(),
                    to: "ToolDiscoveryResult",
                });
            }

            result
                .tool_references
                .unwrap_or_default()
                .into_iter()
                .map(|tool_reference| ToolDiscoveryResultItem {
                    tool_name: tool_reference.tool_name,
                    tool: None,
                    provider_options: None,
                })
                .collect()
        }
        Some(other) => unknown_result_items("tool_search_tool_result.content", other)?,
        None => Vec::new(),
    };

    Ok(ToolDiscoveryResultContentPart {
        tool_call_id,
        discovery_tool_name,
        tools,
        status: None,
        execution: None,
        provider_options: None,
    })
}

pub(super) fn input_result_content(
    tools: Vec<ToolDiscoveryResultItem>,
) -> generated::InputContentBlockContent {
    generated::InputContentBlockContent::RequestWebSearchToolResultError(
        generated::RequestWebSearchToolResultError {
            error_code: None,
            request_web_search_tool_result_error_type:
                generated::RequestWebSearchToolResultErrorType::ToolSearchToolSearchResult,
            content: None,
            retrieved_at: None,
            url: None,
            return_code: None,
            stderr: None,
            stdout: None,
            encrypted_stdout: None,
            error_message: None,
            file_type: None,
            num_lines: None,
            start_line: None,
            total_lines: None,
            is_file_update: None,
            lines: None,
            new_lines: None,
            new_start: None,
            old_lines: None,
            old_start: None,
            tool_references: Some(
                tools
                    .into_iter()
                    .map(|item| generated::RequestToolReferenceBlock {
                        cache_control: None,
                        tool_name: item.tool_name,
                        request_tool_reference_block_type:
                            generated::ToolReferenceType::ToolReference,
                    })
                    .collect(),
            ),
        },
    )
}

pub(super) fn result_from_response_content(
    tool_call_id: String,
    discovery_tool_name: String,
    content: Option<generated::ContentBlockContent>,
) -> Result<ToolDiscoveryResultContentPart, ConvertError> {
    let tools = match content {
        Some(generated::ContentBlockContent::ResponseWebSearchToolResultError(result)) => {
            if result.response_web_search_tool_result_error_type
                == generated::RequestWebSearchToolResultErrorType::ToolSearchToolResultError
            {
                return Err(ConvertError::UnsupportedMapping {
                    from: "Anthropic tool_search_tool_result_error".to_string(),
                    to: "ToolDiscoveryResult",
                });
            }

            result
                .tool_references
                .unwrap_or_default()
                .into_iter()
                .map(|tool_reference| ToolDiscoveryResultItem {
                    tool_name: tool_reference.tool_name,
                    tool: None,
                    provider_options: None,
                })
                .collect()
        }
        Some(other) => unknown_result_items("tool_search_tool_result.content", other)?,
        None => Vec::new(),
    };

    Ok(ToolDiscoveryResultContentPart {
        tool_call_id,
        discovery_tool_name,
        tools,
        status: None,
        execution: None,
        provider_options: None,
    })
}

pub(super) fn response_result_content(
    tools: Vec<ToolDiscoveryResultItem>,
) -> generated::ContentBlockContent {
    generated::ContentBlockContent::ResponseWebSearchToolResultError(
        generated::ResponseWebSearchToolResultError {
            error_code: None,
            response_web_search_tool_result_error_type:
                generated::RequestWebSearchToolResultErrorType::ToolSearchToolSearchResult,
            content: None,
            retrieved_at: None,
            url: None,
            return_code: None,
            stderr: None,
            stdout: None,
            encrypted_stdout: None,
            error_message: None,
            file_type: None,
            num_lines: None,
            start_line: None,
            total_lines: None,
            is_file_update: None,
            lines: None,
            new_lines: None,
            new_start: None,
            old_lines: None,
            old_start: None,
            tool_references: Some(
                tools
                    .into_iter()
                    .map(|item| generated::ResponseToolReferenceBlock {
                        tool_name: item.tool_name,
                        response_tool_reference_block_type:
                            generated::ToolReferenceType::ToolReference,
                    })
                    .collect(),
            ),
        },
    )
}

fn unknown_result_items<T: serde::Serialize>(
    field: &str,
    content: T,
) -> Result<Vec<ToolDiscoveryResultItem>, ConvertError> {
    let value =
        serde_json::to_value(content).map_err(|e| ConvertError::JsonSerializationFailed {
            field: field.to_string(),
            error: e.to_string(),
        })?;
    let mut options = serde_json::Map::new();
    options.insert("content".to_string(), value);
    Ok(vec![ToolDiscoveryResultItem {
        tool_name: "unknown".to_string(),
        tool: None,
        provider_options: Some(ProviderOptions { options }),
    }])
}

pub(super) fn is_anthropic_tool_search_builtin(tool: &UniversalTool) -> bool {
    matches!(
        &tool.tool_type,
        UniversalToolType::Builtin {
            provider: BuiltinToolProvider::Anthropic,
            builtin_type,
            ..
        } if matches!(
            &**builtin_type,
            "tool_search_tool_regex"
                | "tool_search_tool_regex_20251119"
                | "tool_search_tool_bm25"
                | "tool_search_tool_bm25_20251119"
        )
    )
}

pub(super) fn has_tool_discovery(messages: &[Message]) -> bool {
    messages.iter().any(|message| match message {
        Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } => parts
            .iter()
            .any(|part| matches!(part, AssistantContentPart::ToolDiscoveryCall { .. })),
        Message::Tool { content } => content
            .iter()
            .any(|part| matches!(part, ToolContentPart::ToolDiscoveryResult(_))),
        _ => false,
    })
}

pub(super) fn discovered_tools_from_messages(messages: &[Message]) -> Vec<UniversalTool> {
    messages
        .iter()
        .flat_map(|message| match message {
            Message::Tool { content } => content
                .iter()
                .filter_map(|part| match part {
                    ToolContentPart::ToolDiscoveryResult(result) => Some(
                        result
                            .tools
                            .iter()
                            .filter_map(|item| {
                                item.tool.clone().map(|mut tool| {
                                    tool.availability = ToolAvailability::Deferred;
                                    tool
                                })
                            })
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                })
                .flatten()
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

pub(super) fn anthropic_tool_search_tool() -> UniversalTool {
    UniversalTool::builtin(
        "tool_search_tool_regex",
        BuiltinToolProvider::Anthropic,
        "tool_search_tool_regex_20251119",
        Some(serde_json::json!({
            "name": "tool_search_tool_regex",
            "type": "tool_search_tool_regex_20251119"
        })),
    )
}

#[cfg(feature = "openai")]
fn expand_responses_discovery_tool_for_anthropic(
    tool: UniversalTool,
) -> Result<Vec<UniversalTool>, TransformError> {
    let UniversalToolType::Builtin {
        provider: BuiltinToolProvider::Responses,
        builtin_type,
        config,
    } = &tool.tool_type
    else {
        return Ok(vec![tool]);
    };

    match &**builtin_type {
        "tool_search" => Ok(vec![anthropic_tool_search_tool()]),
        "namespace" => {
            let config = config.clone().ok_or_else(|| {
                TransformError::FromUniversalFailed(format!(
                    "missing config for Responses namespace tool '{}'",
                    tool.name
                ))
            })?;
            let namespace: NamespaceToolParam = serde_json::from_value(config).map_err(|e| {
                TransformError::FromUniversalFailed(format!(
                    "invalid Responses namespace tool '{}': {}",
                    tool.name, e
                ))
            })?;
            let mut tools = parse_openai_responses_tools_array(&Value::Array(namespace.tools));
            for tool in &mut tools {
                tool.availability = ToolAvailability::Deferred;
            }
            Ok(tools)
        }
        _ => Ok(vec![tool]),
    }
}

#[cfg(not(feature = "openai"))]
fn expand_responses_discovery_tool_for_anthropic(
    tool: UniversalTool,
) -> Result<Vec<UniversalTool>, TransformError> {
    Ok(vec![tool])
}

pub(super) fn normalize_tools_for_anthropic(
    tools: Vec<UniversalTool>,
) -> Result<Vec<UniversalTool>, TransformError> {
    let mut normalized = Vec::new();

    for tool in tools {
        for expanded_tool in expand_responses_discovery_tool_for_anthropic(tool)? {
            if !normalized
                .iter()
                .any(|existing: &UniversalTool| existing.name == expanded_tool.name)
            {
                normalized.push(expanded_tool);
            }
        }
    }

    Ok(normalized)
}
