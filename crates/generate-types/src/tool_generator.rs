//! Tool Generator Module
//!
//! This module contains utilities for generating tool types (structs and enums)
//! from OpenAPI schemas. Instead of relying on quicktype for tool generation
//! (which requires extensive post-processing), we generate tool structs directly
//! from schema analysis. Tool schemas are simple flat objects with primitive
//! fields, making direct codegen cleaner.

use crate::schema_converter::{schema_type_to_rust, to_rust_field_name};
use big_serde_json as serde_json;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct ToolSchemas {
    pub provider_tools: Vec<ProviderToolSchema>,
    pub client_tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderToolSchema {
    pub schema_name: String,
    pub tool_type: String,
}

/// Helper to extract components.schemas from an OpenAPI spec
fn get_schemas(spec: &serde_json::Value) -> Option<&serde_json::Map<String, serde_json::Value>> {
    spec.get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
}

/// Find the position after the closing brace that matches the opening brace at `open_pos`
fn find_closing_brace(content: &str, open_pos: usize) -> Option<usize> {
    let mut depth = 0isize;
    for (i, ch) in content[open_pos..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn generate_all_tool_code(
    provider: &str,
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let tool_schemas = extract_tool_schemas(provider, spec);

    if tool_schemas.client_tools.is_empty() && tool_schemas.provider_tools.is_empty() {
        return Ok(String::new());
    }

    let mut code_segments = Vec::new();
    let tool_structs = generate_tool_structs(provider, &tool_schemas, spec)?;
    code_segments.extend(tool_structs);
    if provider == "openai"
        && !tool_schemas
            .provider_tools
            .iter()
            .any(|tool| tool.tool_type == "programmatic_tool_calling")
    {
        code_segments.push(
            "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]\n\
#[ts(export_to = \"openai/\")]\n\
pub struct ProgrammaticToolCallingToolParam {}\n"
                .to_string(),
        );
    }

    let tool_enum = generate_tool_enum(provider, &tool_schemas, spec);
    code_segments.push(tool_enum);

    Ok(code_segments.join("\n\n"))
}

pub fn replace_tool_struct_with_enum(existing: &str, tool_code: &str) -> String {
    let filtered_tool_code = filter_tool_code_against_existing(tool_code, existing);
    if let Some((attr_start, struct_end)) = find_tool_struct_span(existing) {
        let mut out = String::new();
        out.push_str(&existing[..attr_start]);
        out.push_str(filtered_tool_code.trim());
        out.push('\n');
        out.push_str(&existing[struct_end..]);
        return out;
    }

    let mut out = existing.to_string();
    out.push('\n');
    out.push_str(filtered_tool_code.trim());
    out
}

pub fn enforce_anthropic_closed_request_types(
    existing: &str,
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let schemas = get_schemas(spec).ok_or("No components.schemas in Anthropic spec")?;
    let mut strict_config_types = HashSet::new();

    for toolset_configs in ["BrowserToolsetConfigs", "ComputerToolsetConfigs"] {
        let properties = schemas
            .get(toolset_configs)
            .and_then(|schema| schema.get("properties"))
            .and_then(|properties| properties.as_object())
            .ok_or_else(|| format!("Missing {toolset_configs} properties"))?;

        for property in properties.values() {
            let Some(schema_name) = nullable_referenced_schema_name(property) else {
                continue;
            };
            if schemas
                .get(schema_name)
                .and_then(|schema| schema.get("additionalProperties"))
                .and_then(|value| value.as_bool())
                == Some(false)
            {
                strict_config_types.insert(schema_name_to_rust_type(schema_name));
            }
        }
    }
    for schema_name in [
        "RequestImageTransformations",
        "ContainerParams",
        "SkillParams",
        "BrowserStateTabEntry",
    ] {
        if schemas
            .get(schema_name)
            .and_then(|schema| schema.get("additionalProperties"))
            .and_then(|value| value.as_bool())
            != Some(false)
        {
            return Err(format!("{schema_name} is no longer a closed object").into());
        }
        strict_config_types.insert(schema_name_to_rust_type(schema_name));
    }

    let mut output = existing.to_string();
    for (name, block) in split_type_definitions(existing) {
        if !strict_config_types.contains(&name) || block.contains("#[serde(deny_unknown_fields)]") {
            continue;
        }
        let derive_end = block
            .find("#[derive(")
            .and_then(|start| block[start..].find('\n').map(|end| start + end + 1))
            .ok_or_else(|| format!("Missing derive attribute for {name}"))?;
        let mut replacement = block.clone();
        replacement.insert_str(derive_end, "#[serde(deny_unknown_fields)]\n");
        output = output.replacen(&block, &replacement, 1);
    }

    Ok(output)
}

pub fn preserve_anthropic_optional_transformation_fields(
    existing: &str,
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let schemas = get_schemas(spec).ok_or("No components.schemas in Anthropic spec")?;
    for (schema_name, field_name) in [
        ("RequestImageBlock", "transformations"),
        ("RequestImageTransformations", "oversized_image"),
    ] {
        let required = schemas
            .get(schema_name)
            .and_then(|schema| schema.get("required"))
            .and_then(|required| required.as_array());
        if required.is_some_and(|required| {
            required
                .iter()
                .any(|field| field.as_str() == Some(field_name))
        }) {
            return Err(format!("{schema_name}.{field_name} is no longer optional").into());
        }
    }

    let mut output = existing.to_string();
    for (field_name, field_type) in [
        ("transformations", "RequestImageTransformations"),
        ("oversized_image", "OversizedImage"),
    ] {
        let old = format!(
            "    #[serde(skip_serializing_if = \"Option::is_none\")]\n    pub {field_name}: Option<{field_type}>,"
        );
        if !output.contains(&old) {
            return Err(format!("Missing generated optional {field_name} field").into());
        }
        let new = format!(
            "    #[serde(skip_serializing_if = \"Option::is_none\")]\n    #[ts(optional = nullable)]\n    pub {field_name}: Option<{field_type}>,"
        );
        output = output.replace(&old, &new);
    }
    Ok(output)
}

pub fn preserve_anthropic_required_nullable_fields(
    existing: &str,
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let schemas = get_schemas(spec).ok_or("No components.schemas in Anthropic spec")?;
    let container = schemas.get("Container").ok_or("Missing Container schema")?;
    let skills_is_required = container
        .get("required")
        .and_then(|required| required.as_array())
        .is_some_and(|required| {
            required
                .iter()
                .any(|field| field.as_str() == Some("skills"))
        });
    let skills_is_nullable = container
        .get("properties")
        .and_then(|properties| properties.get("skills"))
        .and_then(|skills| skills.get("anyOf"))
        .and_then(|variants| variants.as_array())
        .is_some_and(|variants| {
            variants
                .iter()
                .any(|variant| variant.get("type").and_then(|value| value.as_str()) == Some("null"))
        });
    if !skills_is_required || !skills_is_nullable {
        return Err("Container.skills is no longer required and nullable".into());
    }

    let (_, block) = split_type_definitions(existing)
        .into_iter()
        .find(|(name, _)| name == "Container")
        .ok_or("Missing generated Container type")?;
    let old = "    #[serde(skip_serializing_if = \"Option::is_none\")]\n    pub skills: Option<Vec<ContainerSkill>>,";
    let new = "    #[serde(deserialize_with = \"super::deserialize_required_nullable\")]\n    pub skills: Option<Vec<ContainerSkill>>,";
    if !block.contains(old) {
        return Err("Missing generated optional Container.skills field".into());
    }
    let replacement = block.replacen(old, new, 1);
    Ok(existing.replacen(&block, &replacement, 1))
}

// -------------------------------------------------------------------------
// Extraction functions
// -------------------------------------------------------------------------

pub fn extract_tool_schemas(provider: &str, spec: &serde_json::Value) -> ToolSchemas {
    match provider {
        "openai" => extract_openai_tool_schemas(spec),
        "anthropic" => extract_anthropic_tool_schemas(spec),
        _ => ToolSchemas::default(),
    }
}

fn extract_openai_tool_schemas(spec: &serde_json::Value) -> ToolSchemas {
    let Some(schemas) = get_schemas(spec) else {
        return ToolSchemas::default();
    };
    let Some(tool_schema) = schemas.get("Tool") else {
        return ToolSchemas::default();
    };
    let Some(any_of) = tool_schema
        .get("anyOf")
        .or_else(|| tool_schema.get("oneOf"))
        .and_then(|a| a.as_array())
    else {
        return ToolSchemas::default();
    };

    let mut result = ToolSchemas::default();

    for ref_item in any_of {
        let Some(schema_ref) = ref_item.get("$ref").and_then(|r| r.as_str()) else {
            continue;
        };
        let Some(schema_name) = schema_ref.split('/').next_back() else {
            continue;
        };
        let Some(schema_def) = schemas.get(schema_name) else {
            continue;
        };
        let Some(type_val) = schema_def
            .get("properties")
            .and_then(|p| p.get("type"))
            .and_then(|t| {
                t.get("const").and_then(|v| v.as_str()).or_else(|| {
                    t.get("enum")
                        .and_then(|e| e.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                })
            })
        else {
            continue;
        };

        if type_val == "function" || type_val == "custom" {
            result.client_tools.push(schema_name.to_string());
        } else {
            result.provider_tools.push(ProviderToolSchema {
                schema_name: schema_name.to_string(),
                tool_type: type_val.to_string(),
            });
        }
    }

    result
}

fn extract_anthropic_tool_schemas(spec: &serde_json::Value) -> ToolSchemas {
    let Some(schemas) = get_schemas(spec) else {
        return ToolSchemas::default();
    };

    let mut result = ToolSchemas::default();

    for (schema_name, schema_def) in schemas {
        // Skip beta tools for now - Lingua does not (yet) support Anthropic beta features
        if schema_name.starts_with("Beta") {
            continue;
        }
        if schema_name.starts_with("ServerToolCaller") {
            continue;
        }
        let Some(props) = schema_def.get("properties").and_then(|p| p.as_object()) else {
            continue;
        };
        let Some(type_prop) = props.get("type") else {
            continue;
        };

        if let Some(const_val) = type_prop.get("const").and_then(|v| v.as_str()) {
            if is_versioned_tool_type(const_val) {
                result.provider_tools.push(ProviderToolSchema {
                    schema_name: schema_name.clone(),
                    tool_type: const_val.to_string(),
                });
            }
        } else if props.contains_key("input_schema") {
            result.client_tools.push(schema_name.clone());
        }
    }

    result
}

fn is_versioned_tool_type(s: &str) -> bool {
    s.len() > 9
        && s.chars().rev().take(8).all(|c| c.is_ascii_digit())
        && s.chars().rev().nth(8) == Some('_')
}

// -------------------------------------------------------------------------
// Generation functions
// -------------------------------------------------------------------------

fn generate_tool_structs(
    provider: &str,
    tool_schemas: &ToolSchemas,
    spec: &serde_json::Value,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let all_schemas = get_schemas(spec).ok_or("No components.schemas in spec")?;

    let mut generated_structs = Vec::new();
    let mut seen = HashSet::new();

    // Generate client tool structs (e.g., CustomTool)
    for client_schema in &tool_schemas.client_tools {
        // Get the actual schema name to use for generation
        let gen_name = if client_schema == "Tool" {
            "CustomTool"
        } else {
            client_schema
        };

        if let Some(schema) = all_schemas.get(client_schema) {
            if seen.insert(gen_name.to_string()) {
                // Generate the struct with potentially renamed name
                let mut code = generate_tool_struct_direct(
                    client_schema,
                    schema,
                    all_schemas,
                    provider,
                    false,
                );

                // Rename Tool -> CustomTool if needed
                if client_schema == "Tool" {
                    code = code.replace("pub struct Tool {", "pub struct CustomTool {");
                }

                generated_structs.push(code);
            }
        }
    }

    // Generate provider tool structs (e.g., WebSearchTool20250305, BashTool20250124)
    for provider_tool in &tool_schemas.provider_tools {
        let schema_name = &provider_tool.schema_name;

        if let Some(schema) = all_schemas.get(schema_name) {
            if provider == "anthropic" {
                if let Some(config_schema_name) = schema
                    .get("properties")
                    .and_then(|properties| properties.get("configs"))
                    .and_then(nullable_referenced_schema_name)
                {
                    generate_referenced_schema_structs(
                        config_schema_name,
                        all_schemas,
                        provider,
                        &mut seen,
                        &mut generated_structs,
                    );
                }
            }

            let rust_name = schema_name_to_rust_type(schema_name);

            if seen.insert(rust_name.clone()) {
                let code =
                    generate_tool_struct_direct(schema_name, schema, all_schemas, provider, false);
                generated_structs.push(code);
            }
        }
    }

    Ok(generated_structs)
}

fn generate_tool_enum(
    provider: &str,
    tool_schemas: &ToolSchemas,
    _spec: &serde_json::Value,
) -> String {
    let mut enum_def = String::new();
    enum_def.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]\n");
    enum_def.push_str("#[serde(tag = \"type\")]\n");
    enum_def.push_str(&format!("#[ts(export_to = \"{}/\")]\n", provider));
    enum_def.push_str("pub enum Tool {\n");
    // Provider tools (bash, text_editor, web_search, etc.) come first and use tagged
    // deserialization via #[serde(rename = "...")]. The enum-level #[serde(tag = "type")]
    // tells serde to look for a "type" field in the JSON to determine which variant to use.
    // When JSON contains {"type": "bash_20250124", ...}, serde matches the "type" value
    // against each variant's rename and deserializes into the matching one.
    for provider_tool in &tool_schemas.provider_tools {
        let variant_name = schema_name_to_variant(&provider_tool.schema_name);
        let type_name = schema_name_to_rust_type(&provider_tool.schema_name);
        enum_def.push_str(&format!(
            "    #[serde(rename = \"{}\")]\n    {}({}),\n\n",
            provider_tool.tool_type, variant_name, type_name
        ));
    }

    // Client tools (Custom) use #[serde(untagged)] which makes them a fallback. When serde
    // can't match any tagged variant (either because "type" is missing or has an unknown
    // value), it tries untagged variants in order, attempting to deserialize the JSON
    // directly into the variant's inner type based on structure alone. This is essential
    // because Anthropic's API doesn't require a "type" field for custom tools - a tool like
    // {"name": "get_weather", "input_schema": {...}} has no "type" but should deserialize
    // as Tool::Custom. Order matters: provider tools must come first so they match when
    // "type" is present, with Custom last as the catch-all fallback.
    for client_schema in &tool_schemas.client_tools {
        let variant_name = schema_name_to_variant(client_schema);
        let type_name = schema_name_to_rust_type(client_schema);
        enum_def.push_str(&format!(
            "    #[serde(untagged)]\n    {}({}),\n\n",
            variant_name, type_name
        ));
    }

    enum_def.push_str("}\n");
    enum_def
}

fn generate_tool_struct_direct(
    schema_name: &str,
    schema: &serde_json::Value,
    all_schemas: &serde_json::Map<String, serde_json::Value>,
    provider: &str,
    typed_references: bool,
) -> String {
    let rust_name = schema_name_to_rust_type(schema_name);

    let mut output = String::new();

    // Extract description if available
    if let Some(desc) = schema.get("description").and_then(|d| d.as_str()) {
        for line in desc.lines() {
            output.push_str(&format!("/// {}\n", line));
        }
    }

    // Add derives
    output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]\n");
    let strict_toolset = matches!(
        schema_name,
        "BrowserToolset_20260801" | "ComputerToolset_20260801"
    );
    let preserves_browser_state_optionality = provider == "anthropic"
        && matches!(
            schema_name,
            "RequestBrowserStateBlock"
                | "BrowserStateChangeDownloadCompleted"
                | "BrowserStateChangeDownloadFailed"
        );
    if (typed_references || strict_toolset)
        && schema
            .get("additionalProperties")
            .and_then(|value| value.as_bool())
            == Some(false)
    {
        output.push_str("#[serde(deny_unknown_fields)]\n");
    }
    output.push_str(&format!("#[ts(export_to = \"{}/\")]\n", provider));
    output.push_str(&format!("pub struct {} {{\n", rust_name));

    // Get properties and required fields
    let props = schema.get("properties").and_then(|p| p.as_object());

    let required: HashSet<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    if let Some(properties) = props {
        for (prop_name, prop_schema) in properties {
            // Skip discriminators. The two toolset config maps are the exception: `type` is the
            // name of a configurable member tool there, not this object's discriminator.
            let preserves_type_member = matches!(
                schema_name,
                "BrowserToolsetConfigs" | "ComputerToolsetConfigs"
            );
            if prop_name == "type" && !preserves_type_member {
                continue;
            }

            let field_name = to_rust_field_name(prop_name);
            let rust_type = match (provider, prop_name.as_str()) {
                ("anthropic", "allowed_callers") => "Vec<AllowedCaller>".to_string(),
                ("anthropic", "response_inclusion") => "ResponseInclusion".to_string(),
                ("anthropic", "configs") => typed_referenced_schema_type(prop_schema)
                    .unwrap_or_else(|| schema_type_to_rust(prop_schema, all_schemas)),
                _ if typed_references => typed_referenced_schema_type(prop_schema)
                    .unwrap_or_else(|| schema_type_to_rust(prop_schema, all_schemas)),
                _ => schema_type_to_rust(prop_schema, all_schemas),
            };
            let is_required = required.contains(prop_name);

            // Add field documentation if available
            if let Some(desc) = prop_schema.get("description").and_then(|d| d.as_str()) {
                for line in desc.lines() {
                    output.push_str(&format!("    /// {}\n", line));
                }
            }

            // Handle serde rename if field name differs from property name
            let needs_rename = field_name != *prop_name && !field_name.starts_with("r#");

            if !is_required {
                output.push_str("    #[serde(skip_serializing_if = \"Option::is_none\")]\n");
                if preserves_browser_state_optionality {
                    if schema_allows_null(prop_schema) {
                        output.push_str("    #[ts(optional = nullable)]\n");
                    } else {
                        output.push_str("    #[ts(optional)]\n");
                    }
                }
            }

            if needs_rename {
                output.push_str(&format!("    #[serde(rename = \"{}\")]\n", prop_name));
            }

            // Add ts(type = "unknown") for serde_json::Value fields
            if rust_type.contains("serde_json::Value") {
                output.push_str("    #[ts(type = \"unknown\")]\n");
            }

            if is_required {
                output.push_str(&format!("    pub {}: {},\n", field_name, rust_type));
            } else {
                output.push_str(&format!("    pub {}: Option<{}>,\n", field_name, rust_type));
            }
        }
    }

    output.push_str("}\n");
    output
}

fn schema_allows_null(schema: &serde_json::Value) -> bool {
    schema.get("type").and_then(|value| value.as_str()) == Some("null")
        || ["anyOf", "oneOf"].into_iter().any(|union| {
            schema
                .get(union)
                .and_then(|variants| variants.as_array())
                .is_some_and(|variants| variants.iter().any(schema_allows_null))
        })
}

fn generate_referenced_schema_structs(
    schema_name: &str,
    all_schemas: &serde_json::Map<String, serde_json::Value>,
    provider: &str,
    seen: &mut HashSet<String>,
    generated_structs: &mut Vec<String>,
) {
    let rust_name = schema_name_to_rust_type(schema_name);
    if !seen.insert(rust_name) {
        return;
    }

    let Some(schema) = all_schemas.get(schema_name) else {
        return;
    };

    let mut dependencies = Vec::new();
    collect_referenced_schema_names(schema, &mut dependencies);
    for dependency in dependencies {
        generate_referenced_schema_structs(
            &dependency,
            all_schemas,
            provider,
            seen,
            generated_structs,
        );
    }

    generated_structs.push(generate_tool_struct_direct(
        schema_name,
        schema,
        all_schemas,
        provider,
        true,
    ));
}

fn collect_referenced_schema_names(schema: &serde_json::Value, names: &mut Vec<String>) {
    match schema {
        serde_json::Value::Object(object) => {
            if let Some(name) = object
                .get("$ref")
                .and_then(|reference| reference.as_str())
                .and_then(|reference| reference.split('/').next_back())
            {
                if !names.iter().any(|existing| existing == name) {
                    names.push(name.to_string());
                }
            }
            for value in object.values() {
                collect_referenced_schema_names(value, names);
            }
        }
        serde_json::Value::Array(array) => {
            for value in array {
                collect_referenced_schema_names(value, names);
            }
        }
        _ => {}
    }
}

fn nullable_referenced_schema_name(schema: &serde_json::Value) -> Option<&str> {
    schema
        .get("$ref")
        .and_then(|reference| reference.as_str())
        .or_else(|| {
            schema
                .get("anyOf")
                .or_else(|| schema.get("oneOf"))
                .and_then(|variants| variants.as_array())
                .and_then(|variants| {
                    let mut references = variants.iter().filter_map(|variant| {
                        variant.get("$ref").and_then(|reference| reference.as_str())
                    });
                    let reference = references.next()?;
                    references.next().is_none().then_some(reference)
                })
        })?
        .split('/')
        .next_back()
}

fn typed_referenced_schema_type(schema: &serde_json::Value) -> Option<String> {
    if let Some(schema_name) = schema
        .get("$ref")
        .and_then(|reference| reference.as_str())
        .and_then(|reference| reference.split('/').next_back())
    {
        return Some(schema_name_to_rust_type(schema_name));
    }

    if let Some(variants) = schema
        .get("anyOf")
        .or_else(|| schema.get("oneOf"))
        .and_then(|variants| variants.as_array())
    {
        let non_null = variants
            .iter()
            .filter(|variant| variant.get("type").and_then(|value| value.as_str()) != Some("null"))
            .collect::<Vec<_>>();
        if non_null.len() == 1 {
            return typed_referenced_schema_type(non_null[0]);
        }

        let referenced_names = non_null
            .iter()
            .filter_map(|variant| {
                variant
                    .get("$ref")
                    .and_then(|reference| reference.as_str())
                    .and_then(|reference| reference.split('/').next_back())
            })
            .collect::<Vec<_>>();
        if schema.get("discriminator").is_some() && referenced_names.len() == non_null.len() {
            return common_schema_name_prefix(&referenced_names).map(schema_name_to_rust_type);
        }
    }

    if schema.get("type").and_then(|value| value.as_str()) == Some("array") {
        return schema
            .get("items")
            .and_then(typed_referenced_schema_type)
            .map(|item_type| format!("Vec<{item_type}>"));
    }

    None
}

fn common_schema_name_prefix<'a>(names: &[&'a str]) -> Option<&'a str> {
    let first = *names.first()?;
    let mut end = first.len();
    for name in &names[1..] {
        end = first
            .char_indices()
            .take_while(|(index, character)| {
                *index < end && *index < name.len() && name[*index..].starts_with(*character)
            })
            .map(|(index, character)| index + character.len_utf8())
            .last()
            .unwrap_or(0);
    }
    let prefix = first[..end].trim_end_matches('_');
    (!prefix.is_empty()).then_some(prefix)
}

