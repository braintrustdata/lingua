use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

// Import our types and conversion traits
use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::convert::ChatCompletionRequestMessageExt;
use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::universal::{convert::TryFromLLM, Message};

/// Convert Python object to Rust type via JSON
fn py_to_rust<T>(py: Python, value: &PyAny) -> PyResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    // Convert Python object to JSON string
    let json_str = pyo3::types::PyModule::import(py, "json")?
        .getattr("dumps")?
        .call1((value,))?
        .extract::<String>()?;

    // Deserialize from JSON
    serde_json::from_str(&json_str).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to parse input: {}", e))
    })
}

/// Convert Rust type to Python object via JSON
fn rust_to_py<T>(py: Python, value: &T) -> PyResult<PyObject>
where
    T: Serialize,
{
    // Serialize to JSON string
    let json_str = serde_json::to_string(value).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to serialize: {}", e))
    })?;

    // Convert JSON string to Python object
    pyo3::types::PyModule::import(py, "json")?
        .getattr("loads")?
        .call1((json_str,))?
        .extract()
}

/// Generic conversion from provider to Lingua
fn convert_to_lingua<T, U>(py: Python, value: &PyAny) -> PyResult<PyObject>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    let provider_msg: T = py_to_rust(py, value)?;
    let lingua_msg = U::try_from(provider_msg).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Conversion error: {:?}", e))
    })?;
    rust_to_py(py, &lingua_msg)
}

/// Generic conversion from Lingua to provider
fn convert_from_lingua<T, U>(py: Python, value: &PyAny) -> PyResult<PyObject>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    let lingua_msg: T = py_to_rust(py, value)?;
    let provider_msg = U::try_from(lingua_msg).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Conversion error: {:?}", e))
    })?;
    rust_to_py(py, &provider_msg)
}

// ============================================================================
// Conversion functions
// ============================================================================

/// Convert array of Chat Completions messages to Lingua Messages
#[pyfunction]
fn chat_completions_messages_to_lingua(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_to_lingua::<Vec<ChatCompletionRequestMessageExt>, Vec<Message>>(py, value)
}

/// Convert array of Lingua Messages to Chat Completions messages
#[pyfunction]
fn lingua_to_chat_completions_messages(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_from_lingua::<Vec<Message>, Vec<ChatCompletionRequestMessageExt>>(py, value)
}

/// Convert array of Responses API messages to Lingua Messages
#[pyfunction]
fn responses_messages_to_lingua(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_to_lingua::<Vec<openai::InputItem>, Vec<Message>>(py, value)
}

/// Convert array of Lingua Messages to Responses API messages
#[pyfunction]
fn lingua_to_responses_messages(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_from_lingua::<Vec<Message>, Vec<openai::InputItem>>(py, value)
}

/// Convert array of Anthropic messages to Lingua Messages
#[pyfunction]
fn anthropic_messages_to_lingua(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_to_lingua::<Vec<anthropic::InputMessage>, Vec<Message>>(py, value)
}

/// Convert array of Lingua Messages to Anthropic messages
#[pyfunction]
fn lingua_to_anthropic_messages(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_from_lingua::<Vec<Message>, Vec<anthropic::InputMessage>>(py, value)
}

// ============================================================================
// Processing functions
// ============================================================================

/// Deduplicate messages based on role and content
#[pyfunction]
fn deduplicate_messages(py: Python, value: &PyAny) -> PyResult<PyObject> {
    use crate::processing::dedup::deduplicate_messages as dedup;
    use crate::universal::Message;

    // Convert Python value to Vec<Message>
    let messages: Vec<Message> = py_to_rust(py, value)?;

    // Deduplicate
    let deduplicated = dedup(messages);

    // Convert back to Python
    rust_to_py(py, &deduplicated)
}

/// Import messages from spans
#[pyfunction]
fn import_messages_from_spans(py: Python, value: &PyAny) -> PyResult<PyObject> {
    use crate::processing::import::{import_messages_from_spans as import, Span};

    // Convert Python value to Vec<Span>
    let spans: Vec<Span> = py_to_rust(py, value)?;

    // Import messages
    let messages = import(spans);

    // Convert back to Python
    rust_to_py(py, &messages)
}

/// Import and deduplicate messages from spans in a single operation
#[pyfunction]
fn import_and_deduplicate_messages(py: Python, value: &PyAny) -> PyResult<PyObject> {
    use crate::processing::import::{import_and_deduplicate_messages as import_dedup, Span};

    // Convert Python value to Vec<Span>
    let spans: Vec<Span> = py_to_rust(py, value)?;

    // Import and deduplicate messages
    let messages = import_dedup(spans);

    // Convert back to Python
    rust_to_py(py, &messages)
}

// ============================================================================
// Validation functions
// ============================================================================

/// Validate a JSON string as a Chat Completions request
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_chat_completions_request(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::openai::validate_chat_completions_request as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
}

/// Validate a JSON string as a Chat Completions response
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_chat_completions_response(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::openai::validate_chat_completions_response as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
}

/// Validate a JSON string as a Responses API request
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_responses_request(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::openai::validate_responses_request as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
}

/// Validate a JSON string as a Responses API response
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_responses_response(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::openai::validate_responses_response as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
}

/// Validate a JSON string as an OpenAI request
/// @deprecated Use validate_chat_completions_request instead
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_openai_request(py: Python, json: &str) -> PyResult<PyObject> {
    validate_chat_completions_request(py, json)
}

/// Validate a JSON string as an OpenAI response
/// @deprecated Use validate_chat_completions_response instead
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_openai_response(py: Python, json: &str) -> PyResult<PyObject> {
    validate_chat_completions_response(py, json)
}

