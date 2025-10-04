use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

// Import our types and conversion traits
use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::generated as openai;
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
    convert_to_lingua::<Vec<openai::ChatCompletionRequestMessage>, Vec<Message>>(value)
}

/// Convert array of Lingua Messages to Chat Completions messages
#[wasm_bindgen]
pub fn lingua_to_chat_completions_messages(value: JsValue) -> Result<JsValue, JsValue> {
    convert_from_lingua::<Vec<Message>, Vec<openai::ChatCompletionRequestMessage>>(value)
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
// Validation exports
// ============================================================================

/// Validate a JSON string as an OpenAI request
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_openai_request(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::openai::validate_openai_request as validate;
    validate(json)
        .map(|req| serde_wasm_bindgen::to_value(&req).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as an OpenAI response
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_openai_response(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::openai::validate_openai_response as validate;
    validate(json)
        .map(|res| serde_wasm_bindgen::to_value(&res).unwrap())
        .map_err(|e| JsValue::from_str(&e.to_string()))
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