/// Replace quicktype's flattened request tool-result browser state shapes with the
/// discriminated types from the Anthropic schema. Non-browser tool-result blocks retain
/// quicktype's existing flattened representation to keep this correction narrowly scoped.
pub fn preserve_anthropic_browser_state_types(
    existing: &str,
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let definitions = split_type_definitions(existing);
    let block = definition_named(&definitions, "Block")?;
    let block_type = definition_named(&definitions, "WebSearchToolResultBlockItemType")?;
    let flattened_state_change = definition_named(&definitions, "BrowserStateChange")?;
    let flattened_state_change_type = definition_named(&definitions, "StateChangeType")?;

    let non_browser_block = remove_struct_fields(
        &block.replace("pub struct Block {", "pub struct NonBrowserBlock {"),
        &["block_type", "state_changes", "tabs"],
    );
    let precise_types = generate_anthropic_browser_state_types(spec)?;
    let replacement = format!("\n{precise_types}\n\n{non_browser_block}");

    let mut processed = existing.replacen(block, &replacement, 1);
    for obsolete in [
        block_type,
        flattened_state_change,
        flattened_state_change_type,
    ] {
        processed = processed.replacen(obsolete, "", 1);
    }
    preserve_anthropic_browser_tab_active(&processed, spec)
}

