/*!
Capability detection for different providers.
*/

pub mod detection;
pub mod format;

// Re-export main types and functions
pub use detection::{Capabilities, ProviderCapabilities, detect_capabilities};
pub use format::ProviderFormat;
