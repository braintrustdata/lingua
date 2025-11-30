use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::{ClientTool, ProviderTool, Tool};

#[test]
fn test_client_tool_to_openai() {
    let lingua_tool = Tool::Client(ClientTool {
        name: "get_weather".to_string(),
        description: "Get current weather".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            },
            "required": ["location"]
        }),
        provider_options: None,
    });

    let openai_tool: openai::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    match openai_tool {
        openai::Tool::Function(function) => {
            assert_eq!(function.name, "get_weather");
            assert_eq!(
                function.description,
                Some("Get current weather".to_string())
            );
            assert!(!function.parameters.is_empty());
        }
        other => panic!("Expected Function tool, got {:?}", other),
    }
}

#[test]
fn test_client_tool_with_strict_mode() {
    let lingua_tool = Tool::Client(ClientTool {
        name: "query_db".to_string(),
        description: "Query database".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        }),
        provider_options: Some(serde_json::json!({
            "strict": true
        })),
    });

    let openai_tool: openai::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    match openai_tool {
        openai::Tool::Function(function) => assert!(function.strict),
        other => panic!("Expected Function tool, got {:?}", other),
    }
}

#[test]
fn test_client_tool_roundtrip() {
    let original = Tool::Client(ClientTool {
        name: "calculate".to_string(),
        description: "Perform calculations".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "expression": { "type": "string" }
            },
            "required": ["expression"]
        }),
        provider_options: None,
    });

    // Convert to OpenAI
    let openai_tool: openai::Tool = TryFromLLM::try_from(original.clone()).unwrap();

    // Convert back to Lingua
    let round_trip: Tool = TryFromLLM::try_from(openai_tool).unwrap();

    // Verify key fields match
    if let (Tool::Client(orig), Tool::Client(rt)) = (&original, &round_trip) {
        assert_eq!(rt.name, orig.name);
        assert_eq!(rt.description, orig.description);
        assert_eq!(rt.input_schema, orig.input_schema);
    } else {
        panic!("Tool type changed during roundtrip");
    }
}

#[test]
fn test_computer_use_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "computer_use_preview".to_string(),
        name: None,
        config: Some(serde_json::json!({
            "display_width_px": 1920,
            "display_height_px": 1080
        })),
    });

    let openai_tool: openai::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    match openai_tool {
        openai::Tool::ComputerUsePreview(tool) => {
            assert_eq!(tool.display_width, 1920);
            assert_eq!(tool.display_height, 1080);
        }
        other => panic!("Expected ComputerUsePreview, got {:?}", other),
    }
}

#[test]
fn test_code_interpreter_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "code_interpreter".to_string(),
        name: Some("my_interpreter".to_string()),
        config: None,
    });

    let openai_tool: openai::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    match openai_tool {
        openai::Tool::CodeInterpreter(_tool) => {}
        other => panic!("Expected CodeInterpreter, got {:?}", other),
    }
}

#[test]
fn test_web_search_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "web_search".to_string(),
        name: None,
        config: None,
    });

    let openai_tool: openai::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    match openai_tool {
        openai::Tool::WebSearch(_tool) => {
            // WebSearch tool created successfully
        }
        other => panic!("Expected WebSearch, got {:?}", other),
    }
}

#[test]
fn test_provider_tool_roundtrip() {
    let original = Tool::Provider(ProviderTool {
        tool_type: "computer_use_preview".to_string(),
        name: Some("computer".to_string()),
        config: Some(serde_json::json!({
            "display_width_px": 800,
            "display_height_px": 600
        })),
    });

    // Convert to OpenAI
    let openai_tool: openai::Tool = TryFromLLM::try_from(original.clone()).unwrap();

    // Convert back to Lingua
    let round_trip: Tool = TryFromLLM::try_from(openai_tool).unwrap();

    // Verify
    if let (Tool::Provider(orig), Tool::Provider(rt)) = (&original, &round_trip) {
        assert_eq!(rt.tool_type, orig.tool_type);
        assert_eq!(rt.name, None);
        let mut rt_config = rt.config.clone().unwrap_or_default();
        if let Some(map) = rt_config.as_object_mut() {
            map.remove("environment"); // environment may be defaulted
        }
        let expected = orig.config.clone().unwrap();
        assert_eq!(rt_config, expected);
    } else {
        panic!("Tool type changed during roundtrip");
    }
}

#[test]
fn test_unsupported_provider_tool_errors() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "bash_20250124".to_string(), // Not supported by OpenAI
        name: None,
        config: None,
    });

    let result: Result<openai::Tool, _> = TryFromLLM::try_from(lingua_tool);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("Unsupported OpenAI provider tool type"),
        "Expected unsupported tool error, got: {}",
        err
    );
}

