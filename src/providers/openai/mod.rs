/*!
OpenAI API provider types.

This module contains complete type definitions for OpenAI's chat completion API
that match the TypeScript SDK v5.16.0 exactly.
*/

pub mod request;
pub mod response;

// Re-export main types for convenience
pub use request::{
    ChatCompletionCreateParams,
    ChatCompletionMessageParam,
    MessageContent,
    ChatCompletionContentPart,
    ChatCompletionTool,
    ChatCompletionToolChoiceOption,
    ResponseFormat,
    ServiceTier,
    StopSequences,
};

pub use response::{
    ChatCompletion,
    ChatCompletionChoice,
    ChatCompletionMessage,
    ChatCompletionChunk,
    ChatCompletionChunkChoice,
    CompletionUsage,
    FinishReason,
};