/*!
OpenAI API provider types.

This module contains complete type definitions for OpenAI's chat completion API
that match the TypeScript SDK exactly, now automatically generated from the official
OpenAI OpenAPI specification.
*/

pub mod adapter;
pub mod capabilities;
pub mod convert;
pub mod detect;
pub mod generated;
pub(crate) mod import;
pub mod params;
pub mod responses_adapter;
pub(crate) mod tool_parsing;

// Re-export adapters and transformations
pub use adapter::OpenAIAdapter;
pub use responses_adapter::ResponsesAdapter;

#[cfg(test)]
pub mod test_responses;

#[cfg(test)]
pub mod test_chat_completions;

// Re-export detection functions
pub use detect::{try_parse_openai, try_parse_responses, DetectionError};

// Re-export capability functions
pub use capabilities::model_needs_transforms;

// Re-export conversion functions and extension types
pub use convert::{
    universal_to_responses_input, ChatCompletionRequestMessageExt, ChatCompletionResponseMessageExt,
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
