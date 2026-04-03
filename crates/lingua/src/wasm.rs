use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;

// Import our types and conversion traits
use crate::providers::anthropic::generated as anthropic;
use crate::providers::google::generated as google;
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

fn transform_result_to_js(
    pass_through: bool,
    bytes: bytes::Bytes,
    source_format: Option<crate::capabilities::ProviderFormat>,
) -> Result<JsValue, JsValue> {
    let data_str = String::from_utf8_lossy(&bytes);
    let data =
        js_sys::JSON::parse(&data_str).map_err(|_| JsValue::from_str("Failed to parse JSON"))?;

    let obj = js_sys::Object::new();
    if pass_through {
        js_sys::Reflect::set(&obj, &"passThrough".into(), &JsValue::TRUE)?;
    } else {
        js_sys::Reflect::set(&obj, &"transformed".into(), &JsValue::TRUE)?;
        if let Some(sf) = source_format {
            js_sys::Reflect::set(&obj, &"sourceFormat".into(), &sf.to_string().into())?;
        }
    }
    js_sys::Reflect::set(&obj, &"data".into(), &data)?;
    Ok(obj.into())
}

fn stream_output_chunks_to_js(
    chunks: Vec<crate::processing::stream::StreamOutputChunk>,
) -> Result<JsValue, JsValue> {
    let out = js_sys::Array::new();
    for chunk in chunks {
        let data_str = String::from_utf8_lossy(&chunk.data);
        let data = js_sys::JSON::parse(&data_str)
            .map_err(|_| JsValue::from_str("Failed to parse stream JSON"))?;
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"data".into(), &data)?;
        if let Some(event_type) = chunk.event_type {
            js_sys::Reflect::set(&obj, &"eventType".into(), &event_type.into())?;
        }
        out.push(&obj);
    }
    Ok(out.into())
}

fn string_vec_to_js(values: Vec<String>) -> JsValue {
    let out = js_sys::Array::new();
    for value in values {
        out.push(&value.into());
    }
    out.into()
}