fn preserve_anthropic_browser_tab_active(
    existing: &str,
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let schema = get_schemas(spec)
        .and_then(|schemas| schemas.get("BrowserStateTabEntry"))
        .ok_or("BrowserStateTabEntry schema not found")?;
    let active = schema
        .get("properties")
        .and_then(|properties| properties.get("active"))
        .ok_or("BrowserStateTabEntry.active schema not found")?;
    let active_is_required = schema
        .get("required")
        .and_then(|required| required.as_array())
        .is_some_and(|required| {
            required
                .iter()
                .any(|field| field.as_str() == Some("active"))
        });
    if active_is_required || schema_allows_null(active) {
        return Err("BrowserStateTabEntry.active is no longer optional and non-null".into());
    }

    let old =
        "    #[serde(skip_serializing_if = \"Option::is_none\")]\n    pub active: Option<bool>,";
    if !existing.contains(old) {
        return Err("Missing generated optional BrowserStateTabEntry.active field".into());
    }
    let new = "    #[serde(default, deserialize_with = \"super::deserialize_optional_non_null\")]\n    #[serde(skip_serializing_if = \"Option::is_none\")]\n    #[ts(optional)]\n    pub active: Option<bool>,";
    Ok(existing.replacen(old, new, 1))
}

/// Replace quicktype's flattened image/document source struct with a tagged enum.
/// Anthropic requires variant-specific fields (notably `file_id` for `type: "file"`),
/// which a single struct with optional fields cannot express at either the Rust or
/// TypeScript boundary.
pub fn preserve_anthropic_source_types(
    existing: &str,
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let schemas = get_schemas(spec).ok_or("No components.schemas in Anthropic spec")?;
    for schema_name in ["FileImageSource", "FileDocumentSource"] {
        let required = schemas
            .get(schema_name)
            .and_then(|schema| schema.get("required"))
            .and_then(|required| required.as_array())
            .ok_or_else(|| format!("{schema_name}.required not found"))?;
        if !required
            .iter()
            .any(|field| field.as_str() == Some("file_id"))
        {
            return Err(format!("{schema_name}.file_id is not required").into());
        }
    }

    let definitions = split_type_definitions(existing);
    let source = definition_named(&definitions, "Source")?;
    let document_source = definition_named(&definitions, "RequestDocumentBlockSource")?;
    let nested_image_source = definition_named(&definitions, "SourceSource")?;
    let nested_image_source_type = definition_named(&definitions, "PurpleType")?;
    let precise_source = r##"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[ts(export_to = "anthropic/")]
pub enum Source {
    Base64 {
        data: String,
        media_type: Base64ImageSourceMediaType,
    },
    Content {
        content: Base64ImageSourceContent,
    },
    File {
        file_id: String,
    },
    Text {
        data: String,
        media_type: Base64ImageSourceMediaType,
    },
    Url {
        url: String,
    },
}"##;
    let precise_document_source = r##"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[ts(export_to = "anthropic/")]
pub enum RequestDocumentBlockSource {
    Base64 {
        data: String,
        media_type: FluffyMediaType,
    },
    Content {
        content: Base64ImageSourceContent,
    },
    File {
        file_id: String,
    },
    Text {
        data: String,
        media_type: FluffyMediaType,
    },
    Url {
        url: String,
    },
}"##;
    let precise_nested_image_source = r##"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[ts(export_to = "anthropic/")]
pub enum SourceSource {
    Base64 {
        data: String,
        media_type: PurpleMediaType,
    },
    File {
        file_id: String,
    },
    Url {
        url: String,
    },
}"##;

    Ok(existing
        .replacen(source, precise_source, 1)
        .replacen(document_source, precise_document_source, 1)
        .replacen(nested_image_source, precise_nested_image_source, 1)
        .replacen(nested_image_source_type, "", 1))
}

