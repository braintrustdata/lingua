use llmir::providers::openai::{ChatCompletionRequestMessage, ChatCompletionResponseMessage};
use llmir::universal::ModelMessage;
use serde_json;
use std::convert::TryFrom;
use std::fs;

#[derive(serde::Deserialize, Debug)]
struct SimpleRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionRequestMessage>,
    pub max_completion_tokens: Option<i64>,
}

#[derive(serde::Deserialize, Debug)]
struct SimpleResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<SimpleChoice>,
    pub usage: Option<serde_json::Value>,
    pub service_tier: Option<String>,
    pub system_fingerprint: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
struct SimpleChoice {
    pub index: i64,
    pub message: ChatCompletionResponseMessage,
    pub finish_reason: Option<String>,
}

fn load_request_payload() -> Result<SimpleRequest, Box<dyn std::error::Error>> {
    let content = fs::read_to_string("paylods/snapshots/openai-simpleRequest-request.json")?;
    let request: SimpleRequest = serde_json::from_str(&content)?;
    Ok(request)
}

fn load_non_streaming_response() -> Result<SimpleResponse, Box<dyn std::error::Error>> {
    let content =
        fs::read_to_string("paylods/snapshots/openai-simpleRequest-response-non-streaming.json")?;
    let response: SimpleResponse = serde_json::from_str(&content)?;
    Ok(response)
}

#[test]
fn test_request_payload_loads() {
    let request = load_request_payload().expect("Should load request payload");

    // Verify basic structure
    assert_eq!(request.model, "gpt-5-nano");
    assert_eq!(request.messages.len(), 1);

    // Verify message content
    if let Some(first_message) = request.messages.first() {
        println!("First message: {:?}", first_message);
        // Just verify we can load the message without panicking
        println!("Message role: {:?}", first_message.role);
    }
}

#[test]
fn test_response_payload_loads() {
    let response = load_non_streaming_response().expect("Should load response payload");

    // Verify basic structure
    assert_eq!(response.id, "chatcmpl-CI0cV14PY6vjohzL2px2PBMKyAzLU");
    assert_eq!(response.object, "chat.completion");
    assert_eq!(response.model, "gpt-5-nano-2025-08-07");
    assert_eq!(response.choices.len(), 1);

    // Verify choice structure
    if let Some(choice) = response.choices.first() {
        assert_eq!(choice.index, 0);
        assert_eq!(choice.finish_reason, Some("length".to_string()));
        // Note: The exact message structure depends on the generated OpenAI types
        println!("Choice message: {:?}", choice.message);
    }
}

#[test]
fn test_universal_to_openai_conversion() {
    // Test that we can convert from universal format to OpenAI format
    let universal_user = ModelMessage::User {
        content: llmir::universal::UserContent::String(
            "What is the capital of France?".to_string(),
        ),
    };

    let openai_message = ChatCompletionRequestMessage::try_from(universal_user)
        .expect("Should convert universal user message to OpenAI format");

    println!("Converted universal -> OpenAI: {:?}", openai_message);

    // Verify the conversion worked - just check that it has some role
    println!("Converted role: {:?}", openai_message.role);
    println!("✓ Successfully converted universal message to OpenAI format");
}

#[test]
fn test_payload_structure_validation() {
    // Test that our payload snapshots have the expected structure for testing
    let request = load_request_payload().expect("Should load request payload");
    let response = load_non_streaming_response().expect("Should load response payload");

    // Verify request structure for testing
    assert!(!request.model.is_empty());
    assert!(!request.messages.is_empty());

    // Verify response structure for testing
    assert!(!response.id.is_empty());
    assert!(!response.choices.is_empty());

    println!("✓ Payload snapshots are valid for testing");
    println!("Request has {} messages", request.messages.len());
    println!("Response has {} choices", response.choices.len());
}
