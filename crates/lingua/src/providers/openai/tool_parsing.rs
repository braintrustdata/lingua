use crate::serde_json::Value;
use crate::universal::tools::{BuiltinToolProvider, UniversalTool};

fn parse_openai_chat_tool(value: &Value) -> Option<UniversalTool> {
    let tool_type = value.get("type").and_then(Value::as_str)?;

    match tool_type {
        "function" => {
            let func = value.get("function")?;
            let name = func.get("name").and_then(Value::as_str)?;
            let description = func
                .get("description")
                .and_then(Value::as_str)
                .map(String::from);
            let parameters = func.get("parameters").cloned();
            let strict = func.get("strict").and_then(Value::as_bool);

            Some(UniversalTool::function(
                name,
                description,
                parameters,
                strict,
            ))
        }
        "custom" => {
            let custom = value.get("custom")?;
            let name = custom.get("name").and_then(Value::as_str)?;
            let description = custom
                .get("description")
                .and_then(Value::as_str)
                .map(String::from);
            let format = custom.get("format").cloned();
            Some(UniversalTool::custom(name, description, format))
        }
        _ => None,
    }
}

fn parse_openai_responses_tool(value: &Value) -> Option<UniversalTool> {
    let tool_type = value.get("type").and_then(Value::as_str)?;

    match tool_type {
        "function" => {
            let name = value.get("name").and_then(Value::as_str)?;
            let description = value
                .get("description")
                .and_then(Value::as_str)
                .map(String::from);
            let parameters = value.get("parameters").cloned();
            let strict = value.get("strict").and_then(Value::as_bool);

            Some(UniversalTool::function(
                name,
                description,
                parameters,
                strict,
            ))
        }
        "custom" => {
            let name = value.get("name").and_then(Value::as_str)?;
            let description = value
                .get("description")
                .and_then(Value::as_str)
                .map(String::from);
            let format = value.get("format").cloned();
            Some(UniversalTool::custom(name, description, format))
        }
        "code_interpreter"
        | "web_search_preview"
        | "mcp"
        | "file_search"
        | "computer_use_preview" => Some(UniversalTool::builtin(
            tool_type,
            BuiltinToolProvider::Responses,
            tool_type,
            Some(value.clone()),
        )),
        _ => None,
    }
}

pub(crate) fn parse_openai_chat_tools_array(tools: &Value) -> Vec<UniversalTool> {
    let Some(arr) = tools.as_array() else {
        return Vec::new();
    };
    arr.iter().filter_map(parse_openai_chat_tool).collect()
}

pub(crate) fn parse_openai_responses_tools_array(tools: &Value) -> Vec<UniversalTool> {
    let Some(arr) = tools.as_array() else {
        return Vec::new();
    };
    arr.iter().filter_map(parse_openai_responses_tool).collect()
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
