// Generated Google AI types from official protobuf files
// Essential types for LLMIR Google AI integration

// This file contains Google AI v1beta types with function calling support
// Generated from: https://raw.githubusercontent.com/googleapis/googleapis/master/google/ai/generativelanguage/v1beta/

/// Request to generate content using Google AI
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenerateContentRequest {
    /// Required. The name of the Model to use for generating the completion.
    #[prost(string, tag = "1")]
    pub model: ::prost::alloc::string::String,

    /// Required. The content of the current conversation with the model.
    #[prost(message, repeated, tag = "2")]
    pub contents: ::prost::alloc::vec::Vec<Content>,

    /// Optional. A list of Tools the Model may use to generate the next response.
    #[prost(message, repeated, tag = "5")]
    pub tools: ::prost::alloc::vec::Vec<Tool>,

    /// Optional. Tool configuration for any Tool specified in the request.
    #[prost(message, optional, tag = "7")]
    pub tool_config: ::core::option::Option<ToolConfig>,

    /// Optional. A list of unique SafetySetting instances for blocking unsafe content.
    #[prost(message, repeated, tag = "3")]
    pub safety_settings: ::prost::alloc::vec::Vec<SafetySetting>,

    /// Optional. Configuration options for model generation and outputs.
    #[prost(message, optional, tag = "4")]
    pub generation_config: ::core::option::Option<GenerationConfig>,
}

/// Response from content generation
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenerateContentResponse {
    /// Candidate responses from the model.
    #[prost(message, repeated, tag = "1")]
    pub candidates: ::prost::alloc::vec::Vec<Candidate>,

    /// Usage metadata about the request and response.
    #[prost(message, optional, tag = "2")]
    pub usage_metadata: ::core::option::Option<UsageMetadata>,
}

/// The base structured datatype containing multi-part content of a message.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Content {
    /// Ordered Parts that constitute a single message.
    #[prost(message, repeated, tag = "1")]
    pub parts: ::prost::alloc::vec::Vec<Part>,

    /// Optional. The producer of the content.
    #[prost(string, optional, tag = "2")]
    pub role: ::core::option::Option<::prost::alloc::string::String>,
}

/// A datatype containing media that is part of a multi-part Content message.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Part {
    /// The part data.
    #[prost(oneof = "part::Data", tags = "2, 3, 4")]
    pub data: ::core::option::Option<part::Data>,
}

/// Nested message and enum types in Part.
pub mod part {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        /// Inline text.
        #[prost(string, tag = "2")]
        Text(::prost::alloc::string::String),
        /// A predicted FunctionCall returned from the model.
        #[prost(message, tag = "3")]
        FunctionCall(super::FunctionCall),
        /// The result output of a FunctionCall.
        #[prost(message, tag = "4")]
        FunctionResponse(super::FunctionResponse),
    }
}

/// A predicted FunctionCall returned from the model that contains function name and arguments.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FunctionCall {
    /// Required. The name of the function to call.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,

    /// Optional. The function parameters and values in JSON object format.
    #[prost(message, optional, tag = "2")]
    pub args: ::core::option::Option<::prost_types::Struct>,
}

/// The result output from a FunctionCall.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FunctionResponse {
    /// Required. The name of the function that was called.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,

    /// Required. The function response in JSON object format.
    #[prost(message, optional, tag = "2")]
    pub response: ::core::option::Option<::prost_types::Struct>,
}

/// Tool details that the model may use to generate response.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Tool {
    /// Optional. A list of FunctionDeclarations available to the model.
    #[prost(message, repeated, tag = "1")]
    pub function_declarations: ::prost::alloc::vec::Vec<FunctionDeclaration>,
}

/// Structured representation of a function declaration.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FunctionDeclaration {
    /// Required. The name of the function.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,

    /// Required. A brief description of the function.
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,

    /// Optional. Describes the parameters to this function in JSON Schema Object format.
    #[prost(message, optional, tag = "3")]
    pub parameters: ::core::option::Option<::prost_types::Struct>,
}

/// The Tool configuration containing parameters for specifying Tool use.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToolConfig {
    /// Optional. Function calling config.
    #[prost(message, optional, tag = "1")]
    pub function_calling_config: ::core::option::Option<FunctionCallingConfig>,
}

/// Configuration for specifying function calling behavior.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct FunctionCallingConfig {
    /// Optional. Specifies the mode in which function calling should execute.
    #[prost(enumeration = "function_calling_config::Mode", tag = "1")]
    pub mode: i32,
}

/// Nested message and enum types in FunctionCallingConfig.
pub mod function_calling_config {
    /// Defines the execution behavior for function calling.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Mode {
        /// Unspecified function calling mode.
        Unspecified = 0,
        /// Default model behavior, model decides to predict function calls or not.
        Auto = 1,
        /// Model is constrained to always predicting function calls only.
        Any = 2,
        /// Model will not predict any function calls.
        None = 3,
    }
}

