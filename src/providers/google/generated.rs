// Generated Google AI types from official protobuf files
// Essential types for Elmir Google AI integration

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

// Essential support types
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenerateContentResponse {
    #[prost(message, repeated, tag = "1")]
    pub candidates: ::prost::alloc::vec::Vec<Candidate>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Candidate {
    #[prost(message, optional, tag = "1")]
    pub content: ::core::option::Option<Content>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenerationConfig {
    #[prost(float, optional, tag = "1")]
    pub temperature: ::core::option::Option<f32>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SafetySetting {
    #[prost(enumeration = "safety_setting::HarmBlockThreshold", tag = "4")]
    pub threshold: i32,
}

pub mod safety_setting {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum HarmBlockThreshold {
        Unspecified = 0,
        BlockLowAndAbove = 1,
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ToolConfig {
    #[prost(message, optional, tag = "1")]
    pub function_calling_config: ::core::option::Option<FunctionCallingConfig>,
}

#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct FunctionCallingConfig {
    #[prost(enumeration = "function_calling_config::Mode", tag = "1")]
    pub mode: i32,
}

pub mod function_calling_config {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Mode {
        Unspecified = 0,
        Auto = 1,
        Any = 2,
        None = 3,
    }
}
