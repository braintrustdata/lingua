//! Standalone type generation script for Lingua providers
//!
//! Usage: cargo run --bin generate-types -- [provider]

use std::path::{Path, PathBuf};

use big_serde_json as serde_json;
use tool_generator::{generate_all_tool_code, replace_tool_struct_with_enum};

mod schema_converter;
mod tool_generator;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let provider = if args.len() > 1 {
        &args[1]
    } else {
        println!("Usage: {} [provider]", args[0]);
        println!("Providers: openai, anthropic, google, all");
        std::process::exit(1);
    };

    println!("🔄 Generating types for provider: {}", provider);

    match provider.as_str() {
        "openai" => generate_openai_types(),
        "anthropic" => generate_anthropic_types(),
        "google" => generate_google_protobuf_types_from_git(),
        "all" => {
            generate_openai_types();
            generate_anthropic_types();
            generate_google_protobuf_types_from_git();
        }
        _ => {
            println!("❌ Unknown provider: {}", provider);
            println!("Available providers: openai, anthropic, google, all");
            std::process::exit(1);
        }
    }

    println!("✅ Type generation completed successfully!");
}

fn generate_openai_types() {
    println!("📦 Generating OpenAI types from OpenAPI spec using quicktype...");

    let spec_file_path = "specs/openai/openapi.yml";

    let openai_spec = match std::fs::read_to_string(spec_file_path) {
        Ok(content) => content,
        Err(e) => {
            println!(
                "❌ Failed to read OpenAPI spec at {}: {}",
                spec_file_path, e
            );
            println!(
                "Run './pipelines/generate-provider-types.sh openai' to download the spec first"
            );
            return;
        }
    };

    println!("🔍 Parsing YAML OpenAPI spec...");

    let schema: serde_json::Value = match serde_yaml::from_str(&openai_spec) {
        Ok(value) => value,
        Err(e) => {
            println!("❌ Failed to parse OpenAPI spec as YAML: {}", e);
            return;
        }
    };

    let schemas = schema.get("components").and_then(|c| c.get("schemas"));

    if let Some(_schemas) = schemas {
        println!("✅ Found OpenAI components/schemas section");

        // Generate essential OpenAI types for chat completion APIs using quicktype
        println!("🏗️  Generating essential OpenAI types for chat completions");
        generate_openai_specific_types(&openai_spec);
    } else {
        println!("❌ No components/schemas section found in OpenAPI spec");
    }
}

fn generate_anthropic_types() {
    println!("📦 Generating Anthropic types from OpenAPI spec...");

    let spec_file_path = "specs/anthropic/openapi.yml";

    let anthropic_spec = match std::fs::read_to_string(spec_file_path) {
        Ok(content) => content,
        Err(e) => {
            println!(
                "❌ Failed to read Anthropic OpenAPI spec at {}: {}",
                spec_file_path, e
            );
            println!(
                "Run './pipelines/generate-provider-types.sh anthropic' to download the spec first"
            );
            return;
        }
    };

    println!("🔍 Parsing YAML OpenAPI spec...");

    let schema: serde_json::Value = match serde_yaml::from_str(&anthropic_spec) {
        Ok(value) => value,
        Err(e) => {
            println!("❌ Failed to parse Anthropic OpenAPI spec as YAML: {}", e);
            return;
        }
    };

    let schemas = schema.get("components").and_then(|c| c.get("schemas"));

    if let Some(_schemas) = schemas {
        println!("✅ Found Anthropic components/schemas section");

        // Generate essential Anthropic types for messages API using quicktype
        println!("🏗️  Generating essential Anthropic types for messages API");
        generate_anthropic_specific_types(&anthropic_spec);
    } else {
        println!("❌ No components/schemas section found in Anthropic OpenAPI spec");
    }
}

fn generate_openai_specific_types(openai_spec: &str) {
    println!("🏗️  Using quicktype for OpenAI type generation...");

    // Extract OpenAI OpenAPI spec
    let full_spec: serde_json::Value =
        serde_yaml::from_str(openai_spec).expect("Failed to parse OpenAI OpenAPI spec");

    // Generate types using quicktype approach
    match generate_openai_types_with_quicktype(&serde_json::to_string_pretty(&full_spec).unwrap()) {
        Ok(()) => {
            println!("✅ OpenAI types generated successfully with quicktype");
        }
        Err(e) => {
            println!("❌ Quicktype generation failed for OpenAI: {}", e);
            println!("📝 Falling back to minimal types");
            let _ = std::fs::write(
                "crates/lingua/src/providers/openai/generated.rs",
                "// Quicktype generation failed",
            );
        }
    }
}

