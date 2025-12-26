/*!
Universal format definitions - ModelMessage format from Vercel AI SDK.

This module provides a 1:1 Rust implementation of the AI SDK ModelMessage format with:
* ModelMessage - Role-based messaging (system, user, assistant, tool)
* Content parts - Multi-modal content support (text, images, files, reasoning, tool calls)
* Exact JSON serialization compatibility with the AI SDK
* Provider options support
* Universal transformations (message flattening, system extraction)
*/

pub mod convert;
pub mod message;
pub mod request;
pub mod response;
pub mod stream;
pub mod transform;

// Re-export main types for convenience
pub use message::*;
pub use request::{UniversalParams, UniversalRequest};
pub use response::{FinishReason, UniversalResponse, UniversalUsage};
pub use stream::{UniversalStreamChoice, UniversalStreamChunk};
pub use transform::{extract_system_messages, flatten_consecutive_messages};
