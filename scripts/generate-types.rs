#!/usr/bin/env cargo +nightly -Zscript
//! Standalone type generation script for LLMIR providers
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
        "google" => generate_google_types(),
        "all" => {
            generate_openai_types();
            generate_anthropic_types();
            generate_google_types();
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
    println!("📦 Generating OpenAI types from OpenAPI spec...");

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

    if let Some(schemas) = schemas {
        println!("✅ Found components/schemas section");

        // Generate essential OpenAI types for chat completion APIs
        println!("🏗️  Generating essential OpenAI types for chat completions");
        generate_openai_specific_types(schemas);
    } else {
        println!("❌ No components/schemas section found in OpenAPI spec");
    }
}

fn generate_anthropic_types() {
    println!("📦 Generating Anthropic types from OpenAPI spec...");

    let spec_file_path = "specs/anthropic/openapi.json";

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

    if let Some(schemas) = schemas {
        println!("✅ Found Anthropic components/schemas section");

        // Generate essential Anthropic types for messages API
        println!("🏗️  Generating essential Anthropic types for messages API");
        generate_anthropic_specific_types(schemas);
    } else {
        println!("❌ No components/schemas section found in Anthropic OpenAPI spec");
    }
}

fn generate_google_types() {
    println!("📦 Generating Google types from protobuf files...");

    let proto_dir = "specs/google/protos";

    // Check if protobuf files exist
    if !std::path::Path::new(proto_dir).exists() {
        println!("❌ Google protobuf files not found at {}", proto_dir);
        println!(
            "Run './pipelines/generate-provider-types.sh google' to download protobuf files first"
        );
        return;
    }

    // Essential protobuf files for Google AI
    let proto_files = [
        "google/ai/generativelanguage/v1/generative_service.proto",
        "google/ai/generativelanguage/v1/content.proto",
        "google/ai/generativelanguage/v1/safety.proto",
        "google/ai/generativelanguage/v1/citation.proto",
    ];

    let proto_paths: Vec<String> = proto_files
        .iter()
        .map(|f| format!("{}/{}", proto_dir, f))
        .collect();

    // Check if all required proto files exist
    for path in &proto_paths {
        if !std::path::Path::new(path).exists() {
            println!("❌ Required proto file not found: {}", path);
            return;
        }
    }

    println!("✅ Found all required protobuf files, compiling...");

    // Generate protobuf types directly to src directory
    generate_google_protobuf_types(&proto_paths, proto_dir);
}