fn definition_named<'a>(
    definitions: &'a [(String, String)],
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    definitions
        .iter()
        .find_map(|(definition_name, block)| (definition_name == name).then_some(block.as_str()))
        .ok_or_else(|| format!("generated Anthropic type `{name}` was not found").into())
}

fn remove_struct_fields(block: &str, removed_fields: &[&str]) -> String {
    let mut output = Vec::new();
    let mut pending = Vec::new();
    let mut inside_struct = false;

    for line in block.lines() {
        if !inside_struct {
            output.push(line);
            inside_struct = line.trim_start().starts_with("pub struct ");
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("///") || trimmed.starts_with("#[") || trimmed.is_empty() {
            pending.push(line);
            continue;
        }

        if let Some(field_name) = trimmed
            .strip_prefix("pub ")
            .and_then(|field| field.split(':').next())
        {
            if !removed_fields.contains(&field_name) {
                output.append(&mut pending);
                output.push(line);
            } else {
                pending.clear();
            }
            continue;
        }

        output.append(&mut pending);
        output.push(line);
    }

    output.join("\n")
}

fn generate_anthropic_browser_state_types(
    spec: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let schemas = get_schemas(spec).ok_or("No components.schemas in Anthropic spec")?;
    let browser_state_schema = schemas
        .get("RequestBrowserStateBlock")
        .ok_or("RequestBrowserStateBlock schema not found")?;
    let state_change_union = browser_state_schema
        .get("properties")
        .and_then(|properties| properties.get("state_changes"))
        .and_then(non_null_schema)
        .and_then(|array| array.get("items"))
        .ok_or("RequestBrowserStateBlock.state_changes union not found")?;

    let mut segments = vec![generate_request_tool_result_block_enum(schemas)?];
    segments.push(generate_tool_struct_direct(
        "RequestBrowserStateBlock",
        browser_state_schema,
        schemas,
        "anthropic",
        true,
    ));
    segments.push(generate_tagged_reference_enum(
        "BrowserStateChange",
        state_change_union,
        schemas,
        "anthropic",
    )?);

    for schema_name in referenced_variant_names(state_change_union)? {
        let schema = schemas
            .get(schema_name)
            .ok_or_else(|| format!("referenced Anthropic schema `{schema_name}` not found"))?;
        segments.push(generate_tool_struct_direct(
            schema_name,
            schema,
            schemas,
            "anthropic",
            true,
        ));
    }

    Ok(segments.join("\n\n"))
}

fn generate_request_tool_result_block_enum(
    schemas: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, Box<dyn std::error::Error>> {
    let item_union = schemas
        .get("RequestToolResultBlock")
        .and_then(|schema| schema.get("properties"))
        .and_then(|properties| properties.get("content"))
        .and_then(|content| content.get("anyOf"))
        .and_then(|variants| variants.as_array())
        .and_then(|variants| {
            variants.iter().find_map(|variant| {
                (variant.get("type").and_then(|value| value.as_str()) == Some("array"))
                    .then(|| variant.get("items"))
                    .flatten()
            })
        })
        .ok_or("RequestToolResultBlock content item union not found")?;
    let mapping = item_union
        .get("discriminator")
        .and_then(|discriminator| discriminator.get("mapping"))
        .and_then(|mapping| mapping.as_object())
        .ok_or("RequestToolResultBlock content discriminator mapping not found")?;

    let mut output = String::from(
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]\n\
#[serde(tag = \"type\")]\n\
#[ts(export_to = \"anthropic/\")]\n\
pub enum Block {\n",
    );
    for (tag, reference) in mapping {
        let schema_name = reference
            .as_str()
            .and_then(|reference| reference.split('/').next_back())
            .ok_or("invalid request tool-result content schema reference")?;
        let rust_name = schema_name_to_rust_type(schema_name);
        let variant = rust_name
            .strip_prefix("Request")
            .and_then(|name| name.strip_suffix("Block"))
            .unwrap_or(&rust_name);
        let payload = if schema_name == "RequestBrowserStateBlock" {
            "RequestBrowserStateBlock"
        } else {
            "NonBrowserBlock"
        };
        output.push_str(&format!(
            "    #[serde(rename = \"{tag}\")]\n    {variant}({payload}),\n"
        ));
    }
    // Quicktype shares this `Block` type with RequestWebSearchToolResultBlock.content.
    // Preserve that schema arm while tightening only the browser-state variant.
    if let Some(web_search_result) = schemas.get("RequestWebSearchResultBlock") {
        let tag = web_search_result
            .get("properties")
            .and_then(|properties| properties.get("type"))
            .and_then(|schema_type| schema_type.get("const"))
            .and_then(|value| value.as_str())
            .ok_or("RequestWebSearchResultBlock has no constant type discriminator")?;
        output.push_str(&format!(
            "    #[serde(rename = \"{tag}\")]\n    WebSearchResult(NonBrowserBlock),\n"
        ));
    }
    output.push_str("}\n");
    Ok(output)
}

fn generate_tagged_reference_enum(
    enum_name: &str,
    union_schema: &serde_json::Value,
    schemas: &serde_json::Map<String, serde_json::Value>,
    provider: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = format!(
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]\n\
#[serde(tag = \"type\")]\n\
#[ts(export_to = \"{provider}/\")]\n\
pub enum {enum_name} {{\n"
    );
    for schema_name in referenced_variant_names(union_schema)? {
        let schema = schemas
            .get(schema_name)
            .ok_or_else(|| format!("referenced Anthropic schema `{schema_name}` not found"))?;
        let tag = schema
            .get("properties")
            .and_then(|properties| properties.get("type"))
            .and_then(|schema_type| schema_type.get("const"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| format!("schema `{schema_name}` has no constant type discriminator"))?;
        let rust_name = schema_name_to_rust_type(schema_name);
        let variant = rust_name.strip_prefix(enum_name).unwrap_or(&rust_name);
        output.push_str(&format!(
            "    #[serde(rename = \"{tag}\")]\n    {variant}({rust_name}),\n"
        ));
    }
    output.push_str("}\n");
    Ok(output)
}

fn referenced_variant_names(
    union_schema: &serde_json::Value,
) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    let variants = union_schema
        .get("oneOf")
        .or_else(|| union_schema.get("anyOf"))
        .and_then(|variants| variants.as_array())
        .ok_or("discriminated union variants not found")?;
    variants
        .iter()
        .map(|variant| {
            variant
                .get("$ref")
                .and_then(|reference| reference.as_str())
                .and_then(|reference| reference.split('/').next_back())
                .ok_or("discriminated union variant is not a schema reference")
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn non_null_schema(schema: &serde_json::Value) -> Option<&serde_json::Value> {
    schema
        .get("anyOf")
        .or_else(|| schema.get("oneOf"))
        .and_then(|variants| variants.as_array())
        .and_then(|variants| {
            let mut non_null = variants.iter().filter(|variant| {
                variant.get("type").and_then(|value| value.as_str()) != Some("null")
            });
            let schema = non_null.next()?;
            non_null.next().is_none().then_some(schema)
        })
}

// -------------------------------------------------------------------------
// Replacement helpers
// -------------------------------------------------------------------------

fn filter_tool_code_against_existing(tool_code: &str, existing: &str) -> String {
    let existing_names: HashSet<String> = split_type_definitions(existing)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let mut blocks = Vec::new();
    let mut seen = HashSet::new();

    for (name, block) in split_type_definitions(tool_code) {
        if (name == "Tool" || !existing_names.contains(&name)) && seen.insert(name.clone()) {
            blocks.push(block);
        }
    }

    blocks.join("\n\n")
}

fn find_tool_struct_span(content: &str) -> Option<(usize, usize)> {
    let struct_pos = content.find("pub struct Tool {")?;
    let attr_start = content[..struct_pos]
        .rfind("#[derive(")
        .unwrap_or(struct_pos);
    let end = find_closing_brace(content, attr_start)?;
    Some((attr_start, end))
}

// -------------------------------------------------------------------------
// Utilities
// -------------------------------------------------------------------------

fn schema_name_to_rust_type(schema_name: &str) -> String {
    // Quicktype uses the schema name directly; normalize by stripping underscores and
    // renaming the top-level Tool (custom) schema to avoid colliding with the enum name.
    if schema_name == "Tool" {
        return "CustomTool".to_string();
    }
    schema_name.replace('_', "")
}

fn schema_name_to_variant(schema_name: &str) -> String {
    if schema_name == "Tool" || schema_name == "BetaTool" {
        return "Custom".to_string();
    }

    if let Some(idx) = schema_name.rfind('_') {
        let version = &schema_name[idx + 1..];
        let name_part = strip_tool_suffix(&schema_name[..idx]);
        return format!("{}{}", name_part, version);
    }

    strip_tool_suffix(schema_name).to_string()
}

/// Drop the trailing `Tool` from a schema name so `WebSearchTool` becomes `WebSearch`.
///
/// Only the suffix is removed: names such as `BrowserToolset` describe a toolset family
/// rather than a single tool, and stripping every `Tool` occurrence would mangle them into
/// `Browserset`.
fn strip_tool_suffix(schema_name: &str) -> &str {
    schema_name.strip_suffix("Tool").unwrap_or(schema_name)
}

fn split_type_definitions(content: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i].trim_start();
        let is_struct = line.starts_with("pub struct ");
        let is_enum = line.starts_with("pub enum ");

        if is_struct || is_enum {
            let mut start = i;
            while start > 0 {
                let prev = lines[start - 1].trim_start();
                if prev.starts_with("#[") || prev.starts_with("///") || prev.is_empty() {
                    start -= 1;
                } else {
                    break;
                }
            }

            let def_line = lines[i];
            let parts: Vec<&str> = def_line.split_whitespace().collect();
            if parts.len() >= 3 {
                let mut name = parts[2]
                    .trim_end_matches('{')
                    .trim_end_matches('<')
                    .to_string();
                if name.ends_with(',') {
                    name.pop();
                }

                let mut depth = 0isize;
                let mut end = i;
                for (j, line) in lines.iter().enumerate().skip(i) {
                    for ch in line.chars() {
                        if ch == '{' {
                            depth += 1;
                        } else if ch == '}' {
                            depth -= 1;
                            if depth == 0 {
                                end = j;
                                break;
                            }
                        }
                    }
                    if depth == 0 && j >= i {
                        end = j;
                        break;
                    }
                }

                let block = lines[start..=end].join("\n");
                result.push((name, block));
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{
        enforce_anthropic_closed_request_types, generate_all_tool_code,
        generate_anthropic_browser_state_types, preserve_anthropic_browser_tab_active,
        preserve_anthropic_optional_transformation_fields,
        preserve_anthropic_required_nullable_fields, preserve_anthropic_source_types,
    };
    use big_serde_json as serde_json;

    #[test]
    fn anthropic_response_inclusion_uses_generated_enum() {
        let spec: serde_json::Value = serde_json::from_str(
            r#"{
                "components": {
                    "schemas": {
                        "WebFetchTool_20260318": {
                            "type": "object",
                            "properties": {
                                "name": { "const": "web_fetch", "type": "string" },
                                "response_inclusion": {
                                    "enum": ["full", "excluded"],
                                    "title": "Response Inclusion",
                                    "type": "string"
                                },
                                "type": { "const": "web_fetch_20260318", "type": "string" }
                            },
                            "required": ["name", "type"]
                        },
                        "WebSearchTool_20260318": {
                            "type": "object",
                            "properties": {
                                "name": { "const": "web_search", "type": "string" },
                                "response_inclusion": {
                                    "enum": ["full", "excluded"],
                                    "title": "Response Inclusion",
                                    "type": "string"
                                },
                                "type": { "const": "web_search_20260318", "type": "string" }
                            },
                            "required": ["name", "type"]
                        }
                    }
                }
            }"#,
        )
        .expect("test spec should be valid JSON");

        let generated = generate_all_tool_code("anthropic", &spec)
            .expect("Anthropic tool generation should succeed");

        assert_eq!(
            generated
                .matches("pub response_inclusion: Option<ResponseInclusion>,")
                .count(),
            2
        );
        assert!(!generated.contains("pub response_inclusion: Option<String>,"));
    }

    #[test]
    fn anthropic_toolset_tool_variants_keep_their_schema_derived_names() {
        let spec: serde_json::Value = serde_json::from_str(
            r#"{
                "components": {
                    "schemas": {
                        "BrowserToolset_20260801": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "type": { "const": "browser_toolset_20260801", "type": "string" }
                            },
                            "required": ["type"]
                        },
                        "ComputerToolset_20260801": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "type": { "const": "computer_toolset_20260801", "type": "string" }
                            },
                            "required": ["type"]
                        },
                        "WebSearchTool_20250305": {
                            "type": "object",
                            "properties": {
                                "name": { "const": "web_search", "type": "string" },
                                "type": { "const": "web_search_20250305", "type": "string" }
                            },
                            "required": ["name", "type"]
                        }
                    }
                }
            }"#,
        )
        .expect("test spec should be valid JSON");

        let generated = generate_all_tool_code("anthropic", &spec)
            .expect("Anthropic tool generation should succeed");

        assert!(generated.contains("    BrowserToolset20260801(BrowserToolset20260801),"));
        assert!(generated.contains("    ComputerToolset20260801(ComputerToolset20260801),"));
        assert!(generated.contains(
            "#[serde(deny_unknown_fields)]\n#[ts(export_to = \"anthropic/\")]\npub struct BrowserToolset20260801"
        ));
        assert!(generated.contains(
            "#[serde(deny_unknown_fields)]\n#[ts(export_to = \"anthropic/\")]\npub struct ComputerToolset20260801"
        ));
        assert!(!generated.contains("Browserset20260801"));
        assert!(!generated.contains("Computerset20260801"));
        // Single tools keep the established variant names that drop the `Tool` suffix.
        assert!(generated.contains("    WebSearch20250305(WebSearchTool20250305),"));
    }

    #[test]
    fn anthropic_toolset_configs_use_referenced_schema_types() {
        let spec: serde_json::Value = serde_json::from_str(
            r##"{
                "components": {
                    "schemas": {
                        "BrowserToolsetConfigs": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "navigate": {
                                    "anyOf": [
                                        { "$ref": "#/components/schemas/BrowserNavigateConfig" },
                                        { "type": "null" }
                                    ]
                                },
                                "type": {
                                    "anyOf": [
                                        { "$ref": "#/components/schemas/BrowserTypeConfig" },
                                        { "type": "null" }
                                    ]
                                }
                            }
                        },
                        "BrowserNavigateConfig": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "enabled": {
                                    "anyOf": [
                                        { "type": "boolean" },
                                        { "type": "null" }
                                    ]
                                }
                            }
                        },
                        "BrowserTypeConfig": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "enabled": {
                                    "anyOf": [
                                        { "type": "boolean" },
                                        { "type": "null" }
                                    ]
                                }
                            }
                        },
                        "BrowserToolset_20260801": {
                            "type": "object",
                            "properties": {
                                "configs": {
                                    "anyOf": [
                                        { "$ref": "#/components/schemas/BrowserToolsetConfigs" },
                                        { "type": "null" }
                                    ]
                                },
                                "type": { "const": "browser_toolset_20260801", "type": "string" }
                            },
                            "required": ["type"]
                        }
                    }
                }
            }"##,
        )
        .expect("test spec should be valid JSON");

        let generated = generate_all_tool_code("anthropic", &spec)
            .expect("Anthropic tool generation should succeed");

        assert!(generated.contains("pub configs: Option<BrowserToolsetConfigs>,"));
        assert!(generated.contains("pub navigate: Option<BrowserNavigateConfig>,"));
        assert!(generated.contains("pub r#type: Option<BrowserTypeConfig>,"));
        assert!(generated.contains("pub struct BrowserNavigateConfig"));
        assert!(generated.contains("pub struct BrowserTypeConfig"));
        assert_eq!(
            generated.matches("#[serde(deny_unknown_fields)]").count(),
            3
        );
        assert!(!generated.contains("pub r#type: String,"));
        assert!(!generated.contains("pub configs: Option<serde_json::Value>,"));
    }

    #[test]
    fn anthropic_browser_state_types_preserve_discriminated_requirements() {
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/anthropic/openapi.yml"))
                .expect("checked-in Anthropic spec should be valid JSON");

        let generated = generate_anthropic_browser_state_types(&spec)
            .expect("browser state types should be generated from the Anthropic schema");

        assert!(generated.contains("#[serde(tag = \"type\")]"));
        assert!(generated.contains("pub enum Block"));
        assert!(generated.contains("BrowserState(RequestBrowserStateBlock)"));
        assert!(generated.contains("WebSearchResult(NonBrowserBlock)"));
        assert!(generated.contains("pub tabs: Vec<BrowserStateTabEntry>,"));
        assert!(generated.contains(
            "#[ts(optional = nullable)]\n    pub state_changes: Option<Vec<BrowserStateChange>>"
        ));
        assert!(generated.contains(
            "#[ts(optional = nullable)]\n    pub cache_control: Option<CacheControlEphemeral>"
        ));
        assert!(generated.contains("#[ts(optional = nullable)]\n    pub path: Option<String>"));
        assert!(generated.contains("#[ts(optional = nullable)]\n    pub size_bytes: Option<i64>"));
        assert!(generated.contains("#[ts(optional = nullable)]\n    pub error: Option<String>"));
        assert!(generated.contains("pub enum BrowserStateChange"));
        assert!(generated.contains("DownloadStarted(BrowserStateChangeDownloadStarted)"));
        assert!(generated.contains("pub download_id: String,"));
        assert!(generated.contains("pub url: String,"));
    }

    #[test]
    fn anthropic_browser_tab_active_remains_optional_and_non_null() {
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/anthropic/openapi.yml"))
                .expect("checked-in Anthropic spec should be valid JSON");
        let quicktype = r#"pub struct BrowserStateTabEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}"#;

        let generated = preserve_anthropic_browser_tab_active(quicktype, &spec)
            .expect("active should remain optional and non-null");

        assert!(generated.contains(
            "#[serde(default, deserialize_with = \"super::deserialize_optional_non_null\")]"
        ));
        assert!(generated.contains("#[ts(optional)]\n    pub active: Option<bool>"));
    }

    #[test]
    fn anthropic_source_types_preserve_variant_requirements() {
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/anthropic/openapi.yml"))
                .expect("checked-in Anthropic spec should be valid JSON");
        let quicktype = r#"#[derive(Debug)]
pub struct Source {
    pub data: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Debug)]