fn generate_openai_types_with_quicktype(
    openapi_spec: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Parsing OpenAI OpenAPI spec...");

    let spec: serde_json::Value = serde_json::from_str(openapi_spec)?;

    // Extract essential OpenAI schemas for chat completions
    let essential_schemas = create_essential_openai_schemas(&spec);

    println!("🏗️  Generating OpenAI types with quicktype...");

    // Create a temporary JSON schema file for quicktype
    let temp_schema_path = std::env::temp_dir().join("openai_schemas.json");
    std::fs::write(
        &temp_schema_path,
        serde_json::to_string_pretty(&essential_schemas)?,
    )?;

    // Use quicktype to generate types
    let output = std::process::Command::new("quicktype")
        .arg("--src-lang")
        .arg("schema")
        .arg("--lang")
        .arg("rust")
        .arg("--derive-debug")
        .arg("--derive-clone")
        .arg("--derive-partial-eq")
        .arg("--visibility")
        .arg("public")
        .arg("--density")
        .arg("dense")
        .arg(&temp_schema_path)
        .output();

    let quicktype_output = match output {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8(output.stdout)?
            } else {
                return Err(format!(
                    "quicktype failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }
        Err(e) => return Err(format!("Failed to run quicktype: {}", e).into()),
    };

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_schema_path);

    // Post-process the quicktype output
    let mut processed_output = post_process_quicktype_output_for_openai(&quicktype_output);

    // Quicktype generates a flat `pub struct Tool` with all fields optional, but the
    // OpenAPI spec defines Tool as a discriminated union (anyOf with a "type" tag).
    // We replace quicktype's struct with a proper Rust enum using #[serde(tag = "type")]
    // so each tool variant (function, web_search, code_interpreter, etc.) serializes
    // correctly with its specific fields.
    if let Ok(tool_code) = generate_all_tool_code("openai", &spec) {
        processed_output = replace_tool_struct_with_enum(&processed_output, &tool_code);
    }

    let dest_path = "crates/lingua/src/providers/openai/generated.rs";

    // Create directory if needed
    if let Some(parent) = std::path::Path::new(dest_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write generated types
    std::fs::write(dest_path, &processed_output)?;

    // Format with cargo fmt
    let _ = std::process::Command::new("cargo")
        .args(["fmt", "--", dest_path])
        .output();

    println!("📝 Generated OpenAI types to: {}", dest_path);

    Ok(())
}

fn create_essential_openai_schemas(spec: &serde_json::Value) -> serde_json::Value {
    // Simplified approach: just specify input/output types, let dependency resolution handle the rest
    let chat_request_type = "CreateChatCompletionRequest";
    let chat_response_type = "CreateChatCompletionResponse";
    let chat_stream_response_type = "CreateChatCompletionStreamResponse";
    let responses_request_type = "CreateResponse";
    let responses_response_type = "Response";

    let default_map = serde_json::Map::new();
    let all_schemas = spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
        .unwrap_or(&default_map);

    let mut essential_schemas = serde_json::Map::new();
    let mut processed = std::collections::HashSet::new();

    // Add chat completion types with their dependencies
    add_openai_schema_with_dependencies(
        chat_request_type,
        all_schemas,
        &mut essential_schemas,
        &mut processed,
    );
    add_openai_schema_with_dependencies(
        chat_response_type,
        all_schemas,
        &mut essential_schemas,
        &mut processed,
    );
    add_openai_schema_with_dependencies(
        chat_stream_response_type,
        all_schemas,
        &mut essential_schemas,
        &mut processed,
    );

    // Add responses API types with their dependencies
    add_openai_schema_with_dependencies(
        responses_request_type,
        all_schemas,
        &mut essential_schemas,
        &mut processed,
    );
    add_openai_schema_with_dependencies(
        responses_response_type,
        all_schemas,
        &mut essential_schemas,
        &mut processed,
    );

    // Fix all $ref paths to point to #/definitions/ instead of #/components/schemas/
    let mut fixed_schemas = serde_json::Map::new();
    for (name, schema) in essential_schemas {
        fixed_schemas.insert(name, fix_openai_schema_refs(&schema));
    }

    // Create a clean root schema with separated input/output types for both APIs
    let root_schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "oneOf": [
            {
                "title": "ChatCompletionTypes",
                "type": "object",
                "properties": {
                    "chat_request": {"$ref": "#/definitions/CreateChatCompletionRequest"},
                    "chat_response": {"$ref": "#/definitions/CreateChatCompletionResponse"},
                    "chat_stream_response": {"$ref": "#/definitions/CreateChatCompletionStreamResponse"}
                }
            },
            {
                "title": "ResponsesTypes",
                "type": "object",
                "properties": {
                    "responses_request": {"$ref": "#/definitions/CreateResponse"},
                    "responses_response": {"$ref": "#/definitions/Response"}
                }
            }
        ],
        "definitions": fixed_schemas
    });

    root_schema
}

fn add_openai_schema_with_dependencies(
    type_name: &str,
    all_schemas: &serde_json::Map<String, serde_json::Value>,
    essential_schemas: &mut serde_json::Map<String, serde_json::Value>,
    processed: &mut std::collections::HashSet<String>,
) {
    if processed.contains(type_name) {
        return;
    }

    processed.insert(type_name.to_string());

    if let Some(schema) = all_schemas.get(type_name) {
        essential_schemas.insert(type_name.to_string(), schema.clone());

        // Find and add referenced types
        let mut refs = std::collections::HashSet::new();
        extract_schema_refs(schema, &mut refs);

        for ref_name in refs {
            add_openai_schema_with_dependencies(
                &ref_name,
                all_schemas,
                essential_schemas,
                processed,
            );
        }
    }
}

fn fix_openai_schema_refs(schema: &serde_json::Value) -> serde_json::Value {
    match schema {
        serde_json::Value::Object(obj) => {
            let mut fixed_obj = serde_json::Map::new();

            for (key, value) in obj {
                if key == "$ref" {
                    if let Some(ref_str) = value.as_str() {
                        // Fix the reference path
                        if ref_str.starts_with("#/components/schemas/") {
                            let new_ref =
                                ref_str.replace("#/components/schemas/", "#/definitions/");
                            fixed_obj.insert(key.clone(), serde_json::Value::String(new_ref));
                        } else {
                            fixed_obj.insert(key.clone(), value.clone());
                        }
                    } else {
                        fixed_obj.insert(key.clone(), value.clone());
                    }
                } else {
                    fixed_obj.insert(key.clone(), fix_openai_schema_refs(value));
                }
            }

            serde_json::Value::Object(fixed_obj)
        }
        serde_json::Value::Array(arr) => {
            let fixed_arr: Vec<serde_json::Value> =
                arr.iter().map(fix_openai_schema_refs).collect();
            serde_json::Value::Array(fixed_arr)
        }
        other => other.clone(),
    }
}

