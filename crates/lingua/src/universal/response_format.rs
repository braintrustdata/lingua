/*!
Response format conversion utilities for cross-provider semantic translation.

This module provides bidirectional conversion between different providers'
response format configurations:
- OpenAI Chat: `{ type: "text" | "json_object" | "json_schema", json_schema? }`
- OpenAI Responses: nested under `text.format` with flattened schema
- Google: `response_mime_type` + `response_schema`
- Anthropic: `{ type: "json_schema", schema }` (no name/strict/description)

## Design

The conversion uses canonical fields (`format_type`, `json_schema`) for cross-provider
semantic translation. Same-provider round-trips are handled at a higher level via
passthrough optimization.

## Usage

```ignore
use std::convert::TryInto;
use crate::capabilities::ProviderFormat;
use crate::universal::request::ResponseFormatConfig;

// FROM: Parse provider-specific value to universal config
let config: ResponseFormatConfig = (ProviderFormat::ChatCompletions, &raw_json).try_into()?;

// TO: Convert universal config to provider-specific value
let output = config.to_provider(ProviderFormat::ChatCompletions)?;
```
*/

use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::processing::transform::TransformError;
use crate::providers::anthropic::generated::JsonOutputFormat;
use crate::providers::google::generated::GenerationConfig;
use crate::serde_json::{self, json, Map, Value};
use crate::universal::request::{JsonSchemaConfig, ResponseFormatConfig, ResponseFormatType};

// =============================================================================
// TryFrom Implementation for FROM Conversions
// =============================================================================

impl<'a> TryFrom<(ProviderFormat, &'a Value)> for ResponseFormatConfig {
    type Error = TransformError;

