// Re-export big_serde_json as serde_json so all code can use serde_json:: after importing
// `use crate::serde_json;`. This wrapper isolates the arbitrary_precision feature.
pub use big_serde_json as serde_json;

pub mod bridge;
pub mod capabilities;
pub mod error;
mod extraction;
pub mod processing;
pub mod providers;
pub mod universal;
pub mod util;
pub mod validation;

pub use extraction::extract_model_from_body;

// Re-export key types for external use
pub use capabilities::ProviderFormat;
pub use processing::{
    apply_provider_defaults, from_universal_request, from_universal_response, is_valid_for_format,
    parse_stream_event, sanitize_payload, to_universal_request, to_universal_response,
    transform_request, transform_response, transform_stream_chunk, validate_or_transform,
    ParsedStreamEvent, TransformError, TransformResult,
};
// Re-export OpenAI error type for error handling
pub use providers::openai::OpenAITransformError;
pub use universal::{
    FinishReason, Message, UniversalParams, UniversalRequest, UniversalResponse,
    UniversalStreamChoice, UniversalStreamChunk, UniversalUsage,
};

// Bridge functions for converting between standard serde_json and lingua's serde_json (big_serde_json)
pub use bridge::{lingua_value_to_serde, serde_value_to_lingua};

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(feature = "python")]
pub mod python;
