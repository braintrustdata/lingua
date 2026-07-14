use crate::error::ConvertError;
use crate::providers::openai::generated as openai;
use crate::serde_json::{self, Value};
use crate::universal::tools::{BuiltinToolProvider, UniversalTool, UniversalToolType};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, ToolContentPart,
    ToolDiscoveryResultContentPart, ToolDiscoveryResultItem,
};

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct ToolDiscoveryQueryView {
    query: Option<String>,
}

fn arguments_to_value(arguments: openai::Arguments) -> serde_json::Value {
    match arguments {
        openai::Arguments::String(value) => serde_json::Value::String(value),
        openai::Arguments::AnythingMap(value) => serde_json::Value::Object(value),
    }
}

fn arguments_from_value(value: Option<serde_json::Value>) -> Option<openai::Arguments> {
    value.map(|value| match value {
        serde_json::Value::Object(map) => openai::Arguments::AnythingMap(map),
        serde_json::Value::String(text) => openai::Arguments::String(text),
        other => openai::Arguments::String(other.to_string()),
    })
}

fn status_to_string(status: Option<openai::Status>) -> Option<String> {
    status
        .and_then(|status| serde_json::to_value(status).ok())
        .and_then(|value| serde_json::from_value(value).ok())
}

fn status_from_string(status: Option<String>) -> Option<openai::Status> {
    status.and_then(|status| serde_json::from_value(serde_json::Value::String(status)).ok())
}

fn execution_to_string(execution: Option<openai::ToolSearchExecutionType>) -> Option<String> {
    execution
        .and_then(|execution| serde_json::to_value(execution).ok())
        .and_then(|value| serde_json::from_value(value).ok())
}

fn execution_from_string(execution: Option<String>) -> Option<openai::ToolSearchExecutionType> {
    execution.and_then(|execution| serde_json::from_value(Value::String(execution)).ok())
}

fn query_from_arguments(
    arguments: &Option<openai::Arguments>,
) -> Result<Option<String>, ConvertError> {
    match arguments {
        Some(openai::Arguments::AnythingMap(map)) => {
            let view: ToolDiscoveryQueryView = serde_json::from_value(Value::Object(map.clone()))
                .map_err(|e| {
                ConvertError::JsonSerializationFailed {
                    field: "tool_discovery.arguments".to_string(),
                    error: e.to_string(),
                }
            })?;
            Ok(view.query)
        }
        Some(openai::Arguments::String(text)) => {
            let Ok(view) = serde_json::from_str::<ToolDiscoveryQueryView>(text) else {
                return Ok(None);
            };
            Ok(view.query)
        }
        None => Ok(None),
    }
}

fn tool_type_name(tool_type: &openai::ToolType) -> Option<String> {
    serde_json::to_value(tool_type)
        .ok()
        .and_then(|value| serde_json::from_value(value).ok())
}

fn universal_tool_from_input_item_tool(tool: &openai::InputItemTool) -> UniversalTool {
    let type_name = tool
        .tool_type
        .as_ref()
        .and_then(tool_type_name)
        .unwrap_or_else(|| "unknown".to_string());
    let name = tool.name.clone().unwrap_or_else(|| type_name.clone());

    let mut universal_tool = match tool.tool_type {
        Some(openai::ToolType::Function) => UniversalTool::function(
            name,
            tool.description.clone(),
            tool.parameters.clone().map(serde_json::Value::Object),
            tool.strict,
        ),
        Some(openai::ToolType::Custom) => UniversalTool::custom(
            name,
            tool.description.clone(),
            tool.format
                .as_ref()
                .and_then(|format| serde_json::to_value(format).ok()),
        ),
        _ => UniversalTool::builtin(
            name,
            BuiltinToolProvider::Responses,
            type_name,
            serde_json::to_value(tool).ok(),
        ),
    };
    if tool.defer_loading == Some(true) {
        universal_tool.availability = crate::universal::tools::ToolAvailability::Deferred;
    }
    universal_tool
}

