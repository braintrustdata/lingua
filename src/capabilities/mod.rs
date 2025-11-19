/*!
Capability detection for different providers.
*/

pub mod detection;
pub mod universal;

// Re-export main types and functions
pub use detection::{detect_capabilities, Capabilities, ProviderCapabilities};
pub use universal::UniversalCapabilities;
