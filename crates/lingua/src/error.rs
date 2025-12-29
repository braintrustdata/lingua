use thiserror::Error;

/// Errors that can occur during conversion between provider formats and universal formats
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("Unsupported input type: {type_info}")]
    UnsupportedInputType { type_info: String },

    #[error("Missing required field: {field}")]
    MissingRequiredField { field: String },

    #[error("Invalid role: {role}")]
    InvalidRole { role: String },

    #[error("Content conversion failed: {reason}")]
    ContentConversionFailed { reason: String },

    #[error("JSON serialization failed for field '{field}': {error}")]
    JsonSerializationFailed { field: String, error: String },
}
