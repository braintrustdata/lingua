/*!
Universal format definitions - exact port of Vercel AI SDK's LanguageModelV2 structure.

This module provides a 1:1 Rust implementation of the AI SDK message format with:
* LanguageModelV2Message - Role-based messaging (system, user, assistant, tool)
* LanguageModelV2Content - Multi-modal content support (text, files, sources, reasoning, tool calls)
* Exact JSON serialization compatibility with the AI SDK
* Provider metadata and options support
*/

pub mod message;
pub mod ts_export;

#[cfg(test)]
mod message_test;

// Re-export main types for convenience
pub use message::*;
