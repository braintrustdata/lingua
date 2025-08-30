/*!
Anthropic API provider types.

This module contains type definitions for Anthropic's messages API
automatically generated from the unofficial OpenAPI specification.
*/

pub mod generated;

// Re-export generated types (Anthropic API types from OpenAPI spec)
pub use generated::{
    ContentBlock, CreateMessageParams, InputMessage, Message, RequestTextBlock, ResponseTextBlock,
    Tool, ToolChoice, Usage,
};
