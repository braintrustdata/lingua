// Re-export big_serde_json as serde_json so that all code can use serde_json:: after importing
// `use crate::serde_json;`. This wrapper isolates the arbitrary_precision feature.

pub use big_serde_json as serde_json;

// Re-export bytes::Bytes for convenience - transform functions use Bytes in/out
pub use bytes::Bytes;

pub mod capabilities;
pub mod error;
mod extraction;
pub mod processing;
pub mod providers;
pub mod universal;
pub mod util;
pub mod validation;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(feature = "python")]
pub mod python;

// ============================================================================
// Root-level re-exports for router integration
// ============================================================================

// Re-export extraction functions
pub use extraction::{extract_request_hints, RequestHints};

// Re-export capabilities
pub use capabilities::ProviderFormat;

// Re-export key processing functions (bytes-based API)
pub use processing::{
    extract_model, parse_stream_event, response_to_universal, sanitize_payload, transform_request,
    transform_response, transform_stream_chunk, ParsedStreamEvent, TransformError, TransformResult,
};

// Re-export universal types
pub use universal::{
    FinishReason, Message, UniversalParams, UniversalRequest, UniversalResponse,
    UniversalStreamChoice, UniversalStreamChunk, UniversalUsage,
};
