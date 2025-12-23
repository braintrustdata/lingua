// Re-export big_serde_json as serde_json so all code can use serde_json:: after importing
// `use crate::serde_json;`. This wrapper isolates the arbitrary_precision feature.
pub use big_serde_json as serde_json;

pub mod capabilities;
pub mod error;
pub mod processing;
pub mod providers;
pub mod universal;
pub mod util;
pub mod validation;

// Re-export key types for external use
pub use capabilities::ProviderFormat;
pub use processing::{
    adapter_for_format, adapters, is_valid_for_format, transform_request, transform_response,
    validate_or_transform, ProviderAdapter, TransformError, TransformResult,
};

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(feature = "python")]
pub mod python;