fn stream_output_chunk_from_js(
    data: &str,
    event_type: Option<String>,
) -> crate::processing::stream::StreamOutputChunk {
    crate::processing::stream::StreamOutputChunk {
        data: bytes::Bytes::from(data.to_owned()),
        event_type,
    }
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

/// Convert array of Google Content items to Lingua Messages
#[wasm_bindgen]
pub fn google_contents_to_lingua(value: JsValue) -> Result<JsValue, JsValue> {
    convert_to_lingua::<Vec<google::Content>, Vec<Message>>(value)
}

/// Convert array of Lingua Messages to Google Content items
#[wasm_bindgen]
pub fn lingua_to_google_contents(value: JsValue) -> Result<JsValue, JsValue> {
    convert_from_lingua::<Vec<Message>, Vec<google::Content>>(value)
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

/// Validate a JSON string as a Chat Completions stream chunk
#[wasm_bindgen]
#[cfg(feature = "openai")]
pub fn validate_chat_completions_stream_chunk(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::openai::validate_chat_completions_stream_chunk as validate;
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

/// Validate a JSON string as a Google GenerateContent request
#[wasm_bindgen]
#[cfg(feature = "google")]
pub fn validate_google_request(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::google::validate_google_request as validate;
    validate(json)
        .map(|_| JsValue::NULL)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a JSON string as a Google GenerateContent response
#[wasm_bindgen]
#[cfg(feature = "google")]
pub fn validate_google_response(json: &str) -> Result<JsValue, JsValue> {
    use crate::validation::google::validate_google_response as validate;
    validate(json)
        .map(|_| JsValue::NULL)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

// ============================================================================
// Transform exports
// ============================================================================

/// Transform a request payload to the target format.
///
/// Takes a JSON string and target format, auto-detects the source format,
/// and transforms to the target format.
///
/// Returns an object with either:
/// - `{ passThrough: true, data: ... }` if payload is already valid for target
/// - `{ transformed: true, data: ..., sourceFormat: "..." }` if transformed
#[wasm_bindgen]
pub fn transform_request(
    input: &str,
    target_format: &str,
    model: Option<String>,
) -> Result<JsValue, JsValue> {
    use crate::capabilities::ProviderFormat;
    use crate::processing::transform::{transform_request as transform, TransformResult};
    use bytes::Bytes;

    let target: ProviderFormat = target_format
        .parse()
        .map_err(|_| JsValue::from_str(&format!("Unknown target format: {}", target_format)))?;

    let input_bytes = Bytes::from(input.to_owned());
    let result = transform(input_bytes, target, model.as_deref())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Use JS native JSON.parse to avoid serde_wasm_bindgen serialization issues
    // (Map objects, $serde_json::private::Number from arbitrary_precision)
    let (pass_through, bytes, source_format) = match result {
        TransformResult::PassThrough(bytes) => (true, bytes, None),
        TransformResult::Transformed {
            bytes,
            source_format,
        } => (false, bytes, Some(source_format)),
    };

    let data_str = String::from_utf8_lossy(&bytes);
    let data =
        js_sys::JSON::parse(&data_str).map_err(|_| JsValue::from_str("Failed to parse JSON"))?;

    let obj = js_sys::Object::new();
    if pass_through {
        js_sys::Reflect::set(&obj, &"passThrough".into(), &JsValue::TRUE)?;
    } else {
        js_sys::Reflect::set(&obj, &"transformed".into(), &JsValue::TRUE)?;
        if let Some(sf) = source_format {
            js_sys::Reflect::set(&obj, &"sourceFormat".into(), &sf.to_string().into())?;
        }
    }
    js_sys::Reflect::set(&obj, &"data".into(), &data)?;
    Ok(obj.into())
}

/// Transform a response payload from one format to another.
///
/// Takes a JSON string and target format, auto-detects the source format,
/// and transforms to the target format.
///
/// Returns an object with either:
/// - `{ passThrough: true, data: ... }` if payload is already valid for target
/// - `{ transformed: true, data: ..., sourceFormat: "..." }` if transformed
#[wasm_bindgen]
pub fn transform_response(input: &str, target_format: &str) -> Result<JsValue, JsValue> {
    use crate::capabilities::ProviderFormat;
    use crate::processing::transform::{transform_response as transform, TransformResult};
    use bytes::Bytes;

    let target: ProviderFormat = target_format
        .parse()
        .map_err(|_| JsValue::from_str(&format!("Unknown target format: {}", target_format)))?;

    let input_bytes = Bytes::from(input.to_owned());
    let result = transform(input_bytes, target).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Use JS native JSON.parse to avoid serde_wasm_bindgen serialization issues
    // (Map objects, $serde_json::private::Number from arbitrary_precision)
    let (pass_through, bytes, source_format) = match result {
        TransformResult::PassThrough(bytes) => (true, bytes, None),
        TransformResult::Transformed {
            bytes,
            source_format,
        } => (false, bytes, Some(source_format)),
    };

    transform_result_to_js(pass_through, bytes, source_format)
}

/// Transform a streaming chunk payload from one format to another.
///
/// Takes a JSON string chunk and target format, auto-detects the source format,
/// and transforms to the target format.
///
/// Returns an object with either:
/// - `{ passThrough: true, data: ... }` if chunk is already valid for target
/// - `{ transformed: true, data: ..., sourceFormat: "..." }` if transformed
#[wasm_bindgen]
pub fn transform_stream_chunk(input: &str, target_format: &str) -> Result<JsValue, JsValue> {
    use crate::capabilities::ProviderFormat;
    use crate::processing::transform::{transform_stream_chunk as transform, TransformResult};
    use bytes::Bytes;

    let target: ProviderFormat = target_format
        .parse()
        .map_err(|_| JsValue::from_str(&format!("Unknown target format: {}", target_format)))?;

    let input_bytes = Bytes::from(input.to_owned());
    let result = transform(input_bytes, target).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let (pass_through, bytes, source_format) = match result {
        TransformResult::PassThrough(bytes) => (true, bytes, None),
        TransformResult::Transformed {
            bytes,
            source_format,
        } => (false, bytes, Some(source_format)),
    };

    transform_result_to_js(pass_through, bytes, source_format)
}

#[wasm_bindgen]
pub struct TransformStreamSession {
    inner: crate::processing::stream::StreamTransformSession,
}

#[wasm_bindgen]
impl TransformStreamSession {
    #[wasm_bindgen(constructor)]
    pub fn new(target_format: &str) -> Result<TransformStreamSession, JsValue> {
        let target: crate::capabilities::ProviderFormat = target_format
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Unknown target format: {}", target_format)))?;

        Ok(Self {
            inner: crate::processing::stream::StreamTransformSession::new(target),
        })
    }

    pub fn push(&mut self, input: &str) -> Result<JsValue, JsValue> {
        let chunks = self
            .inner
            .push(bytes::Bytes::from(input.to_owned()))
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        stream_output_chunks_to_js(chunks)
    }

    pub fn finish(&mut self) -> Result<JsValue, JsValue> {
        stream_output_chunks_to_js(self.inner.finish())
    }

    #[wasm_bindgen(js_name = pushSse)]
    pub fn push_sse(&mut self, input: &str) -> Result<JsValue, JsValue> {
        let chunks = self
            .inner
            .push_sse(bytes::Bytes::from(input.to_owned()))
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let strings = chunks
            .into_iter()
            .map(|bytes| String::from_utf8(bytes.to_vec()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(string_vec_to_js(strings))
    }

    #[wasm_bindgen(js_name = finishSse)]
    pub fn finish_sse(&mut self) -> Result<JsValue, JsValue> {
        let strings = self
            .inner
            .finish_sse()
            .into_iter()
            .map(|bytes| String::from_utf8(bytes.to_vec()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(string_vec_to_js(strings))
    }
}

/// Extract model name from request without full transformation.
///
/// This is a fast path for routing decisions that only need the model name.
/// Returns the model string if found, or undefined if not present.
#[wasm_bindgen]
pub fn extract_model(input: &str) -> Option<String> {
    use crate::processing::transform::extract_model as extract;
    extract(input.as_bytes())
}

#[wasm_bindgen]
pub fn format_stream_chunk_as_sse(
    data: &str,
    event_type: Option<String>,
    target_format: &str,
) -> Result<String, JsValue> {
    let target: crate::capabilities::ProviderFormat = target_format
        .parse()
        .map_err(|_| JsValue::from_str(&format!("Unknown target format: {}", target_format)))?;
    let chunk = stream_output_chunk_from_js(data, event_type);
    let bytes = crate::processing::stream::format_stream_chunk_as_sse(&chunk, target);
    String::from_utf8(bytes.to_vec()).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn stream_done_marker(target_format: &str) -> Result<Option<String>, JsValue> {
    let target: crate::capabilities::ProviderFormat = target_format
        .parse()
        .map_err(|_| JsValue::from_str(&format!("Unknown target format: {}", target_format)))?;
    crate::processing::stream::sse_done_marker(target)
        .map(|bytes| String::from_utf8(bytes.to_vec()))
        .transpose()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
