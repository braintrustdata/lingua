use crate::capabilities::ProviderFormat;
use thiserror::Error;

/// Errors that can occur during conversion between provider formats and universal formats
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("Unsupported input type: {type_info}")]
    UnsupportedInputType { type_info: String },

    #[error("Missing required field: {field}")]
    MissingRequiredField { field: String },

    #[error("Content conversion failed: {reason}")]
    ContentConversionFailed { reason: String },

    #[error("JSON serialization failed for field '{field}': {error}")]
    JsonSerializationFailed { field: String, error: String },

    #[error("Invalid {type_name} value: '{value}'")]
    InvalidEnumValue {
        type_name: &'static str,
        value: String,
    },

    #[error("Tool '{tool_name}' of type '{tool_type}' is not supported by {target_provider}")]
    UnsupportedToolType {
        tool_name: String,
        tool_type: String,
        target_provider: ProviderFormat,
    },
}
