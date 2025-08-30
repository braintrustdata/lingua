/*!
OpenAI API provider types.

This module contains complete type definitions for OpenAI's chat completion API
that match the TypeScript SDK exactly, now automatically generated from the official
OpenAI OpenAPI specification.
*/

pub mod generated;
pub mod request;
pub mod response;

// Re-export generated types (official OpenAI API types from OpenAPI spec)
pub use generated::{
    ChatCompletionRequestMessage, ChatCompletionResponseMessage, ChatCompletionTool,
    CompletionUsage as GeneratedCompletionUsage, CreateChatCompletionRequest,
    CreateChatCompletionResponse, CreateChatCompletionStreamResponse,
};

// Re-export manual types for backwards compatibility (will be deprecated)
pub use request::{
    ChatCompletionAudioParam, ChatCompletionContentPart, ChatCompletionCreateParams,
    ChatCompletionCreateParamsNonStreaming, ChatCompletionCreateParamsStreaming,
    ChatCompletionMessageParam, ChatCompletionModality, ChatCompletionToolChoiceOption,
    FunctionDefinition, MessageContentTextOnly, MessageContentWithParts, MessageContentWithRefusal,
    ReasoningEffort, ResponseFormat, ServiceTier, StopSequences, Verbosity,
};

// Re-export manual response types for backwards compatibility
pub use response::{
    ChatCompletion, ChatCompletionChoice, ChatCompletionChunk, ChatCompletionChunkChoice,
    ChatCompletionDeleted, ChatCompletionMessage, ChatCompletionStoreMessage, CompletionUsage,
    FinishReason,
};
