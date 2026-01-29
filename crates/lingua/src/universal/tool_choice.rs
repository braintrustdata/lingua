/*!
Tool choice conversion utilities for cross-provider semantic translation.

This module provides bidirectional conversion between different providers'
tool choice configurations:
- OpenAI Chat: `"auto"` | `"none"` | `"required"` | `{ type: "function", function: { name } }`
- OpenAI Responses: `"auto"` | `{ type: "function", name }`
- Anthropic: `{ type: "auto" | "any" | "none" | "tool", name?, disable_parallel_tool_use? }`

## Design

Uses canonical fields (`mode`, `tool_name`) for cross-provider conversion.

## Usage

```ignore
use std::convert::TryInto;
use crate::capabilities::ProviderFormat;
use crate::universal::request::ToolChoiceConfig;

// FROM: Parse provider-specific value to universal config
let config: ToolChoiceConfig = (ProviderFormat::Anthropic, &raw_json).try_into()?;

// TO: Convert universal config to provider-specific value
// parallel_tool_calls: Some(false) disables parallel calls; None uses config.disable_parallel
let output = config.to_provider(ProviderFormat::Anthropic, Some(false))?;
```
*/

use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::processing::transform::TransformError;
use crate::serde_json::{json, Map, Value};
use crate::universal::request::{ToolChoiceConfig, ToolChoiceMode};

// =============================================================================
// TryFrom Implementation for FROM Conversions
// =============================================================================

impl<'a> TryFrom<(ProviderFormat, &'a Value)> for ToolChoiceConfig {
    type Error = TransformError;

    fn try_from((provider, value): (ProviderFormat, &'a Value)) -> Result<Self, Self::Error> {
        match provider {
            ProviderFormat::OpenAI => from_openai_chat(value),
            ProviderFormat::Responses => from_openai_responses(value),
            ProviderFormat::Anthropic => from_anthropic(value),
            _ => Ok(Self::default()),
        }
    }
}

// =============================================================================
// to_provider Method for TO Conversions
// =============================================================================

impl ToolChoiceConfig {
    /// Convert this config to a provider-specific value.
    ///
    /// # Arguments
    /// * `provider` - Target provider format
    /// * `parallel_tool_calls` - Whether parallel tool calls are enabled (for Anthropic's disable_parallel_tool_use)
    ///
    /// # Returns
    /// `Ok(Some(value))` if conversion succeeded
    /// `Ok(None)` if no value should be set (e.g., mode is None)
    /// `Err(_)` if conversion failed
    pub fn to_provider(
        &self,
        provider: ProviderFormat,
        parallel_tool_calls: Option<bool>,
    ) -> Result<Option<Value>, TransformError> {
        match provider {
            ProviderFormat::OpenAI => Ok(to_openai_chat(self)),
            ProviderFormat::Responses => Ok(to_openai_responses(self)),
            ProviderFormat::Anthropic => Ok(to_anthropic(self, parallel_tool_calls)),
            _ => Ok(None),
        }
    }
}

// =============================================================================
// Private Helper Functions - FROM Provider Formats
// =============================================================================

/// Parse OpenAI Chat `tool_choice` into ToolChoiceConfig.
///
/// Handles:
/// - String: `"auto"`, `"none"`, `"required"`
/// - Object: `{ type: "function", function: { name: "..." } }`
fn from_openai_chat(value: &Value) -> Result<ToolChoiceConfig, TransformError> {
    match value {
        Value::String(s) => {
            let mode = s
                .parse()
                .map_err(|e| TransformError::ToUniversalFailed(format!("{}", e)))?;
            Ok(ToolChoiceConfig {
                mode: Some(mode),
                tool_name: None,
                disable_parallel: None,
            })
        }
        Value::Object(obj) => {
            // OpenAI Chat uses { type: "function", function: { name: "..." } }
            let type_str = obj.get("type").and_then(Value::as_str);
            match type_str {
                Some("function") | None => {}
                Some(other) => {
                    return Err(TransformError::ToUniversalFailed(format!(
                        "unrecognized tool_choice type: '{}'",
                        other
                    )))
                }
            }

            let tool_name = obj
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
                .map(String::from);

            Ok(ToolChoiceConfig {
                mode: Some(ToolChoiceMode::Tool),
                tool_name,
                disable_parallel: None,
            })
        }
        _ => Ok(ToolChoiceConfig::default()),
    }
}

/// Parse OpenAI Responses API `tool_choice` into ToolChoiceConfig.
///
/// Handles:
/// - String: `"auto"`, `"none"`, `"required"`
/// - Object: `{ type: "function", name: "..." }` (flatter than Chat)
fn from_openai_responses(value: &Value) -> Result<ToolChoiceConfig, TransformError> {
    match value {
        Value::String(s) => {
            let mode = s
                .parse()
                .map_err(|e| TransformError::ToUniversalFailed(format!("{}", e)))?;
            Ok(ToolChoiceConfig {
                mode: Some(mode),
                tool_name: None,
                disable_parallel: None,
            })
        }
        Value::Object(obj) => {
            let tool_name = obj.get("name").and_then(Value::as_str).map(String::from);

            // OpenAI Responses uses { type: "function", name: "..." }
            let type_str = obj.get("type").and_then(Value::as_str);
            let mode = match type_str {
                Some("function") | None => Some(ToolChoiceMode::Tool),
                Some(other) => {
                    return Err(TransformError::ToUniversalFailed(format!(
                        "unrecognized tool_choice type: '{}'",
                        other
                    )))
                }
            };

            Ok(ToolChoiceConfig {
                mode,
                tool_name,
                disable_parallel: None,
            })
        }
        _ => Ok(ToolChoiceConfig::default()),
    }
}

/// Parse Anthropic `tool_choice` into ToolChoiceConfig.
///
/// Handles:
/// - `{ type: "auto" }`
/// - `{ type: "any" }`
/// - `{ type: "none" }`
/// - `{ type: "tool", name: "..." }`
/// - `{ ..., disable_parallel_tool_use: true }`
fn from_anthropic(value: &Value) -> Result<ToolChoiceConfig, TransformError> {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return Ok(ToolChoiceConfig::default()),
    };

