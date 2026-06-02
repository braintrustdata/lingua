use crate::providers::openai::generated as openai;
use crate::serde_json::{self, Value};
use crate::universal::tools::{BuiltinToolProvider, UniversalTool, UniversalToolType};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OpenAIChatFunctionWire {
    name: String,
    description: Option<String>,
    parameters: Option<Value>,
    strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatCustomWire {
    name: String,
    description: Option<String>,
    format: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponsesToolHeader {
    #[serde(rename = "type")]
    tool_type: openai::ToolType,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponsesFunctionWire {
    name: String,
    description: Option<String>,
    parameters: Option<Value>,
    strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponsesCustomWire {
    name: String,
    description: Option<String>,
    format: Option<Value>,
}

fn parse_tool_array(tools: &Value) -> Vec<Value> {
    serde_json::from_value(tools.clone()).unwrap_or_default()
}

fn parse_openai_chat_tool(value: &Value) -> Option<UniversalTool> {
    #[derive(Debug, Deserialize)]
    struct OpenAIChatToolHeader {
        #[serde(rename = "type")]
        tool_type: String,
    }

    let header: OpenAIChatToolHeader = serde_json::from_value(value.clone()).ok()?;

    match header.tool_type.as_ref() {
        "function" => {
            #[derive(Debug, Deserialize)]
            struct OpenAIChatFunctionEnvelope {
                function: OpenAIChatFunctionWire,
            }

            let function: OpenAIChatFunctionEnvelope =
                serde_json::from_value(value.clone()).ok()?;
            Some(UniversalTool::function(
                function.function.name,
                function.function.description,
                function.function.parameters,
                function.function.strict,
            ))
        }
        "custom" => {
            #[derive(Debug, Deserialize)]
            struct OpenAIChatCustomEnvelope {
                custom: OpenAIChatCustomWire,
            }

            let custom: OpenAIChatCustomEnvelope = serde_json::from_value(value.clone()).ok()?;
            Some(UniversalTool::custom(
                custom.custom.name,
                custom.custom.description,
                custom.custom.format,
            ))
        }
        _ => None,
    }
}

fn parse_openai_responses_tool(value: &Value) -> Option<UniversalTool> {
    let header: OpenAIResponsesToolHeader = serde_json::from_value(value.clone()).ok()?;

    match header.tool_type {
        openai::ToolType::Function => {
            let function: OpenAIResponsesFunctionWire =
                serde_json::from_value(value.clone()).ok()?;
            Some(UniversalTool::function(
                function.name,
                function.description,
                function.parameters,
                function.strict,
            ))
        }
        openai::ToolType::Custom => {
            let custom: OpenAIResponsesCustomWire = serde_json::from_value(value.clone()).ok()?;
            Some(UniversalTool::custom(
                custom.name,
                custom.description,
                custom.format,
            ))
        }
        tool_type => {
            let tool_type_name = generated_tool_type_name(tool_type)?;
            Some(UniversalTool::builtin(
                tool_type_name.clone(),
                BuiltinToolProvider::Responses,
                tool_type_name,
                Some(value.clone()),
            ))
        }
    }
}

fn generated_tool_type_name(tool_type: openai::ToolType) -> Option<String> {
    match serde_json::to_value(tool_type).ok()? {
        Value::String(tool_type) => Some(tool_type),
        _ => None,
    }
}

pub(crate) fn parse_openai_chat_tools_array(tools: &Value) -> Vec<UniversalTool> {
    parse_tool_array(tools)
        .iter()
        .filter_map(parse_openai_chat_tool)
        .collect()
}

pub(crate) fn parse_openai_responses_tools_array(tools: &Value) -> Vec<UniversalTool> {
    parse_tool_array(tools)
        .iter()
        .filter_map(parse_openai_responses_tool)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_parse_chat_custom_tool() {
        let openai = json!({
            "type": "custom",
            "custom": {
                "name": "translate",
                "description": "Translate text",
                "format": {"type": "text"}
            }
        });

        let tools = parse_openai_chat_tools_array(&json!([openai]));
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "translate");
        assert!(tools[0].is_custom());
    }

    #[test]
    fn test_parse_responses_custom_tool() {
        let responses = json!({
            "type": "custom",
            "name": "code_exec",
            "description": "Executes arbitrary Python code.",
            "format": {"type": "text"}
        });

        let tools = parse_openai_responses_tools_array(&json!([responses]));
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "code_exec");
        assert!(tools[0].is_custom());
    }

    #[test]
    fn test_parse_responses_function_tool_uses_generated_shape() {
        let responses = json!({
            "type": "function",
            "name": "lookup_inventory_sku",
            "description": "Look up the internal inventory SKU for a named item.",
            "parameters": {
                "type": "object",
                "properties": {
                    "item_name": {"type": "string"}
                },
                "required": ["item_name"]
            },
            "strict": true
        });

        let tools = parse_openai_responses_tools_array(&json!([responses]));
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "lookup_inventory_sku");
        assert!(tools[0].is_function());
        assert_eq!(tools[0].strict, Some(true));
    }

    #[test]
    fn test_parse_responses_generated_builtin_passthrough() {
        let apply_patch = json!({
            "type": "apply_patch"
        });

        let tools = parse_openai_responses_tools_array(&json!([apply_patch]));

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "apply_patch");
        assert!(tools[0].is_builtin());
        assert_eq!(
            tools[0].builtin_provider(),
            Some(BuiltinToolProvider::Responses)
        );
    }

    #[test]
    fn test_parse_responses_builtin_preserves_unknown_config_values() {
        let mcp = json!({
            "type": "mcp",
            "connector_id": "connector_new_service",
            "server_label": "new-service"
        });

        let tools = parse_openai_responses_tools_array(&json!([mcp.clone()]));

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "mcp");
        assert!(tools[0].is_builtin());
        assert_eq!(
            tools[0].builtin_provider(),
            Some(BuiltinToolProvider::Responses)
        );
        match &tools[0].tool_type {
            UniversalToolType::Builtin { config, .. } => assert_eq!(config.as_ref(), Some(&mcp)),
            other => panic!("expected builtin tool, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chat_tools_is_schema_scoped() {
        let responses_like = json!([{
            "type": "function",
            "name": "test3",
            "description": "desc3",
            "parameters": {}
        }]);
        let tools = parse_openai_chat_tools_array(&responses_like);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_parse_responses_tools_is_schema_scoped() {
        let chat_like = json!([{
            "type": "function",
            "function": {"name": "test1", "description": "desc1", "parameters": {}}
        }]);
        let tools = parse_openai_responses_tools_array(&chat_like);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_parse_responses_deferred_namespace_tool_search_tools() {
        let namespace = json!({
            "type": "namespace",
            "name": "inventory",
            "description": "Deferred inventory lookup tools.",
            "tools": [{
                "type": "function",
                "name": "lookup_inventory_sku",
                "description": "Look up the internal inventory SKU for a named item.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "item_name": {"type": "string"}
                    },
                    "required": ["item_name"]
                },
                "defer_loading": true
            }]
        });
        let tool_search = json!({
            "type": "tool_search"
        });

        let tools = parse_openai_responses_tools_array(&json!([namespace, tool_search]));

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "namespace");
        assert_eq!(tools[1].name, "tool_search");
        assert!(tools[0].is_builtin());
        assert!(tools[1].is_builtin());
        assert_eq!(
            tools[0].builtin_provider(),
            Some(BuiltinToolProvider::Responses)
        );
        assert_eq!(
            tools[1].builtin_provider(),
            Some(BuiltinToolProvider::Responses)
        );
    }
}
