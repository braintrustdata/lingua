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

## UniversalTool

The `UniversalTool` type provides a typed representation that can convert to/from
any provider format. It distinguishes between:
- Function tools (user-defined, work across all providers)
- Builtin tools (provider-specific, may not translate)
*/

use serde::{Deserialize, Serialize};

use crate::error::ConvertError;
use crate::serde_json::{json, Map, Value};

// =============================================================================
// Universal Tool Types
// =============================================================================

/// A tool definition in universal format.
///
/// This provides a typed representation that normalizes the different tool formats
/// across providers (Anthropic, OpenAI Chat, OpenAI Responses API, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniversalTool {
    /// Tool name (required for all tool types)
    pub name: String,

    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Parameters/input schema (JSON Schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,

    /// Whether to enforce strict schema validation (OpenAI Responses API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    /// Tool type classification
    #[serde(flatten)]
    pub tool_type: UniversalToolType,
}

/// Classification of tool types.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UniversalToolType {
    /// User-defined function tool (works across all providers)
    #[default]
    #[serde(rename = "function")]
    Function,

    /// Provider-specific built-in tool (may not translate to other providers)
    #[serde(rename = "builtin")]
    Builtin {
        /// Provider identifier (e.g., "anthropic", "openai_responses")
        provider: String,
        /// Original type name (e.g., "bash_20250124", "code_interpreter")
        builtin_type: String,
        /// Provider-specific configuration
        #[serde(skip_serializing_if = "Option::is_none")]
        config: Option<Value>,
    },
}

// =============================================================================
// UniversalTool Constructors
// =============================================================================

impl UniversalTool {
    /// Create a new function tool.
    pub fn function(
        name: impl Into<String>,
        description: Option<String>,
        parameters: Option<Value>,
        strict: Option<bool>,
    ) -> Self {
        Self {
            name: name.into(),
            description,
            parameters,
            strict,
            tool_type: UniversalToolType::Function,
        }
    }

    /// Create a new builtin tool.
    pub fn builtin(
        name: impl Into<String>,
        provider: impl Into<String>,
        builtin_type: impl Into<String>,
        config: Option<Value>,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            parameters: None,
            strict: None,
            tool_type: UniversalToolType::Builtin {
                provider: provider.into(),
                builtin_type: builtin_type.into(),
                config,
            },
        }
    }

    /// Check if this is a function tool.
    pub fn is_function(&self) -> bool {
        matches!(self.tool_type, UniversalToolType::Function)
    }

    /// Check if this is a builtin tool.
    pub fn is_builtin(&self) -> bool {
        matches!(self.tool_type, UniversalToolType::Builtin { .. })
    }

    /// Get the builtin provider, if this is a builtin tool.
    pub fn builtin_provider(&self) -> Option<&str> {
        match &self.tool_type {
            UniversalToolType::Builtin { provider, .. } => Some(provider),
            _ => None,
        }
    }
}

// =============================================================================
// Conversion from Provider Formats
// =============================================================================

impl UniversalTool {
    /// Parse a tool from Anthropic format (JSON Value).
    ///
    /// Handles both custom tools and built-in tools (bash, text_editor, web_search).
    pub fn from_anthropic_value(value: &Value) -> Option<Self> {
        // Check for built-in tools first (have "type" field)
        if let Some(tool_type) = value.get("type").and_then(Value::as_str) {
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(tool_type)
                .to_string();

            // Determine builtin type from the type field
            if tool_type.starts_with("bash_")
                || tool_type.starts_with("text_editor_")
                || tool_type.starts_with("web_search_")
            {
                return Some(Self::builtin(
                    name,
                    "anthropic",
                    tool_type,
                    Some(value.clone()),
                ));
            }
        }

        // Custom tool format: {"name", "description", "input_schema", "strict"}
        let name = value.get("name").and_then(Value::as_str)?;
        let description = value
            .get("description")
            .and_then(Value::as_str)
            .map(String::from);
        let parameters = value.get("input_schema").cloned();
        let strict = value.get("strict").and_then(Value::as_bool);

        Some(Self::function(name, description, parameters, strict))
    }

