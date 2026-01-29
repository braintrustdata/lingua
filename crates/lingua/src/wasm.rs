use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

// Import our types and conversion traits
use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::generated as openai;
use crate::providers::openai::ChatCompletionRequestMessageExt;
use crate::universal::{convert::TryFromLLM, Message};

fn convert_to_lingua<T, U>(value: JsValue) -> Result<JsValue, JsValue>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    // Convert JS value to provider type
    let provider_msg: T = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse input: {}", e)))?;

    // Convert to Lingua type
    let lingua_msg = U::try_from(provider_msg)
        .map_err(|e| JsValue::from_str(&format!("Conversion error: {:?}", e)))?;

    // Convert back to JS value
    serde_wasm_bindgen::to_value(&lingua_msg)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

fn convert_from_lingua<T, U>(value: JsValue) -> Result<JsValue, JsValue>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    // Convert JS value to Lingua type
    let lingua_msg: T = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse input: {}", e)))?;

    // Convert to provider type
    let provider_msg = U::try_from(lingua_msg)
        .map_err(|e| JsValue::from_str(&format!("Conversion error: {:?}", e)))?;

    // Convert back to JS value
    serde_wasm_bindgen::to_value(&provider_msg)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

// ============================================================================
// WASM exports - thin wrappers around generic functions that must be implemented for every
// provider
// ============================================================================

/// Convert array of Chat Completions messages to Lingua Messages
#[wasm_bindgen]
pub fn chat_completions_messages_to_lingua(value: JsValue) -> Result<JsValue, JsValue> {
    convert_to_lingua::<Vec<ChatCompletionRequestMessageExt>, Vec<Message>>(value)
}

/// Convert array of Lingua Messages to Chat Completions messages
#[wasm_bindgen]
pub fn lingua_to_chat_completions_messages(value: JsValue) -> Result<JsValue, JsValue> {
    convert_from_lingua::<Vec<Message>, Vec<ChatCompletionRequestMessageExt>>(value)
}

/// Convert array of Responses API messages to Lingua Messages
#[wasm_bindgen]
pub fn responses_messages_to_lingua(value: JsValue) -> Result<JsValue, JsValue> {
    convert_to_lingua::<Vec<openai::InputItem>, Vec<Message>>(value)
}

/// Convert array of Lingua Messages to Responses API messages
#[wasm_bindgen]
pub fn lingua_to_responses_messages(value: JsValue) -> Result<JsValue, JsValue> {
    convert_from_lingua::<Vec<Message>, Vec<openai::InputItem>>(value)
}

/// Convert array of Anthropic messages to Lingua Messages
#[wasm_bindgen]
pub fn anthropic_messages_to_lingua(value: JsValue) -> Result<JsValue, JsValue> {
    convert_to_lingua::<Vec<anthropic::InputMessage>, Vec<Message>>(value)
}

/// Convert array of Lingua Messages to Anthropic messages
#[wasm_bindgen]
pub fn lingua_to_anthropic_messages(value: JsValue) -> Result<JsValue, JsValue> {
    convert_from_lingua::<Vec<Message>, Vec<anthropic::InputMessage>>(value)
}

// ============================================================================
// Processing exports
// ============================================================================

/// Deduplicate messages based on role and content
#[wasm_bindgen]
pub fn deduplicate_messages(value: JsValue) -> Result<JsValue, JsValue> {
    use crate::processing::dedup::deduplicate_messages as dedup;
    use crate::universal::Message;

    // Convert JS value to Vec<Message>
    let messages: Vec<Message> = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse messages: {}", e)))?;

    // Deduplicate
    let deduplicated = dedup(messages);

    // Convert back to JS value
    serde_wasm_bindgen::to_value(&deduplicated)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// Import messages from spans
#[wasm_bindgen]
pub fn import_messages_from_spans(value: JsValue) -> Result<JsValue, JsValue> {
    use crate::processing::import::{import_messages_from_spans as import, Span};

    // Convert JS value to Vec<Span>
    let spans: Vec<Span> = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse spans: {}", e)))?;

    // Import messages
    let messages = import(spans);

    // Convert back to JS value
    serde_wasm_bindgen::to_value(&messages)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// Import and deduplicate messages from spans in a single operation
#[wasm_bindgen]
pub fn import_and_deduplicate_messages(value: JsValue) -> Result<JsValue, JsValue> {
    use crate::processing::import::{import_and_deduplicate_messages as import_dedup, Span};

    // Convert JS value to Vec<Span>
    let spans: Vec<Span> = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse spans: {}", e)))?;

    // Import and deduplicate messages
    let messages = import_dedup(spans);

    // Convert back to JS value
    serde_wasm_bindgen::to_value(&messages)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

// ============================================================================
// Validation exports
// ============================================================================

/// Validate a JSON string as a Chat Completions request
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_chat_completions_request(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::openai::validate_chat_completions_request as validate;
    validate(json)
        .map(|req| serde_wasm_bindgen::to_value(&req).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Chat Completions response
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_chat_completions_response(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::openai::validate_chat_completions_response as validate;
    validate(json)
        .map(|res| serde_wasm_bindgen::to_value(&res).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Responses API request
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_responses_request(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::openai::validate_responses_request as validate;
    validate(json)
        .map(|req| serde_wasm_bindgen::to_value(&req).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Responses API response
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_responses_response(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::openai::validate_responses_response as validate;
    validate(json)
        .map(|res| serde_wasm_bindgen::to_value(&res).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as an OpenAI request
/// @deprecated Use validate_chat_completions_request instead
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_openai_request(json: &str) -> Result<JsValue, JsValue> {
    validate_chat_completions_request(json)
}

/// Validate a JSON string as an OpenAI response
/// @deprecated Use validate_chat_completions_response instead
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_openai_response(json: &str) -> Result<JsValue, JsValue> {
    validate_chat_completions_response(json)
}

/// Validate a JSON string as an Anthropic request
#[wasm_bindgen]
#[cfg(feature = "anthropic")]
pub fn validate_anthropic_request(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::anthropic::validate_anthropic_request as validate;
    validate(json)
        .map(|req| serde_wasm_bindgen::to_value(&req).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as an Anthropic response
#[wasm_bindgen]
#[cfg(feature = "anthropic")]
pub fn validate_anthropic_response(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::anthropic::validate_anthropic_response as validate;
    validate(json)
        .map(|res| serde_wasm_bindgen::to_value(&res).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Bedrock request
#[wasm_bindgen]
#[cfg(feature = "bedrock")]
pub fn validate_bedrock_request(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::bedrock::validate_bedrock_request as validate;
    validate(json)
        .map(|req| serde_wasm_bindgen::to_value(&req).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Bedrock response
#[wasm_bindgen]
#[cfg(feature = "bedrock")]
pub fn validate_bedrock_response(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::bedrock::validate_bedrock_response as validate;
    validate(json)
        .map(|res| serde_wasm_bindgen::to_value(&res).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Google request (not supported - protobuf types)
#[wasm_bindgen]
#[cfg(feature = "google")]
pub fn validate_google_request(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::google::validate_google_request as validate;
    validate(json)
        .map(|_| JsValue::NULL)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Google response (not supported - protobuf types)
#[wasm_bindgen]
#[cfg(feature = "google")]
pub fn validate_google_response(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::google::validate_google_response as validate;
    validate(json)
        .map(|_| JsValue::NULL)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
