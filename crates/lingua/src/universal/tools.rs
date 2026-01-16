/*!
Tool format conversion utilities for cross-provider semantic translation.

This module provides bidirectional conversion between different providers'
tool formats:
- Anthropic: `{"name": "...", "description": "...", "input_schema": {...}}`
- OpenAI: `{"type": "function", "function": {"name": "...", "description": "...", "parameters": {...}}}`

## Design

Tools are a complex case because different providers have fundamentally different
structures. Unlike simple fields like `stop` or `tool_choice`, tools require
structural transformation rather than just field renaming.

Anthropic built-in tools (bash, text_editor, web_search) have a "type" field
at the root level, but custom tools do not. OpenAI always requires "type": "function"
with the tool definition nested under "function".
*/

use crate::serde_json::{json, Value};

// =============================================================================
// Format Detection
// =============================================================================

/// Check if tools are in OpenAI format (have "type": "function" wrapper).
pub fn is_openai_format(tools: &Value) -> bool {
    tools
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|t| t.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|t| t == "function")
}

/// Check if tools are in Anthropic custom tool format (have "name" at root, no "type").
pub fn is_anthropic_custom_format(tools: &Value) -> bool {
    tools
        .as_array()
        .and_then(|arr| arr.first())
        .map(|t| t.get("name").is_some() && t.get("type").is_none())
        .unwrap_or(false)
}

/// Check if tools are in OpenAI Responses API format.
///
/// Responses API tools have `type` at top level (function, code_interpreter, web_search_preview, etc.)
/// WITHOUT a nested `function` object:
/// - Function tools: `{type: "function", name: "...", description: "...", parameters: {...}, strict: ...}`
/// - Built-in tools: `{type: "code_interpreter", ...}` or `{type: "web_search_preview"}`
///
/// Contrast with OpenAI Chat format which nests function definition under `function`:
/// `{type: "function", function: {name: "...", description: "...", parameters: {...}}}`
///
/// This returns true for ANY Responses API tool format (function or built-in).
pub fn is_responses_tool_format(tools: &Value) -> bool {
    tools
        .as_array()
        .and_then(|arr| arr.first())
        .is_some_and(|tool| {
            // Must have a "type" field (distinguishes from Anthropic custom format)
            let has_type = tool.get("type").and_then(Value::as_str).is_some();
            // Must NOT have a nested "function" field (distinguishes from OpenAI Chat format)
            let has_function_wrapper = tool.get("function").is_some();
            has_type && !has_function_wrapper
        })
}

/// Check if tools contain Anthropic built-in tools (bash, text_editor, web_search).
///
/// Returns the name of the first built-in tool found, or None if no built-in tools.
pub fn find_builtin_tool(tools: &Value) -> Option<String> {
    let arr = tools.as_array()?;
    for tool in arr {
        if let Some(tool_type) = tool.get("type").and_then(Value::as_str) {
            if tool_type.starts_with("bash_")
                || tool_type.starts_with("text_editor_")
                || tool_type.starts_with("web_search_")
            {
                return Some(tool_type.to_string());
            }
        }
    }
    None
}

// =============================================================================
// Anthropic → OpenAI Conversion
// =============================================================================

/// Convert Anthropic tool format to OpenAI format.
///
/// Handles:
/// - Custom tools: `{"name", "description", "input_schema"}` → `{"type": "function", "function": {...}}`
/// - Built-in tools: Skipped (they have no OpenAI equivalent)
///
/// # Returns
///
/// - `Some(Value::Array)` with converted tools
/// - `None` if no convertible tools found
pub fn anthropic_to_openai_tools(anthropic_tools: &Value) -> Option<Value> {
    let arr = anthropic_tools.as_array()?;
    let converted: Vec<Value> = arr
        .iter()
        .filter_map(|tool| {
            // Check for built-in tools (have "type" field like "bash_20250124")
            // Skip them as they can't be converted to OpenAI format
            if tool.get("type").and_then(Value::as_str).is_some() {
                return None;
            }

            let name = tool.get("name")?.as_str()?;
            let description = tool.get("description").and_then(Value::as_str);
            let input_schema = tool.get("input_schema").cloned().unwrap_or(json!({}));

            Some(json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": description,
                    "parameters": input_schema
                }
            }))
        })
        .collect();

    if converted.is_empty() {
        None
    } else {
        Some(Value::Array(converted))
    }
}

// =============================================================================
// OpenAI → Anthropic Conversion
// =============================================================================