fn input_item_tool_from_universal_tool(
    tool: &UniversalTool,
) -> Result<openai::InputItemTool, ConvertError> {
    match &tool.tool_type {
        UniversalToolType::Function => {
            let mut value = serde_json::json!({
                "type": "function",
                "name": tool.name,
            });
            let Value::Object(ref mut obj) = value else {
                unreachable!("object literal");
            };
            if let Some(description) = &tool.description {
                obj.insert(
                    "description".to_string(),
                    Value::String(description.clone()),
                );
            }
            if let Some(parameters) = &tool.parameters {
                obj.insert("parameters".to_string(), parameters.clone());
            }
            if let Some(strict) = tool.strict {
                obj.insert("strict".to_string(), Value::Bool(strict));
            }
            if tool.availability == crate::universal::tools::ToolAvailability::Deferred {
                obj.insert("defer_loading".to_string(), Value::Bool(true));
            }
            if let Some(allowed_callers) = &tool.allowed_callers {
                obj.insert(
                    "allowed_callers".to_string(),
                    serde_json::to_value(allowed_callers).map_err(|e| {
                        ConvertError::JsonSerializationFailed {
                            field: format!(
                                "Responses discovery function tool allowed_callers '{}'",
                                tool.name
                            ),
                            error: e.to_string(),
                        }
                    })?,
                );
            }
            if let Some(output_schema) = &tool.output_schema {
                obj.insert("output_schema".to_string(), output_schema.clone());
            }
            serde_json::from_value(value).map_err(|e| ConvertError::JsonSerializationFailed {
                field: format!("Responses discovery function tool '{}'", tool.name),
                error: e.to_string(),
            })
        }
        UniversalToolType::Custom { format } => {
            let mut value = serde_json::json!({
                "type": "custom",
                "name": tool.name,
            });
            let Value::Object(ref mut obj) = value else {
                unreachable!("object literal");
            };
            if let Some(description) = &tool.description {
                obj.insert(
                    "description".to_string(),
                    Value::String(description.clone()),
                );
            }
            if let Some(format) = format {
                obj.insert("format".to_string(), format.clone());
            }
            if tool.availability == crate::universal::tools::ToolAvailability::Deferred {
                obj.insert("defer_loading".to_string(), Value::Bool(true));
            }
            if let Some(allowed_callers) = &tool.allowed_callers {
                obj.insert(
                    "allowed_callers".to_string(),
                    serde_json::to_value(allowed_callers).map_err(|e| {
                        ConvertError::JsonSerializationFailed {
                            field: format!(
                                "Responses discovery custom tool allowed_callers '{}'",
                                tool.name
                            ),
                            error: e.to_string(),
                        }
                    })?,
                );
            }
            if let Some(output_schema) = &tool.output_schema {
                obj.insert("output_schema".to_string(), output_schema.clone());
            }
            serde_json::from_value(value).map_err(|e| ConvertError::JsonSerializationFailed {
                field: format!("Responses discovery custom tool '{}'", tool.name),
                error: e.to_string(),
            })
        }
        UniversalToolType::Builtin {
            provider: BuiltinToolProvider::Responses,
            config: Some(config),
            ..
        } => serde_json::from_value::<openai::InputItemTool>(config.clone()).map_err(|e| {
            ConvertError::JsonSerializationFailed {
                field: format!("Responses discovery tool '{}'", tool.name),
                error: e.to_string(),
            }
        }),
        UniversalToolType::Builtin { builtin_type, .. } => Err(ConvertError::UnsupportedToolType {
            tool_name: tool.name.clone(),
            tool_type: builtin_type.clone(),
            target_provider: crate::capabilities::ProviderFormat::Responses,
        }),
    }
}

pub(super) fn message_from_input_additional_tools(
    input: openai::InputItem,
) -> Result<Message, ConvertError> {
    match input.role {
        Some(openai::InputItemRole::Developer) => {}
        Some(role) => {
            return Err(ConvertError::UnsupportedMapping {
                from: format!("InputItemRole::{role:?}"),
                to: "universal AdditionalTools",
            });
        }
        None => {
            return Err(ConvertError::MissingRequiredField {
                field: "additional_tools role".to_string(),
            });
        }
    }

    let tools = input
        .tools
        .ok_or_else(|| ConvertError::MissingRequiredField {
            field: "additional_tools tools".to_string(),
        })?
        .into_iter()
        .map(|tool| universal_tool_from_input_item_tool(&tool))
        .collect();

    Ok(Message::AdditionalTools {
        tools,
        id: input.id,
    })
}