    /// Parse a tool from OpenAI Chat Completions format (JSON Value).
    ///
    /// Format: `{"type": "function", "function": {"name", "description", "parameters"}}`
    pub fn from_openai_chat_value(value: &Value) -> Option<Self> {
        // OpenAI Chat format requires type: "function" and nested function object
        if value.get("type").and_then(Value::as_str) != Some("function") {
            return None;
        }

        let func = value.get("function")?;
        let name = func.get("name").and_then(Value::as_str)?;
        let description = func
            .get("description")
            .and_then(Value::as_str)
            .map(String::from);
        let parameters = func.get("parameters").cloned();
        let strict = func.get("strict").and_then(Value::as_bool);

        Some(Self::function(name, description, parameters, strict))
    }

    /// Parse a tool from OpenAI Responses API format (JSON Value).
    ///
    /// Function format: `{"type": "function", "name", "description", "parameters", "strict"}`
    /// Builtin format: `{"type": "code_interpreter"}`, `{"type": "web_search_preview"}`, etc.
    pub fn from_responses_value(value: &Value) -> Option<Self> {
        let tool_type = value.get("type").and_then(Value::as_str)?;

        match tool_type {
            "function" => {
                // Responses API function: name is at top level, not nested
                let name = value.get("name").and_then(Value::as_str)?;
                let description = value
                    .get("description")
                    .and_then(Value::as_str)
                    .map(String::from);
                let parameters = value.get("parameters").cloned();
                let strict = value.get("strict").and_then(Value::as_bool);

                Some(Self::function(name, description, parameters, strict))
            }
            "code_interpreter"
            | "web_search_preview"
            | "mcp"
            | "file_search"
            | "computer_use_preview" => {
                // Responses API built-in tools
                Some(Self::builtin(
                    tool_type,
                    "openai_responses",
                    tool_type,
                    Some(value.clone()),
                ))
            }
            _ => None,
        }
    }

    /// Parse tools from a JSON Value array, auto-detecting the format.
    pub fn from_value_array(tools: &Value) -> Vec<Self> {
        let Some(arr) = tools.as_array() else {
            return Vec::new();
        };

        let format = detect_tools_format(tools);

        arr.iter()
            .filter_map(|tool| match format {
                ToolsFormat::OpenAIChat => Self::from_openai_chat_value(tool),
                ToolsFormat::OpenAIResponses => Self::from_responses_value(tool),
                ToolsFormat::AnthropicCustom | ToolsFormat::AnthropicBuiltin => {
                    Self::from_anthropic_value(tool)
                }
                ToolsFormat::Unknown => {
                    // Try each format in order
                    Self::from_openai_chat_value(tool)
                        .or_else(|| Self::from_responses_value(tool))
                        .or_else(|| Self::from_anthropic_value(tool))
                }
            })
            .collect()
    }
}

// =============================================================================
// Conversion to Provider Formats
// =============================================================================

