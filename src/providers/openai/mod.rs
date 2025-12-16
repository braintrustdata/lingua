/*!
OpenAI API provider types.

This module contains complete type definitions for OpenAI's chat completion API
that match the TypeScript SDK exactly, now automatically generated from the official
OpenAI OpenAPI specification.
*/

pub mod capabilities;
pub mod convert;
pub mod detect;
pub mod generated;
pub mod transformations;

#[cfg(test)]
pub mod test_responses;

#[cfg(test)]
pub mod test_chat_completions;

#[cfg(test)]
pub mod test_transformations;

// Re-export detection functions and detector
pub use detect::{
    is_openai_format, is_openai_format_value, try_parse_openai, DetectionError, OpenAIDetector,
};

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
