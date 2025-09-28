use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

// Import our types and conversion traits
use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::generated as openai;
use crate::universal::{convert::TryFromLLM, Message};

fn convert_to_llmir<T, U>(value: JsValue) -> Result<JsValue, JsValue>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    // Convert JS value to provider type
    let provider_msg: T = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse input: {}", e)))?;

    // Convert to LLMIR type
    let llmir_msg = U::try_from(provider_msg)
        .map_err(|e| JsValue::from_str(&format!("Conversion error: {:?}", e)))?;

    // Convert back to JS value
    serde_wasm_bindgen::to_value(&llmir_msg)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

fn convert_from_llmir<T, U>(value: JsValue) -> Result<JsValue, JsValue>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    // Convert JS value to LLMIR type
    let llmir_msg: T = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse input: {}", e)))?;

    // Convert to provider type
    let provider_msg = U::try_from(llmir_msg)
        .map_err(|e| JsValue::from_str(&format!("Conversion error: {:?}", e)))?;

    // Convert back to JS value
    serde_wasm_bindgen::to_value(&provider_msg)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

// ============================================================================
// WASM exports - thin wrappers around generic functions that must be implemented for every
// provider
// ============================================================================

/// Convert OpenAI ChatCompletionRequestMessage to LLMIR Message
#[wasm_bindgen]
pub fn openai_message_to_llmir(value: JsValue) -> Result<JsValue, JsValue> {
    convert_to_llmir::<openai::ChatCompletionRequestMessage, Message>(value)
}

/// Convert LLMIR Message to OpenAI ChatCompletionRequestMessage
#[wasm_bindgen]
pub fn llmir_to_openai_message(value: JsValue) -> Result<JsValue, JsValue> {
    convert_from_llmir::<Message, openai::ChatCompletionRequestMessage>(value)
}

/// Convert array of OpenAI InputItems to LLMIR Messages
#[wasm_bindgen]
pub fn openai_input_items_to_llmir(value: JsValue) -> Result<JsValue, JsValue> {
    convert_to_llmir::<Vec<openai::InputItem>, Vec<Message>>(value)
}

/// Convert Anthropic InputMessage to LLMIR Message
#[wasm_bindgen]
pub fn anthropic_message_to_llmir(value: JsValue) -> Result<JsValue, JsValue> {
    convert_to_llmir::<anthropic::InputMessage, Message>(value)
}

/// Convert LLMIR Message to Anthropic InputMessage
#[wasm_bindgen]
pub fn llmir_to_anthropic_message(value: JsValue) -> Result<JsValue, JsValue> {
    convert_from_llmir::<Message, anthropic::InputMessage>(value)
}