    fn try_from((provider, value): (ProviderFormat, &'a Value)) -> Result<Self, Self::Error> {
        match provider {
            ProviderFormat::ChatCompletions => Ok(from_openai_chat(value)?),
            ProviderFormat::Responses => Ok(from_openai_responses(value)?),
            ProviderFormat::Anthropic => serde_json::from_value::<JsonOutputFormat>(value.clone())
                .map(|f| ResponseFormatConfig::from(&f))
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string())),
            ProviderFormat::Google => Ok(from_google(value)?),
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
    pub fn to_provider(&self, provider: ProviderFormat) -> Result<Option<Value>, TransformError> {
        match provider {
            ProviderFormat::ChatCompletions => Ok(to_openai_chat(self)),
            ProviderFormat::Responses => Ok(to_openai_responses_text(self)),
            ProviderFormat::Anthropic => Ok(JsonOutputFormat::try_from(self)
                .ok()
                .and_then(|f| serde_json::to_value(&f).ok())),
            ProviderFormat::Google => to_google(self),
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
fn from_openai_chat(value: &Value) -> Result<ResponseFormatConfig, ConvertError> {
    let format_type = match value.get("type").and_then(Value::as_str) {
        Some(s) => Some(s.parse().map_err(|_| ConvertError::InvalidEnumValue {
            type_name: "ResponseFormatType",
            value: s.to_string(),
        })?),
        None => None,
    };

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

    Ok(ResponseFormatConfig {
        format_type,
        json_schema,
    })
}

/// Parse Google `generationConfig` response format fields into ResponseFormatConfig.
///
/// Handles:
/// - `responseMimeType: "text/plain"` → Text
/// - `responseMimeType: "application/json"` → JsonObject or JsonSchema (if responseSchema present)
/// - `responseSchema: {...}` → JsonSchema
/// - No responseMimeType → None (no response format specified)
fn from_google(value: &Value) -> Result<ResponseFormatConfig, ConvertError> {
    let config: GenerationConfig = serde_json::from_value(value.clone()).map_err(|e| {
        ConvertError::ContentConversionFailed {
            reason: format!("Failed to parse Google generationConfig: {}", e),
        }
    })?;
    Ok(ResponseFormatConfig::from(&config))
}

/// Parse OpenAI Responses API `text.format` into ResponseFormatConfig.
///
/// Handles the flattened structure:
/// - `{ type: "json_schema", name, schema, strict?, description? }`
fn from_openai_responses(value: &Value) -> Result<ResponseFormatConfig, ConvertError> {
    let format_type = match value.get("type").and_then(Value::as_str) {
        Some(s) => Some(s.parse().map_err(|_| ConvertError::InvalidEnumValue {
            type_name: "ResponseFormatType",
            value: s.to_string(),
        })?),
        None => None,
    };

    let json_schema = if format_type == Some(ResponseFormatType::JsonSchema) {
        value.get("name").and_then(Value::as_str).and_then(|name| {
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

    Ok(ResponseFormatConfig {
        format_type,
        json_schema,
    })
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

/// Convert ResponseFormatConfig to Google generationConfig fields.
///
/// Output format (as partial generationConfig object):
/// - Text → `{ responseMimeType: "text/plain" }`
/// - JsonObject → `{ responseMimeType: "application/json" }`
/// - JsonSchema → `{ responseMimeType: "application/json", responseJsonSchema: {...} }`
///
/// Note: This returns fields to merge into generationConfig, not a standalone value.
fn to_google(config: &ResponseFormatConfig) -> Result<Option<Value>, TransformError> {
    if config.format_type.is_none() {
        return Ok(None);
    }

    let generation_config = GenerationConfig::try_from(config)
        .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;
    let value = serde_json::to_value(generation_config)
        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::google::generated::GenerationConfig;
    use serde::Deserialize;
    use std::convert::TryInto;

    #[derive(Debug, Deserialize)]
    struct JsonSchemaMetadataView {
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(rename = "additionalProperties", default)]
        additional_properties: Option<bool>,
    }

    #[test]
    fn test_from_openai_chat_text() {
        let value = json!({ "type": "text" });
        let config: ResponseFormatConfig = (ProviderFormat::ChatCompletions, &value)
            .try_into()
            .unwrap();
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
        let config: ResponseFormatConfig = (ProviderFormat::ChatCompletions, &value)
            .try_into()
            .unwrap();
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
        };
        let value = config
            .to_provider(ProviderFormat::ChatCompletions)
            .unwrap()
            .unwrap();
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
        let config: ResponseFormatConfig = (ProviderFormat::ChatCompletions, &original)
            .try_into()
            .unwrap();
        let back = config
            .to_provider(ProviderFormat::ChatCompletions)
            .unwrap()
            .unwrap();
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
        };
        let value = config
            .to_provider(ProviderFormat::Responses)
            .unwrap()
            .unwrap();
        let format = value.get("format").unwrap();
        assert_eq!(format.get("type").unwrap(), "json_schema");
        assert_eq!(format.get("name").unwrap(), "test");
    }

    #[test]
    fn test_cross_provider_openai_to_anthropic() {
        // Parse OpenAI format
        let openai_format = json!({
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
        let config: ResponseFormatConfig = (ProviderFormat::ChatCompletions, &openai_format)
            .try_into()
            .unwrap();

        // Convert to Anthropic format
        let anthropic_format = config
            .to_provider(ProviderFormat::Anthropic)
            .unwrap()
            .unwrap();

        // Verify Anthropic format structure
        assert_eq!(anthropic_format.get("type").unwrap(), "json_schema");
        // Name and strict are NOT included because Anthropic doesn't support them
        assert!(anthropic_format.get("name").is_none());
        assert!(anthropic_format.get("strict").is_none());
        assert!(anthropic_format.get("schema").is_some());
        // Anthropic format doesn't have nested json_schema wrapper
        assert!(anthropic_format.get("json_schema").is_none());
    }

    #[test]
    fn test_from_google_json_schema() {
        let value = json!({
            "responseMimeType": "application/json",
            "responseJsonSchema": {
                "type": "object",
                "title": "person_info",
                "description": "Person schema",
                "properties": {
                    "name": { "type": "string" }
                }
            }
        });
        let config: ResponseFormatConfig = (ProviderFormat::Google, &value).try_into().unwrap();
        assert_eq!(config.format_type, Some(ResponseFormatType::JsonSchema));
        let js = config.json_schema.unwrap();
        assert_eq!(js.name, "person_info");
        assert_eq!(js.description, Some("Person schema".to_string()));
        assert_eq!(
            js.schema,
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            })
        );
    }

    #[test]
    fn test_from_google_json_object() {
        let value = json!({ "responseMimeType": "application/json" });
        let config: ResponseFormatConfig = (ProviderFormat::Google, &value).try_into().unwrap();
        assert_eq!(config.format_type, Some(ResponseFormatType::JsonObject));
        assert!(config.json_schema.is_none());
    }

    #[test]
    fn test_from_google_text() {
        let value = json!({ "responseMimeType": "text/plain" });
        let config: ResponseFormatConfig = (ProviderFormat::Google, &value).try_into().unwrap();
        assert_eq!(config.format_type, Some(ResponseFormatType::Text));
        assert!(config.json_schema.is_none());
    }

    #[test]
    fn test_to_google_formats() {
        let text = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::Text),
            json_schema: None,
        };
        let text_value = text.to_provider(ProviderFormat::Google).unwrap().unwrap();
        let text_generation_config: GenerationConfig =
            serde_json::from_value(text_value).expect("google generationConfig should deserialize");
        assert_eq!(
            text_generation_config.response_mime_type.as_deref(),
            Some("text/plain")
        );

        let json_object = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonObject),
            json_schema: None,
        };
        let value = json_object
            .to_provider(ProviderFormat::Google)
            .unwrap()
            .unwrap();
        let google_generation_config: GenerationConfig =
            serde_json::from_value(value).expect("google generationConfig should deserialize");
        assert_eq!(
            google_generation_config.response_mime_type.as_deref(),
            Some("application/json")
        );
        assert!(google_generation_config
            .generation_config_response_json_schema
            .is_none());

        let json_schema = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "response".into(),
                schema: json!({ "type": "object", "additionalProperties": false }),
                strict: Some(true),
                description: Some("Structured output".to_string()),
            }),
        };
        let value = json_schema
            .to_provider(ProviderFormat::Google)
            .unwrap()
            .unwrap();
        let google_generation_config: GenerationConfig =
            serde_json::from_value(value).expect("google generationConfig should deserialize");
        assert_eq!(
            google_generation_config.response_mime_type.as_deref(),
            Some("application/json")
        );
        assert!(
            google_generation_config.response_schema.is_none(),
            "typed responseSchema should be absent when responseJsonSchema is canonical"
        );
        let schema_value = google_generation_config
            .generation_config_response_json_schema
            .expect("responseJsonSchema should be emitted");
        let schema: JsonSchemaMetadataView = serde_json::from_value(schema_value)
            .expect("responseJsonSchema should deserialize into metadata view");
        assert_eq!(schema.title.as_deref(), Some("response"));
        assert_eq!(schema.description.as_deref(), Some("Structured output"));
        assert_eq!(schema.additional_properties, Some(false));
    }
}
