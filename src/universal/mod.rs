/*!
Universal format definitions - ModelMessage format from Vercel AI SDK.

This module provides a 1:1 Rust implementation of the AI SDK ModelMessage format with:
* ModelMessage - Role-based messaging (system, user, assistant, tool)
* Content parts - Multi-modal content support (text, images, files, reasoning, tool calls)
* Exact JSON serialization compatibility with the AI SDK
* Provider options support
*/

pub mod message;
pub mod ts_export;

#[cfg(test)]
mod message_test;

// Re-export main types for convenience
pub use message::*;