impl UniversalTool {
    /// Convert to Anthropic format (JSON Value).
    ///
    /// Returns an error if the tool is a builtin from a different provider.
    pub fn to_anthropic_value(&self) -> Result<Value, ConvertError> {
        match &self.tool_type {
            UniversalToolType::Function => {
                let mut obj = Map::new();
                obj.insert("name".into(), Value::String(self.name.clone()));

                if let Some(desc) = &self.description {
                    obj.insert("description".into(), Value::String(desc.clone()));
                }

                obj.insert(
                    "input_schema".into(),
                    self.parameters.clone().unwrap_or_else(|| json!({})),
                );

                if let Some(strict) = self.strict {
                    obj.insert("strict".into(), Value::Bool(strict));
                }

                Ok(Value::Object(obj))
            }
            UniversalToolType::Builtin {
                provider,
                builtin_type,
                config,
            } => {
                if provider != "anthropic" {
                    return Err(ConvertError::UnsupportedToolType {
                        tool_name: self.name.clone(),
                        tool_type: builtin_type.clone(),
                        target_provider: "Anthropic".to_string(),
                    });
                }
                // Return the original config for Anthropic builtins
                config
                    .clone()
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: format!("config for Anthropic builtin tool '{}'", self.name),
                    })
            }
        }
    }

    /// Convert to OpenAI Chat Completions format (JSON Value).
    ///
    /// Returns an error if the tool is a builtin (Chat Completions doesn't support builtins).
    pub fn to_openai_chat_value(&self) -> Result<Value, ConvertError> {
        match &self.tool_type {
            UniversalToolType::Function => {
                let mut func = Map::new();
                func.insert("name".into(), Value::String(self.name.clone()));

                if let Some(desc) = &self.description {
                    func.insert("description".into(), Value::String(desc.clone()));
                }

                func.insert(
                    "parameters".into(),
                    self.parameters.clone().unwrap_or_else(|| json!({})),
                );

                if let Some(strict) = self.strict {
                    func.insert("strict".into(), Value::Bool(strict));
                }

                Ok(json!({
                    "type": "function",
                    "function": Value::Object(func)
                }))
            }
            UniversalToolType::Builtin { builtin_type, .. } => {
                Err(ConvertError::UnsupportedToolType {
                    tool_name: self.name.clone(),
                    tool_type: builtin_type.clone(),
                    target_provider: "OpenAI Chat Completions".to_string(),
                })
            }
        }
    }

    /// Convert to OpenAI Responses API format (JSON Value).
    ///
    /// Returns an error if the tool is a builtin from a different provider.
    pub fn to_responses_value(&self) -> Result<Value, ConvertError> {
        match &self.tool_type {
            UniversalToolType::Function => {
                let mut obj = Map::new();
                obj.insert("type".into(), Value::String("function".to_string()));
                obj.insert("name".into(), Value::String(self.name.clone()));

                if let Some(desc) = &self.description {
                    obj.insert("description".into(), Value::String(desc.clone()));
                }

                obj.insert(
                    "parameters".into(),
                    self.parameters.clone().unwrap_or_else(|| json!({})),
                );

                if let Some(strict) = self.strict {
                    obj.insert("strict".into(), Value::Bool(strict));
                }

                Ok(Value::Object(obj))
            }
            UniversalToolType::Builtin {
                provider,
                builtin_type,
                config,
            } => {
                if provider != "openai_responses" {
                    return Err(ConvertError::UnsupportedToolType {
                        tool_name: self.name.clone(),
                        tool_type: builtin_type.clone(),
                        target_provider: "OpenAI Responses API".to_string(),
                    });
                }
                // Return the original config for Responses API builtins
                config
                    .clone()
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: format!("config for Responses API builtin tool '{}'", self.name),
                    })
            }
        }
    }
}

// =============================================================================
// Batch Conversion Utilities
// =============================================================================

/// Convert a slice of UniversalTools to Anthropic format Value array.
///
/// Returns an error if any tool cannot be converted (e.g., non-Anthropic builtins).
pub fn tools_to_anthropic_value(tools: &[UniversalTool]) -> Result<Option<Value>, ConvertError> {
    if tools.is_empty() {
        return Ok(None);
    }
    let converted: Vec<Value> = tools
        .iter()
        .map(|t| t.to_anthropic_value())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Value::Array(converted)))
}

/// Convert a slice of UniversalTools to OpenAI Chat format Value array.
///
/// Returns an error if any tool cannot be converted (e.g., builtins).
pub fn tools_to_openai_chat_value(tools: &[UniversalTool]) -> Result<Option<Value>, ConvertError> {
    if tools.is_empty() {
        return Ok(None);
    }
    let converted: Vec<Value> = tools
        .iter()
        .map(|t| t.to_openai_chat_value())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Value::Array(converted)))
}

/// Convert a slice of UniversalTools to Responses API format Value array.
///
/// Returns an error if any tool cannot be converted (e.g., Anthropic builtins).
pub fn tools_to_responses_value(tools: &[UniversalTool]) -> Result<Option<Value>, ConvertError> {
    if tools.is_empty() {
        return Ok(None);
    }
    let converted: Vec<Value> = tools
        .iter()
        .map(|t| t.to_responses_value())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Value::Array(converted)))
}

// =============================================================================
// Format Detection
// =============================================================================

