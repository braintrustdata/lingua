use crate::providers::anthropic::generated;
use crate::serde_json;
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

    match anthropic_tool {
        generated::Tool::Custom(custom) => {
            assert_eq!(custom.name, "get_weather");
            assert_eq!(custom.description, Some("Get current weather".to_string()));
            // input_schema is now serde_json::Value, access properties through JSON methods
            assert!(custom.input_schema.get("properties").is_some());
        }
        other => panic!("Expected Custom tool, got {:?}", other),
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

    match anthropic_tool {
        generated::Tool::WebSearch20250305(search) => {
            assert_eq!(search.name, "web_search_20250305");
            assert_eq!(search.max_uses, Some(5));
            assert_eq!(
                search.allowed_domains,
                Some(vec!["wikipedia.org".to_string()])
            );
        }
        other => panic!("Expected WebSearch20250305, got {:?}", other),
    }
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

    match anthropic_tool {
        generated::Tool::Bash20250124(bash) => {
            assert_eq!(bash.name, "my_bash");
        }
        other => panic!("Expected Bash20250124, got {:?}", other),
    }
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

    match anthropic_tool {
        generated::Tool::TextEditor20250728(editor) => {
            assert_eq!(editor.name, "text_editor_20250728");
            assert_eq!(editor.max_characters, Some(1000));
        }
        other => panic!("Expected TextEditor20250728, got {:?}", other),
    }
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
    let err = result.unwrap_err();
    assert!(
        err.contains("Unsupported Anthropic provider tool type"),
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
            tool_type: "bash_20250124".to_string(),
            name: None,
            config: None,
        }),
    ];

    // Test Vec conversion
    let anthropic_tools: Vec<generated::Tool> = TryFromLLM::try_from(tools).unwrap();
    assert_eq!(anthropic_tools.len(), 2);

    // Verify first tool (ClientTool)
    match &anthropic_tools[0] {
        generated::Tool::Custom(custom) => {
            assert_eq!(custom.name, "tool1");
            assert_eq!(custom.description, Some("First tool".to_string()));
        }
        other => panic!("Expected first tool to be Custom, got {:?}", other),
    }

    // Verify second tool (ProviderTool - Bash)
    match &anthropic_tools[1] {
        generated::Tool::Bash20250124(bash) => assert_eq!(bash.name, "bash_20250124"),
        other => panic!("Expected second tool to be Bash20250124, got {:?}", other),
    }
}