    let mode = match obj.get("type").and_then(Value::as_str) {
        Some(s) => Some(
            s.parse()
                .map_err(|e| TransformError::ToUniversalFailed(format!("{}", e)))?,
        ),
        None => None,
    };

    let tool_name = obj.get("name").and_then(Value::as_str).map(String::from);

    let disable_parallel = obj
        .get("disable_parallel_tool_use")
        .and_then(Value::as_bool);

    Ok(ToolChoiceConfig {
        mode,
        tool_name,
        disable_parallel,
    })
}

// =============================================================================
// Private Helper Functions - TO Provider Formats
// =============================================================================

/// Convert ToolChoiceConfig to OpenAI Chat `tool_choice` value.
///
/// Output format:
/// - `"auto"`, `"none"`, `"required"` for simple modes
/// - `{ type: "function", function: { name: "..." } }` for specific tool
fn to_openai_chat(config: &ToolChoiceConfig) -> Option<Value> {
    let mode = config.mode?;

    match mode {
        ToolChoiceMode::Auto => Some(Value::String("auto".into())),
        ToolChoiceMode::None => Some(Value::String("none".into())),
        ToolChoiceMode::Required => Some(Value::String("required".into())),
        ToolChoiceMode::Tool => {
            let name = config.tool_name.as_ref()?;
            Some(json!({
                "type": "function",
                "function": {
                    "name": name
                }
            }))
        }
    }
}

/// Convert ToolChoiceConfig to OpenAI Responses API `tool_choice` value.
///
/// Output format:
/// - `"auto"`, `"none"`, `"required"` for simple modes
/// - `{ type: "function", name: "..." }` for specific tool (flatter than Chat)
fn to_openai_responses(config: &ToolChoiceConfig) -> Option<Value> {
    let mode = config.mode?;

    match mode {
        ToolChoiceMode::Auto => Some(Value::String("auto".into())),
        ToolChoiceMode::None => Some(Value::String("none".into())),
        ToolChoiceMode::Required => Some(Value::String("required".into())),
        ToolChoiceMode::Tool => {
            let name = config.tool_name.as_ref()?;
            Some(json!({
                "type": "function",
                "name": name
            }))
        }
    }
}