pub struct RequestDocumentBlockSource {
    pub data: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Debug)]
pub struct SourceSource {
    pub data: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Debug)]
pub enum PurpleType {
    Base64,
    File,
    Url,
}"#;

        let generated = preserve_anthropic_source_types(quicktype, &spec)
            .expect("source types should be generated from the Anthropic schema");

        assert!(generated.contains("#[serde(tag = \"type\", rename_all = \"snake_case\")]"));
        assert!(generated.contains("File {\n        file_id: String,"));
        assert!(generated.contains("Url {\n        url: String,"));
        assert!(!generated.contains("file_id: Option<String>"));
        assert!(generated.contains("pub enum RequestDocumentBlockSource"));
        assert!(generated.contains("pub enum SourceSource"));
        assert!(!generated.contains("pub enum PurpleType"));
        assert_eq!(
            generated.matches("#[serde(deny_unknown_fields)]").count(),
            3
        );
    }

    #[test]
    fn anthropic_toolset_member_configs_reject_unknown_fields() {
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/anthropic/openapi.yml"))
                .expect("checked-in Anthropic spec should be valid JSON");
        let quicktype = r#"#[derive(Debug, Clone)]
pub struct BrowserCloseTabConfig {
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ComputerCursorPositionConfig {
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct RequestImageTransformations {
    pub oversized_image: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContainerParams {
    pub id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SkillParams {
    pub skill_id: String,
}

#[derive(Debug, Clone)]
pub struct BrowserStateTabEntry {
    pub tab_id: String,
}"#;

        let generated = enforce_anthropic_closed_request_types(quicktype, &spec)
            .expect("closed request types should be made strict");

        assert_eq!(
            generated.matches("#[serde(deny_unknown_fields)]").count(),
            6
        );
        assert!(
            generated.contains("#[serde(deny_unknown_fields)]\npub struct BrowserCloseTabConfig")
        );
        assert!(generated
            .contains("#[serde(deny_unknown_fields)]\npub struct ComputerCursorPositionConfig"));
        for type_name in [
            "RequestImageTransformations",
            "ContainerParams",
            "SkillParams",
            "BrowserStateTabEntry",
        ] {
            assert!(generated.contains(&format!(
                "#[serde(deny_unknown_fields)]\npub struct {type_name}"
            )));
        }
    }

    #[test]
    fn anthropic_container_skills_remains_required_and_nullable() {
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/anthropic/openapi.yml"))
                .expect("checked-in Anthropic spec should be valid JSON");
        let quicktype = r#"#[derive(Debug, Clone)]
pub struct Container {
    pub expires_at: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<ContainerSkill>>,
}"#;

        let generated = preserve_anthropic_required_nullable_fields(quicktype, &spec)
            .expect("Container.skills should preserve required-nullable semantics");

        assert!(generated.contains(
            "#[serde(deserialize_with = \"super::deserialize_required_nullable\")]\n    pub skills: Option<Vec<ContainerSkill>>"
        ));
        assert!(!generated.contains("skip_serializing_if = \"Option::is_none\""));
    }

    #[test]
    fn anthropic_transformation_fields_remain_optional_in_typescript() {
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/anthropic/openapi.yml"))
                .expect("checked-in Anthropic spec should be valid JSON");
        let quicktype = r#"#[derive(Debug, Clone)]
pub struct InputContentBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformations: Option<RequestImageTransformations>,
}

#[derive(Debug, Clone)]
pub struct RequestImageTransformations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oversized_image: Option<OversizedImage>,
}"#;

        let generated = preserve_anthropic_optional_transformation_fields(quicktype, &spec)
            .expect("transformation fields should remain optional");

        assert_eq!(generated.matches("#[ts(optional = nullable)]").count(), 2);
        assert!(generated.contains(
            "#[ts(optional = nullable)]\n    pub transformations: Option<RequestImageTransformations>"
        ));
        assert!(generated.contains(
            "#[ts(optional = nullable)]\n    pub oversized_image: Option<OversizedImage>"
        ));
    }
}
