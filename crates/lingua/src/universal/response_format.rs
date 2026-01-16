/*!
Response format conversion utilities for cross-provider semantic translation.

This module provides bidirectional conversion between different providers'
response format configurations:
- OpenAI Chat: `{ type: "text" | "json_object" | "json_schema", json_schema? }`
- OpenAI Responses: nested under `text.format` with flattened schema
- Google: `response_mime_type` + `response_schema`
- Anthropic: Not supported

## Design

The conversion preserves the original value in `raw` for lossless same-provider round-trips,
while extracting canonical fields (`format_type`, `json_schema`) for cross-provider conversion.

## Usage

```ignore
use std::convert::TryInto;
use crate::capabilities::ProviderFormat;
use crate::universal::request::ResponseFormatConfig;

// FROM: Parse provider-specific value to universal config
let config: ResponseFormatConfig = (ProviderFormat::OpenAI, &raw_json).try_into()?;

// TO: Convert universal config to provider-specific value
let output = config.to_provider(ProviderFormat::OpenAI)?;
```
*/

use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::processing::transform::TransformError;
use crate::serde_json::{json, Map, Value};
use crate::universal::request::{JsonSchemaConfig, ResponseFormatConfig, ResponseFormatType};

// =============================================================================
// TryFrom Implementation for FROM Conversions
// =============================================================================

impl<'a> TryFrom<(ProviderFormat, &'a Value)> for ResponseFormatConfig {
    type Error = TransformError;

    fn try_from((provider, value): (ProviderFormat, &'a Value)) -> Result<Self, Self::Error> {
        match provider {
            ProviderFormat::OpenAI => Ok(from_openai_chat(value)),
            ProviderFormat::Responses => Ok(from_openai_responses(value)),
            _ => Ok(Self::default()),
        }
    }
}

// =============================================================================
// to_provider Method for TO Conversions
// =============================================================================

impl ResponseFormatConfig {
    /// Convert this config to a provider-specific value.
    ///
    /// # Arguments
    /// * `provider` - Target provider format
    ///
    /// # Returns
    /// `Ok(Some(value))` if conversion succeeded
    /// `Ok(None)` if no value should be set
    /// `Err(_)` if conversion failed
    pub fn to_provider(
        &self,
        provider: ProviderFormat,
    ) -> Result<Option<Value>, TransformError> {
        match provider {
            ProviderFormat::OpenAI => Ok(to_openai_chat(self)),
            ProviderFormat::Responses => Ok(to_openai_responses_text(self)),
            _ => Ok(None),
        }
    }
}

// =============================================================================
// Private Helper Functions - FROM Provider Formats
// =============================================================================

/// Parse OpenAI Chat `response_format` into ResponseFormatConfig.
///
/// Handles:
/// - `{ type: "text" }`
/// - `{ type: "json_object" }`
/// - `{ type: "json_schema", json_schema: { name, schema, strict?, description? } }`
fn from_openai_chat(value: &Value) -> ResponseFormatConfig {
    let format_type = value
        .get("type")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok());

    let json_schema = if format_type == Some(ResponseFormatType::JsonSchema) {
        value.get("json_schema").and_then(|js| {
            let name = js.get("name").and_then(Value::as_str)?;
            let schema = js.get("schema").cloned()?;
            Some(JsonSchemaConfig {
                name: name.to_string(),
                schema,
                strict: js.get("strict").and_then(Value::as_bool),
                description: js
                    .get("description")
                    .and_then(Value::as_str)
                    .map(String::from),
            })
        })
    } else {
        None
    };

    ResponseFormatConfig {
        format_type,
        json_schema,
        raw: Some(value.clone()),
    }
}

/// Parse OpenAI Responses API `text.format` into ResponseFormatConfig.
///
/// Handles the flattened structure:
/// - `{ type: "json_schema", name, schema, strict?, description? }`
fn from_openai_responses(value: &Value) -> ResponseFormatConfig {
    let format_type = value
        .get("type")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok());

    let json_schema = if format_type == Some(ResponseFormatType::JsonSchema) {
        value
            .get("name")
            .and_then(Value::as_str)
            .and_then(|name| {
                value.get("schema").cloned().map(|schema| JsonSchemaConfig {
                    name: name.to_string(),
                    schema,
                    strict: value.get("strict").and_then(Value::as_bool),
                    description: value
                        .get("description")
                        .and_then(Value::as_str)
                        .map(String::from),
                })
            })
    } else {
        None
    };

    ResponseFormatConfig {
        format_type,
        json_schema,
        raw: Some(value.clone()),
    }
}

// =============================================================================
// Private Helper Functions - TO Provider Formats
// =============================================================================