fn generate_openai_specific_types(schemas: &serde_json::Value) {
    use std::fs;

    // Focus only on essential chat completion types to minimize generated code
    let essential_types = [
        "CreateChatCompletionRequest",
        "CreateChatCompletionResponse",
        "CreateChatCompletionStreamResponse",
        "ChatCompletionRequestMessage",
        "ChatCompletionResponseMessage",
        "ChatCompletionTool",
        "ChatCompletionChoice",
        "CompletionUsage",
    ];

    let mut generated_types = Vec::new();

    for type_name in essential_types {
        if let Some(type_schema) = schemas.get(type_name) {
            println!("  🔨 Processing {} schema", type_name);

            match create_basic_rust_struct(type_name, type_schema) {
                Ok(rust_code) => {
                    generated_types.push(rust_code);
                    println!("  ✅ Generated Rust struct for {}", type_name);
                }
                Err(e) => {
                    println!("  ❌ Failed to generate {} struct: {}", type_name, e);
                }
            }
        } else {
            println!("  ⚠️  {} schema not found", type_name);
        }
    }

    // Check if HashMap is actually used in any of the generated types
    let uses_hashmap = generated_types.iter().any(|code| code.contains("HashMap"));

    let import_section = if uses_hashmap {
        "use serde::{Serialize, Deserialize};\nuse std::collections::HashMap;\n"
    } else {
        "use serde::{Serialize, Deserialize};\n"
    };

    // Combine all generated types into a single file
    let complete_code = format!(
        "// Generated OpenAI types from official OpenAPI spec\n\
        // Essential types for LLMIR OpenAI chat completion integration\n\
        \n\
        {}\n\
        {}\n",
        import_section,
        generated_types.join("\n\n")
    );

    let dest_path = "src/providers/openai/generated.rs";

    // Create the directory if it doesn't exist
    if let Some(parent) = Path::new(dest_path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Write the generated types
    if fs::write(dest_path, &complete_code).is_ok() {
        println!("📝 Generated OpenAI types to: {}", dest_path);

        // Format the file with cargo fmt
        let _ = std::process::Command::new("cargo")
            .args(["fmt", "--", dest_path])
            .output();

        println!("✅ OpenAI types generated and formatted");
    } else {
        println!("❌ Failed to write OpenAI generated types");
    }
}

fn generate_anthropic_specific_types(schemas: &serde_json::Value) {
    // Focus only on essential Anthropic message types to minimize generated code
    let essential_types = [
        "CreateMessageParams",
        "Message",
        "InputMessage",
        "ContentBlock",
        "RequestTextBlock",
        "ResponseTextBlock",
        "Usage",
        "Tool",
        "ToolChoice",
    ];

    let mut generated_types = Vec::new();

    for type_name in essential_types {
        if let Some(type_schema) = schemas.get(type_name) {
            println!("  🔨 Processing Anthropic {} schema", type_name);

            match create_basic_rust_struct(type_name, type_schema) {
                Ok(rust_code) => {
                    generated_types.push(rust_code);
                    println!("  ✅ Generated Rust struct for Anthropic {}", type_name);
                }
                Err(e) => {
                    println!(
                        "  ❌ Failed to generate Anthropic {} struct: {}",
                        type_name, e
                    );
                }
            }
        } else {
            println!("  ⚠️  Anthropic {} schema not found", type_name);
        }
    }

    // Check if HashMap is actually used in any of the generated types
    let uses_hashmap = generated_types.iter().any(|code| code.contains("HashMap"));

    let import_section = if uses_hashmap {
        "use serde::{Serialize, Deserialize};\nuse std::collections::HashMap;\n"
    } else {
        "use serde::{Serialize, Deserialize};\n"
    };

    // Combine all generated types into a single file
    let complete_code = format!(
        "// Generated Anthropic types from unofficial OpenAPI spec\n\
        // Essential types for LLMIR Anthropic messages integration\n\
        \n\
        {}\n\
        {}\n",
        import_section,
        generated_types.join("\n\n")
    );

    let dest_path = "src/providers/anthropic/generated.rs";

    // Create the directory if it doesn't exist
    if let Some(parent) = Path::new(dest_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Write the generated types
    if std::fs::write(dest_path, &complete_code).is_ok() {
        println!("📝 Generated Anthropic types to: {}", dest_path);

        // Format the file with cargo fmt
        let _ = std::process::Command::new("cargo")
            .args(["fmt", "--", dest_path])
            .output();

        println!("✅ Anthropic types generated and formatted");
    } else {
        println!("❌ Failed to write Anthropic generated types");
    }
}

fn generate_google_protobuf_types(_proto_paths: &[String], _proto_dir: &str) {
    // For now, create a simple placeholder until we can properly debug protobuf generation
    let placeholder_content = r#"// Generated Google AI types from official protobuf files
// Essential types for LLMIR Google AI integration

use serde::{Serialize, Deserialize};

// Placeholder types - will be replaced with actual protobuf-generated types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    // Add other part types as needed
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetySetting {
    pub category: i32, // HarmCategory
    pub threshold: i32, // HarmBlockThreshold
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyRating {
    pub category: i32, // HarmCategory
    pub probability: i32, // HarmProbability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<FunctionDeclaration>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // Add parameters schema as needed
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<i32>,
}

// Type aliases for compatibility
pub type SafetySettings = Vec<SafetySetting>;
pub type HarmCategory = i32;
pub type HarmBlockThreshold = i32;
"#;

    let dest_path = "src/providers/google/generated.rs";

    // Create the directory if it doesn't exist
    if let Some(parent) = Path::new(dest_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Write the placeholder types
    if std::fs::write(dest_path, placeholder_content).is_ok() {
        println!("📝 Generated Google placeholder types to: {}", dest_path);

        // Format the file with cargo fmt
        let _ = std::process::Command::new("cargo")
            .args(["fmt", "--", dest_path])
            .output();

        println!("✅ Google placeholder types generated and formatted");
        println!("📝 Note: Using placeholder types. Protobuf integration will be completed later.");
    } else {
        println!("❌ Failed to write Google generated types");
    }
}

// Helper functions from the original build.rs
fn create_basic_rust_struct(
    name: &str,
    schema: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut struct_code = format!(
        "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\npub struct {} {{\n",
        name
    );

    // Handle allOf schemas by merging properties
    let properties = if let Some(all_of) = schema.get("allOf") {
        let mut merged_props = serde_json::Map::new();
        for item in all_of.as_array().unwrap_or(&vec![]) {
            if let Some(props) = item.get("properties").and_then(|p| p.as_object()) {
                for (key, value) in props {
                    merged_props.insert(key.clone(), value.clone());
                }
            }
        }
        serde_json::Value::Object(merged_props)
    } else {
        schema
            .get("properties")
            .cloned()
            .unwrap_or(serde_json::json!({}))
    };

    // Get required fields
    let required_fields: std::collections::HashSet<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    if let Some(props) = properties.as_object() {
        for (field_name, field_schema) in props {
            let is_optional = !required_fields.contains(field_name);
            let rust_type = json_schema_to_rust_type(field_schema);

            let field_type = if is_optional {
                format!("Option<{}>", rust_type)
            } else {
                rust_type
            };

            // Add serde attribute for optional fields
            if is_optional {
                struct_code.push_str("    #[serde(skip_serializing_if = \"Option::is_none\")]\n");
            }

            // Handle reserved keywords by escaping them and adding serde rename
            let (rust_field_name, serde_attr) = if is_rust_keyword(field_name) {
                (
                    format!("r#{}", field_name),
                    format!("    #[serde(rename = \"{}\")]\n", field_name),
                )
            } else {
                (field_name.clone(), String::new())
            };

            struct_code.push_str(&serde_attr);
            struct_code.push_str(&format!("    pub {}: {},\n", rust_field_name, field_type));
        }
    }

    struct_code.push_str("}\n");
    Ok(struct_code)
}

fn json_schema_to_rust_type(schema: &serde_json::Value) -> String {
    // Basic JSON Schema to Rust type conversion
    match schema.get("type").and_then(|t| t.as_str()) {
        Some("string") => "String".to_string(),
        Some("integer") => "i64".to_string(),
        Some("number") => "f64".to_string(),
        Some("boolean") => "bool".to_string(),
        Some("array") => {
            if let Some(items) = schema.get("items") {
                format!("Vec<{}>", json_schema_to_rust_type(items))
            } else {
                "Vec<serde_json::Value>".to_string()
            }
        }
        Some("object") => {
            if schema.get("additionalProperties").is_some() {
                "HashMap<String, serde_json::Value>".to_string()
            } else {
                "serde_json::Value".to_string()
            }
        }
        _ => {
            // Handle $ref, anyOf, oneOf, etc. - all use Value for now
            "serde_json::Value".to_string()
        }
    }
}

fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}