#[test]
fn test_vec_conversion() {
    let tools = vec![
        Tool::Client(ClientTool {
            name: "tool1".to_string(),
            description: "First tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            provider_options: None,
        }),
        Tool::Provider(ProviderTool {
            tool_type: "code_interpreter".to_string(),
            name: None,
            config: None,
        }),
    ];

    // Test Vec conversion
    let openai_tools: Vec<openai::Tool> = TryFromLLM::try_from(tools).unwrap();
    assert_eq!(openai_tools.len(), 2);

    // Verify first tool (ClientTool)
    match &openai_tools[0] {
        openai::Tool::Function(function) => {
            assert_eq!(function.name, "tool1");
            assert_eq!(function.description, Some("First tool".to_string()));
            assert!(!function.parameters.is_empty());
        }
        other => panic!("Expected Function tool, got {:?}", other),
    }

    // Verify second tool (ProviderTool - Code Interpreter)
    match &openai_tools[1] {
        openai::Tool::CodeInterpreter(_) => {}
        other => panic!("Expected CodeInterpreter tool, got {:?}", other),
    }
}

// ============================================================================
// ToolElement Tests (for Chat Completions API)
// ============================================================================

#[test]
fn test_client_tool_to_tool_element() {
    let lingua_tool = Tool::Client(ClientTool {
        name: "get_weather".to_string(),
        description: "Get current weather".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            },
            "required": ["location"]
        }),
        provider_options: None,
    });

    let tool_element: openai::ToolElement = TryFromLLM::try_from(lingua_tool).unwrap();

    assert_eq!(tool_element.tool_type, openai::ToolType::Function);
    assert!(tool_element.custom.is_none());

    let function = tool_element.function.expect("function should be set");
    assert_eq!(function.name, "get_weather");
    assert_eq!(
        function.description,
        Some("Get current weather".to_string())
    );
    assert!(function.parameters.is_some());
}

#[test]
fn test_tool_element_with_strict_mode() {
    let lingua_tool = Tool::Client(ClientTool {
        name: "query_db".to_string(),
        description: "Query database".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        }),
        provider_options: Some(serde_json::json!({
            "strict": true
        })),
    });

    let tool_element: openai::ToolElement = TryFromLLM::try_from(lingua_tool).unwrap();

    let function = tool_element.function.expect("function should be set");
    assert_eq!(function.strict, Some(true));
}

#[test]
fn test_tool_element_roundtrip() {
    let original = Tool::Client(ClientTool {
        name: "calculate".to_string(),
        description: "Perform calculations".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "expression": { "type": "string" }
            },
            "required": ["expression"]
        }),
        provider_options: None,
    });

    // Convert to ToolElement
    let tool_element: openai::ToolElement = TryFromLLM::try_from(original.clone()).unwrap();

    // Convert back to Lingua
    let round_trip: Tool = TryFromLLM::try_from(tool_element).unwrap();

    // Verify key fields match
    if let (Tool::Client(orig), Tool::Client(rt)) = (&original, &round_trip) {
        assert_eq!(rt.name, orig.name);
        assert_eq!(rt.description, orig.description);
        assert_eq!(rt.input_schema, orig.input_schema);
    } else {
        panic!("Tool type changed during roundtrip");
    }
}

#[test]
fn test_provider_tool_to_tool_element_fails() {
    // Provider tools cannot be converted to ToolElement (Chat Completions only supports functions)
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "web_search".to_string(),
        name: None,
        config: None,
    });

    let result: Result<openai::ToolElement, _> = TryFromLLM::try_from(lingua_tool);
    assert!(result.is_err());
}

#[test]
fn test_tool_element_serialization_format() {
    // This test verifies that ToolElement produces the correct nested JSON format
    // that OpenAI's Chat Completions API expects
    let lingua_tool = Tool::Client(ClientTool {
        name: "get_weather".to_string(),
        description: "Get weather".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            }
        }),
        provider_options: None,
    });

    let tool_element: openai::ToolElement = TryFromLLM::try_from(lingua_tool).unwrap();
    let json = serde_json::to_value(&tool_element).unwrap();

    // Verify the nested structure
    assert_eq!(json["type"], "function");
    assert!(json.get("function").is_some());
    assert_eq!(json["function"]["name"], "get_weather");
    assert_eq!(json["function"]["description"], "Get weather");
}

#[test]
fn test_vec_tool_element_conversion() {
    let tools = vec![
        Tool::Client(ClientTool {
            name: "tool1".to_string(),
            description: "First tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            provider_options: None,
        }),
        Tool::Client(ClientTool {
            name: "tool2".to_string(),
            description: "Second tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            provider_options: None,
        }),
    ];

    // Test Vec conversion
    let tool_elements: Vec<openai::ToolElement> = TryFromLLM::try_from(tools).unwrap();
    assert_eq!(tool_elements.len(), 2);

    // Verify first tool
    let func1 = tool_elements[0]
        .function
        .as_ref()
        .expect("function should be set");
    assert_eq!(func1.name, "tool1");

    // Verify second tool
    let func2 = tool_elements[1]
        .function
        .as_ref()
        .expect("function should be set");
    assert_eq!(func2.name, "tool2");
}
