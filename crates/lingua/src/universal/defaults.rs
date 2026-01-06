//! Default/placeholder values for transformed responses.
//!
//! These constants are used when converting between provider formats
//! where the source format doesn't include certain fields.

/// Placeholder for missing model names.
pub const PLACEHOLDER_MODEL: &str = "transformed";

/// Placeholder for missing response IDs.
/// Provider adapters prefix as needed (e.g., "msg_" + PLACEHOLDER_ID).
pub const PLACEHOLDER_ID: &str = "transformed";

/// Empty JSON object string for missing tool arguments.
pub const EMPTY_OBJECT_STR: &str = "{}";

/// Default refusal message text.
pub const REFUSAL_TEXT: &str = "Content was refused";

/// Default image MIME type.
pub const DEFAULT_MIME_TYPE: &str = "image/jpeg";
