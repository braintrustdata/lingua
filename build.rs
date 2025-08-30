use std::path::Path;

fn main() {
    // Generate TypeScript types from Rust types
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ts_dir = Path::new(&out_dir).join("../../../typescript");

    // Create typescript directory
    std::fs::create_dir_all(&ts_dir).unwrap();

    // This will be called automatically by ts-rs when we run the build
    // The TypeScript files will be generated to typescript/bindings/

    // Generate provider types from OpenAPI specs using typify
    generate_openai_types_from_openapi();
    generate_anthropic_types_from_openapi();

    // Copy generated types to src directory
    copy_generated_types_to_src();

    println!("cargo:rerun-if-changed=src/");
}

fn generate_openai_types_from_openapi() {
    use std::fs;
    use std::io::Write;

    println!("Generating OpenAI types from local OpenAPI spec...");

    // Use local OpenAPI spec file instead of downloading
    let spec_file_path = "specs/openai/openapi.yml";

    let openai_spec = match fs::read_to_string(spec_file_path) {
        Ok(content) => content,
        Err(e) => {
            println!(
                "Failed to read local OpenAPI spec at {}: {}",
                spec_file_path, e
            );
            println!("Run './pipelines/generate-provider-types.sh openai' to download the spec first");
            return;
        }
    };

    // Parse the YAML spec using serde_yaml
    println!("Parsing local YAML OpenAPI spec...");

    let schema: serde_json::Value = match serde_yaml::from_str(&openai_spec) {
        Ok(value) => value,
        Err(e) => {
            println!("Failed to parse OpenAPI spec as YAML: {}", e);
            return;
        }
    };

    // Extract the components/schemas section
    let schemas = schema.get("components").and_then(|c| c.get("schemas"));

    let out_dir = std::env::var("OUT_DIR").unwrap();

    if let Some(schemas) = schemas {
        println!("Found components/schemas section");

        // Look for the main chat completion schemas
        let request_schema = schemas.get("CreateChatCompletionRequest");
        let response_schema = schemas.get("CreateChatCompletionResponse");
        let stream_response_schema = schemas.get("CreateChatCompletionStreamResponse");

        let found_schemas = [
            ("CreateChatCompletionRequest", request_schema),
            ("CreateChatCompletionResponse", response_schema),
            ("CreateChatCompletionStreamResponse", stream_response_schema),
        ];

        for (name, schema_opt) in found_schemas {
            if let Some(schema_def) = schema_opt {
                println!("Found {} schema", name);

                // Save individual schemas for inspection
                let schema_path = Path::new(&out_dir).join(format!("{}.json", name.to_lowercase()));
                if let Ok(mut file) = fs::File::create(&schema_path) {
                    let pretty_json = serde_json::to_string_pretty(schema_def)
                        .unwrap_or_else(|_| schema_def.to_string());
                    let _ = file.write_all(pretty_json.as_bytes());
                    println!("Saved {} schema to: {:?}", name, schema_path);
                }
            } else {
                println!("{} schema not found", name);
            }
        }

        // Generate only essential Rust types for chat completion APIs
        println!("Generating essential OpenAI types for chat completions");
        try_generate_specific_types(schemas, &out_dir);

        // Also save a list of all available schema names for reference
        let schema_names: Vec<String> = schemas
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let names_path = Path::new(&out_dir).join("available_schema_names.txt");
        if let Ok(mut file) = fs::File::create(&names_path) {
            let names_list = schema_names.join("\n");
            let _ = file.write_all(names_list.as_bytes());
            println!("Saved schema names list to: {:?}", names_path);
            println!("Found {} total schemas", schema_names.len());
        }
    } else {
        println!("No components/schemas section found in OpenAPI spec");
    }
}

fn generate_anthropic_types_from_openapi() {
    use std::fs;
    use std::io::Write;

    println!("Generating Anthropic types from local OpenAPI spec...");

    // Use local OpenAPI spec file for Anthropic
    let spec_file_path = "specs/anthropic/openapi.json";
    
    let anthropic_spec = match fs::read_to_string(spec_file_path) {
        Ok(content) => content,
        Err(e) => {
            println!(
                "Failed to read local Anthropic OpenAPI spec at {}: {}",
                spec_file_path, e
            );
            println!("Run './pipelines/generate-provider-types.sh anthropic' to download the spec first");
            return;
        }
    };

    // Parse the JSON spec (Anthropic uses JSON format)
    println!("Parsing local JSON OpenAPI spec...");
    
    let schema: serde_json::Value = match serde_json::from_str(&anthropic_spec) {
        Ok(value) => value,
        Err(e) => {
            println!("Failed to parse Anthropic OpenAPI spec as JSON: {}", e);
            return;
        }
    };

    // Extract the components/schemas section
    let schemas = schema.get("components").and_then(|c| c.get("schemas"));

    let out_dir = std::env::var("OUT_DIR").unwrap();

    if let Some(schemas) = schemas {
        println!("Found Anthropic components/schemas section");
        
        // Generate essential Anthropic types for messages API
        println!("Generating essential Anthropic types for messages API");
        try_generate_anthropic_specific_types(schemas, &out_dir);
        
        // Save a list of all available schema names for reference
        let schema_names: Vec<String> = schemas
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();
        
        let names_path = Path::new(&out_dir).join("anthropic_schema_names.txt");
        if let Ok(mut file) = fs::File::create(&names_path) {
            let names_list = schema_names.join("\n");
            let _ = file.write_all(names_list.as_bytes());
            println!("Saved Anthropic schema names list to: {:?}", names_path);
            println!("Found {} total Anthropic schemas", schema_names.len());
        }
    } else {
        println!("No components/schemas section found in Anthropic OpenAPI spec");
    }
}