pub(super) fn message_from_output_additional_tools(
    item: openai::OutputItem,
) -> Result<Message, ConvertError> {
    match item.role {
        Some(openai::RoleEnum::Developer) => {}
        Some(role) => {
            return Err(ConvertError::UnsupportedMapping {
                from: format!("RoleEnum::{role:?}"),
                to: "universal AdditionalTools",
            });
        }
        None => {
            return Err(ConvertError::MissingRequiredField {
                field: "additional_tools role".to_string(),
            });
        }
    }

    let tools = item
        .tools
        .ok_or_else(|| ConvertError::MissingRequiredField {
            field: "additional_tools tools".to_string(),
        })?;
    let input_tools = serde_json::to_value(tools)
        .and_then(serde_json::from_value::<Vec<openai::InputItemTool>>)
        .map_err(|e| ConvertError::JsonSerializationFailed {
            field: "Responses additional_tools output tools".to_string(),
            error: e.to_string(),
        })?;

    Ok(Message::AdditionalTools {
        tools: input_tools
            .into_iter()
            .map(|tool| universal_tool_from_input_item_tool(&tool))
            .collect(),
        id: item.id,
    })
}

pub(super) fn input_item_tools_from_universal_tools(
    tools: &[UniversalTool],
) -> Result<Vec<openai::InputItemTool>, ConvertError> {
    tools
        .iter()
        .map(input_item_tool_from_universal_tool)
        .collect()
}

fn discovery_items_from_input_tools(
    tools: Option<Vec<openai::InputItemTool>>,
) -> Vec<ToolDiscoveryResultItem> {
    tools
        .unwrap_or_default()
        .into_iter()
        .map(|tool| {
            let universal_tool = universal_tool_from_input_item_tool(&tool);
            ToolDiscoveryResultItem {
                tool_name: universal_tool.name.clone(),
                tool: Some(universal_tool),
                provider_options: None,
            }
        })
        .collect()
}

fn discovery_items_from_output_tools(
    tools: Option<Vec<openai::OutputItemTool>>,
) -> Result<Vec<ToolDiscoveryResultItem>, ConvertError> {
    let Some(tools) = tools else {
        return Ok(Vec::new());
    };
    let input_tools = serde_json::to_value(tools)
        .and_then(serde_json::from_value::<Vec<openai::InputItemTool>>)
        .map_err(|e| ConvertError::JsonSerializationFailed {
            field: "Responses discovery output tools".to_string(),
            error: e.to_string(),
        })?;
    Ok(discovery_items_from_input_tools(Some(input_tools)))
}

fn input_tools_from_discovery_items(
    tools: Vec<ToolDiscoveryResultItem>,
) -> Result<Vec<openai::InputItemTool>, ConvertError> {
    tools
        .into_iter()
        .map(|item| {
            if let Some(tool) = item.tool {
                input_item_tool_from_universal_tool(&tool)
            } else {
                serde_json::from_value(serde_json::json!({
                    "type": "function",
                    "name": item.tool_name,
                    "parameters": {"type": "object"}
                }))
                .map_err(|e| ConvertError::JsonSerializationFailed {
                    field: "Responses discovery tool reference".to_string(),
                    error: e.to_string(),
                })
            }
        })
        .collect()
}

fn output_tools_from_discovery_items(
    tools: Vec<ToolDiscoveryResultItem>,
) -> Result<Vec<openai::OutputItemTool>, ConvertError> {
    let input_tools = input_tools_from_discovery_items(tools)?;
    serde_json::to_value(input_tools)
        .and_then(serde_json::from_value)
        .map_err(|e| ConvertError::JsonSerializationFailed {
            field: "Responses discovery output tools".to_string(),
            error: e.to_string(),
        })
}

pub(super) fn message_from_input_call(input: openai::InputItem) -> Result<Message, ConvertError> {
    let id = input.id.clone();
    let tool_call_id = input.call_id.or_else(|| input.id.clone()).ok_or_else(|| {
        ConvertError::MissingRequiredField {
            field: "tool search call_id".to_string(),
        }
    })?;
    let tool_call = AssistantContentPart::ToolDiscoveryCall {
        tool_call_id,
        discovery_tool_name: "tool_search".to_string(),
        query: query_from_arguments(&input.arguments)?,
        arguments: input.arguments.map(arguments_to_value),
        status: status_to_string(input.status),
        execution: execution_to_string(input.execution),
        provider_options: None,
    };
    Ok(Message::Assistant {
        content: AssistantContent::Array(vec![tool_call]),
        id,
    })
}

