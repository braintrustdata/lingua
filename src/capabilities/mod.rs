/*!
Capability detection for different providers.
*/

pub mod detection;

// Re-export main types and functions
pub use detection::{Capabilities, ProviderCapabilities, detect_capabilities};