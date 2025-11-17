use crate::providers::anthropic::generated;
use crate::universal::convert::TryFromLLM;
use crate::universal::{ClientTool, ProviderTool, Tool};

#[test]
fn test_client_tool_to_anthropic() {
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

    let anthropic_tool: generated::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    assert_eq!(anthropic_tool.name, "get_weather");
    assert_eq!(
        anthropic_tool.description,
        Some("Get current weather".to_string())
    );
    assert_eq!(anthropic_tool.tool_type, Some(generated::ToolType::Custom));
    assert!(anthropic_tool.input_schema.is_some());
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

    // Convert to Anthropic
    let anthropic_tool: generated::Tool = TryFromLLM::try_from(original.clone()).unwrap();

    // Convert back to Lingua
    let round_trip: Tool = TryFromLLM::try_from(anthropic_tool).unwrap();

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
fn test_web_search_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "web_search_20250305".to_string(),
        name: None,
        config: Some(serde_json::json!({
            "max_uses": 5,
            "allowed_domains": ["wikipedia.org"]
        })),
    });

    let anthropic_tool: generated::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    assert_eq!(anthropic_tool.name, "web_search_20250305");
    assert_eq!(
        anthropic_tool.tool_type,
        Some(generated::ToolType::WebSearch20250305)
    );
    assert_eq!(anthropic_tool.max_uses, Some(5));
    assert_eq!(
        anthropic_tool.allowed_domains,
        Some(vec!["wikipedia.org".to_string()])
    );
}

#[test]
fn test_bash_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "bash_20250124".to_string(),
        name: Some("my_bash".to_string()),
        config: Some(serde_json::json!({
            "max_uses": 10
        })),
    });

    let anthropic_tool: generated::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    assert_eq!(anthropic_tool.name, "my_bash");
    assert_eq!(
        anthropic_tool.tool_type,
        Some(generated::ToolType::Bash20250124)
    );
    assert_eq!(anthropic_tool.max_uses, Some(10));
}

#[test]
fn test_text_editor_provider_tool() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "text_editor_20250728".to_string(),
        name: None,
        config: Some(serde_json::json!({
            "max_characters": 1000
        })),
    });

    let anthropic_tool: generated::Tool = TryFromLLM::try_from(lingua_tool).unwrap();

    assert_eq!(anthropic_tool.name, "text_editor_20250728");
    assert_eq!(
        anthropic_tool.tool_type,
        Some(generated::ToolType::TextEditor20250728)
    );
    assert_eq!(anthropic_tool.max_characters, Some(1000));
}

#[test]
fn test_provider_tool_roundtrip() {
    let original = Tool::Provider(ProviderTool {
        tool_type: "web_search_20250305".to_string(),
        name: Some("search".to_string()),
        config: Some(serde_json::json!({
            "max_uses": 3,
            "blocked_domains": ["example.com"]
        })),
    });

    // Convert to Anthropic
    let anthropic_tool: generated::Tool = TryFromLLM::try_from(original.clone()).unwrap();

    // Convert back to Lingua
    let round_trip: Tool = TryFromLLM::try_from(anthropic_tool).unwrap();

    // Verify
    if let (Tool::Provider(orig), Tool::Provider(rt)) = (&original, &round_trip) {
        assert_eq!(rt.tool_type, orig.tool_type);
        assert_eq!(rt.name, Some("search".to_string()));
        assert_eq!(rt.config, orig.config);
    } else {
        panic!("Tool type changed during roundtrip");
    }
}

#[test]
fn test_unsupported_provider_tool_errors() {
    let lingua_tool = Tool::Provider(ProviderTool {
        tool_type: "code_execution".to_string(), // Not supported by Anthropic
        name: None,
        config: None,
    });

    let result: Result<generated::Tool, _> = TryFromLLM::try_from(lingua_tool);
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("doesn't support"));
    assert!(error_msg.contains("code_execution"));
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
            tool_type: "bash_20250124".to_string(),
            name: None,
            config: None,
        }),
    ];

    // Test Vec conversion
    let anthropic_tools: Vec<generated::Tool> = TryFromLLM::try_from(tools).unwrap();
    assert_eq!(anthropic_tools.len(), 2);

    // Verify first tool (ClientTool)
    assert_eq!(anthropic_tools[0].name, "tool1");
    assert_eq!(anthropic_tools[0].description, Some("First tool".to_string()));
    assert_eq!(anthropic_tools[0].tool_type, Some(generated::ToolType::Custom));
    assert!(anthropic_tools[0].input_schema.is_some());

    // Verify second tool (ProviderTool - Bash)
    assert_eq!(anthropic_tools[1].name, "bash_20250124");
    assert_eq!(
        anthropic_tools[1].tool_type,
        Some(generated::ToolType::Bash20250124)
    );
}