pub(super) fn message_from_input_output(input: openai::InputItem) -> Result<Message, ConvertError> {
    let tool_call_id = input.call_id.or_else(|| input.id.clone()).ok_or_else(|| {
        ConvertError::MissingRequiredField {
            field: "tool search output call_id".to_string(),
        }
    })?;
    let tool_result = ToolDiscoveryResultContentPart {
        tool_call_id,
        discovery_tool_name: "tool_search".to_string(),
        tools: discovery_items_from_input_tools(input.tools),
        status: status_to_string(input.status),
        execution: execution_to_string(input.execution),
        provider_options: None,
    };
    Ok(Message::Tool {
        content: vec![ToolContentPart::ToolDiscoveryResult(tool_result)],
    })
}

pub(super) fn input_call_from_universal(
    tool_call_id: String,
    query: Option<String>,
    arguments: Option<serde_json::Value>,
    status: Option<String>,
    execution: Option<String>,
    id: Option<String>,
) -> openai::InputItem {
    openai::InputItem {
        role: None,
        content: None,
        input_item_type: Some(openai::InputItemType::ToolSearchCall),
        id,
        call_id: Some(tool_call_id),
        arguments: arguments_from_value(
            arguments.or_else(|| query.map(|query| serde_json::json!({ "query": query }))),
        ),
        status: status_from_string(status),
        execution: execution_from_string(execution),
        ..Default::default()
    }
}

pub(super) fn input_output_from_universal(
    discovery_result: ToolDiscoveryResultContentPart,
) -> Result<openai::InputItem, ConvertError> {
    Ok(openai::InputItem {
        role: None,
        content: None,
        input_item_type: Some(openai::InputItemType::ToolSearchOutput),
        call_id: Some(discovery_result.tool_call_id),
        status: status_from_string(discovery_result.status),
        execution: execution_from_string(discovery_result.execution),
        tools: Some(input_tools_from_discovery_items(discovery_result.tools)?),
        ..Default::default()
    })
}

pub(super) fn message_from_output_output(
    item: openai::OutputItem,
) -> Result<Message, ConvertError> {
    let tool_call_id = item.call_id.or_else(|| item.id.clone()).ok_or_else(|| {
        ConvertError::MissingRequiredField {
            field: "tool search output call_id".to_string(),
        }
    })?;
    let tool_result = ToolDiscoveryResultContentPart {
        tool_call_id,
        discovery_tool_name: item.name.unwrap_or_else(|| "tool_search".to_string()),
        tools: discovery_items_from_output_tools(item.tools)?,
        status: status_to_string(item.status),
        execution: execution_to_string(item.execution),
        provider_options: None,
    };
    Ok(Message::Tool {
        content: vec![ToolContentPart::ToolDiscoveryResult(tool_result)],
    })
}

pub(super) fn part_from_output_call(
    item: openai::OutputItem,
) -> Result<AssistantContentPart, ConvertError> {
    let tool_call_id = item.call_id.or_else(|| item.id.clone()).ok_or_else(|| {
        ConvertError::MissingRequiredField {
            field: "tool search call_id".to_string(),
        }
    })?;
    Ok(AssistantContentPart::ToolDiscoveryCall {
        tool_call_id,
        discovery_tool_name: item.name.unwrap_or_else(|| "tool_search".to_string()),
        query: None,
        arguments: item.arguments,
        status: status_to_string(item.status),
        execution: execution_to_string(item.execution),
        provider_options: None,
    })
}

pub(super) fn output_call_from_universal(
    tool_call_id: String,
    query: Option<String>,
    arguments: Option<serde_json::Value>,
    status: Option<String>,
    execution: Option<String>,
    id: Option<String>,
) -> openai::OutputItem {
    openai::OutputItem {
        output_item_type: Some(openai::OutputItemType::ToolSearchCall),
        id,
        call_id: Some(tool_call_id),
        arguments: arguments.or_else(|| query.map(|query| serde_json::json!({ "query": query }))),
        status: status_from_string(status),
        execution: execution_from_string(execution),
        ..Default::default()
    }
}

pub(super) fn output_output_from_universal(
    discovery_result: ToolDiscoveryResultContentPart,
) -> Result<openai::OutputItem, ConvertError> {
    Ok(openai::OutputItem {
        output_item_type: Some(openai::OutputItemType::ToolSearchOutput),
        call_id: Some(discovery_result.tool_call_id),
        status: status_from_string(discovery_result.status),
        execution: execution_from_string(discovery_result.execution),
        tools: Some(output_tools_from_discovery_items(discovery_result.tools)?),
        ..Default::default()
    })
}
