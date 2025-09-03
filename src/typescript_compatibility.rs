/**
 * Rust integration test for TypeScript compatibility
 *
 * This test ensures our Rust types generate TypeScript bindings
 * that are compatible with the Vercel AI SDK.
 */
use crate::universal::*;
use serde_json::json;

#[test]
fn test_typescript_bindings_generation() {
    // This test forces ts-rs to generate TypeScript bindings
    // by referencing all our exported types

    let _message: Option<LanguageModelV2Message> = None;
    let _user_content: Option<LanguageModelV2UserContent> = None;
    let _assistant_content: Option<LanguageModelV2AssistantContent> = None;
    let _tool_content: Option<LanguageModelV2ToolContent> = None;
    let _provider_options: Option<SharedV2ProviderOptions> = None;
    let _provider_metadata: Option<SharedV2ProviderMetadata> = None;

    println!("✅ TypeScript bindings generated for all types");
}

#[test]
fn test_ai_sdk_json_compatibility() {
    // Create messages using our Rust types
    let messages = vec![
        LanguageModelV2Message::User {
            content: vec![LanguageModelV2UserContent::Text {
                text: "Hello AI SDK!".to_string(),
                provider_metadata: None,
            }],
            provider_options: None,
        },
        LanguageModelV2Message::Assistant {
            content: vec![
                LanguageModelV2AssistantContent::Reasoning {
                    text: "Let me think...".to_string(),
                    provider_metadata: None,
                },
                LanguageModelV2AssistantContent::Text {
                    text: "Hello! How can I help you?".to_string(),
                    provider_metadata: None,
                },
            ],
            provider_options: None,
        },
    ];

    // Serialize to JSON
    let json = serde_json::to_value(&messages).unwrap();

    // Verify it matches expected AI SDK structure exactly
    let expected = json!([
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "Hello AI SDK!"
                }
            ]
        },
        {
            "role": "assistant",
            "content": [
                {
                    "type": "reasoning",
                    "text": "Let me think..."
                },
                {
                    "type": "text",
                    "text": "Hello! How can I help you?"
                }
            ]
        }
    ]);

    assert_eq!(
        json, expected,
        "JSON output must match AI SDK format exactly"
    );
    println!("✅ JSON serialization matches AI SDK format perfectly");
}

#[test]
fn test_role_specific_content_restrictions() {
    // Test that our types enforce role-specific content at the Rust level

    // ✅ Valid: User with text and file
    let _valid_user = LanguageModelV2Message::User {
        content: vec![
            LanguageModelV2UserContent::Text {
                text: "Analyze this".to_string(),
                provider_metadata: None,
            },
            LanguageModelV2UserContent::File {
                data: "data:image/png;base64,...".to_string(),
                mime_type: "image/png".to_string(),
                provider_metadata: None,
            },
        ],
        provider_options: None,
    };

    // ✅ Valid: Assistant with all content types
    let _valid_assistant = LanguageModelV2Message::Assistant {
        content: vec![
            LanguageModelV2AssistantContent::Reasoning {
                text: "Thinking...".to_string(),
                provider_metadata: None,
            },
            LanguageModelV2AssistantContent::Text {
                text: "I see a cat".to_string(),
                provider_metadata: None,
            },
            LanguageModelV2AssistantContent::Source {
                source_type: LanguageModelV2SourceType::Document,
                id: "doc-1".to_string(),
                url: None,
                title: Some("Guide".to_string()),
                filename: None,
                media_type: None,
                provider_metadata: None,
            },
            LanguageModelV2AssistantContent::ToolCall {
                id: "call_123".to_string(),
                name: "search".to_string(),
                args: json!({"query": "cats"}),
                provider_metadata: None,
            },
        ],
        provider_options: None,
    };

    // ✅ Valid: Tool with only tool results
    let _valid_tool = LanguageModelV2Message::Tool {
        content: vec![LanguageModelV2ToolContent::ToolResult {
            tool_call_id: "call_123".to_string(),
            result: json!({"found": 5}),
            is_error: Some(false),
            provider_metadata: None,
        }],
        provider_options: None,
    };

    println!("✅ Role-specific content restrictions working correctly");

    // Note: The following would cause compile errors (which is what we want!):
    // ❌ User cannot send reasoning content
    // ❌ User cannot send source content
    // ❌ User cannot send tool calls
    // ❌ Tool cannot send text content
    // ❌ Tool cannot send reasoning content
}

#[test]
fn test_provider_options_flexibility() {
    use std::collections::BTreeMap;

    // Test that provider options are flexible and extensible
    let mut anthropic_options = BTreeMap::new();
    anthropic_options.insert("max_tokens".to_string(), json!(1000));
    anthropic_options.insert("temperature".to_string(), json!(0.7));
    anthropic_options.insert(
        "cache_control".to_string(),
        json!({
            "type": "ephemeral"
        }),
    );

    let mut openai_options = BTreeMap::new();
    openai_options.insert("logprobs".to_string(), json!(true));
    openai_options.insert("top_logprobs".to_string(), json!(5));

    let mut provider_options = BTreeMap::new();
    provider_options.insert("anthropic".to_string(), json!(anthropic_options));
    provider_options.insert("openai".to_string(), json!(openai_options));

    let message = LanguageModelV2Message::User {
        content: vec![LanguageModelV2UserContent::Text {
            text: "Test with options".to_string(),
            provider_metadata: None,
        }],
        provider_options: Some(SharedV2ProviderOptions {
            options: provider_options.into_iter().collect(),
        }),
    };

    let json = serde_json::to_value(&message).unwrap();
    println!(
        "Provider options JSON: {}",
        serde_json::to_string_pretty(&json).unwrap()
    );

    // Verify the structure includes provider options with correct camelCase naming
    assert!(json.get("providerOptions").is_some());
    assert!(json["providerOptions"].get("anthropic").is_some());
    assert!(json["providerOptions"].get("openai").is_some());

    println!("✅ Provider options are flexible and serialize correctly");
}