// Extract schema references helper function (used by both OpenAI and Anthropic)
fn extract_schema_refs(value: &serde_json::Value, refs: &mut std::collections::HashSet<String>) {
    match value {
        serde_json::Value::Object(obj) => {
            // Check for $ref
            if let Some(ref_value) = obj.get("$ref") {
                if let Some(ref_str) = ref_value.as_str() {
                    if let Some(type_name) = extract_type_name_from_ref(ref_str) {
                        refs.insert(type_name);
                    }
                }
            }

            // Recurse into all object values
            for (_, v) in obj {
                extract_schema_refs(v, refs);
            }
        }
        serde_json::Value::Array(arr) => {
            // Recurse into all array elements
            for item in arr {
                extract_schema_refs(item, refs);
            }
        }
        _ => {}
    }
}

fn extract_type_name_from_ref(ref_str: &str) -> Option<String> {
    // Extract type name from refs like "#/components/schemas/ChatCompletionRequestMessage"
    ref_str
        .rfind('/')
        .map(|last_slash| ref_str[last_slash + 1..].to_string())
}

fn generate_anthropic_specific_types(anthropic_spec: &str) {
    println!("🏗️  Using quicktype for Anthropic type generation...");

    // Extract Anthropic OpenAPI spec (YAML format from Stainless)
    let full_spec: serde_json::Value =
        serde_yaml::from_str(anthropic_spec).expect("Failed to parse Anthropic OpenAPI spec");

    // Generate types using quicktype approach
    match generate_anthropic_types_with_quicktype(
        &serde_json::to_string_pretty(&full_spec).unwrap(),
    ) {
        Ok(()) => {
            println!("✅ Anthropic types generated successfully with quicktype");
        }
        Err(e) => {
            println!("❌ Quicktype generation failed for Anthropic: {}", e);
            println!("📝 Falling back to minimal types");
            let _ = std::fs::write(
                "crates/lingua/src/providers/anthropic/generated.rs",
                "// Quicktype generation failed",
            );
        }
    }
}

fn generate_anthropic_types_with_quicktype(
    openapi_spec: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Parsing Anthropic OpenAPI spec...");

    let spec: serde_json::Value = serde_json::from_str(openapi_spec)?;

    // Extract essential Anthropic schemas for messages API
    let essential_schemas = create_essential_anthropic_schemas(&spec);

    println!("🏗️  Generating Anthropic types with quicktype...");

    // Create a temporary JSON schema file for quicktype
    let temp_schema_path = std::env::temp_dir().join("anthropic_schemas.json");
    let schema_json = serde_json::to_string_pretty(&essential_schemas)?;

    std::fs::write(&temp_schema_path, &schema_json)?;

    // Use quicktype to generate types - specify just one main type to avoid merging
    let output = std::process::Command::new("quicktype")
        .arg("--src-lang")
        .arg("schema")
        .arg("--lang")
        .arg("rust")
        .arg("--derive-debug")
        .arg("--derive-clone")
        .arg("--derive-partial-eq")
        .arg("--visibility")
        .arg("public")
        .arg("--density")
        .arg("dense")
        .arg(&temp_schema_path)
        .output();

    let quicktype_output = match output {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8(output.stdout)?
            } else {
                return Err(format!(
                    "quicktype failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }
        Err(e) => return Err(format!("Failed to run quicktype: {}", e).into()),
    };

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_schema_path);

    // Post-process the quicktype output
    let mut processed_output = post_process_quicktype_output_for_anthropic(&quicktype_output);

    // Quicktype generates a flat `pub struct Tool` with all fields optional, but the
    // OpenAPI spec defines Tool as a discriminated union (oneOf with a "type" tag).
    // We replace quicktype's struct with a proper Rust enum using #[serde(tag = "type")]
    // so each tool variant (custom, computer, text_editor, etc.) serializes correctly
    // with its specific fields.
    if let Ok(tool_code) = generate_all_tool_code("anthropic", &spec) {
        processed_output = replace_tool_struct_with_enum(&processed_output, &tool_code);
    }

    let dest_path = "crates/lingua/src/providers/anthropic/generated.rs";

    // Create directory if needed
    if let Some(parent) = std::path::Path::new(dest_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write generated types
    std::fs::write(dest_path, &processed_output)?;

    // Format with cargo fmt
    let _ = std::process::Command::new("cargo")
        .args(["fmt", "--", dest_path])
        .output();

    println!("📝 Generated Anthropic types to: {}", dest_path);

    Ok(())
}

fn create_essential_anthropic_schemas(spec: &serde_json::Value) -> serde_json::Value {
    // Automated approach: Preprocess schema to separate request/response types
    preprocess_anthropic_schema_for_separation(spec)
}

fn preprocess_anthropic_schema_for_separation(spec: &serde_json::Value) -> serde_json::Value {
    println!("🔧 Preprocessing Anthropic schema for request/response separation...");

    let default_map = serde_json::Map::new();
    let all_schemas = spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
        .unwrap_or(&default_map);

    // Use stable schemas (not Beta) - Beta schemas introduce breaking structural changes
    // (new required fields on content blocks). Beta-only fields like `strict` are added
    // manually to tool structs in tool_generator.rs instead.
    let request_schemas = vec!["CreateMessageParams".to_string()];
    let response_schemas = vec!["Message".to_string()];

    println!(
        "🔍 Identified {} request schemas, {} response schemas",
        request_schemas.len(),
        response_schemas.len()
    );

    let mut separated_schemas = serde_json::Map::new();

    // Recursively add all dependencies for the schemas.
    // Tool schemas will be pulled in automatically via $ref links from CreateMessageParams.
    for schema_name in &request_schemas {
        add_dependencies_recursively(schema_name, all_schemas, &mut separated_schemas);
    }
    for schema_name in &response_schemas {
        add_dependencies_recursively(schema_name, all_schemas, &mut separated_schemas);
    }

    // Clean the main request/response schemas to remove conflicting fields
    for schema_name in &request_schemas {
        if let Some(schema) = separated_schemas.get(schema_name) {
            let cleaned_schema = remove_response_fields_from_schema(schema);
            separated_schemas.insert(schema_name.clone(), cleaned_schema);
        }
    }

    for schema_name in &response_schemas {
        if let Some(schema) = separated_schemas.get(schema_name) {
            let cleaned_schema = remove_request_fields_from_schema(schema);
            separated_schemas.insert(schema_name.clone(), cleaned_schema);
        }
    }

    // Step 5: Create root schema with separated types
    let root_schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "oneOf": [
            {
                "title": "RequestType",
                "type": "object",
                "properties": {
                    "request": {"$ref": "#/definitions/CreateMessageParams"}
                }
            },
            {
                "title": "ResponseType",
                "type": "object",
                "properties": {
                    "response": {"$ref": "#/definitions/Message"}
                }
            },
        ],
        "definitions": separated_schemas
    });

    root_schema
}