/// Convert OpenAI tool format to Anthropic format.
///
/// Handles:
/// - Function tools: `{"type": "function", "function": {...}}` → `{"name", "description", "input_schema"}`
/// - Other tool types: Skipped
///
/// # Returns
///
/// - `Some(Value::Array)` with converted tools
/// - `None` if no convertible tools found
pub fn openai_to_anthropic_tools(openai_tools: &Value) -> Option<Value> {
    let arr = openai_tools.as_array()?;
    let converted: Vec<Value> = arr
        .iter()
        .filter_map(|tool| {
            // Only convert function tools
            if tool.get("type").and_then(Value::as_str) != Some("function") {
                return None;
            }

            let func = tool.get("function")?;
            let name = func.get("name")?.as_str()?;
            let description = func.get("description").and_then(Value::as_str);
            let parameters = func.get("parameters").cloned().unwrap_or(json!({}));

            Some(json!({
                "name": name,
                "description": description,
                "input_schema": parameters
            }))
        })
        .collect();

    if converted.is_empty() {
        None
    } else {
        Some(Value::Array(converted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_to_openai_custom_tool() {
        let anthropic = json!([{
            "name": "get_weather",
            "description": "Get the weather for a location",
            "input_schema": {
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }
        }]);

        let openai = anthropic_to_openai_tools(&anthropic).unwrap();
        let tool = openai.as_array().unwrap().first().unwrap();

        assert_eq!(tool.get("type").unwrap(), "function");
        assert_eq!(tool["function"]["name"], "get_weather");
        assert_eq!(
            tool["function"]["description"],
            "Get the weather for a location"
        );
        assert!(tool["function"]["parameters"]["properties"]["location"].is_object());
    }

    #[test]
    fn test_anthropic_to_openai_skips_builtin() {
        let anthropic = json!([
            {
                "type": "bash_20250124",
                "name": "bash"
            },
            {
                "name": "get_weather",
                "description": "Get weather",
                "input_schema": {}
            }
        ]);

        let openai = anthropic_to_openai_tools(&anthropic).unwrap();
        let arr = openai.as_array().unwrap();

        // Should only have the custom tool, not the built-in
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_anthropic_to_openai_all_builtin_returns_none() {
        let anthropic = json!([{
            "type": "bash_20250124",
            "name": "bash"
        }]);

        let result = anthropic_to_openai_tools(&anthropic);
        assert!(result.is_none());
    }

    #[test]
    fn test_openai_to_anthropic() {
        let openai = json!([{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the weather",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }]);

        let anthropic = openai_to_anthropic_tools(&openai).unwrap();
        let tool = anthropic.as_array().unwrap().first().unwrap();

        assert_eq!(tool.get("name").unwrap(), "get_weather");
        assert_eq!(tool.get("description").unwrap(), "Get the weather");
        assert!(tool.get("input_schema").is_some());
        // Should NOT have "type" field (that's for built-in tools only)
        assert!(tool.get("type").is_none());
    }

    #[test]
    fn test_is_openai_format() {
        let openai = json!([{"type": "function", "function": {"name": "test"}}]);
        let anthropic = json!([{"name": "test", "description": "...", "input_schema": {}}]);

        assert!(is_openai_format(&openai));
        assert!(!is_openai_format(&anthropic));
    }

    #[test]
    fn test_is_anthropic_custom_format() {
        let anthropic_custom = json!([{"name": "test", "description": "...", "input_schema": {}}]);
        let anthropic_builtin = json!([{"type": "bash_20250124", "name": "bash"}]);
        let openai = json!([{"type": "function", "function": {"name": "test"}}]);

        assert!(is_anthropic_custom_format(&anthropic_custom));
        assert!(!is_anthropic_custom_format(&anthropic_builtin)); // has "type"
        assert!(!is_anthropic_custom_format(&openai));
    }

    #[test]
    fn test_find_builtin_tool() {
        let with_bash = json!([{"type": "bash_20250124", "name": "bash"}]);
        let with_text_editor = json!([{"type": "text_editor_20250429", "name": "editor"}]);
        let custom_only = json!([{"name": "test", "input_schema": {}}]);

        assert_eq!(find_builtin_tool(&with_bash), Some("bash_20250124".into()));
        assert_eq!(
            find_builtin_tool(&with_text_editor),
            Some("text_editor_20250429".into())
        );
        assert_eq!(find_builtin_tool(&custom_only), None);
    }

    #[test]
    fn test_roundtrip_openai_anthropic_openai() {
        let original = json!([{
            "type": "function",
            "function": {
                "name": "get_data",
                "description": "Fetches data",
                "parameters": {"type": "object"}
            }
        }]);

        let anthropic = openai_to_anthropic_tools(&original).unwrap();
        let back = anthropic_to_openai_tools(&anthropic).unwrap();

        let orig_tool = original.as_array().unwrap().first().unwrap();
        let back_tool = back.as_array().unwrap().first().unwrap();

        assert_eq!(orig_tool["type"], back_tool["type"]);
        assert_eq!(
            orig_tool["function"]["name"],
            back_tool["function"]["name"]
        );
        assert_eq!(
            orig_tool["function"]["description"],
            back_tool["function"]["description"]
        );
    }

    #[test]
    fn test_is_responses_tool_format() {
        // Responses API function tool: name at top level, no "function" wrapper
        let responses_function = json!([{
            "type": "function",
            "name": "get_weather",
            "description": "Get the weather",
            "parameters": {"type": "object"},
            "strict": false
        }]);

        // Responses API built-in tools (code_interpreter, web_search_preview)
        let responses_code_interpreter = json!([{
            "type": "code_interpreter",
            "container": {"type": "auto"}
        }]);

        let responses_web_search = json!([{
            "type": "web_search_preview"
        }]);

        // OpenAI Chat format: nested under "function"
        let chat_format = json!([{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the weather",
                "parameters": {"type": "object"}
            }
        }]);

        // Anthropic custom format: no "type" field
        let anthropic_custom = json!([{
            "name": "get_weather",
            "description": "Get the weather",
            "input_schema": {"type": "object"}
        }]);

        // All Responses API formats should match
        assert!(is_responses_tool_format(&responses_function));
        assert!(is_responses_tool_format(&responses_code_interpreter));
        assert!(is_responses_tool_format(&responses_web_search));

        // OpenAI Chat format should NOT match (has "function" wrapper)
        assert!(!is_responses_tool_format(&chat_format));

        // Anthropic custom format should NOT match (no "type" field)
        assert!(!is_responses_tool_format(&anthropic_custom));
    }
}
