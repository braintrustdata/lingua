/*!
OpenAI API provider types.

This module contains complete type definitions for OpenAI's chat completion API
that match the TypeScript SDK exactly, extracted from the latest version.
*/

pub mod request;
pub mod response;

// Re-export main request types for convenience
pub use request::{
    ChatCompletionAudioParam, ChatCompletionContentPart, ChatCompletionCreateParams,
    ChatCompletionCreateParamsBase, ChatCompletionCreateParamsNonStreaming,
    ChatCompletionCreateParamsStreaming, ChatCompletionMessageParam, ChatCompletionTool,
    ChatCompletionToolChoiceOption, FunctionDefinition, MessageContent, MessageContentWithParts,
    MessageContentWithRefusal, ResponseFormat, ServiceTier, StopSequences,
};

// Re-export main response types for convenience
pub use response::{
    ChatCompletion, ChatCompletionChoice, ChatCompletionChunk, ChatCompletionChunkChoice,
    ChatCompletionMessage, CompletionUsage, FinishReason,
};