fn remove_response_fields_from_schema(schema: &serde_json::Value) -> serde_json::Value {
    let mut cleaned_schema = schema.clone();

    // Fields that should NOT be in request schemas
    let response_only_fields = [
        "id",
        "created",
        "choices",
        "usage",
        "system_fingerprint",
        "content",
        "role",
        "stop_reason",
        "stop_sequence",
        "type",
    ];

    if let Some(properties) = cleaned_schema.get_mut("properties") {
        if let Some(props_obj) = properties.as_object_mut() {
            for field_name in &response_only_fields {
                props_obj.remove(*field_name);
            }
        }
    }

    // Remove response fields from required array
    if let Some(required) = cleaned_schema.get_mut("required") {
        if let Some(required_array) = required.as_array_mut() {
            required_array.retain(|item| {
                if let Some(field_name) = item.as_str() {
                    !response_only_fields.contains(&field_name)
                } else {
                    true
                }
            });
        }
    }

    cleaned_schema
}

fn remove_request_fields_from_schema(schema: &serde_json::Value) -> serde_json::Value {
    let mut cleaned_schema = schema.clone();

    // Fields that should NOT be in response schemas
    let request_only_fields = [
        "messages",
        "max_tokens",
        "temperature",
        "top_p",
        "top_k",
        "stream",
        "stop_sequences",
        "system",
        "tools",
        "tool_choice",
        "frequency_penalty",
        "presence_penalty",
        "logit_bias",
        "user",
    ];

    if let Some(properties) = cleaned_schema.get_mut("properties") {
        if let Some(props_obj) = properties.as_object_mut() {
            for field_name in &request_only_fields {
                props_obj.remove(*field_name);
            }
        }
    }

    // Remove request fields from required array
    if let Some(required) = cleaned_schema.get_mut("required") {
        if let Some(required_array) = required.as_array_mut() {
            required_array.retain(|item| {
                if let Some(field_name) = item.as_str() {
                    !request_only_fields.contains(&field_name)
                } else {
                    true
                }
            });
        }
    }

    cleaned_schema
}

fn add_dependencies_recursively(
    schema_name: &str,
    all_schemas: &serde_json::Map<String, serde_json::Value>,
    separated_schemas: &mut serde_json::Map<String, serde_json::Value>,
) {
    // Skip if already processed
    if separated_schemas.contains_key(schema_name) {
        return;
    }

    // Add the schema itself
    if let Some(schema) = all_schemas.get(schema_name) {
        let fixed_schema = fix_anthropic_schema_refs(schema);
        separated_schemas.insert(schema_name.to_string(), fixed_schema.clone());

        // Find all references in this schema and recursively add them
        let mut refs = std::collections::HashSet::new();
        extract_schema_refs(&fixed_schema, &mut refs);

        for ref_name in refs {
            add_dependencies_recursively(&ref_name, all_schemas, separated_schemas);
        }
    }
}

fn fix_anthropic_schema_refs(schema: &serde_json::Value) -> serde_json::Value {
    match schema {
        serde_json::Value::Object(obj) => {
            let mut fixed_obj = serde_json::Map::new();

            for (key, value) in obj {
                if key == "$ref" {
                    if let Some(ref_str) = value.as_str() {
                        // Fix the reference path
                        if ref_str.starts_with("#/components/schemas/") {
                            let new_ref =
                                ref_str.replace("#/components/schemas/", "#/definitions/");
                            fixed_obj.insert(key.clone(), serde_json::Value::String(new_ref));
                        } else {
                            fixed_obj.insert(key.clone(), value.clone());
                        }
                    } else {
                        fixed_obj.insert(key.clone(), value.clone());
                    }
                } else {
                    let fixed_value = fix_anthropic_schema_refs(value);

                    // Handle null type issue for quicktype
                    if key == "type" && fixed_value.is_null() {
                        if obj.get("enum").is_some() {
                            fixed_obj.insert(
                                key.clone(),
                                serde_json::Value::String("string".to_string()),
                            );
                        } else if obj.get("anyOf").is_some() || obj.get("oneOf").is_some() {
                            continue; // Skip null type for union types
                        } else {
                            fixed_obj.insert(
                                key.clone(),
                                serde_json::Value::String("object".to_string()),
                            );
                        }
                    } else {
                        fixed_obj.insert(key.clone(), fixed_value);
                    }
                }
            }

            serde_json::Value::Object(fixed_obj)
        }
        serde_json::Value::Array(arr) => {
            let fixed_arr: Vec<serde_json::Value> =
                arr.iter().map(fix_anthropic_schema_refs).collect();
            serde_json::Value::Array(fixed_arr)
        }
        other => other.clone(),
    }
}

