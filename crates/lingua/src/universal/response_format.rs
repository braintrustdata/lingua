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
use crate::serde_json::{self, json, Map, Number, Value};
use crate::universal::request::{JsonSchemaConfig, ResponseFormatConfig, ResponseFormatType};
use serde::{Deserialize, Serialize};

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
            ProviderFormat::ChatCompletions => Ok(to_openai_chat(self)?),
            ProviderFormat::Responses => Ok(to_openai_responses_text(self)?),
            ProviderFormat::Anthropic => {
                let format = JsonOutputFormat::try_from(self).map_err(TransformError::from)?;
                Ok(Some(serde_json::to_value(&format).map_err(|e| {
                    TransformError::SerializationFailed(e.to_string())
                })?))
            }
            ProviderFormat::Google => to_google(self),
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum AdditionalPropertiesNormalizationView {
    Bool(bool),
    Other(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum SchemaScalarConstraintView {
    Number(Number),
    String(String),
    Other(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum SchemaTypeView {
    String(String),
    Array(Vec<String>),
}

impl SchemaTypeView {
    fn contains(&self, expected: &str) -> bool {
        match self {
            Self::String(value) => value == expected,
            Self::Array(values) => values.iter().any(|value| value == expected),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StrictTargetSchemaNodeView {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    schema_type: Option<SchemaTypeView>,
    #[serde(
        rename = "additionalProperties",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    additional_properties: Option<AdditionalPropertiesNormalizationView>,
    #[serde(
        rename = "propertyOrdering",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    property_ordering: Option<Vec<String>>,
    #[serde(rename = "minItems", default, skip_serializing_if = "Option::is_none")]
    min_items: Option<SchemaScalarConstraintView>,
    #[serde(rename = "maxItems", default, skip_serializing_if = "Option::is_none")]
    max_items: Option<SchemaScalarConstraintView>,
    #[serde(rename = "minimum", default, skip_serializing_if = "Option::is_none")]
    minimum: Option<SchemaScalarConstraintView>,
    #[serde(rename = "maximum", default, skip_serializing_if = "Option::is_none")]
    maximum: Option<SchemaScalarConstraintView>,
}

/// Anthropic structured outputs accept a narrower JSON Schema subset than the
/// cross-provider canonical format. When targeting Anthropic we intentionally
/// narrow the schemas by dropping unsupported tuple hints and array/numeric bounds.
fn strip_anthropic_unsupported_schema_keywords(
    map: &mut Map<String, Value>,
    node: &StrictTargetSchemaNodeView,
) {
    if node
        .schema_type
        .as_ref()
        .is_some_and(|schema_type| schema_type.contains("array"))
    {
        map.remove("prefixItems");
        map.remove("minItems");
        map.remove("maxItems");
    }

    if node.schema_type.as_ref().is_some_and(|schema_type| {
        schema_type.contains("integer") || schema_type.contains("number")
    }) {
        map.remove("minimum");
        map.remove("maximum");
    }
}

pub(crate) fn normalize_response_schema_for_strict_target(
    schema: &Value,
    target_provider: ProviderFormat,
) -> Result<Value, ConvertError> {
    fn normalize_node(
        value: &mut Value,
        target_provider: ProviderFormat,
    ) -> Result<(), ConvertError> {
        let node: StrictTargetSchemaNodeView =
            serde_json::from_value(value.clone()).map_err(|e| {
                ConvertError::InvalidResponseSchema {
                    target_provider,
                    reason: format!(
                        "response schema could not be deserialized for normalization: {e}"
                    ),
                }
            })?;

        if let Value::Object(map) = value {
            if target_provider == ProviderFormat::Anthropic {
                strip_anthropic_unsupported_schema_keywords(map, &node);
            }

            if node
                .schema_type
                .as_ref()
                .is_some_and(|schema_type| schema_type.contains("object"))
            {
                if target_provider != ProviderFormat::Google {
                    map.remove("propertyOrdering");
                }

                match node.additional_properties {
                    None => {
                        map.insert("additionalProperties".into(), Value::Bool(false));
                    }
                    Some(AdditionalPropertiesNormalizationView::Bool(true)) => {
                        return Err(ConvertError::InvalidResponseSchema {
                            target_provider,
                            reason: "object schema explicitly sets 'additionalProperties: true'"
                                .to_string(),
                        });
                    }
                    Some(AdditionalPropertiesNormalizationView::Bool(false)) => {}
                    Some(AdditionalPropertiesNormalizationView::Other(_)) => {
                        return Err(ConvertError::InvalidResponseSchema {
                            target_provider,
                            reason:
                                "object schema uses unsupported non-boolean 'additionalProperties'"
                                    .to_string(),
                        });
                    }
                }
            }

            if let Some(Value::Object(properties)) = map.get_mut("properties") {
                for prop_schema in properties.values_mut() {
                    normalize_node(prop_schema, target_provider)?;
                }
            }
            if let Some(items) = map.get_mut("items") {
                normalize_node(items, target_provider)?;
            }
            for key in ["anyOf", "oneOf", "allOf", "prefixItems"] {
                if let Some(Value::Array(items)) = map.get_mut(key) {
                    for item in items {
                        normalize_node(item, target_provider)?;
                    }
                }
            }
        }

        Ok(())
    }

    let mut normalized = schema.clone();
    normalize_node(&mut normalized, target_provider)?;
    Ok(normalized)
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
fn to_openai_chat(config: &ResponseFormatConfig) -> Result<Option<Value>, ConvertError> {
    let Some(format_type) = config.format_type else {
        return Ok(None);
    };

    Ok(match format_type {
        ResponseFormatType::Text => Some(json!({ "type": "text" })),
        ResponseFormatType::JsonObject => Some(json!({ "type": "json_object" })),
        ResponseFormatType::JsonSchema => {
            let js =
                config
                    .json_schema
                    .as_ref()
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: "json_schema".to_string(),
                    })?;
            let mut json_schema = Map::new();
            json_schema.insert("name".into(), Value::String(js.name.clone()));
            json_schema.insert(
                "schema".into(),
                normalize_response_schema_for_strict_target(
                    &js.schema,
                    ProviderFormat::ChatCompletions,
                )?,
            );
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
    })
}

/// Convert ResponseFormatConfig to OpenAI Responses API `text` object.
///
/// Output format (flattened, wrapped in text object):
/// - `{ format: { type: "text" } }`
/// - `{ format: { type: "json_schema", name, schema, strict?, description? } }`
///
/// Returns the full `text` object, not just the format.
fn to_openai_responses_text(config: &ResponseFormatConfig) -> Result<Option<Value>, ConvertError> {
    let Some(format_type) = config.format_type else {
        return Ok(None);
    };

    let format_obj = match format_type {
        ResponseFormatType::Text => json!({ "type": "text" }),
        ResponseFormatType::JsonObject => json!({ "type": "json_object" }),
        ResponseFormatType::JsonSchema => {
            let js =
                config
                    .json_schema
                    .as_ref()
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: "json_schema".to_string(),
                    })?;
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("json_schema".into()));
            obj.insert("name".into(), Value::String(js.name.clone()));
            obj.insert(
                "schema".into(),
                normalize_response_schema_for_strict_target(&js.schema, ProviderFormat::Responses)?,
            );
            if let Some(strict) = js.strict {
                obj.insert("strict".into(), Value::Bool(strict));
            }
            if let Some(ref desc) = js.description {
                obj.insert("description".into(), Value::String(desc.clone()));
            }
            Value::Object(obj)
        }
    };

    Ok(Some(json!({ "format": format_obj })))
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

    #[derive(Debug, Deserialize)]
    struct OpenAiChatResponseFormatView {
        #[serde(rename = "type")]
        format_type: String,
        json_schema: OpenAiChatJsonSchemaView,
    }

    #[derive(Debug, Deserialize)]
    struct OpenAiChatJsonSchemaView {
        name: String,
        schema: JsonSchemaMetadataView,
    }

    #[derive(Debug, Deserialize)]
    struct OpenAiResponsesTextView {
        format: OpenAiResponsesFormatView,
    }

    #[derive(Debug, Deserialize)]
    struct OpenAiResponsesFormatView {
        #[serde(rename = "type")]
        format_type: String,
        name: String,
        schema: Value,
    }

    #[derive(Debug, Deserialize)]
    struct AnthropicJsonSchemaFormatView {
        #[serde(rename = "type")]
        format_type: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        strict: Option<bool>,
        schema: JsonSchemaMetadataView,
        #[serde(default)]
        json_schema: Option<Value>,
    }

    #[derive(Debug, Deserialize)]
    struct NestedSchemaMetadataView {
        #[serde(rename = "additionalProperties", default)]
        additional_properties: Option<bool>,
        #[serde(default)]
        properties: std::collections::HashMap<String, NestedSchemaMetadataView>,
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
        let format: OpenAiChatResponseFormatView =
            serde_json::from_value(value).expect("chat response_format should deserialize");
        assert_eq!(format.format_type, "json_schema");
        assert_eq!(format.json_schema.name, "test_schema");
        assert_eq!(format.json_schema.schema.additional_properties, Some(false));
    }

    #[test]
    fn test_roundtrip_openai_chat_normalizes_missing_additional_properties() {
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
        assert_eq!(
            back,
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "test",
                    "schema": {
                        "type": "object",
                        "additionalProperties": false
                    },
                    "strict": true
                }
            })
        );
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
        let text: OpenAiResponsesTextView =
            serde_json::from_value(value).expect("responses text config should deserialize");
        assert_eq!(text.format.format_type, "json_schema");
        assert_eq!(text.format.name, "test");
        let schema: JsonSchemaMetadataView =
            serde_json::from_value(text.format.schema).expect("schema should deserialize");
        assert_eq!(schema.additional_properties, Some(false));
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

        let format: AnthropicJsonSchemaFormatView =
            serde_json::from_value(anthropic_format).expect("anthropic format should deserialize");
        assert_eq!(format.format_type, "json_schema");
        assert!(format.name.is_none());
        assert!(format.strict.is_none());
        assert_eq!(format.schema.additional_properties, Some(false));
        assert!(format.json_schema.is_none());
    }

    #[test]
    fn test_strict_target_normalizes_nested_object_schemas() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "nested".into(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "outer": {
                            "type": "object",
                            "properties": {
                                "inner": { "type": "string" }
                            }
                        }
                    }
                }),
                strict: None,
                description: None,
            }),
        };

        let value = config
            .to_provider(ProviderFormat::Responses)
            .unwrap()
            .unwrap();
        let text: OpenAiResponsesTextView =
            serde_json::from_value(value).expect("responses text config should deserialize");
        let mut schema: NestedSchemaMetadataView =
            serde_json::from_value(text.format.schema).expect("schema should deserialize");
        assert_eq!(schema.additional_properties, Some(false));
        assert_eq!(
            schema
                .properties
                .remove("outer")
                .and_then(|outer| outer.additional_properties),
            Some(false)
        );
    }

    #[test]
    fn test_strict_target_strips_google_property_ordering_recursively() {
        let schema = json!({
            "type": ["object", "null"],
            "propertyOrdering": ["outer"],
            "properties": {
                "outer": {
                    "type": ["object", "null"],
                    "propertyOrdering": ["inner"],
                    "properties": {
                        "inner": { "type": "string" }
                    }
                }
            }
        });

        for provider in [
            ProviderFormat::Anthropic,
            ProviderFormat::ChatCompletions,
            ProviderFormat::Responses,
        ] {
            let normalized =
                normalize_response_schema_for_strict_target(&schema, provider).unwrap();
            assert_eq!(
                normalized.pointer("/propertyOrdering"),
                None,
                "root propertyOrdering should be stripped for {provider:?}"
            );
            assert_eq!(
                normalized.pointer("/properties/outer/propertyOrdering"),
                None,
                "nested propertyOrdering should be stripped for {provider:?}"
            );
            assert_eq!(
                normalized.pointer("/additionalProperties"),
                Some(&Value::Bool(false)),
                "root additionalProperties should be normalized for {provider:?}"
            );
            assert_eq!(
                normalized.pointer("/properties/outer/additionalProperties"),
                Some(&Value::Bool(false)),
                "nested additionalProperties should be normalized for {provider:?}"
            );
        }
    }

    #[test]
    fn test_google_strict_target_preserves_property_ordering() {
        let schema = json!({
            "type": ["object", "null"],
            "propertyOrdering": ["outer"],
            "properties": {
                "outer": {
                    "type": ["object", "null"],
                    "propertyOrdering": ["inner"],
                    "properties": {
                        "inner": { "type": "string" }
                    }
                }
            }
        });

        let normalized =
            normalize_response_schema_for_strict_target(&schema, ProviderFormat::Google).unwrap();

        assert_eq!(
            normalized.pointer("/propertyOrdering"),
            Some(&json!(["outer"]))
        );
        assert_eq!(
            normalized.pointer("/properties/outer/propertyOrdering"),
            Some(&json!(["inner"]))
        );
        assert_eq!(
            normalized.pointer("/additionalProperties"),
            Some(&Value::Bool(false))
        );
        assert_eq!(
            normalized.pointer("/properties/outer/additionalProperties"),
            Some(&Value::Bool(false))
        );
    }

    #[test]
    fn test_anthropic_lossy_normalization_strips_array_and_numeric_bounds() {
        let schema = json!({
            "type": "object",
            "properties": {
                "tuple": {
                    "type": ["array", "null"],
                    "prefixItems": [
                        { "type": "string" },
                        { "type": "integer" }
                    ],
                    "minItems": 2,
                    "maxItems": 3,
                    "items": { "type": "string" }
                },
                "score": {
                    "type": ["integer", "null"],
                    "minimum": 0,
                    "maximum": 10
                },
                "ratio": {
                    "type": ["number", "null"],
                    "minimum": 0.1,
                    "maximum": 0.9
                }
            }
        });

        let anthropic =
            normalize_response_schema_for_strict_target(&schema, ProviderFormat::Anthropic)
                .unwrap();
        assert_eq!(anthropic.pointer("/properties/tuple/prefixItems"), None);
        assert_eq!(anthropic.pointer("/properties/tuple/minItems"), None);
        assert_eq!(anthropic.pointer("/properties/tuple/maxItems"), None);
        assert_eq!(anthropic.pointer("/properties/score/minimum"), None);
        assert_eq!(anthropic.pointer("/properties/score/maximum"), None);
        assert_eq!(anthropic.pointer("/properties/ratio/minimum"), None);
        assert_eq!(anthropic.pointer("/properties/ratio/maximum"), None);

        let chat =
            normalize_response_schema_for_strict_target(&schema, ProviderFormat::ChatCompletions)
                .unwrap();
        assert_eq!(
            chat.pointer("/properties/tuple/prefixItems/0/type"),
            Some(&Value::String("string".to_string()))
        );
        assert_eq!(chat.pointer("/properties/tuple/minItems"), Some(&json!(2)));
        assert_eq!(chat.pointer("/properties/tuple/maxItems"), Some(&json!(3)));
        assert_eq!(chat.pointer("/properties/score/minimum"), Some(&json!(0)));
        assert_eq!(chat.pointer("/properties/score/maximum"), Some(&json!(10)));
        assert_eq!(chat.pointer("/properties/ratio/minimum"), Some(&json!(0.1)));
        assert_eq!(chat.pointer("/properties/ratio/maximum"), Some(&json!(0.9)));
    }

    #[test]
    fn test_strict_target_accepts_nullable_union_leaf_types() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "query_result".into(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "filter": {
                            "type": ["string", "null"]
                        }
                    },
                    "required": ["filter"]
                }),
                strict: Some(true),
                description: None,
            }),
        };

        let value = config
            .to_provider(ProviderFormat::Responses)
            .unwrap()
            .unwrap();
        let text: OpenAiResponsesTextView =
            serde_json::from_value(value).expect("responses text config should deserialize");
        assert_eq!(
            text.format.schema.pointer("/properties/filter/type"),
            Some(&json!(["string", "null"]))
        );
        assert_eq!(
            text.format.schema.pointer("/additionalProperties"),
            Some(&Value::Bool(false))
        );
    }

    #[test]
    fn test_strict_target_rejects_explicit_additional_properties_true() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "open_object".into(),
                schema: json!({
                    "type": "object",
                    "additionalProperties": true
                }),
                strict: None,
                description: None,
            }),
        };

        for provider in [
            ProviderFormat::ChatCompletions,
            ProviderFormat::Responses,
            ProviderFormat::Anthropic,
        ] {
            let err = config.to_provider(provider).unwrap_err();
            assert!(
                err.to_string().contains("additionalProperties: true"),
                "unexpected error for {provider:?}: {err}"
            );
        }
    }

    #[test]
    fn test_google_json_object_conversion_is_unchanged() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonObject),
            json_schema: None,
        };

        let value = config
            .to_provider(ProviderFormat::ChatCompletions)
            .unwrap()
            .unwrap();
        assert_eq!(value, json!({ "type": "json_object" }));
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