/// A response candidate generated from the model.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Candidate {
    /// Optional. Generated content returned from the model.
    #[prost(message, optional, tag = "1")]
    pub content: ::core::option::Option<Content>,

    /// Optional. The reason why the model stopped generating tokens.
    #[prost(enumeration = "candidate::FinishReason", tag = "3")]
    pub finish_reason: i32,

    /// List of ratings for the safety of a response candidate.
    #[prost(message, repeated, tag = "5")]
    pub safety_ratings: ::prost::alloc::vec::Vec<SafetyRating>,
}

/// Nested message and enum types in Candidate.
pub mod candidate {
    /// Defines the reason why the model stopped generating tokens.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum FinishReason {
        /// Default value. This value is unused.
        Unspecified = 0,
        /// Natural stop point of the model or provided stop sequence.
        Stop = 1,
        /// The maximum number of tokens as specified in the request was reached.
        MaxTokens = 2,
        /// The candidate content was flagged for safety reasons.
        Safety = 3,
        /// The candidate content was flagged for recitation reasons.
        Recitation = 4,
        /// Unknown reason.
        Other = 5,
    }
}

/// Configuration options for model generation and outputs.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenerationConfig {
    /// Optional. Controls the randomness of the output.
    #[prost(float, optional, tag = "1")]
    pub temperature: ::core::option::Option<f32>,

    /// Optional. The maximum cumulative probability of tokens to consider when sampling.
    #[prost(float, optional, tag = "2")]
    pub top_p: ::core::option::Option<f32>,

    /// Optional. The maximum number of tokens to consider when sampling.
    #[prost(int32, optional, tag = "3")]
    pub top_k: ::core::option::Option<i32>,

    /// Optional. The maximum number of tokens to include in a candidate.
    #[prost(int32, optional, tag = "4")]
    pub max_output_tokens: ::core::option::Option<i32>,
}

/// Safety setting, affecting the safety-blocking behavior.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SafetySetting {
    /// Required. The category for this setting.
    #[prost(enumeration = "HarmCategory", tag = "3")]
    pub category: i32,

    /// Required. Controls the probability threshold at which harm is blocked.
    #[prost(enumeration = "safety_setting::HarmBlockThreshold", tag = "4")]
    pub threshold: i32,
}

/// Nested message and enum types in SafetySetting.
pub mod safety_setting {
    /// Block at and beyond a specified harm probability.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum HarmBlockThreshold {
        /// Threshold is unspecified.
        Unspecified = 0,
        /// Content with NEGLIGIBLE will be allowed.
        BlockLowAndAbove = 1,
        /// Content with NEGLIGIBLE and LOW will be allowed.
        BlockMediumAndAbove = 2,
        /// Content with NEGLIGIBLE, LOW, and MEDIUM will be allowed.
        BlockOnlyHigh = 3,
        /// All content will be allowed.
        BlockNone = 4,
    }
}

/// Safety rating for a piece of content.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SafetyRating {
    /// Required. The category for this rating.
    #[prost(enumeration = "HarmCategory", tag = "3")]
    pub category: i32,

    /// Required. The probability of harm for this content.
    #[prost(enumeration = "safety_rating::HarmProbability", tag = "4")]
    pub probability: i32,

    /// Was this content blocked because of this rating?
    #[prost(bool, tag = "5")]
    pub blocked: bool,
}

/// Nested message and enum types in SafetyRating.
pub mod safety_rating {
    /// The probability that a piece of content is harmful.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum HarmProbability {
        /// Probability is unspecified.
        Unspecified = 0,
        /// Content has a negligible chance of being unsafe.
        Negligible = 1,
        /// Content has a low chance of being unsafe.
        Low = 2,
        /// Content has a medium chance of being unsafe.
        Medium = 3,
        /// Content has a high chance of being unsafe.
        High = 4,
    }
}

/// The category of a rating.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum HarmCategory {
    /// Category is unspecified.
    Unspecified = 0,
    /// Negative or harmful comments targeting identity and/or protected attribute.
    Derogatory = 1,
    /// Content that is rude, disrespectful, or profane.
    Toxicity = 2,
    /// Describes scenarios depicting violence against an individual or group.
    Violence = 3,
    /// Contains references to sexual acts or other lewd content.
    Sexual = 4,
    /// Promotes unchecked medical advice.
    Medical = 5,
    /// Dangerous content that promotes, facilitates, or encourages harmful acts.
    Dangerous = 6,
    /// Harasment content.
    Harassment = 7,
    /// Hate speech and content.
    HateSpeech = 8,
    /// Sexually explicit content.
    SexuallyExplicit = 9,
    /// Dangerous content.
    DangerousContent = 10,
}

/// Metadata on the generation request's token usage.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UsageMetadata {
    /// Number of tokens in the request.
    #[prost(int32, tag = "1")]
    pub prompt_token_count: i32,

    /// Total number of tokens across all the generated response candidates.
    #[prost(int32, tag = "2")]
    pub candidates_token_count: i32,

    /// Total number of tokens in both the request and response.
    #[prost(int32, tag = "3")]
    pub total_token_count: i32,
}