/// Detected tools format for cross-provider translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsFormat {
    /// OpenAI Chat Completions format: `{type: "function", function: {name, description, parameters}}`
    OpenAIChat,
    /// OpenAI Responses API format: `{type: "function", name, description, parameters}` (no wrapper)
    OpenAIResponses,
    /// Anthropic custom tool format: `{name, description, input_schema}` (no type field)
    AnthropicCustom,
    /// Anthropic built-in tool format: `{type: "bash_20250124", name: "bash"}` etc.
    AnthropicBuiltin,
    /// Unknown or unrecognized format
    Unknown,
}

/// Detect the format of a tools array.
///
/// # Detection logic
///
/// 1. If first tool has `type` field and `function` wrapper → OpenAIChat
/// 2. If first tool has `type` field, no `function`, and type is builtin → AnthropicBuiltin
/// 3. If first tool has `type` field, no `function`, not builtin → OpenAIResponses
/// 4. If first tool has `name` but no `type` → AnthropicCustom
/// 5. Otherwise → Unknown
fn detect_tools_format(tools: &Value) -> ToolsFormat {
    let Some(arr) = tools.as_array() else {
        return ToolsFormat::Unknown;
    };
    let Some(first) = arr.first() else {
        return ToolsFormat::Unknown;
    };

    let has_type = first.get("type").and_then(Value::as_str);
    let has_function_wrapper = first.get("function").is_some();
    let has_name = first.get("name").is_some();

    match (has_type, has_function_wrapper, has_name) {
        // Has type and function wrapper → OpenAI Chat format
        (Some("function"), true, _) => ToolsFormat::OpenAIChat,

        // Has type, no function wrapper → check if Anthropic builtin or Responses API
        (Some(t), false, _) => {
            // Anthropic built-in tools use versioned type names (e.g., bash_20250124).
            // Update this list when Anthropic adds new built-in tool types.
            if t.starts_with("bash_")
                || t.starts_with("text_editor_")
                || t.starts_with("web_search_")
            {
                ToolsFormat::AnthropicBuiltin
            } else {
                ToolsFormat::OpenAIResponses
            }
        }

        // Has name but no type → Anthropic custom format
        (None, _, true) => ToolsFormat::AnthropicCustom,

        // Anything else
        _ => ToolsFormat::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_tool_function_constructor() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get the weather".to_string()),
            Some(json!({"type": "object"})),
            None,
        );

        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.description, Some("Get the weather".to_string()));
        assert!(tool.is_function());
        assert!(!tool.is_builtin());
        assert!(tool.builtin_provider().is_none());
        assert_eq!(tool.strict, None);
    }

    #[test]
    fn test_universal_tool_builtin_constructor() {
        let tool = UniversalTool::builtin(
            "bash",
            "anthropic",
            "bash_20250124",
            Some(json!({"name": "bash"})),
        );

        assert_eq!(tool.name, "bash");
        assert!(!tool.is_function());
        assert!(tool.is_builtin());
        assert_eq!(tool.builtin_provider(), Some("anthropic"));
    }

    #[test]
    fn test_universal_tool_from_anthropic_custom() {
        let anthropic = json!({
            "name": "get_weather",
            "description": "Get weather info",
            "input_schema": {"type": "object", "properties": {"location": {"type": "string"}}}
        });

        let tool = UniversalTool::from_anthropic_value(&anthropic).unwrap();

        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.description, Some("Get weather info".to_string()));
        assert!(tool.is_function());
        assert!(tool.parameters.is_some());
    }

    #[test]
    fn test_universal_tool_from_anthropic_builtin() {
        let anthropic = json!({
            "type": "bash_20250124",
            "name": "bash"
        });

        let tool = UniversalTool::from_anthropic_value(&anthropic).unwrap();

        assert_eq!(tool.name, "bash");
        assert!(tool.is_builtin());
        assert_eq!(tool.builtin_provider(), Some("anthropic"));

        if let UniversalToolType::Builtin { builtin_type, .. } = &tool.tool_type {
            assert_eq!(builtin_type, "bash_20250124");
        } else {
            panic!("Expected Builtin type");
        }
    }

    #[test]
    fn test_universal_tool_from_openai_chat() {
        let openai = json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {"type": "object"}
            }
        });

        let tool = UniversalTool::from_openai_chat_value(&openai).unwrap();

        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.description, Some("Get weather".to_string()));
        assert!(tool.is_function());
    }

    #[test]
    fn test_universal_tool_from_responses_function() {
        let responses = json!({
            "type": "function",
            "name": "get_weather",
            "description": "Get weather",
            "parameters": {"type": "object"},
            "strict": false
        });

        let tool = UniversalTool::from_responses_value(&responses).unwrap();

        assert_eq!(tool.name, "get_weather");
        assert!(tool.is_function());
    }

    #[test]
    fn test_universal_tool_from_responses_builtin() {
        let responses = json!({
            "type": "code_interpreter"
        });

        let tool = UniversalTool::from_responses_value(&responses).unwrap();

        assert_eq!(tool.name, "code_interpreter");
        assert!(tool.is_builtin());
        assert_eq!(tool.builtin_provider(), Some("openai_responses"));
    }

    #[test]
    fn test_universal_tool_to_anthropic_function() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({"type": "object"})),
            None,
        );

        let value = tool.to_anthropic_value().unwrap();

        assert_eq!(value["name"], "get_weather");
        assert_eq!(value["description"], "Get weather");
        assert!(value["input_schema"].is_object());
        assert!(value.get("type").is_none()); // Custom tools don't have type field
    }

    #[test]
    fn test_universal_tool_to_anthropic_builtin_passthrough() {
        let config = json!({
            "type": "bash_20250124",
            "name": "bash"
        });
        let tool =
            UniversalTool::builtin("bash", "anthropic", "bash_20250124", Some(config.clone()));

        let value = tool.to_anthropic_value().unwrap();
        assert_eq!(value, config);
    }

    #[test]
    fn test_universal_tool_to_anthropic_builtin_wrong_provider() {
        let tool = UniversalTool::builtin(
            "code_interpreter",
            "openai_responses",
            "code_interpreter",
            Some(json!({})),
        );

        let result = tool.to_anthropic_value();
        assert!(result.is_err());
    }

    #[test]
    fn test_universal_tool_to_openai_chat_function() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({"type": "object"})),
            None,
        );

        let value = tool.to_openai_chat_value().unwrap();

        assert_eq!(value["type"], "function");
        assert_eq!(value["function"]["name"], "get_weather");
        assert_eq!(value["function"]["description"], "Get weather");
    }

    #[test]
    fn test_universal_tool_to_openai_chat_builtin_error() {
        let tool = UniversalTool::builtin("bash", "anthropic", "bash_20250124", Some(json!({})));

        let result = tool.to_openai_chat_value();
        assert!(result.is_err());
    }

    #[test]
    fn test_universal_tool_to_responses_function() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({"type": "object"})),
            None,
        );

        let value = tool.to_responses_value().unwrap();

        assert_eq!(value["type"], "function");
        assert_eq!(value["name"], "get_weather");
        assert_eq!(value["description"], "Get weather");
        assert!(value.get("strict").is_none()); // strict only output when explicitly set
    }

    #[test]
    fn test_universal_tool_to_responses_function_with_strict() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({"type": "object"})),
            Some(true),
        );

        let value = tool.to_responses_value().unwrap();

        assert_eq!(value["type"], "function");
        assert_eq!(value["name"], "get_weather");
        assert_eq!(value["strict"], true);
    }

    #[test]
    fn test_universal_tool_to_responses_builtin_passthrough() {
        let config = json!({"type": "code_interpreter"});
        let tool = UniversalTool::builtin(
            "code_interpreter",
            "openai_responses",
            "code_interpreter",
            Some(config.clone()),
        );

        let value = tool.to_responses_value().unwrap();
        assert_eq!(value, config);
    }

    #[test]
    fn test_universal_tool_roundtrip_anthropic() {
        let original = json!({
            "name": "get_weather",
            "description": "Get weather info",
            "input_schema": {"type": "object", "properties": {"location": {"type": "string"}}}
        });

        let tool = UniversalTool::from_anthropic_value(&original).unwrap();
        let back = tool.to_anthropic_value().unwrap();

        assert_eq!(back["name"], original["name"]);
        assert_eq!(back["description"], original["description"]);
        // Note: input_schema may have empty object default if original was missing
    }

    #[test]
    fn test_universal_tool_roundtrip_openai_chat() {
        let original = json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {"type": "object"}
            }
        });

        let tool = UniversalTool::from_openai_chat_value(&original).unwrap();
        let back = tool.to_openai_chat_value().unwrap();

        assert_eq!(back["type"], "function");
        assert_eq!(back["function"]["name"], original["function"]["name"]);
        assert_eq!(
            back["function"]["description"],
            original["function"]["description"]
        );
    }

    #[test]
    fn test_universal_tool_cross_provider_anthropic_to_openai() {
        let anthropic = json!({
            "name": "get_weather",
            "description": "Get weather",
            "input_schema": {"type": "object"}
        });

        let tool = UniversalTool::from_anthropic_value(&anthropic).unwrap();
        let openai = tool.to_openai_chat_value().unwrap();

        assert_eq!(openai["type"], "function");
        assert_eq!(openai["function"]["name"], "get_weather");
        assert_eq!(openai["function"]["description"], "Get weather");
    }

    #[test]
    fn test_universal_tool_cross_provider_openai_to_anthropic() {
        let openai = json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {"type": "object"}
            }
        });

        let tool = UniversalTool::from_openai_chat_value(&openai).unwrap();
        let anthropic = tool.to_anthropic_value().unwrap();

        assert_eq!(anthropic["name"], "get_weather");
        assert_eq!(anthropic["description"], "Get weather");
        assert!(anthropic.get("type").is_none());
    }

    #[test]
    fn test_batch_conversion_to_anthropic() {
        let tools = vec![
            UniversalTool::function("tool1", Some("desc1".to_string()), None, None),
            UniversalTool::function("tool2", Some("desc2".to_string()), None, None),
        ];

        let result = tools_to_anthropic_value(&tools).unwrap();
        let arr = result.unwrap().as_array().cloned().unwrap();

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "tool1");
        assert_eq!(arr[1]["name"], "tool2");
    }

    #[test]
    fn test_batch_conversion_to_anthropic_fails_on_wrong_provider() {
        let tools = vec![
            UniversalTool::function("tool1", Some("desc1".to_string()), None, None),
            UniversalTool::builtin(
                "code_interpreter",
                "openai_responses",
                "code_interpreter",
                Some(json!({})),
            ),
        ];

        let result = tools_to_anthropic_value(&tools);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_conversion_to_openai_chat() {
        let tools = vec![
            UniversalTool::function("tool1", Some("desc1".to_string()), None, None),
            UniversalTool::function("tool2", Some("desc2".to_string()), None, None),
        ];

        let result = tools_to_openai_chat_value(&tools).unwrap();
        let arr = result.unwrap().as_array().cloned().unwrap();

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["function"]["name"], "tool1");
    }

    #[test]
    fn test_batch_conversion_to_openai_chat_fails_on_builtin() {
        let tools = vec![
            UniversalTool::function("tool1", Some("desc1".to_string()), None, None),
            UniversalTool::builtin("bash", "anthropic", "bash_20250124", Some(json!({}))),
        ];

        let result = tools_to_openai_chat_value(&tools);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_value_array_auto_detect() {
        // OpenAI Chat format
        let openai = json!([{
            "type": "function",
            "function": {"name": "test1", "description": "desc1", "parameters": {}}
        }]);
        let tools = UniversalTool::from_value_array(&openai);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test1");

        // Anthropic format
        let anthropic = json!([{
            "name": "test2",
            "description": "desc2",
            "input_schema": {}
        }]);
        let tools = UniversalTool::from_value_array(&anthropic);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test2");

        // Responses API format
        let responses = json!([{
            "type": "function",
            "name": "test3",
            "description": "desc3",
            "parameters": {}
        }]);
        let tools = UniversalTool::from_value_array(&responses);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test3");
    }
}
