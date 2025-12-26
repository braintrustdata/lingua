//! Bridge functions for converting between standard serde_json and lingua's big_serde_json.
//!
//! Lingua uses a custom serde_json with arbitrary_precision enabled (big_serde_json)
//! to handle large numbers in JSON payloads. This module provides conversion functions
//! between the two JSON value types.

use crate::serde_json as lingua_json;
use ::serde_json::Value;

/// Convert lingua's serde_json::Value to standard serde_json::Value.
///
/// Uses efficient byte-level serialization when possible, with fallback to
/// recursive conversion for edge cases.
pub fn lingua_value_to_serde(value: lingua_json::Value) -> Value {
    if let Ok(bytes) = lingua_json::to_vec(&value) {
        if let Ok(converted) = ::serde_json::from_slice(&bytes) {
            return converted;
        }
    }

    match value {
        lingua_json::Value::Null => Value::Null,
        lingua_json::Value::Bool(b) => Value::Bool(b),
        lingua_json::Value::Number(n) => {
            ::serde_json::from_str(&n.to_string()).unwrap_or(Value::Null)
        }
        lingua_json::Value::String(s) => Value::String(s),
        lingua_json::Value::Array(items) => {
            Value::Array(items.into_iter().map(lingua_value_to_serde).collect())
        }
        lingua_json::Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, lingua_value_to_serde(v)))
                .collect(),
        ),
    }
}

/// Convert standard serde_json::Value to lingua's serde_json::Value.
///
/// Uses efficient byte-level serialization when possible, with fallback to
/// recursive conversion for edge cases.
pub fn serde_value_to_lingua(value: Value) -> lingua_json::Value {
    if let Ok(bytes) = ::serde_json::to_vec(&value) {
        if let Ok(converted) = lingua_json::from_slice(&bytes) {
            return converted;
        }
    }

    match value {
        Value::Null => lingua_json::Value::Null,
        Value::Bool(b) => lingua_json::Value::Bool(b),
        Value::Number(n) => {
            lingua_json::from_str(&n.to_string()).unwrap_or(lingua_json::Value::Null)
        }
        Value::String(s) => lingua_json::Value::String(s),
        Value::Array(items) => {
            lingua_json::Value::Array(items.into_iter().map(serde_value_to_lingua).collect())
        }
        Value::Object(map) => lingua_json::Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, serde_value_to_lingua(v)))
                .collect(),
        ),
    }
}
