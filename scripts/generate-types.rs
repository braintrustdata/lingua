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
    let essential_types = [
        "CreateChatCompletionRequest",
        "CreateChatCompletionResponse",
        "CreateChatCompletionStreamResponse",
        "ChatCompletionRequestMessage",
        "ChatCompletionResponseMessage",
        "ChatCompletionTool",
        "CompletionUsage",
    ];

    let default_map = serde_json::Map::new();
    let all_schemas = spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
        .unwrap_or(&default_map);

    let mut essential_schemas = serde_json::Map::new();
    let mut processed = std::collections::HashSet::new();

    // Recursively add essential types and their dependencies
    for type_name in &essential_types {
        add_openai_schema_with_dependencies(
            type_name,
            all_schemas,
            &mut essential_schemas,
            &mut processed,
        );
    }

    // Fix all $ref paths to point to #/definitions/ instead of #/components/schemas/
    let mut fixed_schemas = serde_json::Map::new();
    for (name, schema) in essential_schemas {
        fixed_schemas.insert(name, fix_openai_schema_refs(&schema));
    }

    // Create a root schema that includes all our essential types
    // This approach works better with quicktype
    let root_schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "anyOf": essential_types.iter().map(|t| serde_json::json!({"$ref": format!("#/definitions/{}", t)})).collect::<Vec<_>>(),
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

fn create_essential_anthropic_schemas(all_schemas: &serde_json::Value) -> serde_json::Value {
    // Key Anthropic API types for messages endpoint
    let essential_types = [
        "InputMessage",
        "Message",
        "MessageRequest",
        "MessageResponse",
        "ContentBlock",
        "InputContentBlock",
        "TextBlock",
        "ImageBlock",
        "ToolUseBlock",
        "ToolResultBlock",
        "Tool",
        "ToolChoice",
        "Usage",
        "Metadata",
        "StopReason",
        "WebSearchToolResultError", // This should generate proper enum!
        "RequestWebSearchToolResultError",
    ];

    let default_map = serde_json::Map::new();
    let schemas = all_schemas
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
        .unwrap_or(&default_map);

    let mut essential_schemas = serde_json::Map::new();
    let mut processed = std::collections::HashSet::new();

    // Add essential types and recursively resolve their dependencies
    for type_name in &essential_types {
        add_schema_with_dependencies(type_name, schemas, &mut essential_schemas, &mut processed);
    }

    // Resolve $ref references
    let resolved_schemas = resolve_schema_refs(
        &serde_json::Value::Object(essential_schemas.clone()),
        schemas,
    );

    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": resolved_schemas.as_object().unwrap()
    })
}

fn add_schema_with_dependencies(
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
            add_schema_with_dependencies(&ref_name, all_schemas, essential_schemas, processed);
        }
    }
}

fn resolve_schema_refs(
    schema: &serde_json::Value,
    all_schemas: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    match schema {
        serde_json::Value::Object(obj) => {
            let mut resolved_obj = serde_json::Map::new();

            for (key, value) in obj {
                if key == "$ref" {
                    if let Some(ref_str) = value.as_str() {
                        if let Some(type_name) = extract_type_name_from_ref(ref_str) {
                            if let Some(resolved_schema) = all_schemas.get(&type_name) {
                                return resolve_schema_refs(resolved_schema, all_schemas);
                            }
                        }
                    }
                }

                let resolved_value = resolve_schema_refs(value, all_schemas);

                // Handle null type issue for quicktype
                if key == "type" && resolved_value.is_null() {
                    if obj.get("enum").is_some() {
                        resolved_obj
                            .insert(key.clone(), serde_json::Value::String("string".to_string()));
                    } else if obj.get("anyOf").is_some() || obj.get("oneOf").is_some() {
                        continue; // Skip null type for union types
                    } else {
                        resolved_obj
                            .insert(key.clone(), serde_json::Value::String("object".to_string()));
                    }
                } else {
                    resolved_obj.insert(key.clone(), resolved_value);
                }
            }

            serde_json::Value::Object(resolved_obj)
        }
        serde_json::Value::Array(arr) => {
            let resolved_arr: Vec<serde_json::Value> = arr
                .iter()
                .map(|item| resolve_schema_refs(item, all_schemas))
                .collect();
            serde_json::Value::Array(resolved_arr)
        }
        other => other.clone(),
    }
}

fn post_process_quicktype_output_for_anthropic(quicktype_output: &str) -> String {
    let mut processed = quicktype_output.to_string();

    // Add proper header
    processed = format!(
        "// Generated Anthropic types using quicktype\n// Essential types for Elmir Anthropic integration\n#![allow(non_camel_case_types)]\n\n{}",
        processed
    );

    // Fix specific type mappings that quicktype might miss
    processed = processed.replace("serde_json::Value", "WebSearchToolResultErrorCode");

    // Ensure proper serde attributes for discriminated unions
    if processed.contains("ContentBlock") && processed.contains("#[derive(") {
        processed = processed.replace(
            "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\npub enum ContentBlock",
            "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n#[serde(tag = \"type\")]\npub enum ContentBlock"
        );
    }

    processed
}

fn post_process_quicktype_output_for_openai(quicktype_output: &str) -> String {
    let mut processed = quicktype_output.to_string();

    // Add proper header
    processed = format!(
        "// Generated OpenAI types using quicktype\n// Essential types for Elmir OpenAI integration\n\n{}",
        processed
    );

    // Fix any specific type mappings that quicktype might miss for OpenAI
    // (Add any OpenAI-specific replacements here as needed)

    processed
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
