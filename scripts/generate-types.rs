#!/usr/bin/env cargo +nightly -Zscript
//! ```cargo
//! [dependencies]
//! serde_json = "1.0"
//! serde_yaml = "0.9"
//! prost-build = "0.13"
//! ```

//! Standalone type generation script for Elmir providers
//!
//! Usage: cargo run --bin generate-types -- [provider]
//!        ./scripts/generate-types.rs [provider]

use std::path::Path;

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

    println!("🔍 Parsing JSON OpenAPI spec...");

    let schema: serde_json::Value = match serde_json::from_str(&anthropic_spec) {
        Ok(value) => value,
        Err(e) => {
            println!("❌ Failed to parse Anthropic OpenAPI spec as JSON: {}", e);
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
                "src/providers/openai/generated.rs",
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
    let processed_output = post_process_quicktype_output_for_openai(&quicktype_output);

    let dest_path = "src/providers/openai/generated.rs";

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

    // Extract Anthropic OpenAPI spec
    let full_spec: serde_json::Value =
        serde_json::from_str(anthropic_spec).expect("Failed to parse Anthropic OpenAPI spec");

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
                "src/providers/anthropic/generated.rs",
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
    let processed_output = post_process_quicktype_output_for_anthropic(&quicktype_output);

    let dest_path = "src/providers/anthropic/generated.rs";

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

    // Step 1: Analyze endpoints to identify request vs response schemas
    let (request_schemas, response_schemas) = analyze_anthropic_endpoints(spec);

    println!(
        "🔍 Identified {} request schemas, {} response schemas",
        request_schemas.len(),
        response_schemas.len()
    );

    let mut separated_schemas = serde_json::Map::new();

    // Step 2: First recursively add all dependencies for the original schemas
    for schema_name in &request_schemas {
        add_dependencies_recursively(schema_name, all_schemas, &mut separated_schemas);
    }
    for schema_name in &response_schemas {
        add_dependencies_recursively(schema_name, all_schemas, &mut separated_schemas);
    }

    // All other types will be included automatically through dependency resolution

    // Step 3: Now clean the main request/response schemas to remove conflicting fields
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
    // Use a different approach: create separate top-level object types to avoid merging
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

fn analyze_anthropic_endpoints(spec: &serde_json::Value) -> (Vec<String>, Vec<String>) {
    let mut request_schemas = Vec::new();
    let mut response_schemas = Vec::new();

    // Analyze the /v1/messages endpoint
    if let Some(paths) = spec.get("paths") {
        if let Some(messages_path) = paths.get("/v1/messages") {
            if let Some(post_op) = messages_path.get("post") {
                // Extract request schema from requestBody
                if let Some(request_body) = post_op.get("requestBody") {
                    if let Some(content) = request_body.get("content") {
                        if let Some(json_content) = content.get("application/json") {
                            if let Some(schema) = json_content.get("schema") {
                                if let Some(schema_ref) = schema.get("$ref") {
                                    if let Some(schema_name) = extract_schema_name_from_ref(
                                        schema_ref.as_str().unwrap_or(""),
                                    ) {
                                        request_schemas.push(schema_name);
                                    }
                                }
                            }
                        }
                    }
                }

                // Extract response schemas from responses
                if let Some(responses) = post_op.get("responses") {
                    if let Some(success_response) = responses.get("200") {
                        if let Some(content) = success_response.get("content") {
                            if let Some(json_content) = content.get("application/json") {
                                if let Some(schema) = json_content.get("schema") {
                                    if let Some(schema_ref) = schema.get("$ref") {
                                        if let Some(schema_name) = extract_schema_name_from_ref(
                                            schema_ref.as_str().unwrap_or(""),
                                        ) {
                                            response_schemas.push(schema_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("🔍 Found request schemas: {:?}", request_schemas);
    println!("🔍 Found response schemas: {:?}", response_schemas);

    (request_schemas, response_schemas)
}

fn extract_schema_name_from_ref(ref_str: &str) -> Option<String> {
    // Extract schema name from "#/components/schemas/CreateMessageParams"
    ref_str
        .rfind('/')
        .map(|last_slash| ref_str[last_slash + 1..].to_string())
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

fn post_process_quicktype_output_for_anthropic(quicktype_output: &str) -> String {
    let mut processed = quicktype_output.to_string();

    // Add proper header with clippy allows for generated code
    processed = format!(
        "// Generated Anthropic types using quicktype\n// Essential types for Elmir Anthropic integration\n#![allow(non_camel_case_types)]\n#![allow(clippy::large_enum_variant)]\n#![allow(clippy::doc_lazy_continuation)]\n\n{}",
        processed
    );

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

    // Add proper header with clippy allows for generated code
    processed = format!(
        "// Generated OpenAI types using quicktype\n// Essential types for Elmir OpenAI integration\n#![allow(clippy::large_enum_variant)]\n#![allow(clippy::doc_lazy_continuation)]\n\n{}",
        processed
    );

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
            let _ = std::fs::write("src/providers/google/generated.rs", "// Git clone failed");
            return;
        }
        Err(e) => {
            println!("❌ Error running git clone: {}", e);
            let _ = std::fs::remove_dir_all(&temp_dir);
            let _ = std::fs::write("src/providers/google/generated.rs", "// Git clone error");
            return;
        }
    }

    // Now compile with complete dependency tree including google.type
    let proto_file = temp_dir.join("google/ai/generativelanguage/v1beta/generative_service.proto");
    let interval_proto = temp_dir.join("google/type/interval.proto");

    if !proto_file.exists() {
        println!("❌ Could not find generative_service.proto in cloned repository");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::write(
            "src/providers/google/generated.rs",
            "// Proto file not found",
        );
        return;
    }

    if !interval_proto.exists() {
        println!("❌ Could not find google/type/interval.proto in cloned repository");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::write(
            "src/providers/google/generated.rs",
            "// Interval proto not found",
        );
        return;
    }

    println!("✅ Found protobuf files, compiling with complete dependencies...");

    // Include both the main service proto and the interval type proto
    let proto_paths = vec![
        proto_file.to_string_lossy().to_string(),
        interval_proto.to_string_lossy().to_string(),
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

    // Configure prost-build
    let mut config = prost_build::Config::new();
    config.out_dir(&temp_dir);

    // Include directories for resolving imports
    let include_dirs = vec![proto_dir];

    println!("📁 Include directories: {:?}", include_dirs);
    println!("📄 Proto files: {:?}", proto_paths);

    // Compile the protobuf files
    match config.compile_protos(proto_paths, &include_dirs) {
        Ok(()) => {
            println!("✅ Protobuf compilation successful");

            // Read the generated mod.rs file
            let mod_file_path = temp_dir.join("mod.rs");
            match std::fs::read_to_string(&mod_file_path) {
                Ok(mod_content) => {
                    println!(
                        "📋 Generated modules: {:?}",
                        mod_content.lines().take(10).collect::<Vec<_>>()
                    );

                    // Create a combined output file with the essential types
                    create_google_combined_output(&temp_dir);
                }
                Err(e) => {
                    println!("❌ Failed to read generated mod.rs: {}", e);
                    let _ = std::fs::write(
                        "src/providers/google/generated.rs",
                        "// Protobuf generation failed",
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ Protobuf compilation failed: {}", e);
            println!("📝 Falling back to empty types file");
            let _ = std::fs::write(
                "src/providers/google/generated.rs",
                "// Protobuf generation failed",
            );
        }
    }

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn create_google_combined_output(temp_dir: &std::path::Path) {
    println!("🔧 Creating combined Google types output...");

    // Look for generated files in the temp directory
    let generated_files = std::fs::read_dir(temp_dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    println!("📁 Found {} generated files", generated_files.len());

    // Read the main generated file
    let mut all_content = String::new();
    all_content.push_str("// Generated Google AI types from official protobuf files\n");
    all_content.push_str("// Essential types for Elmir Google AI integration\n\n");
    all_content.push_str("// This file is @generated by prost-build.\n");
    all_content.push_str("#![allow(clippy::doc_lazy_continuation)]\n");
    all_content.push_str("#![allow(clippy::doc_overindented_list_items)]\n");
    all_content.push_str("#![allow(clippy::large_enum_variant)]\n");

    // Find the main generated file (should be the Google AI one)
    let main_file = generated_files.iter().find(|entry| {
        entry
            .file_name()
            .to_str()
            .map(|name| name.contains("google.ai.generativelanguage.v1beta"))
            .unwrap_or(false)
    });

    if let Some(main_file) = main_file {
        if let Ok(content) = std::fs::read_to_string(main_file.path()) {
            // Fix problematic type references that prost-build generates incorrectly
            let fixed_content = fix_google_type_references(content);
            all_content.push_str(&fixed_content);
        }
    } else {
        println!("⚠️  No Google AI generative language file found, checking all files");
        for file_entry in generated_files {
            if let Ok(content) = std::fs::read_to_string(file_entry.path()) {
                if content.contains("GenerateContentRequest") || content.contains("Content") {
                    println!("📄 Adding content from: {:?}", file_entry.file_name());
                    let fixed_content = fix_google_type_references(content);
                    all_content.push_str(&fixed_content);
                    all_content.push('\n');
                }
            }
        }
    }

    // If we didn't get much content, fall back to minimal file
    if all_content.len() < 500 {
        println!("⚠️  Generated content too small, falling back to minimal file");
        let _ = std::fs::write(
            "src/providers/google/generated.rs",
            "// Protobuf generation incomplete",
        );
        return;
    }

    let dest_path = "src/providers/google/generated.rs";

    // Create the directory if it doesn't exist
    if let Some(parent) = Path::new(dest_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

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
            "src/providers/google/generated.rs",
            "// Protobuf write failed",
        );
    }
}

fn fix_google_type_references(content: String) -> String {
    // Fix the problematic google.type.Interval reference that prost-build generates incorrectly
    let mut fixed = content;

    // Remove the prost-generated @generated comment since we add our own header
    fixed = fixed.replace("// This file is @generated by prost-build.\n", "");

    // Fix malformed JSON in doctests that have escaped brackets
    // This fixes doctest compilation errors where \["foo"\] should be ["foo"]
    fixed = fixed.replace("\\[\"", "[\"");
    fixed = fixed.replace("\"\\]", "\"]");

    // Fix doctests that contain JSON by marking them as non-executable
    // Replace ``` with ```json to prevent Rust compilation of JSON examples
    if fixed.contains("\"type\": \"object\"") {
        // This is a JSON schema example, mark it as json to avoid Rust doctest compilation
        fixed = fixed.replace(
            "    /// ```\n    /// {\n    ///    \"type\": \"object\",",
            "    /// ```json\n    /// {\n    ///    \"type\": \"object\",",
        );
    }

    // Replace the incorrect super::super::super::super::r#type::Interval reference
    fixed = fixed.replace(
        "super::super::super::super::r#type::Interval",
        "TimeRangeFilter",
    );

    let has_time_range_filter_ref =
        fixed.contains("time_range_filter") && fixed.contains("TimeRangeFilter");
    let has_time_range_filter_def = fixed.contains("pub struct TimeRangeFilter");

    // If we have a TimeRangeFilter reference but no definition, add it to the tool module
    if has_time_range_filter_ref && !has_time_range_filter_def {
        // Find the GoogleSearch struct and add TimeRangeFilter definition right after it
        if let Some(google_search_pos) = fixed.find("pub struct GoogleSearch {") {
            // Find the end of the GoogleSearch struct
            if let Some(struct_start) = fixed[..google_search_pos].rfind("#[derive(") {
                let after_struct_start = &fixed[struct_start..];
                if let Some(struct_end) = after_struct_start.find("\n    }") {
                    let insert_pos = struct_start + struct_end + 6; // After "\n    }"

                    let before = &fixed[..insert_pos];
                    let after = &fixed[insert_pos..];

                    let time_range_filter_def = r#"

    /// Simple placeholder for TimeRangeFilter until google.type module is properly included
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct TimeRangeFilter {
        #[prost(string, optional, tag = "1")]
        pub start_time: ::core::option::Option<::prost::alloc::string::String>,
        #[prost(string, optional, tag = "2")]
        pub end_time: ::core::option::Option<::prost::alloc::string::String>,
    }"#;

                    fixed = format!("{}{}{}", before, time_range_filter_def, after);
                }
            }
        }

        // Also need to remove Copy trait from all structs that contain non-Copy fields
        fixed = fixed.replace(
            "#[derive(Clone, Copy, PartialEq, ::prost::Message)]",
            "#[derive(Clone, PartialEq, ::prost::Message)]",
        );
        fixed = fixed.replace(
            "#[derive(Clone, Copy, PartialEq, ::prost::Oneof)]",
            "#[derive(Clone, PartialEq, ::prost::Oneof)]",
        );
    }

    fixed
}