/// Convert ToolChoiceConfig to Anthropic `tool_choice` value.
///
/// Output format:
/// - `{ type: "auto" }`, `{ type: "any" }`, `{ type: "none" }`
/// - `{ type: "tool", name: "..." }`
/// - Includes `disable_parallel_tool_use` if set
fn to_anthropic(config: &ToolChoiceConfig, parallel_tool_calls: Option<bool>) -> Option<Value> {
    // If parallel_tool_calls is explicitly false, we MUST emit tool_choice with disable_parallel_tool_use
    let needs_disable_parallel =
        parallel_tool_calls == Some(false) || config.disable_parallel == Some(true);

    // Get mode, defaulting to Auto if we need to disable parallel (so we can emit the field)
    let mode = match config.mode {
        Some(m) => m,
        None if needs_disable_parallel => ToolChoiceMode::Auto,
        None => return None,
    };

    let mut obj = Map::new();
    obj.insert("type".into(), Value::String(mode.as_anthropic_str().into()));

    if mode == ToolChoiceMode::Tool {
        if let Some(ref name) = config.tool_name {
            obj.insert("name".into(), Value::String(name.clone()));
        }
    }

    if needs_disable_parallel {
        obj.insert("disable_parallel_tool_use".into(), Value::Bool(true));
    }

    Some(Value::Object(obj))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_from_openai_chat_string() {
        let value = json!("auto");
        let config: ToolChoiceConfig = (ProviderFormat::OpenAI, &value).try_into().unwrap();
        assert_eq!(config.mode, Some(ToolChoiceMode::Auto));
        assert_eq!(config.tool_name, None);
    }

    #[test]
    fn test_from_openai_chat_function() {
        let value = json!({
            "type": "function",
            "function": { "name": "get_weather" }
        });
        let config: ToolChoiceConfig = (ProviderFormat::OpenAI, &value).try_into().unwrap();
        assert_eq!(config.mode, Some(ToolChoiceMode::Tool));
        assert_eq!(config.tool_name, Some("get_weather".into()));
    }

    #[test]
    fn test_from_anthropic_tool() {
        let value = json!({
            "type": "tool",
            "name": "get_weather"
        });
        let config: ToolChoiceConfig = (ProviderFormat::Anthropic, &value).try_into().unwrap();
        assert_eq!(config.mode, Some(ToolChoiceMode::Tool));
        assert_eq!(config.tool_name, Some("get_weather".into()));
    }

    #[test]
    fn test_from_anthropic_with_disable_parallel() {
        let value = json!({
            "type": "auto",
            "disable_parallel_tool_use": true
        });
        let config: ToolChoiceConfig = (ProviderFormat::Anthropic, &value).try_into().unwrap();
        assert_eq!(config.mode, Some(ToolChoiceMode::Auto));
        assert_eq!(config.disable_parallel, Some(true));
    }

    #[test]
    fn test_to_openai_chat_auto() {
        let config = ToolChoiceConfig {
            mode: Some(ToolChoiceMode::Auto),
            ..Default::default()
        };
        let value = config
            .to_provider(ProviderFormat::OpenAI, None)
            .unwrap()
            .unwrap();
        assert_eq!(value, json!("auto"));
    }

    #[test]
    fn test_to_openai_chat_function() {
        let config = ToolChoiceConfig {
            mode: Some(ToolChoiceMode::Tool),
            tool_name: Some("get_weather".into()),
            ..Default::default()
        };
        let value = config
            .to_provider(ProviderFormat::OpenAI, None)
            .unwrap()
            .unwrap();
        assert_eq!(
            value,
            json!({
                "type": "function",
                "function": { "name": "get_weather" }
            })
        );
    }

    #[test]
    fn test_to_anthropic_any() {
        let config = ToolChoiceConfig {
            mode: Some(ToolChoiceMode::Required),
            ..Default::default()
        };
        let value = config
            .to_provider(ProviderFormat::Anthropic, None)
            .unwrap()
            .unwrap();
        assert_eq!(value.get("type").unwrap(), "any");
    }

    #[test]
    fn test_to_anthropic_with_parallel_disabled() {
        let config = ToolChoiceConfig {
            mode: Some(ToolChoiceMode::Auto),
            ..Default::default()
        };
        // parallel_tool_calls: false → disable_parallel_tool_use: true
        let value = config
            .to_provider(ProviderFormat::Anthropic, Some(false))
            .unwrap()
            .unwrap();
        assert_eq!(value.get("type").unwrap(), "auto");
        assert_eq!(value.get("disable_parallel_tool_use").unwrap(), true);
    }

    #[test]
    fn test_roundtrip_openai_chat() {
        let original = json!({
            "type": "function",
            "function": { "name": "get_weather" }
        });
        let config: ToolChoiceConfig = (ProviderFormat::OpenAI, &original).try_into().unwrap();
        let back = config
            .to_provider(ProviderFormat::OpenAI, None)
            .unwrap()
            .unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_cross_provider_openai_to_anthropic() {
        // OpenAI required → Anthropic any
        let openai_value = json!("required");
        let config: ToolChoiceConfig = (ProviderFormat::OpenAI, &openai_value).try_into().unwrap();
        let anthropic_value = config
            .to_provider(ProviderFormat::Anthropic, None)
            .unwrap()
            .unwrap();
        assert_eq!(anthropic_value.get("type").unwrap(), "any");
    }

    #[test]
    fn test_invalid_string_mode_errors() {
        // Unrecognized string mode should error
        let value = json!("invalid_mode");
        let result: Result<ToolChoiceConfig, _> = (ProviderFormat::OpenAI, &value).try_into();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid_mode"));
    }

    #[test]
    fn test_invalid_object_type_errors() {
        // Unrecognized type in object should error
        let value = json!({
            "type": "unknown_type",
            "name": "some_tool"
        });
        let result: Result<ToolChoiceConfig, _> = (ProviderFormat::Responses, &value).try_into();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown_type"));
    }
}
