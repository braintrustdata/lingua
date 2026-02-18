use crate::serde_json::{self, Value};
use crate::universal::tools::{BuiltinToolProvider, UniversalTool};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum OpenAIChatToolWire {
    #[serde(rename = "function")]
    Function { function: OpenAIChatFunctionWire },
    #[serde(rename = "custom")]
    Custom { custom: OpenAIChatCustomWire },
}

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
    tool_type: String,
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
    let parsed: OpenAIChatToolWire = serde_json::from_value(value.clone()).ok()?;

    match parsed {
        OpenAIChatToolWire::Function { function } => Some(UniversalTool::function(
            function.name,
            function.description,
            function.parameters,
            function.strict,
        )),
        OpenAIChatToolWire::Custom { custom } => Some(UniversalTool::custom(
            custom.name,
            custom.description,
            custom.format,
        )),
    }
}

fn parse_openai_responses_tool(value: &Value) -> Option<UniversalTool> {
    let header: OpenAIResponsesToolHeader = serde_json::from_value(value.clone()).ok()?;

    match header.tool_type.as_ref() {
        "function" => {
            let function: OpenAIResponsesFunctionWire =
                serde_json::from_value(value.clone()).ok()?;
            Some(UniversalTool::function(
                function.name,
                function.description,
                function.parameters,
                function.strict,
            ))
        }
        "custom" => {
            let custom: OpenAIResponsesCustomWire = serde_json::from_value(value.clone()).ok()?;
            Some(UniversalTool::custom(
                custom.name,
                custom.description,
                custom.format,
            ))
        }
        "code_interpreter"
        | "web_search_preview"
        | "mcp"
        | "file_search"
        | "computer_use_preview" => Some(UniversalTool::builtin(
            header.tool_type.clone(),
            BuiltinToolProvider::Responses,
            header.tool_type,
            Some(value.clone()),
        )),
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
}
