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
    ChatCompletionCreateParamsNonStreaming, ChatCompletionCreateParamsStreaming,
    ChatCompletionMessageParam, ChatCompletionModality, ChatCompletionTool,
    ChatCompletionToolChoiceOption, FunctionDefinition, MessageContentTextOnly,
    MessageContentWithParts, MessageContentWithRefusal, ReasoningEffort, ResponseFormat,
    ServiceTier, StopSequences, Verbosity,
};

// Re-export main response types for convenience
pub use response::{
    ChatCompletion, ChatCompletionChoice, ChatCompletionChunk, ChatCompletionChunkChoice,
    ChatCompletionDeleted, ChatCompletionMessage, ChatCompletionStoreMessage, CompletionUsage,
    FinishReason,
};