fn try_generate_specific_types(schemas: &serde_json::Value, out_dir: &str) {
    use std::fs;
    use std::io::Write;

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
            println!("Processing {} schema", type_name);

            match create_basic_rust_struct(type_name, type_schema) {
                Ok(rust_code) => {
                    generated_types.push(rust_code);
                    println!("Generated Rust struct for {}", type_name);
                }
                Err(e) => {
                    println!(
                        "Failed to generate {} struct: {}",
                        type_name, e
                    );
                }
            }
        }
    }

    // Combine all generated types into a single file
    let complete_code = format!(
        "// Generated OpenAI types from official OpenAPI spec\n\
        // Essential types for LLMIR OpenAI chat completion integration\n\
        \n\
        use serde::{{Serialize, Deserialize}};\n\
        use std::collections::HashMap;\n\
        \n\
        {}\n",
        generated_types.join("\n\n")
    );

    let generated_file_path = Path::new(out_dir).join("openai_generated_key_types.rs");
    if let Ok(mut file) = fs::File::create(&generated_file_path) {
        let _ = file.write_all(complete_code.as_bytes());
        println!(
            "Generated key OpenAI types: {:?}",
            generated_file_path
        );
    }
}

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

fn copy_generated_types_to_src() {
    use std::fs;

    let out_dir = std::env::var("OUT_DIR").unwrap();
    
    // Copy OpenAI generated types
    let openai_generated_file_path = Path::new(&out_dir).join("openai_generated_key_types.rs");
    if openai_generated_file_path.exists() {
        let dest_path = "src/providers/openai/generated.rs";

        // Create the directory if it doesn't exist
        if let Some(parent) = Path::new(dest_path).parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Copy the generated file to src
        if let Ok(contents) = fs::read_to_string(&openai_generated_file_path) {
            if fs::write(dest_path, contents).is_ok() {
                println!("Copied OpenAI generated types to: {}", dest_path);
            }
        }
    }

    // Copy Anthropic generated types
    let anthropic_generated_file_path = Path::new(&out_dir).join("anthropic_generated_key_types.rs");
    if anthropic_generated_file_path.exists() {
        let dest_path = "src/providers/anthropic/generated.rs";

        // Create the directory if it doesn't exist
        if let Some(parent) = Path::new(dest_path).parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Copy the generated file to src
        if let Ok(contents) = fs::read_to_string(&anthropic_generated_file_path) {
            if fs::write(dest_path, contents).is_ok() {
                println!("Copied Anthropic generated types to: {}", dest_path);
            }
        }
    }
}

fn try_generate_anthropic_specific_types(schemas: &serde_json::Value, out_dir: &str) {
    use std::fs;
    use std::io::Write;

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
            println!("Processing Anthropic {} schema", type_name);

            match create_basic_rust_struct(type_name, type_schema) {
                Ok(rust_code) => {
                    generated_types.push(rust_code);
                    println!("Generated Rust struct for Anthropic {}", type_name);
                }
                Err(e) => {
                    println!(
                        "Failed to generate Anthropic {} struct: {}",
                        type_name, e
                    );
                }
            }
        } else {
            println!("Anthropic {} schema not found", type_name);
        }
    }

    // Combine all generated types into a single file
    let complete_code = format!(
        "// Generated Anthropic types from unofficial OpenAPI spec\n\
        // Essential types for LLMIR Anthropic messages integration\n\
        \n\
        use serde::{{Serialize, Deserialize}};\n\
        use std::collections::HashMap;\n\
        \n\
        {}\n",
        generated_types.join("\n\n")
    );

    let generated_file_path = Path::new(out_dir).join("anthropic_generated_key_types.rs");
    if let Ok(mut file) = fs::File::create(&generated_file_path) {
        let _ = file.write_all(complete_code.as_bytes());
        println!(
            "Generated key Anthropic types: {:?}",
            generated_file_path
        );
    }
}
