/*!
Capability detection for different providers.
*/

pub mod detection;

// Re-export main types and functions
pub use detection::{detect_capabilities, Capabilities, ProviderCapabilities};
