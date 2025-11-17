use crate::providers::openai::generated as openai;
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

    assert_eq!(openai_tool.name, Some("get_weather".to_string()));
    assert_eq!(
        openai_tool.description,
        Some("Get current weather".to_string())
    );
    assert_eq!(openai_tool.tool_type, openai::ToolTypeEnum::Function);
    assert!(openai_tool.parameters.is_some());
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

    assert_eq!(openai_tool.strict, Some(true));
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

    assert_eq!(
        openai_tool.tool_type,
        openai::ToolTypeEnum::ComputerUsePreview
    );
    assert_eq!(openai_tool.display_width, Some(1920));
    assert_eq!(openai_tool.display_height, Some(1080));
}

#[test]
fn test_code_interpreter_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "code_interpreter".to_string(),
        name: Some("my_interpreter".to_string()),
        config: None,
    });

    let openai_tool: openai::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    assert_eq!(openai_tool.name, Some("my_interpreter".to_string()));
    assert_eq!(openai_tool.tool_type, openai::ToolTypeEnum::CodeInterpreter);
}

#[test]
fn test_web_search_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "web_search".to_string(),
        name: None,
        config: None,
    });

    let openai_tool: openai::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    assert_eq!(openai_tool.tool_type, openai::ToolTypeEnum::WebSearch);
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
        assert_eq!(rt.name, Some("computer".to_string()));
        assert_eq!(rt.config, orig.config);
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
    if let Err(err) = result {
        let error_msg = format!("{:?}", err);
        assert!(
            error_msg.contains("doesn't support") || error_msg.contains("UnsupportedInputType")
        );
    }
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
    assert_eq!(openai_tools[0].name, Some("tool1".to_string()));
    assert_eq!(openai_tools[0].description, Some("First tool".to_string()));
    assert_eq!(openai_tools[0].tool_type, openai::ToolTypeEnum::Function);
    assert!(openai_tools[0].parameters.is_some());

    // Verify second tool (ProviderTool - Code Interpreter)
    assert_eq!(openai_tools[1].name, None);
    assert_eq!(openai_tools[1].tool_type, openai::ToolTypeEnum::CodeInterpreter);
}
