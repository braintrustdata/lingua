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
    if typed_references
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
            // Skip the "type" field - it's handled by serde tag on the enum
            if prop_name == "type" {
                continue;
            }

            let field_name = to_rust_field_name(prop_name);
            let rust_type = match (provider, prop_name.as_str()) {
                ("anthropic", "allowed_callers") => "Vec<AllowedCaller>".to_string(),
                ("anthropic", "response_inclusion") => "ResponseInclusion".to_string(),
                ("anthropic", "configs") => nullable_referenced_schema_type(prop_schema)
                    .unwrap_or_else(|| schema_type_to_rust(prop_schema, all_schemas)),
                _ if typed_references => nullable_referenced_schema_type(prop_schema)
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

fn nullable_referenced_schema_type(schema: &serde_json::Value) -> Option<String> {
    nullable_referenced_schema_name(schema).map(schema_name_to_rust_type)
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
    use super::generate_all_tool_code;
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
                            "properties": {
                                "type": { "const": "browser_toolset_20260801", "type": "string" }
                            },
                            "required": ["type"]
                        },
                        "ComputerToolset_20260801": {
                            "type": "object",
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
        assert!(generated.contains("pub struct BrowserNavigateConfig"));
        assert_eq!(
            generated.matches("#[serde(deny_unknown_fields)]").count(),
            2
        );
        assert!(!generated.contains("pub configs: Option<serde_json::Value>,"));
    }
}
