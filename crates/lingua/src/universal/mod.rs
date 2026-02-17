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
pub mod defaults;
pub mod message;
pub mod reasoning;
pub mod request;
pub mod response;
pub mod response_format;
pub mod stream;
pub mod tool_choice;
pub mod tools;
pub mod transform;

// Re-export main types for convenience
pub use defaults::*;
pub use message::*;
pub use request::{
    parse_stop_sequences, JsonSchemaConfig, ReasoningCanonical, ReasoningConfig, ReasoningEffort,
    ResponseFormatConfig, ResponseFormatType, SummaryMode, ToolChoiceConfig, ToolChoiceMode,
    UniversalParams, UniversalRequest,
};
pub use response::{FinishReason, UniversalResponse, UniversalUsage};
pub use stream::{
    UniversalReasoningDelta, UniversalStreamChoice, UniversalStreamChunk, UniversalStreamDelta,
    UniversalToolCallDelta, UniversalToolFunctionDelta,
};
pub use tools::{
    tools_to_openai_chat_value, tools_to_responses_value, UniversalTool, UniversalToolType,
};
pub use transform::{extract_system_messages, flatten_consecutive_messages};
