/*!
Format detection utilities.

This module previously provided `parse()` and `TypedPayload` for strongly-typed
payload detection, but these have been removed as they were unused.

Format detection is now handled by `transform.rs` via struct-based validation
(`is_valid_for_format`, `validate_or_transform`).
*/

use thiserror::Error;

/// Errors that can occur during payload detection.
///
/// Note: Each provider module also defines its own `DetectionError` type
/// for provider-specific parsing errors.
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("JSON parsing failed: {0}")]
    JsonParseFailed(String),

    #[error("Invalid payload structure: {0}")]
    InvalidPayload(String),

    #[error("Unable to determine payload format")]
    UnableToDetermine,
}
