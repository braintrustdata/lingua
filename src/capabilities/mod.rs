/*!
Capability detection and management.

This module handles detecting what features each provider supports,
enabling capability-based access to features.
*/

pub mod detection;

pub use detection::{Capabilities, ProviderCapabilities, detect_capabilities};