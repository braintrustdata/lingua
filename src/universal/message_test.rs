use super::message::*;
use serde_json::json;

#[test]
fn test_exact_ai_sdk_format() {
    let messages: ModelPrompt = vec![
        ModelMessage::System {
            content: "You are a helpful assistant.".to_string(),
            provider_options: None,
        },
        ModelMessage::User {
            content: UserContent::Array(vec![UserContentPart::Text(TextPart {
                r#type: "text".to_string(),
                text: "What's 2+2?".to_string(),
                provider_options: None,
            })]),
            provider_options: None,
        },
        ModelMessage::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::Text(TextPart {
                r#type: "text".to_string(),
                text: "2+2 equals 4.".to_string(),
                provider_options: None,
            })]),
            provider_options: None,
        },
    ];

    let serialized = serde_json::to_string_pretty(&messages).unwrap();
    eprintln!("Basic conversation format:");
    eprintln!("{}", serialized);

    // Verify it matches expected AI SDK format
    let expected_structure = json!([
        {
            "role": "system",
            "content": "You are a helpful assistant."
        },
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "What's 2+2?"
                }
            ]
        },
        {
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "2+2 equals 4."
                }
            ]
        }
    ]);

    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed, expected_structure);
}

#[test]
fn test_multimodal_with_reasoning() {
    let messages: ModelPrompt = vec![
        ModelMessage::User {
            content: UserContent::Array(vec![
                UserContentPart::Text(TextPart {
                    r#type: "text".to_string(),
                    text: "Analyze this image".to_string(),
                    provider_options: None,
                }),
                UserContentPart::File(FilePart {
                    r#type: "file".to_string(),
                    data: json!("data:image/jpeg;base64,/9j/4AAQSkZJRg..."),
                    filename: None,
                    media_type: "image/jpeg".to_string(),
                    provider_options: None,
                }),
            ]),
            provider_options: None,
        },
        ModelMessage::Assistant {
            content: AssistantContent::Array(vec![
                AssistantContentPart::Reasoning(ReasoningPart {
                    r#type: "reasoning".to_string(),
                    text: "Let me analyze this image step by step...".to_string(),
                    provider_options: None,
                }),
                AssistantContentPart::Text(TextPart {
                    r#type: "text".to_string(),
                    text: "I can see a cat in the image.".to_string(),
                    provider_options: None,
                }),
            ]),
            provider_options: None,
        },
    ];

    let serialized = serde_json::to_string_pretty(&messages).unwrap();
    eprintln!("Multimodal with reasoning:");
    eprintln!("{}", serialized);
}

#[test]
fn test_tool_calling_flow() {
    let messages: ModelPrompt = vec![
        ModelMessage::User {
            content: UserContent::Array(vec![UserContentPart::Text(TextPart {
                r#type: "text".to_string(),
                text: "What's the weather in SF?".to_string(),
                provider_options: None,
            })]),
            provider_options: None,
        },
        ModelMessage::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall(ToolCallPart {
                r#type: "tool-call".to_string(),
                tool_call_id: "call_abc123".to_string(),
                tool_name: "get_weather".to_string(),
                input: json!({"location": "San Francisco"}),
                provider_options: None,
                provider_executed: None,
            })]),
            provider_options: None,
        },
        ModelMessage::Tool {
            content: vec![ToolResultPart {
                r#type: "tool-result".to_string(),
                tool_call_id: "call_abc123".to_string(),
                tool_name: "get_weather".to_string(),
                output: json!({"temperature": "72°F", "condition": "sunny"}),
                provider_options: None,
            }],
            provider_options: None,
        },
        ModelMessage::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::Text(TextPart {
                r#type: "text".to_string(),
                text: "The weather in San Francisco is currently 72°F and sunny.".to_string(),
                provider_options: None,
            })]),
            provider_options: None,
        },
    ];

    let serialized = serde_json::to_string_pretty(&messages).unwrap();
    eprintln!("Tool calling flow:");
    eprintln!("{}", serialized);
}

#[test]
fn test_provider_metadata() {
    let mut metadata = serde_json::Map::new();
    metadata.insert("openai".to_string(), json!({"model": "gpt-4"}));
    metadata.insert(
        "anthropic".to_string(),
        json!({"cache_control": {"type": "ephemeral"}}),
    );

    let message = ModelMessage::Assistant {
        content: AssistantContent::Array(vec![AssistantContentPart::Text(TextPart {
            r#type: "text".to_string(),
            text: "Response with metadata".to_string(),
            provider_options: Some(ProviderOptions { options: metadata }),
        })]),
        provider_options: None,
    };

    let serialized = serde_json::to_string_pretty(&message).unwrap();
    eprintln!("Provider metadata:");
    eprintln!("{}", serialized);
}