/// Validate a JSON string as an Anthropic request
#[pyfunction]
#[cfg(feature = "anthropic")]
fn validate_anthropic_request(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::anthropic::validate_anthropic_request as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
}

/// Validate a JSON string as an Anthropic response
#[pyfunction]
#[cfg(feature = "anthropic")]
fn validate_anthropic_response(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::anthropic::validate_anthropic_response as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
}

// ============================================================================
// Transform functions
// ============================================================================

/// Transform a request payload to the target format.
///
/// Takes a JSON string and target format, auto-detects the source format,
/// and transforms to the target format.
///
/// Returns a dict with either:
/// - `{ "pass_through": True, "data": ... }` if payload is already valid for target
/// - `{ "transformed": True, "data": ..., "source_format": "..." }` if transformed
#[pyfunction]
#[pyo3(signature = (json, target_format, model=None))]
fn transform_request(
    py: Python,
    json: &str,
    target_format: &str,
    model: Option<String>,
) -> PyResult<PyObject> {
    use crate::capabilities::ProviderFormat;
    use crate::processing::transform::{transform_request as transform, TransformResult};
    use bytes::Bytes;

    let target: ProviderFormat = target_format.parse().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unknown target format: {}",
            target_format
        ))
    })?;

    let input_bytes = Bytes::from(json.to_owned());
    let result = transform(input_bytes, target, model.as_deref())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    match result {
        TransformResult::PassThrough(bytes) => {
            let data: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to parse result: {}",
                    e
                ))
            })?;
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("pass_through", true)?;
            dict.set_item("data", rust_to_py(py, &data)?)?;
            Ok(dict.into())
        }
        TransformResult::Transformed {
            bytes,
            source_format,
        } => {
            let data: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to parse result: {}",
                    e
                ))
            })?;
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("transformed", true)?;
            dict.set_item("data", rust_to_py(py, &data)?)?;
            dict.set_item("source_format", source_format.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Transform a response payload from one format to another.
///
/// Takes a JSON string and target format, auto-detects the source format,
/// and transforms to the target format.
///
/// Returns a dict with either:
/// - `{ "pass_through": True, "data": ... }` if payload is already valid for target
/// - `{ "transformed": True, "data": ..., "source_format": "..." }` if transformed
#[pyfunction]
fn transform_response(py: Python, json: &str, target_format: &str) -> PyResult<PyObject> {
    use crate::capabilities::ProviderFormat;
    use crate::processing::transform::{transform_response as transform, TransformResult};
    use bytes::Bytes;

    let target: ProviderFormat = target_format.parse().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unknown target format: {}",
            target_format
        ))
    })?;

    let input_bytes = Bytes::from(json.to_owned());
    let result = transform(input_bytes, target)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    match result {
        TransformResult::PassThrough(bytes) => {
            let data: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to parse result: {}",
                    e
                ))
            })?;
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("pass_through", true)?;
            dict.set_item("data", rust_to_py(py, &data)?)?;
            Ok(dict.into())
        }
        TransformResult::Transformed {
            bytes,
            source_format,
        } => {
            let data: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to parse result: {}",
                    e
                ))
            })?;
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("transformed", true)?;
            dict.set_item("data", rust_to_py(py, &data)?)?;
            dict.set_item("source_format", source_format.to_string())?;
            Ok(dict.into())
        }
    }
}

/// Extract model name from request without full transformation.
///
/// This is a fast path for routing decisions that only need the model name.
/// Returns the model string if found, or None if not present.
#[pyfunction]
fn extract_model(json: &str) -> Option<String> {
    use crate::processing::transform::extract_model as extract;
    extract(json.as_bytes())
}

// ============================================================================
// Python module definition
// ============================================================================

/// Python module for Lingua
#[pymodule]
fn _lingua(_py: Python, m: &PyModule) -> PyResult<()> {
    // Conversion functions
    m.add_function(wrap_pyfunction!(chat_completions_messages_to_lingua, m)?)?;
    m.add_function(wrap_pyfunction!(lingua_to_chat_completions_messages, m)?)?;
    m.add_function(wrap_pyfunction!(responses_messages_to_lingua, m)?)?;
    m.add_function(wrap_pyfunction!(lingua_to_responses_messages, m)?)?;
    m.add_function(wrap_pyfunction!(anthropic_messages_to_lingua, m)?)?;
    m.add_function(wrap_pyfunction!(lingua_to_anthropic_messages, m)?)?;

    // Processing functions
    m.add_function(wrap_pyfunction!(deduplicate_messages, m)?)?;
    m.add_function(wrap_pyfunction!(import_messages_from_spans, m)?)?;
    m.add_function(wrap_pyfunction!(import_and_deduplicate_messages, m)?)?;

    // Validation functions
    #[cfg(feature = "openai")]
    {
        m.add_function(wrap_pyfunction!(validate_chat_completions_request, m)?)?;
        m.add_function(wrap_pyfunction!(validate_chat_completions_response, m)?)?;
        m.add_function(wrap_pyfunction!(validate_responses_request, m)?)?;
        m.add_function(wrap_pyfunction!(validate_responses_response, m)?)?;
        m.add_function(wrap_pyfunction!(validate_openai_request, m)?)?;
        m.add_function(wrap_pyfunction!(validate_openai_response, m)?)?;
    }

    #[cfg(feature = "anthropic")]
    {
        m.add_function(wrap_pyfunction!(validate_anthropic_request, m)?)?;
        m.add_function(wrap_pyfunction!(validate_anthropic_response, m)?)?;
    }

    // Transform functions
    m.add_function(wrap_pyfunction!(transform_request, m)?)?;
    m.add_function(wrap_pyfunction!(transform_response, m)?)?;
    m.add_function(wrap_pyfunction!(extract_model, m)?)?;

    Ok(())
}