/// Ensures serde_json imports are present after the last use statement
/// This handles the case where imports need to be added after header prepending
fn ensure_serde_json_imports(content: &str) -> String {
    // Check if imports already exist
    if content.contains("use crate::serde_json;") {
        return content.to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines = Vec::new();
    let mut imports_added = false;

    for (i, line) in lines.iter().enumerate() {
        new_lines.push(line.to_string());

        // Add serde_json imports after the last use statement
        if !imports_added && line.starts_with("use ") && !line.contains("crate::serde_json") {
            // Check if next line is also a use statement (not a comment or blank)
            let next_is_use = lines
                .get(i + 1)
                .map(|l| l.trim_start().starts_with("use "))
                .unwrap_or(false);

            if !next_is_use {
                // This is the last use statement, add serde_json module import
                // Note: We only import the module, not Value specifically, to avoid name conflicts
                // with provider-defined Value types
                new_lines.push("use crate::serde_json;".to_string());
                imports_added = true;
            }
        }
    }

    new_lines.join("\n")
}

fn post_process_quicktype_output_for_anthropic(quicktype_output: &str) -> String {
    let mut processed = quicktype_output.to_string();

    // Add ts-rs import
    let lines: Vec<&str> = processed.lines().collect();
    let mut new_lines = Vec::new();
    let mut ts_import_added = false;

    for (i, line) in lines.iter().enumerate() {
        new_lines.push(line.to_string());

        // Add ts-rs import after the last use statement
        if !ts_import_added && line.starts_with("use ") {
            // Check if next line is also a use statement
            let next_is_use = lines
                .get(i + 1)
                .map(|l| l.starts_with("use "))
                .unwrap_or(false);
            if !next_is_use {
                // This is the last use statement, add ts-rs import
                new_lines.push("use ts_rs::TS;".to_string());
                // Note: serde_json imports are added later by ensure_serde_json_imports()
                ts_import_added = true;
            }
        }
    }
    processed = new_lines.join("\n");

    // Add proper header with clippy allows for generated code
    processed = format!(
        "// Generated Anthropic types using quicktype\n// Essential types for Elmir Anthropic integration\n#![allow(non_camel_case_types)]\n#![allow(clippy::large_enum_variant)]\n#![allow(clippy::doc_lazy_continuation)]\n\n{}",
        processed
    );

    // Ensure serde_json imports are present after the header
    // This fixes the import location after header prepending
    processed = ensure_serde_json_imports(&processed);

    // Add TS derive to all structs and enums
    processed = processed.replace(
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]",
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]",
    );

    // Add export_to path to all derived TS types so they export to the correct subdirectory
    // Note that we add this to *all* derived TS types so that any transitive dependencies
    // that get exported also land in this directory
    processed = add_export_path_to_all_ts_types(&processed, "anthropic/");

    // Add ts-rs type annotations for serde_json types to generate better TypeScript
    // This must happen AFTER we add the TS derives and export_to
    processed = add_ts_type_annotations(&processed);

    // Only export entry point types that are actually used in our public API
    // ts-rs will automatically export their transitive dependencies to the same directory
    let entry_points = vec![
        "InputMessage", // Used by linguaToAnthropicMessages
    ];
    processed = add_ts_export_to_types(&processed, &entry_points, "anthropic/");

    // Fix HashMap to serde_json::Map for proper JavaScript object serialization
    // This ensures that JSON objects serialize to plain JS objects {} instead of Maps
    processed = processed.replace(
        "HashMap<String, Option<serde_json::Value>>",
        "serde_json::Map<String, serde_json::Value>",
    );
    processed = processed.replace(
        "HashMap<String, serde_json::Value>",
        "serde_json::Map<String, serde_json::Value>",
    );
    // Remove HashMap import if it's no longer needed
    processed = processed.replace("use std::collections::HashMap;\n", "");

    // Fix specific type mappings that quicktype might miss - be very specific to avoid over-replacement
    // Only replace serde_json::Value in error_code fields, not in general input/properties fields
    processed = processed.replace(
        "pub error_code: serde_json::Value",
        "pub error_code: WebSearchToolResultErrorCode",
    );

    // Ensure proper serde attributes for discriminated unions
    if processed.contains("ContentBlock") && processed.contains("#[derive(") {
        processed = processed.replace(
            "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\npub enum ContentBlock",
            "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(tag = \"type\")]\npub enum ContentBlock"
        );
    }

    // Add serde skip_serializing_if for Optional fields
    processed = add_serde_skip_if_none(&processed);

    processed
}

