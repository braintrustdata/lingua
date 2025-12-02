/*!
Capability detection for different providers.
*/

pub mod detection;
pub mod format;
pub mod universal;

// Re-export main types and functions
pub use detection::{detect_capabilities, Capabilities, ProviderCapabilities};
pub use format::ProviderFormat;
pub use universal::UniversalCapabilities;
