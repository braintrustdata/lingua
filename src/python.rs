use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

// Import our types and conversion traits
use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::generated as openai;
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

/// Generic conversion from provider to LLMIR
fn convert_to_llmir<T, U>(py: Python, value: &PyAny) -> PyResult<PyObject>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    let provider_msg: T = py_to_rust(py, value)?;
    let llmir_msg = U::try_from(provider_msg).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Conversion error: {:?}", e))
    })?;
    rust_to_py(py, &llmir_msg)
}

/// Generic conversion from LLMIR to provider
fn convert_from_llmir<T, U>(py: Python, value: &PyAny) -> PyResult<PyObject>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    let llmir_msg: T = py_to_rust(py, value)?;
    let provider_msg = U::try_from(llmir_msg).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Conversion error: {:?}", e))
    })?;
    rust_to_py(py, &provider_msg)
}

// ============================================================================
// Conversion functions
// ============================================================================

/// Convert array of OpenAI messages to LLMIR Messages
#[pyfunction]
fn openai_messages_to_llmir(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_to_llmir::<Vec<openai::InputItem>, Vec<Message>>(py, value)
}

/// Convert array of LLMIR Messages to OpenAI messages
#[pyfunction]
fn llmir_to_openai_messages(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_from_llmir::<Vec<Message>, Vec<openai::InputItem>>(py, value)
}

/// Convert array of Anthropic messages to LLMIR Messages
#[pyfunction]
fn anthropic_messages_to_llmir(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_to_llmir::<Vec<anthropic::InputMessage>, Vec<Message>>(py, value)
}

/// Convert array of LLMIR Messages to Anthropic messages
#[pyfunction]
fn llmir_to_anthropic_messages(py: Python, value: &PyAny) -> PyResult<PyObject> {
    convert_from_llmir::<Vec<Message>, Vec<anthropic::InputMessage>>(py, value)
}

// ============================================================================
// Validation functions
// ============================================================================

/// Validate a JSON string as an OpenAI request
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_openai_request(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::openai::validate_openai_request as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
}

/// Validate a JSON string as an OpenAI response
#[pyfunction]
#[cfg(feature = "openai")]
fn validate_openai_response(py: Python, json: &str) -> PyResult<PyObject> {
    use crate::validation::openai::validate_openai_response as validate;
    let result = validate(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    rust_to_py(py, &result)
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
// Python module definition
// ============================================================================

/// Python module for LLMIR
#[pymodule]
fn _llmir(_py: Python, m: &PyModule) -> PyResult<()> {
    // Conversion functions
    m.add_function(wrap_pyfunction!(openai_messages_to_llmir, m)?)?;
    m.add_function(wrap_pyfunction!(llmir_to_openai_messages, m)?)?;
    m.add_function(wrap_pyfunction!(anthropic_messages_to_llmir, m)?)?;
    m.add_function(wrap_pyfunction!(llmir_to_anthropic_messages, m)?)?;

    // Validation functions
    #[cfg(feature = "openai")]
    {
        m.add_function(wrap_pyfunction!(validate_openai_request, m)?)?;
        m.add_function(wrap_pyfunction!(validate_openai_response, m)?)?;
    }

    #[cfg(feature = "anthropic")]
    {
        m.add_function(wrap_pyfunction!(validate_anthropic_request, m)?)?;
        m.add_function(wrap_pyfunction!(validate_anthropic_response, m)?)?;
    }

    Ok(())
}
