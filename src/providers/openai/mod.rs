/*!
OpenAI API provider types.

This module contains complete type definitions for OpenAI's chat completion API
that match the TypeScript SDK exactly, now automatically generated from the official
OpenAI OpenAPI specification.
*/

pub mod convert;
pub mod generated;

// Re-export generated types (official OpenAI API types from OpenAPI spec)
pub use generated::{
    ChatCompletionRequestMessage,
    ChatCompletionResponseMessage,
    CompletionUsage as GeneratedCompletionUsage,
    // Note: CreateChatCompletionRequest, CreateChatCompletionResponse, CreateChatCompletionStreamResponse
    // are embedded in the main OpenaiSchemas struct due to quicktype's union handling
    // TODO: Extract these as separate type aliases
    OpenaiSchemas as GeneratedOpenaiSchemas,
    Tool as ChatCompletionTool,
};