/// Convert ResponseFormatConfig to OpenAI Chat `response_format` value.
///
/// Output format:
/// - `{ type: "text" }`
/// - `{ type: "json_object" }`
/// - `{ type: "json_schema", json_schema: { name, schema, strict?, description? } }`
fn to_openai_chat(config: &ResponseFormatConfig) -> Option<Value> {
    // If we have raw and it looks like OpenAI Chat format, prefer it
    if let Some(ref raw) = config.raw {
        if raw.as_object().map_or(false, |o| {
            o.contains_key("type")
                && (o.contains_key("json_schema") || !o.contains_key("schema"))
        }) {
            return Some(raw.clone());
        }
    }

    let format_type = config.format_type?;

    match format_type {
        ResponseFormatType::Text => Some(json!({ "type": "text" })),
        ResponseFormatType::JsonObject => Some(json!({ "type": "json_object" })),
        ResponseFormatType::JsonSchema => {
            let js = config.json_schema.as_ref()?;
            let mut json_schema = Map::new();
            json_schema.insert("name".into(), Value::String(js.name.clone()));
            json_schema.insert("schema".into(), js.schema.clone());
            if let Some(strict) = js.strict {
                json_schema.insert("strict".into(), Value::Bool(strict));
            }
            if let Some(ref desc) = js.description {
                json_schema.insert("description".into(), Value::String(desc.clone()));
            }
            Some(json!({
                "type": "json_schema",
                "json_schema": json_schema
            }))
        }
    }
}

/// Convert ResponseFormatConfig to OpenAI Responses API `text` object.
///
/// Output format (flattened, wrapped in text object):
/// - `{ format: { type: "text" } }`
/// - `{ format: { type: "json_schema", name, schema, strict?, description? } }`
///
/// Returns the full `text` object, not just the format.
fn to_openai_responses_text(config: &ResponseFormatConfig) -> Option<Value> {
    // If we have raw from Responses API format, reconstruct text wrapper
    if let Some(ref raw) = config.raw {
        // Check if it's Responses format (flat, has name at top level)
        if raw.as_object().map_or(false, |o| {
            o.contains_key("type") && (o.contains_key("name") || o.contains_key("schema"))
        }) {
            return Some(json!({ "format": raw }));
        }
    }

    let format_type = config.format_type?;

    let format_obj = match format_type {
        ResponseFormatType::Text => json!({ "type": "text" }),
        ResponseFormatType::JsonObject => json!({ "type": "json_object" }),
        ResponseFormatType::JsonSchema => {
            let js = config.json_schema.as_ref()?;
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("json_schema".into()));
            obj.insert("name".into(), Value::String(js.name.clone()));
            obj.insert("schema".into(), js.schema.clone());
            if let Some(strict) = js.strict {
                obj.insert("strict".into(), Value::Bool(strict));
            }
            if let Some(ref desc) = js.description {
                obj.insert("description".into(), Value::String(desc.clone()));
            }
            Value::Object(obj)
        }
    };

    Some(json!({ "format": format_obj }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_from_openai_chat_text() {
        let value = json!({ "type": "text" });
        let config: ResponseFormatConfig = (ProviderFormat::OpenAI, &value).try_into().unwrap();
        assert_eq!(config.format_type, Some(ResponseFormatType::Text));
        assert!(config.json_schema.is_none());
    }

    #[test]
    fn test_from_openai_chat_json_schema() {
        let value = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "person_info",
                "schema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    }
                },
                "strict": true
            }
        });
        let config: ResponseFormatConfig = (ProviderFormat::OpenAI, &value).try_into().unwrap();
        assert_eq!(config.format_type, Some(ResponseFormatType::JsonSchema));
        let js = config.json_schema.unwrap();
        assert_eq!(js.name, "person_info");
        assert_eq!(js.strict, Some(true));
    }

    #[test]
    fn test_to_openai_chat_json_schema() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "test_schema".into(),
                schema: json!({ "type": "object" }),
                strict: Some(true),
                description: None,
            }),
            raw: None,
        };
        let value = config.to_provider(ProviderFormat::OpenAI).unwrap().unwrap();
        assert_eq!(value.get("type").unwrap(), "json_schema");
        assert!(value.get("json_schema").is_some());
        assert_eq!(
            value
                .get("json_schema")
                .unwrap()
                .get("name")
                .unwrap()
                .as_str()
                .unwrap(),
            "test_schema"
        );
    }

    #[test]
    fn test_roundtrip_openai_chat() {
        let original = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "test",
                "schema": { "type": "object" },
                "strict": true
            }
        });
        let config: ResponseFormatConfig = (ProviderFormat::OpenAI, &original).try_into().unwrap();
        let back = config.to_provider(ProviderFormat::OpenAI).unwrap().unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_to_responses_text_format() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "test".into(),
                schema: json!({ "type": "object" }),
                strict: Some(true),
                description: None,
            }),
            raw: None,
        };
        let value = config.to_provider(ProviderFormat::Responses).unwrap().unwrap();
        let format = value.get("format").unwrap();
        assert_eq!(format.get("type").unwrap(), "json_schema");
        assert_eq!(format.get("name").unwrap(), "test");
    }
}
