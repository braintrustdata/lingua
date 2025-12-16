pub mod catalog;
pub mod dedup;
pub mod detect;
pub mod detector;
pub mod import;
pub mod transform;

pub use catalog::{catalog_lookup, set_catalog_lookup};
pub use dedup::deduplicate_messages;
pub use detect::{parse, parse_from_str, DetectedPayload, DetectionError, TypedPayload};
pub use detector::FormatDetector;
pub use import::{import_and_deduplicate_messages, import_messages_from_spans, Span};
pub use transform::{is_valid_for_format, validate_or_transform, TransformError, TransformResult};

// Re-export payload wrappers from provider modules (feature-gated)
#[cfg(feature = "bedrock")]
pub use crate::providers::bedrock::BedrockPayload;
#[cfg(feature = "google")]
pub use crate::providers::google::GooglePayload;