fn post_process_quicktype_output_for_openai(quicktype_output: &str) -> String {
    let mut processed = quicktype_output.to_string();

    // Add ts-rs import
    let lines: Vec<&str> = processed.lines().collect();
    let mut new_lines = Vec::new();
    let mut ts_import_added = false;

    for (i, line) in lines.iter().enumerate() {
        new_lines.push(line.to_string());

        // Add ts-rs import after the last use statement
        if !ts_import_added && line.starts_with("use ") {
            // Check if next line is also a use statement
            let next_is_use = lines
                .get(i + 1)
                .map(|l| l.starts_with("use "))
                .unwrap_or(false);
            if !next_is_use {
                // This is the last use statement, add ts-rs import
                new_lines.push("use ts_rs::TS;".to_string());
                ts_import_added = true;
            }
        }
    }
    processed = new_lines.join("\n");

    // Add proper header with clippy allows for generated code
    processed = format!(
        "// Generated OpenAI types using quicktype\n// Essential types for Elmir OpenAI integration\n#![allow(clippy::large_enum_variant)]\n#![allow(clippy::doc_lazy_continuation)]\n\n{}",
        processed
    );

    // Ensure serde_json imports are present after the header
    // This fixes the import location after header prepending
    processed = ensure_serde_json_imports(&processed);

    // Add TS derive to all structs and enums
    processed = processed.replace(
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]",
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]",
    );

    // Add export_to path to all derived TS types so they export to the correct subdirectory
    // Note that we add this to *all* derived TS types so that any transitive dependencies
    // that get exported also land in this directory
    processed = add_export_path_to_all_ts_types(&processed, "openai/");

    // Add ts-rs type annotations for serde_json types to generate better TypeScript
    // This must happen AFTER we add the TS derives and export_to
    processed = add_ts_type_annotations(&processed);

    // Only export entry point types that are actually used in our public API
    // ts-rs will automatically export their transitive dependencies to the same directory
    let entry_points = vec![
        "ChatCompletionRequestMessage", // Used by linguaToChatCompletionsMessages
        "InputItem",                    // Used by linguaToResponsesMessages
    ];
    processed = add_ts_export_to_types(&processed, &entry_points, "openai/");

    // Fix doctest JSON examples that fail to compile
    processed = processed.replace(
        "    /// ```\n    /// [\n    /// { x: 100, y: 200 },\n    /// { x: 200, y: 300 }\n    /// ]",
        "    /// ```json\n    /// [\n    /// { \"x\": 100, \"y\": 200 },\n    /// { \"x\": 200, \"y\": 300 }\n    /// ]"
    );

    // Add serde skip_serializing_if for Optional fields
    processed = add_serde_skip_if_none(&processed);

    // Fix any specific type mappings that quicktype might miss for OpenAI
    // Fix call_id fields that quicktype incorrectly generates as serde_json::Value
    processed = processed.replace(
        "pub call_id: Option<serde_json::Value>,",
        "pub call_id: Option<String>,",
    );

    // Fix output field that quicktype incorrectly generates as Refusal instead of String
    // This is specific to function call outputs where output should be a plain string
    processed = processed.replace(
        "pub output: Option<Refusal>,",
        "pub output: Option<String>,",
    );

    // Add automatic rename_all for enums that need consistent snake_case naming
    processed = processed.replace(
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\npub enum InputItemType {",
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(rename_all = \"snake_case\")]\npub enum InputItemType {"
    );

    processed = processed.replace(
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\npub enum OutputItemType {",
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(rename_all = \"snake_case\")]\npub enum OutputItemType {"
    );

    // Make output_item_type field optional to handle cases where original data doesn't have it
    processed = processed.replace(
        "pub output_item_type: OutputItemType,",
        "#[serde(skip_serializing_if = \"Option::is_none\")]\n    pub output_item_type: Option<OutputItemType>,"
    );

    processed
}

/// Add #[ts(export_to = "...")] to all TS-enabled type definitions
fn add_export_path_to_all_ts_types(content: &str, export_path: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines = Vec::new();
    let mut pending_export_to: Option<&str> = None;
    let mut has_ts_export_to = false;

    for line in lines {
        // Step 1: Check if this line has #[derive(..., TS)]
        if line.contains("#[derive(") && line.contains("TS") {
            pending_export_to = Some(export_path);
            has_ts_export_to = false; // Reset for new type
        }

        // Step 2: Check if there's already a ts(export_to) attribute
        if line.contains("#[ts(export_to") || line.contains("#[ts(export,") {
            has_ts_export_to = true;
        }

        // Step 3: Check if we hit a type definition
        if line.starts_with("pub struct ") || line.starts_with("pub enum ") {
            if let Some(path) = pending_export_to.take() {
                // Step 4: Only add export_to if not already present
                if !has_ts_export_to {
                    result_lines.push(format!("#[ts(export_to = \"{}\")]", path));
                }
            }
            has_ts_export_to = false; // Reset for next type
        }

        result_lines.push(line.to_string());
    }

    result_lines.join("\n")
}

/// Add #[ts(export, export_to = "...")] to specific type definitions
/// Replaces the existing export_to attribute with one that has the export flag
fn add_ts_export_to_types(content: &str, type_names: &[&str], export_dir: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines = Vec::new();
    let mut export_to_line_index: Option<usize> = None;

    for line in lines {
        // Step 1: Track when we see #[ts(export_to = "...")]
        if line.contains("#[ts(export_to =") {
            export_to_line_index = Some(result_lines.len());
        }

        // Step 3: Check if we hit a type definition
        if line.starts_with("pub struct ") || line.starts_with("pub enum ") {
            // Extract the type name
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let type_name = parts[2].trim_end_matches(" {").trim_end_matches('<');

                // Step 4: If this is an entry point type, replace the export_to line
                if type_names.contains(&type_name) {
                    if let Some(index) = export_to_line_index.take() {
                        // Replace the previous #[ts(export_to = "...")] with the export version
                        result_lines[index] =
                            format!("#[ts(export, export_to = \"{}\")]", export_dir);
                    }
                } else {
                    // Clear the flag - not an entry point
                    export_to_line_index = None;
                }
            }
        }

        result_lines.push(line.to_string());
    }

    result_lines.join("\n")
}

/// Add #[ts(type = "...")] annotations for serde_json types to generate better TypeScript
fn add_ts_type_annotations(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines = Vec::new();

    for i in 0..lines.len() {
        let line = lines[i];

        // Check if this line contains a pub field with serde_json types
        // Use the full line text, not split parts, since types can contain spaces
        if line.trim_start().starts_with("pub ") && line.ends_with(",") {
            // Check if the previous line already has a ts attribute
            let prev_line = if i > 0 { lines[i - 1].trim() } else { "" };
            let has_ts_attr = prev_line.starts_with("#[ts(");

            // Determine if we need to add ts annotation
            // Check the FULL line for serde_json::Value (handles complex generic types)
            let needs_ts_annotation = line.contains("serde_json::Value");

            if needs_ts_annotation {
                // Get the indentation level from the current line
                let indent = line.len() - line.trim_start().len();
                let ts_attr = format!("{}#[ts(type = \"unknown\")]", " ".repeat(indent));

                // Add the ts attribute BEFORE the field line unless it's already present
                if !has_ts_attr {
                    result_lines.push(ts_attr);
                }
            }
        }

        result_lines.push(line.to_string());
    }

    result_lines.join("\n")
}

/// Add #[serde(skip_serializing_if = "Option::is_none")] to all Option<T> fields
fn add_serde_skip_if_none(content: &str) -> String {
    let processed = content.to_string();

    // Use regex-like approach to find Option<T> fields and add serde attributes
    let lines: Vec<&str> = processed.lines().collect();
    let mut result_lines = Vec::new();

    for i in 0..lines.len() {
        let line = lines[i];
        result_lines.push(line.to_string());

        // Check if this line contains a pub field with Option<T> (but not nested Options)
        if line.trim_start().starts_with("pub ") && line.ends_with(",") {
            // More precise matching: field type must START with Option<
            let field_parts: Vec<&str> = line.split_whitespace().collect();
            if field_parts.len() >= 3 {
                let field_type = field_parts[2].trim_end_matches(',');
                if field_type.starts_with("Option<") {
                    // Check if the next line already has a serde attribute
                    let next_line = lines.get(i + 1).map(|l| l.trim()).unwrap_or("");
                    if !next_line.starts_with("#[serde(") && !line.contains("#[serde(") {
                        // Get the indentation level from the current line
                        let indent = line.len() - line.trim_start().len();
                        let serde_attr = format!(
                            "{}#[serde(skip_serializing_if = \"Option::is_none\")]",
                            " ".repeat(indent)
                        );

                        // Insert the serde attribute before the field
                        result_lines.insert(result_lines.len() - 1, serde_attr);
                    }
                }
            }
        }
    }

    result_lines.join("\n")
}

fn generate_google_protobuf_types_from_git() {
    let temp_dir = std::env::temp_dir().join("googleapis_clone");

    // Clean up any existing clone
    let _ = std::fs::remove_dir_all(&temp_dir);

    println!("📦 Cloning googleapis repository for complete protobuf definitions...");
    println!("📁 Using temporary directory: {:?}", temp_dir);

    // Clone the googleapis repository
    let clone_result = std::process::Command::new("git")
        .args([
            "clone",
            "--depth=1", // Shallow clone for faster download
            "https://github.com/googleapis/googleapis.git",
            temp_dir.to_str().unwrap(),
        ])
        .output();

    match clone_result {
        Ok(result) if result.status.success() => {
            println!("✅ Successfully cloned googleapis repository");
        }
        Ok(result) => {
            println!(
                "❌ Failed to clone googleapis: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            let _ = std::fs::remove_dir_all(&temp_dir);
            let _ = std::fs::write(
                "crates/lingua/src/providers/google/generated.rs",
                "// Git clone failed",
            );
            return;
        }
        Err(e) => {
            println!("❌ Error running git clone: {}", e);
            let _ = std::fs::remove_dir_all(&temp_dir);
            let _ = std::fs::write(
                "crates/lingua/src/providers/google/generated.rs",
                "// Git clone error",
            );
            return;
        }
    }

    // Now compile with complete dependency tree including google.type
    let proto_file = temp_dir.join("google/ai/generativelanguage/v1beta/generative_service.proto");
    let interval_proto = temp_dir.join("google/type/interval.proto");
    let latlng_proto = temp_dir.join("google/type/latlng.proto");

    if !proto_file.exists() {
        println!("❌ Could not find generative_service.proto in cloned repository");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::write(
            "crates/lingua/src/providers/google/generated.rs",
            "// Proto file not found",
        );
        return;
    }

    if !interval_proto.exists() {
        println!("❌ Could not find google/type/interval.proto in cloned repository");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::write(
            "crates/lingua/src/providers/google/generated.rs",
            "// Interval proto not found",
        );
        return;
    }

    if !latlng_proto.exists() {
        println!("❌ Could not find google/type/latlng.proto in cloned repository");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::write(
            "crates/lingua/src/providers/google/generated.rs",
            "// LatLng proto not found",
        );
        return;
    }

    println!("✅ Found protobuf files, compiling with complete dependencies...");

    // Include both the main service proto and google.type dependencies
    let proto_paths = vec![
        proto_file.to_string_lossy().to_string(),
        interval_proto.to_string_lossy().to_string(),
        latlng_proto.to_string_lossy().to_string(),
    ];

    generate_google_protobuf_types(&proto_paths, &temp_dir.to_string_lossy());

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn generate_google_protobuf_types(proto_paths: &[String], proto_dir: &str) {
    println!("🔨 Compiling protobuf files with prost-build...");

    // Create a temporary directory for generated types
    let temp_dir = std::env::temp_dir().join("google_generated");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
    let descriptor_path = temp_dir.join("google_descriptor.bin");

    // Configure prost-build
    let mut config = prost_build::Config::new();
    config.out_dir(&temp_dir);
    config.file_descriptor_set_path(&descriptor_path);
    config.compile_well_known_types();
    config.extern_path(".google.protobuf", "::pbjson_types");
    config.disable_comments([".google.api"]);

    // Include directories for resolving imports
    let include_dirs = vec![proto_dir];

    println!("📁 Include directories: {:?}", include_dirs);
    println!("📄 Proto files: {:?}", proto_paths);

    // Compile the protobuf files
    match config.compile_protos(proto_paths, &include_dirs) {
        Ok(()) => {
            println!("✅ Protobuf compilation successful");
            let pbjson_dir = generate_google_pbjson_types(&descriptor_path, &temp_dir);
            // Create a combined output file with the essential types (and serde support if present)
            create_google_combined_output(&temp_dir, pbjson_dir.as_deref());
        }
        Err(e) => {
            println!("❌ Protobuf compilation failed: {}", e);
            println!("📝 Falling back to empty types file");
            let _ = std::fs::write(
                "crates/lingua/src/providers/google/generated.rs",
                "// Protobuf generation failed",
            );
        }
    }

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn generate_google_pbjson_types(descriptor_path: &Path, temp_dir: &Path) -> Option<PathBuf> {
    let pbjson_dir = temp_dir.join("google_pbjson");
    let _ = std::fs::remove_dir_all(&pbjson_dir);
    if std::fs::create_dir_all(&pbjson_dir).is_err() {
        println!("⚠️  Failed to create pbjson output directory");
        return None;
    }

    println!("🔧 Generating pbjson serde implementations...");
    let descriptor_set = match std::fs::read(descriptor_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("⚠️  Failed to read protobuf descriptor set: {}", e);
            return None;
        }
    };

    let mut builder = pbjson_build::Builder::new();
    builder.out_dir(&pbjson_dir);
    builder.extern_path(".google.protobuf", "::pbjson_types");
    if let Err(e) = builder.register_descriptors(&descriptor_set) {
        println!("⚠️  Failed to register protobuf descriptors: {}", e);
        return None;
    }

    if let Err(e) = builder.build(&[".google.ai.generativelanguage.v1beta", ".google.type"]) {
        println!("⚠️  Failed to generate pbjson serde output: {}", e);
        return None;
    }

    Some(pbjson_dir)
}

fn create_google_combined_output(temp_dir: &std::path::Path, pbjson_dir: Option<&Path>) {
    println!("🔧 Creating combined Google types output...");

    // Look for generated files in the temp directory
    let mut generated_files = std::fs::read_dir(temp_dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    println!("📁 Found {} generated files", generated_files.len());

    let mut all_content = String::new();
    all_content.push_str("// Generated Google AI types from official protobuf files\n");
    all_content.push_str("// Essential types for Elmir Google AI integration\n\n");
    all_content.push_str("// This file is @generated by prost-build.\n");
    all_content.push_str("#![allow(clippy::doc_lazy_continuation)]\n");
    all_content.push_str("#![allow(clippy::doc_overindented_list_items)]\n");
    all_content.push_str("#![allow(clippy::large_enum_variant)]\n");

    generated_files.sort_by_key(|entry| entry.file_name());

    for file_entry in &generated_files {
        if let Ok(content) = std::fs::read_to_string(file_entry.path()) {
            // Fix problematic type references that prost-build generates incorrectly
            let fixed_content = fix_google_type_references(content);
            all_content.push_str(&fixed_content);
            all_content.push('\n');
        }
    }

    // If we didn't get much content, fall back to minimal file
    if all_content.len() < 500 {
        println!("⚠️  Generated content too small, falling back to minimal file");
        let _ = std::fs::write(
            "crates/lingua/src/providers/google/generated.rs",
            "// Protobuf generation incomplete",
        );
        return;
    }

    let dest_path = "crates/lingua/src/providers/google/generated.rs";
    let pbjson_dest_path = "crates/lingua/src/providers/google/generated_pbjson.rs";

    // Create the directory if it doesn't exist
    if let Some(parent) = Path::new(dest_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Write the pbjson serde implementations to a separate file
    let mut pbjson_content = String::new();
    if let Some(pbjson_dir) = pbjson_dir {
        let mut pbjson_files = std::fs::read_dir(pbjson_dir)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs")
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        pbjson_files.sort_by_key(|entry| entry.file_name());

        for file_entry in pbjson_files {
            if let Ok(content) = std::fs::read_to_string(file_entry.path()) {
                pbjson_content.push_str(&content);
                pbjson_content.push('\n');
            }
        }
    }

    if pbjson_content.is_empty() {
        pbjson_content.push_str("// pbjson serde output unavailable\n");
    }

    if let Some(parent) = Path::new(pbjson_dest_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(pbjson_dest_path, &pbjson_content);

    // Link the pbjson output from the main generated file
    all_content.push_str("\n// Serde support generated by pbjson-build\n");
    all_content.push_str("include!(\"generated_pbjson.rs\");\n");

    // Write the combined types
    if std::fs::write(dest_path, &all_content).is_ok() {
        println!("📝 Generated Google protobuf types to: {}", dest_path);

        // Format the file with cargo fmt
        let _ = std::process::Command::new("cargo")
            .args(["fmt", "--", dest_path])
            .output();

        println!("✅ Google protobuf types generated and formatted");
    } else {
        println!("❌ Failed to write Google generated types");
        let _ = std::fs::write(
            "crates/lingua/src/providers/google/generated.rs",
            "// Protobuf write failed",
        );
    }
}

fn fix_google_type_references(content: String) -> String {
    // Fix the problematic google.type.Interval reference that prost-build generates incorrectly
    let mut fixed = content;

    // Remove the prost-generated @generated comment since we add our own header
    fixed = fixed.replace("// This file is @generated by prost-build.\n", "");

    // Fix malformed JSON in doctests that have escaped brackets.
    fixed = fixed.replace("\\[\"", "[\"");
    fixed = fixed.replace("\"\\]", "\"]");

    // Mark the JSON schema example as json to prevent Rust doctest compilation.
    if fixed.contains("\"type\": \"object\"") {
        fixed = fixed.replace(
            "    /// ```\n    /// {\n    ///    \"type\": \"object\",",
            "    /// ```json\n    /// {\n    ///    \"type\": \"object\",",
        );
    }

    // Replace incorrect paths to google.type types with absolute paths so they resolve
    // from any nested module in the combined output.
    fixed = fixed.replace(
        "super::super::super::super::r#type::Interval",
        "crate::providers::google::generated::Interval",
    );
    fixed = fixed.replace(
        "super::super::super::r#type::LatLng",
        "crate::providers::google::generated::LatLng",
    );

    fixed
}
