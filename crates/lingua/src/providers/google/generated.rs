// Generated Google AI types from Discovery JSON spec
// Essential types for Lingua Google AI integration
#![allow(clippy::large_enum_variant)]
#![allow(clippy::doc_lazy_continuation)]

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::google_schemas;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: google_schemas = serde_json::from_str(&json).unwrap();
// }

use crate::serde_json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

fn deserialize_optional_i64_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum I64String {
        String(String),
        I64(i64),
        U64(u64),
    }

    Ok(
        Option::<I64String>::deserialize(deserializer)?.map(|value| match value {
            I64String::String(value) => value,
            I64String::I64(value) => value.to_string(),
            I64String::U64(value) => value.to_string(),
        }),
    )
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct GoogleSchemas {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<GenerateContentRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<GenerateContentResponse>,
}

/// Request to generate a completion from the model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GenerateContentRequest {
    /// Optional. The name of the content [cached](https://ai.google.dev/gemini-api/docs/caching)
    /// to use as context to serve the prediction. Format: `cachedContents/{cachedContent}`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
    /// Required. The content of the current conversation with the model. For single-turn
    /// queries, this is a single instance. For multi-turn queries like
    /// [chat](https://ai.google.dev/gemini-api/docs/text-generation#chat), this is a repeated
    /// field that contains the conversation history and the latest request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<Content>>,
    /// Optional. Configuration options for model generation and outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    /// Required. The name of the `Model` to use for generating the completion. Format:
    /// `models/{model}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional. A list of unique `SafetySetting` instances for blocking unsafe content. This
    /// will be enforced on the `GenerateContentRequest.contents` and
    /// `GenerateContentResponse.candidates`. There should not be more than one setting for each
    /// `SafetyCategory` type. The API will block any contents and responses that fail to meet
    /// the thresholds set by these settings. This list overrides the default settings for each
    /// `SafetyCategory` specified in the safety_settings. If there is no `SafetySetting` for a
    /// given `SafetyCategory` provided in the list, the API will use the default safety setting
    /// for that category. Harm categories HARM_CATEGORY_HATE_SPEECH,
    /// HARM_CATEGORY_SEXUALLY_EXPLICIT, HARM_CATEGORY_DANGEROUS_CONTENT,
    /// HARM_CATEGORY_HARASSMENT, HARM_CATEGORY_CIVIC_INTEGRITY are supported. Refer to the
    /// [guide](https://ai.google.dev/gemini-api/docs/safety-settings) for detailed information
    /// on available safety settings. Also refer to the [Safety
    /// guidance](https://ai.google.dev/gemini-api/docs/safety-guidance) to learn how to
    /// incorporate safety considerations in your AI applications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    /// Optional. The service tier of the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    /// Optional. Configures the logging behavior for a given request. If set, it takes
    /// precedence over the project-level logging config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    /// Optional. Developer set [system
    /// instruction(s)](https://ai.google.dev/gemini-api/docs/system-instructions). Currently,
    /// text only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    /// Optional. Tool configuration for any `Tool` specified in the request. Refer to the
    /// [Function calling
    /// guide](https://ai.google.dev/gemini-api/docs/function-calling#function_calling_mode) for
    /// a usage example.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    /// Optional. A list of `Tools` the `Model` may use to generate the next response. A `Tool`
    /// is a piece of code that enables the system to interact with external systems to perform
    /// an action, or set of actions, outside of knowledge and scope of the `Model`. Supported
    /// `Tool`s are `Function` and `code_execution`. Refer to the [Function
    /// calling](https://ai.google.dev/gemini-api/docs/function-calling) and the [Code
    /// execution](https://ai.google.dev/gemini-api/docs/code-execution) guides to learn more.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

/// The base structured datatype containing multi-part content of a message. A `Content`
/// includes a `role` field designating the producer of the `Content` and a `parts` field
/// containing multi-part data that contains the content of the message turn.
///
/// Optional. Developer set [system
/// instruction(s)](https://ai.google.dev/gemini-api/docs/system-instructions). Currently,
/// text only.
///
/// Output only. Generated content returned from the model.
///
/// Grounding source content that makes up this attribution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "google/")]
pub struct Content {
    /// Ordered `Parts` that constitute a single message. Parts may have different MIME types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<Part>>,
    /// Optional. The producer of the content. Must be either 'user' or 'model'. Useful to set
    /// for multi-turn conversations, otherwise can be left blank or unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// A datatype containing media that is part of a multi-part `Content` message. A `Part`
/// consists of data which has an associated datatype. A `Part` can only contain one of the
/// accepted types in `Part.data`. A `Part` must have a fixed IANA MIME type identifying the
/// type and subtype of the media if the `inline_data` field is filled with raw bytes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Part {
    /// Result of executing the `ExecutableCode`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution_result: Option<CodeExecutionResult>,
    /// Code generated by the model that is meant to be executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_code: Option<ExecutableCode>,
    /// URI based data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<FileData>,
    /// A predicted `FunctionCall` returned from the model that contains a string representing
    /// the `FunctionDeclaration.name` with the arguments and their values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    /// The result output of a `FunctionCall` that contains a string representing the
    /// `FunctionDeclaration.name` and a structured JSON object containing any output from the
    /// function is used as context to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
    /// Inline media bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<Blob>,
    /// Optional. Media resolution for the input media.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_resolution: Option<MediaResolution>,
    /// Custom metadata associated with the Part. Agents using genai.Part as content
    /// representation may need to keep track of the additional information. For example it can
    /// be name of a file/source from which the Part originates or a way to multiplex multiple
    /// Part streams.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_metadata: Option<serde_json::Map<String, serde_json::Value>>,
    /// Inline text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Optional. Indicates if the part is thought from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<bool>,
    /// Optional. An opaque signature for the thought so it can be reused in subsequent requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
    /// Server-side tool call. This field is populated when the model predicts a tool invocation
    /// that should be executed on the server. The client is expected to echo this message back
    /// to the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCall>,
    /// The output from a server-side `ToolCall` execution. This field is populated by the client
    /// with the results of executing the corresponding `ToolCall`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_response: Option<ToolResponse>,
    /// Optional. Video metadata. The metadata should only be specified while the video data is
    /// presented in inline_data or file_data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_metadata: Option<VideoMetadata>,
}

/// Result of executing the `ExecutableCode`.
///
/// Result of executing the `ExecutableCode`. Generated only when the `CodeExecution` tool is
/// used.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct CodeExecutionResult {
    /// Optional. The identifier of the `ExecutableCode` part this result is for. Only populated
    /// if the corresponding `ExecutableCode` has an id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Required. Outcome of the code execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    /// Optional. Contains stdout when code execution is successful, stderr or other description
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// Required. Outcome of the code execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Outcome {
    #[serde(rename = "OUTCOME_DEADLINE_EXCEEDED")]
    OutcomeDeadlineExceeded,
    #[serde(rename = "OUTCOME_FAILED")]
    OutcomeFailed,
    #[serde(rename = "OUTCOME_OK")]
    OutcomeOk,
    #[serde(rename = "OUTCOME_UNSPECIFIED")]
    OutcomeUnspecified,
}

/// Code generated by the model that is meant to be executed.
///
/// Code generated by the model that is meant to be executed, and the result returned to the
/// model. Only generated when using the `CodeExecution` tool, in which the code will be
/// automatically executed, and a corresponding `CodeExecutionResult` will also be generated.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct ExecutableCode {
    /// Required. The code to be executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional. Unique identifier of the `ExecutableCode` part. The server returns the
    /// `CodeExecutionResult` with the matching `id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Required. Programming language of the `code`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
}

/// Required. Programming language of the `code`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Language {
    #[serde(rename = "LANGUAGE_UNSPECIFIED")]
    LanguageUnspecified,
    Python,
}

/// URI based data.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct FileData {
    /// Required. URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_uri: Option<String>,
    /// Optional. The IANA standard MIME type of the source data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// A predicted `FunctionCall` returned from the model that contains a string representing
/// the `FunctionDeclaration.name` with the arguments and their values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct FunctionCall {
    /// Optional. The function parameters and values in JSON object format.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Map<String, serde_json::Value>>,
    /// Optional. Unique identifier of the function call. If populated, the client to execute the
    /// `function_call` and return the response with the matching `id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Required. The name of the function to call. Must be a-z, A-Z, 0-9, or contain underscores
    /// and dashes, with a maximum length of 128.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The result output of a `FunctionCall` that contains a string representing the
/// `FunctionDeclaration.name` and a structured JSON object containing any output from the
/// function is used as context to the model.
///
/// The result output from a `FunctionCall` that contains a string representing the
/// `FunctionDeclaration.name` and a structured JSON object containing any output from the
/// function is used as context to the model. This should contain the result of
/// a`FunctionCall` made based on model prediction.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct FunctionResponse {
    /// Optional. The identifier of the function call this response is for. Populated by the
    /// client to match the corresponding function call `id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Required. The name of the function to call. Must be a-z, A-Z, 0-9, or contain underscores
    /// and dashes, with a maximum length of 128.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional. Ordered `Parts` that constitute a function response. Parts may have different
    /// IANA MIME types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<FunctionResponsePart>>,
    /// Required. The function response in JSON object format. Callers can use any keys of their
    /// choice that fit the function's syntax to return the function output, e.g. "output",
    /// "result", etc. In particular, if the function call failed to execute, the response can
    /// have an "error" key to return error details to the model. Multimedia can be included by
    /// using a subobject containing a single "$ref" key whose value is the
    /// `inline_data.display_name` of a `FunctionResponsePart` holding the multimedia. See
    /// https://ai.google.dev/gemini-api/docs/function-calling#multimodal.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Map<String, serde_json::Value>>,
    /// Optional. Specifies how the response should be scheduled in the conversation. Only
    /// applicable to NON_BLOCKING function calls, is ignored otherwise. Defaults to WHEN_IDLE.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<Scheduling>,
    /// Optional. Signals that function call continues, and more responses will be returned,
    /// turning the function call into a generator. Is only applicable to NON_BLOCKING function
    /// calls, is ignored otherwise. If set to false, future responses will not be considered. It
    /// is allowed to return empty `response` with `will_continue=False` to signal that the
    /// function call is finished. This may still trigger the model generation. To avoid
    /// triggering the generation and finish the function call, additionally set `scheduling` to
    /// `SILENT`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub will_continue: Option<bool>,
}

/// A datatype containing media that is part of a `FunctionResponse` message. A
/// `FunctionResponsePart` consists of data which has an associated datatype. A
/// `FunctionResponsePart` can only contain one of the accepted types in
/// `FunctionResponsePart.data`. A `FunctionResponsePart` must have a fixed IANA MIME type
/// identifying the type and subtype of the media if the `inline_data` field is filled with
/// raw bytes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct FunctionResponsePart {
    /// Inline media bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<FunctionResponseBlob>,
}

/// Inline media bytes.
///
/// Raw media bytes for function response. Text should not be sent as raw bytes, use the
/// 'FunctionResponse.response' field.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct FunctionResponseBlob {
    /// Raw bytes for media formats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// The IANA standard MIME type of the source data. Examples: - image/png - image/jpeg If an
    /// unsupported MIME type is provided, an error will be returned. For a complete list of
    /// supported types, see [Supported file
    /// formats](https://ai.google.dev/gemini-api/docs/prompting_with_media#supported_file_formats).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Optional. Specifies how the response should be scheduled in the conversation. Only
/// applicable to NON_BLOCKING function calls, is ignored otherwise. Defaults to WHEN_IDLE.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Scheduling {
    Interrupt,
    #[serde(rename = "SCHEDULING_UNSPECIFIED")]
    SchedulingUnspecified,
    Silent,
    #[serde(rename = "WHEN_IDLE")]
    WhenIdle,
}

/// Inline media bytes.
///
/// Raw media bytes. Text should not be sent as raw bytes, use the 'text' field.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Blob {
    /// Raw bytes for media formats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// The IANA standard MIME type of the source data. Examples of supported types: - Images:
    /// image/png, image/jpeg, image/jpg, image/webp, image/heic, image/heif, image/gif,
    /// image/avif - Audio: audio/*, video/audio/s16le, video/audio/wav - Video: video/* - Text:
    /// text/plain, text/html, text/css, text/javascript, text/x-typescript, text/csv,
    /// text/markdown, text/x-python, text/xml, text/rtf, video/text/timestamp - Applications:
    /// application/x-javascript, application/x-typescript, application/x-python-code,
    /// application/json, application/x-ipynb+json, application/rtf, application/pdf For
    /// additional context, see [Supported file
    /// formats](https://ai.google.dev/gemini-api/docs/file-input-methods#supported-content-types).
    /// //
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Optional. Media resolution for the input media.
///
/// Media resolution for the input media.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct MediaResolution {
    /// The media resolution level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
}

/// The media resolution level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Level {
    #[serde(rename = "MEDIA_RESOLUTION_HIGH")]
    MediaResolutionHigh,
    #[serde(rename = "MEDIA_RESOLUTION_LOW")]
    MediaResolutionLow,
    #[serde(rename = "MEDIA_RESOLUTION_MEDIUM")]
    MediaResolutionMedium,
    #[serde(rename = "MEDIA_RESOLUTION_ULTRA_HIGH")]
    MediaResolutionUltraHigh,
    #[serde(rename = "MEDIA_RESOLUTION_UNSPECIFIED")]
    MediaResolutionUnspecified,
}

/// Server-side tool call. This field is populated when the model predicts a tool invocation
/// that should be executed on the server. The client is expected to echo this message back
/// to the API.
///
/// A predicted server-side `ToolCall` returned from the model. This message contains
/// information about a tool that the model wants to invoke. The client is NOT expected to
/// execute this `ToolCall`. Instead, the client should pass this `ToolCall` back to the API
/// in a subsequent turn within a `Content` message, along with the corresponding
/// `ToolResponse`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ToolCall {
    /// Optional. The tool call arguments. Example: {"arg1" : "value1", "arg2" : "value2" , ...}
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Map<String, serde_json::Value>>,
    /// Optional. Unique identifier of the tool call. The server returns the tool response with
    /// the matching `id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Required. The type of tool that was called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<ToolType>,
}

/// Required. The type of tool that was called.
///
/// Required. The type of tool that was called, matching the `tool_type` in the corresponding
/// `ToolCall`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum ToolType {
    #[serde(rename = "FILE_SEARCH")]
    FileSearch,
    #[serde(rename = "GOOGLE_MAPS")]
    GoogleMaps,
    #[serde(rename = "GOOGLE_SEARCH_IMAGE")]
    GoogleSearchImage,
    #[serde(rename = "GOOGLE_SEARCH_WEB")]
    GoogleSearchWeb,
    #[serde(rename = "TOOL_TYPE_UNSPECIFIED")]
    ToolTypeUnspecified,
    #[serde(rename = "URL_CONTEXT")]
    UrlContext,
}

/// The output from a server-side `ToolCall` execution. This field is populated by the client
/// with the results of executing the corresponding `ToolCall`.
///
/// The output from a server-side `ToolCall` execution. This message contains the results of
/// a tool invocation that was initiated by a `ToolCall` from the model. The client should
/// pass this `ToolResponse` back to the API in a subsequent turn within a `Content` message,
/// along with the corresponding `ToolCall`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ToolResponse {
    /// Optional. The identifier of the tool call this response is for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional. The tool response.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Map<String, serde_json::Value>>,
    /// Required. The type of tool that was called, matching the `tool_type` in the corresponding
    /// `ToolCall`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<ToolType>,
}

/// Optional. Video metadata. The metadata should only be specified while the video data is
/// presented in inline_data or file_data.
///
/// Deprecated: Use `GenerateContentRequest.processing_options` instead. Metadata describes
/// the input video content.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct VideoMetadata {
    /// Optional. The end offset of the video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<String>,
    /// Optional. The frame rate of the video sent to the model. If not specified, the default
    /// value will be 1.0. The fps range is (0.0, 24.0].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    /// Optional. The start offset of the video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<String>,
}

/// Optional. Configuration options for model generation and outputs.
///
/// Configuration options for model generation and outputs. Not all parameters are
/// configurable for every model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GenerationConfig {
    #[serde(rename = "_responseJsonSchema")]
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_json_schema: Option<serde_json::Value>,
    /// Optional. Number of generated responses to return. If unset, this will default to 1.
    /// Please note that this doesn't work for previous generation models (Gemini 1.0 family)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i64>,
    /// Optional. Enables enhanced civic answers. It may not be available for all models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_enhanced_civic_answers: Option<bool>,
    /// Optional. Frequency penalty applied to the next token's logprobs, multiplied by the
    /// number of times each token has been seen in the respponse so far. A positive penalty will
    /// discourage the use of tokens that have already been used, proportional to the number of
    /// times the token has been used: The more a token is used, the more difficult it is for the
    /// model to use that token again increasing the vocabulary of responses. Caution: A
    /// _negative_ penalty will encourage the model to reuse tokens proportional to the number of
    /// times the token has been used. Small negative values will reduce the vocabulary of a
    /// response. Larger negative values will cause the model to start repeating a common token
    /// until it hits the max_output_tokens limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Optional. Config for image generation. An error will be returned if this field is set for
    /// models that don't support these config options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_config: Option<ImageConfig>,
    /// Optional. Only valid if response_logprobs=True. This sets the number of top logprobs,
    /// including the chosen candidate, to return at each decoding step in the
    /// Candidate.logprobs_result. The number must be in the range of [0, 20].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<i64>,
    /// Optional. The maximum number of tokens to include in a response candidate. Note: The
    /// default value varies by model, see the `Model.output_token_limit` attribute of the
    /// `Model` returned from the `getModel` function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    /// Optional. If specified, the media resolution specified will be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_resolution: Option<MediaResolutionEnum>,
    /// Optional. Presence penalty applied to the next token's logprobs if the token has already
    /// been seen in the response. This penalty is binary on/off and not dependant on the number
    /// of times the token is used (after the first). Use frequency_penalty for a penalty that
    /// increases with each use. A positive penalty will discourage the use of tokens that have
    /// already been used in the response, increasing the vocabulary. A negative penalty will
    /// encourage the use of tokens that have already been used in the response, decreasing the
    /// vocabulary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Optional. Configuration for the response output format. Allows specifying output
    /// configuration per modality (text, audio, image) in a flat structure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormatConfig>,
    #[serde(rename = "responseJsonSchema")]
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config_response_json_schema: Option<serde_json::Value>,
    /// Optional. If true, export the logprobs results in response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_logprobs: Option<bool>,
    /// Optional. MIME type of the generated candidate text. Supported MIME types are:
    /// `text/plain`: (default) Text output. `application/json`: JSON response in the response
    /// candidates. `text/x.enum`: ENUM as a string response in the response candidates. Refer to
    /// the [docs](https://ai.google.dev/gemini-api/docs/prompting_with_media#plain_text_formats)
    /// for a list of all supported text MIME types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    /// Optional. The requested modalities of the response. Represents the set of modalities that
    /// the model can return, and should be expected in the response. This is an exact match to
    /// the modalities of the response. A model may have multiple combinations of supported
    /// modalities. If the requested modalities do not match any of the supported combinations,
    /// an error will be returned. An empty list is equivalent to requesting only text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<ResponseModality>>,
    /// Optional. Output schema of the generated candidate text. Schemas must be a subset of the
    /// [OpenAPI schema](https://spec.openapis.org/oas/v3.0.3#schema) and can be objects,
    /// primitives or arrays. If set, a compatible `response_mime_type` must also be set.
    /// Compatible MIME types: `application/json`: Schema for JSON response. Refer to the [JSON
    /// text generation guide](https://ai.google.dev/gemini-api/docs/json-mode) for more details.
    pub response_schema: Box<Option<Schema>>,
    /// Optional. Seed used in decoding. If not set, the request uses a randomly generated seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Optional. The speech generation config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_config: Option<SpeechConfig>,
    /// Optional. The set of character sequences (up to 5) that will stop output generation. If
    /// specified, the API will stop at the first appearance of a `stop_sequence`. The stop
    /// sequence will not be included as part of the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Optional. Controls the randomness of the output. Note: The default value varies by model,
    /// see the `Model.temperature` attribute of the `Model` returned from the `getModel`
    /// function. Values can range from [0.0, 2.0].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Optional. Config for thinking features. An error will be returned if this field is set
    /// for models that don't support thinking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
    /// Optional. The maximum number of tokens to consider when sampling. Gemini models use Top-p
    /// (nucleus) sampling or a combination of Top-k and nucleus sampling. Top-k sampling
    /// considers the set of `top_k` most probable tokens. Models running with nucleus sampling
    /// don't allow top_k setting. Note: The default value varies by `Model` and is specified by
    /// the`Model.top_p` attribute returned from the `getModel` function. An empty `top_k`
    /// attribute indicates that the model doesn't apply top-k sampling and doesn't allow setting
    /// `top_k` on requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,
    /// Optional. The maximum cumulative probability of tokens to consider when sampling. The
    /// model uses combined Top-k and Top-p (nucleus) sampling. Tokens are sorted based on their
    /// assigned probabilities so that only the most likely tokens are considered. Top-k sampling
    /// directly limits the maximum number of tokens to consider, while Nucleus sampling limits
    /// the number of tokens based on the cumulative probability. Note: The default value varies
    /// by `Model` and is specified by the`Model.top_p` attribute returned from the `getModel`
    /// function. An empty `top_k` attribute indicates that the model doesn't apply top-k
    /// sampling and doesn't allow setting `top_k` on requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Optional. Config for translation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_config: Option<TranslationConfig>,
}

/// Optional. Config for image generation. An error will be returned if this field is set for
/// models that don't support these config options.
///
/// Config for image generation features.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ImageConfig {
    /// Optional. The aspect ratio of the image to generate. Supported aspect ratios: `1:1`,
    /// `1:4`, `4:1`, `1:8`, `8:1`, `2:3`, `3:2`, `3:4`, `4:3`, `4:5`, `5:4`, `9:16`, `16:9`, or
    /// `21:9`. If not specified, the model will choose a default aspect ratio based on any
    /// reference images provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    /// Optional. Specifies the size of generated images. Supported values are `512`, `1K`, `2K`,
    /// `4K`. If not specified, the model will use default value `1K`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size: Option<String>,
}

/// Optional. If specified, the media resolution specified will be used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum MediaResolutionEnum {
    #[serde(rename = "MEDIA_RESOLUTION_HIGH")]
    MediaResolutionHigh,
    #[serde(rename = "MEDIA_RESOLUTION_LOW")]
    MediaResolutionLow,
    #[serde(rename = "MEDIA_RESOLUTION_MEDIUM")]
    MediaResolutionMedium,
    #[serde(rename = "MEDIA_RESOLUTION_UNSPECIFIED")]
    MediaResolutionUnspecified,
}

/// Optional. Configuration for the response output format. Allows specifying output
/// configuration per modality (text, audio, image) in a flat structure.
///
/// Configuration for the response output format. This is a flat object where each optional
/// sub-field configures a specific output modality.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct ResponseFormatConfig {
    /// Optional. Audio output format configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioResponseFormat>,
    /// Optional. Image output format configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageResponseFormat>,
    /// Optional. Text output format configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextResponseFormat>,
}

/// Optional. Audio output format configuration.
///
/// Configuration for audio output format.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct AudioResponseFormat {
    /// Optional. Bit rate in bits per second (bps). Only applicable for compressed formats (MP3,
    /// Opus).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<i64>,
    /// Optional. The delivery mode for the audio output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
    /// Optional. The MIME type of the audio output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<AudioMimeType>,
    /// Optional. Sample rate in Hz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<i64>,
}

/// Optional. The delivery mode for the audio output.
///
/// Optional. The delivery mode for the image output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Delivery {
    #[serde(rename = "DELIVERY_UNSPECIFIED")]
    DeliveryUnspecified,
    Inline,
    Uri,
}

/// Optional. The MIME type of the audio output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum AudioMimeType {
    #[serde(rename = "AUDIO_ALAW")]
    AudioAlaw,
    #[serde(rename = "AUDIO_L16")]
    AudioL16,
    #[serde(rename = "AUDIO_MP3")]
    AudioMp3,
    #[serde(rename = "AUDIO_MULAW")]
    AudioMulaw,
    #[serde(rename = "AUDIO_OGG_OPUS")]
    AudioOggOpus,
    #[serde(rename = "AUDIO_WAV")]
    AudioWav,
    #[serde(rename = "MIME_TYPE_UNSPECIFIED")]
    MimeTypeUnspecified,
}

/// Optional. Image output format configuration.
///
/// Configuration for image output format.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ImageResponseFormat {
    /// Optional. The aspect ratio for the image output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<AspectRatio>,
    /// Optional. The delivery mode for the image output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
    /// Optional. The size of the image output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size: Option<ImageSize>,
    /// Optional. The MIME type of the image output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<ImageMimeType>,
}

/// Optional. The aspect ratio for the image output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum AspectRatio {
    #[serde(rename = "ASPECT_RATIO_EIGHT_BY_ONE")]
    AspectRatioEightByOne,
    #[serde(rename = "ASPECT_RATIO_FIVE_BY_FOUR")]
    AspectRatioFiveByFour,
    #[serde(rename = "ASPECT_RATIO_FOUR_BY_FIVE")]
    AspectRatioFourByFive,
    #[serde(rename = "ASPECT_RATIO_FOUR_BY_ONE")]
    AspectRatioFourByOne,
    #[serde(rename = "ASPECT_RATIO_FOUR_BY_THREE")]
    AspectRatioFourByThree,
    #[serde(rename = "ASPECT_RATIO_NINE_BY_SIXTEEN")]
    AspectRatioNineBySixteen,
    #[serde(rename = "ASPECT_RATIO_ONE_BY_EIGHT")]
    AspectRatioOneByEight,
    #[serde(rename = "ASPECT_RATIO_ONE_BY_FOUR")]
    AspectRatioOneByFour,
    #[serde(rename = "ASPECT_RATIO_ONE_BY_ONE")]
    AspectRatioOneByOne,
    #[serde(rename = "ASPECT_RATIO_SIXTEEN_BY_NINE")]
    AspectRatioSixteenByNine,
    #[serde(rename = "ASPECT_RATIO_THREE_BY_FOUR")]
    AspectRatioThreeByFour,
    #[serde(rename = "ASPECT_RATIO_THREE_BY_TWO")]
    AspectRatioThreeByTwo,
    #[serde(rename = "ASPECT_RATIO_TWENTY_ONE_BY_NINE")]
    AspectRatioTwentyOneByNine,
    #[serde(rename = "ASPECT_RATIO_TWO_BY_THREE")]
    AspectRatioTwoByThree,
    #[serde(rename = "ASPECT_RATIO_UNSPECIFIED")]
    AspectRatioUnspecified,
}

/// Optional. The size of the image output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum ImageSize {
    #[serde(rename = "IMAGE_SIZE_FIVE_TWELVE")]
    ImageSizeFiveTwelve,
    #[serde(rename = "IMAGE_SIZE_FOUR_K")]
    ImageSizeFourK,
    #[serde(rename = "IMAGE_SIZE_ONE_K")]
    ImageSizeOneK,
    #[serde(rename = "IMAGE_SIZE_TWO_K")]
    ImageSizeTwoK,
    #[serde(rename = "IMAGE_SIZE_UNSPECIFIED")]
    ImageSizeUnspecified,
}

/// Optional. The MIME type of the image output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum ImageMimeType {
    #[serde(rename = "IMAGE_JPEG")]
    ImageJpeg,
    #[serde(rename = "MIME_TYPE_UNSPECIFIED")]
    MimeTypeUnspecified,
}

/// Optional. Text output format configuration.
///
/// Configuration for text output format.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct TextResponseFormat {
    /// Optional. The MIME type of the text output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<TextMimeType>,
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Optional. The MIME type of the text output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum TextMimeType {
    #[serde(rename = "APPLICATION_JSON")]
    ApplicationJson,
    #[serde(rename = "MIME_TYPE_UNSPECIFIED")]
    MimeTypeUnspecified,
    #[serde(rename = "TEXT_PLAIN")]
    TextPlain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum ResponseModality {
    Audio,
    Image,
    #[serde(rename = "MODALITY_UNSPECIFIED")]
    ModalityUnspecified,
    Text,
}

/// Optional. Output schema of the generated candidate text. Schemas must be a subset of the
/// [OpenAPI schema](https://spec.openapis.org/oas/v3.0.3#schema) and can be objects,
/// primitives or arrays. If set, a compatible `response_mime_type` must also be set.
/// Compatible MIME types: `application/json`: Schema for JSON response. Refer to the [JSON
/// text generation guide](https://ai.google.dev/gemini-api/docs/json-mode) for more
/// details.
///
/// The `Schema` object allows the definition of input and output data types. These types can
/// be objects, but also primitives and arrays. Represents a select subset of an [OpenAPI 3.0
/// schema object](https://spec.openapis.org/oas/v3.0.3#schema).
///
/// Optional. Schema of the elements of Type.ARRAY.
///
/// Optional. Describes the parameters to this function. Reflects the Open API 3.03 Parameter
/// Object string Key: the name of the parameter. Parameter names are case sensitive. Schema
/// Value: the Schema defining the type used for the parameter.
///
/// Optional. Describes the output from this function in JSON Schema format. Reflects the
/// Open API 3.03 Response Object. The Schema defines the type used for the response value of
/// the function.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Schema {
    /// Optional. The value should be validated against any (one or more) of the subschemas in
    /// the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(rename = "default")]
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_default: Option<serde_json::Value>,
    /// Optional. A brief description of the parameter. This could contain examples of use.
    /// Parameter description may be formatted as Markdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional. Possible values of the element of Type.STRING with enum format. For example we
    /// can define an Enum Direction as : {type:STRING, format:enum, enum:["EAST", NORTH",
    /// "SOUTH", "WEST"]}
    #[serde(rename = "enum")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_enum: Option<Vec<String>>,
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    /// Optional. The format of the data. Any value is allowed, but most do not trigger any
    /// special functionality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Optional. Schema of the elements of Type.ARRAY.
    pub items: Box<Option<Schema>>,
    /// Optional. Maximum value of the Type.INTEGER and Type.NUMBER
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// Optional. Maximum number of the elements for Type.ARRAY.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, deserialize_with = "deserialize_optional_i64_string")]
    pub max_items: Option<String>,
    /// Optional. Maximum length of the Type.STRING
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, deserialize_with = "deserialize_optional_i64_string")]
    pub max_length: Option<String>,
    /// Optional. Maximum number of the properties for Type.OBJECT.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, deserialize_with = "deserialize_optional_i64_string")]
    pub max_properties: Option<String>,
    /// Optional. SCHEMA FIELDS FOR TYPE INTEGER and NUMBER Minimum value of the Type.INTEGER and
    /// Type.NUMBER
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Optional. Minimum number of the elements for Type.ARRAY.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, deserialize_with = "deserialize_optional_i64_string")]
    pub min_items: Option<String>,
    /// Optional. SCHEMA FIELDS FOR TYPE STRING Minimum length of the Type.STRING
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, deserialize_with = "deserialize_optional_i64_string")]
    pub min_length: Option<String>,
    /// Optional. Minimum number of the properties for Type.OBJECT.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, deserialize_with = "deserialize_optional_i64_string")]
    pub min_properties: Option<String>,
    /// Optional. Indicates if the value may be null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    /// Optional. Pattern of the Type.STRING to restrict a string to a regular expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Optional. Properties of Type.OBJECT.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Schema>>,
    /// Optional. The order of the properties. Not a standard field in open api spec. Used to
    /// determine the order of the properties in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_ordering: Option<Vec<String>>,
    /// Optional. Required properties of Type.OBJECT.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Optional. The title of the schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Required. Data type.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<Type>,
}

/// Required. Data type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Type {
    #[serde(alias = "array")]
    Array,
    #[serde(alias = "boolean")]
    Boolean,
    #[serde(alias = "integer")]
    Integer,
    #[serde(alias = "null")]
    Null,
    #[serde(alias = "number")]
    Number,
    #[serde(alias = "object")]
    Object,
    #[serde(alias = "string")]
    String,
    #[serde(rename = "TYPE_UNSPECIFIED")]
    TypeUnspecified,
}

/// Optional. The speech generation config.
///
/// Config for speech generation and transcription.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct SpeechConfig {
    /// Optional. The IETF [BCP-47](https://www.rfc-editor.org/rfc/bcp/bcp47.txt) language code
    /// that the user configured the app to use. Used for speech recognition and synthesis. Valid
    /// values are: `de-DE`, `en-AU`, `en-GB`, `en-IN`, `en-US`, `es-US`, `fr-FR`, `hi-IN`,
    /// `pt-BR`, `ar-XA`, `es-ES`, `fr-CA`, `id-ID`, `it-IT`, `ja-JP`, `tr-TR`, `vi-VN`, `bn-IN`,
    /// `gu-IN`, `kn-IN`, `ml-IN`, `mr-IN`, `ta-IN`, `te-IN`, `nl-NL`, `ko-KR`, `cmn-CN`,
    /// `pl-PL`, `ru-RU`, and `th-TH`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    /// Optional. The configuration for the multi-speaker setup. It is mutually exclusive with
    /// the voice_config field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_speaker_voice_config: Option<MultiSpeakerVoiceConfig>,
    /// The configuration in case of single-voice output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_config: Option<VoiceConfig>,
}

/// Optional. The configuration for the multi-speaker setup. It is mutually exclusive with
/// the voice_config field.
///
/// The configuration for the multi-speaker setup.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct MultiSpeakerVoiceConfig {
    /// Required. All the enabled speaker voices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_voice_configs: Option<Vec<SpeakerVoiceConfig>>,
}

/// The configuration for a single speaker in a multi speaker setup.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct SpeakerVoiceConfig {
    /// Required. The name of the speaker to use. Should be the same as in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    /// Required. The configuration for the voice to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_config: Option<VoiceConfig>,
}

/// Required. The configuration for the voice to use.
///
/// The configuration for the voice to use.
///
/// The configuration in case of single-voice output.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct VoiceConfig {
    /// The configuration for the prebuilt voice to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prebuilt_voice_config: Option<PrebuiltVoiceConfig>,
}

/// The configuration for the prebuilt voice to use.
///
/// The configuration for the prebuilt speaker to use.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct PrebuiltVoiceConfig {
    /// The name of the preset voice to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,
}

/// Optional. Config for thinking features. An error will be returned if this field is set
/// for models that don't support thinking.
///
/// Config for thinking features.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ThinkingConfig {
    /// Indicates whether to include thoughts in the response. If true, thoughts are returned
    /// only when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_thoughts: Option<bool>,
    /// The number of thoughts tokens that the model should generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i64>,
    /// Optional. Controls the maximum depth of the model's internal reasoning process before it
    /// produces a response. The default value is model-dependent. Refer to the [Thinking levels
    /// guide](https://ai.google.dev/gemini-api/docs/thinking#thinking-levels) for more details.
    /// Recommended for Gemini 3 or later models. Use with earlier models results in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<ThinkingLevel>,
}

/// Optional. Controls the maximum depth of the model's internal reasoning process before it
/// produces a response. The default value is model-dependent. Refer to the [Thinking levels
/// guide](https://ai.google.dev/gemini-api/docs/thinking#thinking-levels) for more details.
/// Recommended for Gemini 3 or later models. Use with earlier models results in an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum ThinkingLevel {
    High,
    Low,
    Medium,
    Minimal,
    #[serde(rename = "THINKING_LEVEL_UNSPECIFIED")]
    ThinkingLevelUnspecified,
}

/// Optional. Config for translation.
///
/// Config for translation features.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct TranslationConfig {
    /// Optional. If true, the model will generate audio when the target language is spoken,
    /// essentially it will parrot the input. If false, we will not produce audio for the target
    /// language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo_target_language: Option<bool>,
    /// Required. The target language for translation. Supported values are BCP-47 language codes
    /// (e.g. "en", "es", "fr").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_language_code: Option<String>,
}

/// Safety setting, affecting the safety-blocking behavior. Passing a safety setting for a
/// category changes the allowed probability that content is blocked.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct SafetySetting {
    /// Required. The category for this setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    /// Required. Controls the probability threshold at which harm is blocked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Threshold>,
}

/// Required. The category for this setting.
///
/// Required. The category for this rating.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Category {
    #[serde(rename = "HARM_CATEGORY_CIVIC_INTEGRITY")]
    HarmCategoryCivicIntegrity,
    #[serde(rename = "HARM_CATEGORY_DANGEROUS")]
    HarmCategoryDangerous,
    #[serde(rename = "HARM_CATEGORY_DANGEROUS_CONTENT")]
    HarmCategoryDangerousContent,
    #[serde(rename = "HARM_CATEGORY_DEROGATORY")]
    HarmCategoryDerogatory,
    #[serde(rename = "HARM_CATEGORY_HARASSMENT")]
    HarmCategoryHarassment,
    #[serde(rename = "HARM_CATEGORY_HATE_SPEECH")]
    HarmCategoryHateSpeech,
    #[serde(rename = "HARM_CATEGORY_MEDICAL")]
    HarmCategoryMedical,
    #[serde(rename = "HARM_CATEGORY_SEXUAL")]
    HarmCategorySexual,
    #[serde(rename = "HARM_CATEGORY_SEXUALLY_EXPLICIT")]
    HarmCategorySexuallyExplicit,
    #[serde(rename = "HARM_CATEGORY_TOXICITY")]
    HarmCategoryToxicity,
    #[serde(rename = "HARM_CATEGORY_UNSPECIFIED")]
    HarmCategoryUnspecified,
    #[serde(rename = "HARM_CATEGORY_VIOLENCE")]
    HarmCategoryViolence,
}

/// Required. Controls the probability threshold at which harm is blocked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Threshold {
    #[serde(rename = "BLOCK_LOW_AND_ABOVE")]
    BlockLowAndAbove,
    #[serde(rename = "BLOCK_MEDIUM_AND_ABOVE")]
    BlockMediumAndAbove,
    #[serde(rename = "BLOCK_NONE")]
    BlockNone,
    #[serde(rename = "BLOCK_ONLY_HIGH")]
    BlockOnlyHigh,
    #[serde(rename = "HARM_BLOCK_THRESHOLD_UNSPECIFIED")]
    HarmBlockThresholdUnspecified,
    Off,
}

/// Optional. The service tier of the request.
///
/// Output only. Service tier of the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "google/")]
pub enum ServiceTier {
    Flex,
    Priority,
    Standard,
    Unspecified,
}

/// Optional. Tool configuration for any `Tool` specified in the request. Refer to the
/// [Function calling
/// guide](https://ai.google.dev/gemini-api/docs/function-calling#function_calling_mode) for
/// a usage example.
///
/// The Tool configuration containing parameters for specifying `Tool` use in the request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ToolConfig {
    /// Optional. Function calling config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling_config: Option<FunctionCallingConfig>,
    /// Optional. If true, the API response will include the server-side tool calls and responses
    /// within the `Content` message. This allows clients to observe the server's tool
    /// interactions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_server_side_tool_invocations: Option<bool>,
    /// Optional. Retrieval config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_config: Option<RetrievalConfig>,
}

/// Optional. Function calling config.
///
/// Configuration for specifying function calling behavior.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct FunctionCallingConfig {
    /// Optional. A set of function names that, when provided, limits the functions the model
    /// will call. This should only be set when the Mode is ANY or VALIDATED. Function names
    /// should match [FunctionDeclaration.name]. When set, model will predict a function call
    /// from only allowed function names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
    /// Optional. Specifies the mode in which function calling should execute. If unspecified,
    /// the default value will be set to AUTO.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<FunctionCallingConfigMode>,
}

/// Optional. Specifies the mode in which function calling should execute. If unspecified,
/// the default value will be set to AUTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum FunctionCallingConfigMode {
    Any,
    Auto,
    #[serde(rename = "MODE_UNSPECIFIED")]
    ModeUnspecified,
    None,
    Validated,
}

/// Optional. Retrieval config.
///
/// Retrieval config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct RetrievalConfig {
    /// Optional. The language code of the user. Language code for content. Use language tags
    /// defined by [BCP47](https://www.rfc-editor.org/rfc/bcp/bcp47.txt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    /// Optional. The location of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat_lng: Option<LatLng>,
}

/// Optional. The location of the user.
///
/// An object that represents a latitude/longitude pair. This is expressed as a pair of
/// doubles to represent degrees latitude and degrees longitude. Unless specified otherwise,
/// this object must conform to the WGS84 standard. Values must be within normalized ranges.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct LatLng {
    /// The latitude in degrees. It must be in the range [-90.0, +90.0].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// The longitude in degrees. It must be in the range [-180.0, +180.0].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

/// Tool details that the model may use to generate response. A `Tool` is a piece of code
/// that enables the system to interact with external systems to perform an action, or set of
/// actions, outside of knowledge and scope of the model. Next ID: 16
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Tool {
    /// Optional. Enables the model to execute code as part of generation.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<serde_json::Map<String, serde_json::Value>>,
    /// Optional. Tool to support the model interacting directly with the computer. If enabled,
    /// it automatically populates computer-use specific Function Declarations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computer_use: Option<ComputerUse>,
    /// Optional. FileSearch tool type. Tool to retrieve knowledge from Semantic Retrieval
    /// corpora.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search: Option<FileSearch>,
    /// Optional. A list of `FunctionDeclarations` available to the model that can be used for
    /// function calling. The model or system does not execute the function. Instead the defined
    /// function may be returned as a FunctionCall with arguments to the client side for
    /// execution. The model may decide to call a subset of these functions by populating
    /// FunctionCall in the response. The next conversation turn may contain a FunctionResponse
    /// with the Content.role "function" generation context for the next model turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<FunctionDeclaration>>,
    /// Optional. Tool that allows grounding the model's response with geospatial context related
    /// to the user's query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_maps: Option<GoogleMaps>,
    /// Optional. GoogleSearch tool type. Tool to support Google Search in Model. Powered by
    /// Google.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_search: Option<GoogleSearch>,
    /// Optional. Retrieval tool that is powered by Google search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_search_retrieval: Option<GoogleSearchRetrieval>,
    /// Optional. MCP Servers to connect to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<McpServer>>,
    /// Optional. Tool to support URL context retrieval.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Optional. Tool to support the model interacting directly with the computer. If enabled,
/// it automatically populates computer-use specific Function Declarations.
///
/// Computer Use tool type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ComputerUse {
    /// Optional. Whether enable the prompt injection detection check on computer-use request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_prompt_injection_detection: Option<bool>,
    /// Required. The environment being operated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    /// Optional. By default, predefined functions are included in the final model call. Some of
    /// them can be explicitly excluded from being automatically included. This can serve two
    /// purposes: 1. Using a more restricted / different action space. 2. Improving the
    /// definitions / instructions of predefined functions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_predefined_functions: Option<Vec<String>>,
}

/// Required. The environment being operated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Environment {
    #[serde(rename = "ENVIRONMENT_BROWSER")]
    EnvironmentBrowser,
    #[serde(rename = "ENVIRONMENT_DESKTOP")]
    EnvironmentDesktop,
    #[serde(rename = "ENVIRONMENT_MOBILE")]
    EnvironmentMobile,
    #[serde(rename = "ENVIRONMENT_UNSPECIFIED")]
    EnvironmentUnspecified,
}

/// Optional. FileSearch tool type. Tool to retrieve knowledge from Semantic Retrieval
/// corpora.
///
/// The FileSearch tool that retrieves knowledge from Semantic Retrieval corpora. Files are
/// imported to Semantic Retrieval corpora using the ImportFile API.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct FileSearch {
    /// Required. The names of the file_search_stores to retrieve from. Example:
    /// `fileSearchStores/my-file-search-store-123`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search_store_names: Option<Vec<String>>,
    /// Optional. Metadata filter to apply to the semantic retrieval documents and chunks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_filter: Option<String>,
    /// Optional. The number of semantic retrieval chunks to retrieve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,
}

/// Structured representation of a function declaration as defined by the [OpenAPI 3.03
/// specification](https://spec.openapis.org/oas/v3.0.3). Included in this declaration are
/// the function name and parameters. This FunctionDeclaration is a representation of a block
/// of code that can be used as a `Tool` by the model and executed by the client.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct FunctionDeclaration {
    /// Optional. Specifies the function Behavior. Currently only supported by the
    /// BidiGenerateContent method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<Behavior>,
    /// Required. A brief description of the function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Required. The name of the function. Must be a-z, A-Z, 0-9, or contain underscores,
    /// colons, dots, and dashes, with a maximum length of 128.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional. Describes the parameters to this function. Reflects the Open API 3.03 Parameter
    /// Object string Key: the name of the parameter. Parameter names are case sensitive. Schema
    /// Value: the Schema defining the type used for the parameter.
    pub parameters: Box<Option<Schema>>,
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_json_schema: Option<serde_json::Value>,
    /// Optional. Describes the output from this function in JSON Schema format. Reflects the
    /// Open API 3.03 Response Object. The Schema defines the type used for the response value of
    /// the function.
    pub response: Box<Option<Schema>>,
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_json_schema: Option<serde_json::Value>,
}

/// Optional. Specifies the function Behavior. Currently only supported by the
/// BidiGenerateContent method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Behavior {
    Blocking,
    #[serde(rename = "NON_BLOCKING")]
    NonBlocking,
    Unspecified,
}

/// Optional. Tool that allows grounding the model's response with geospatial context related
/// to the user's query.
///
/// The GoogleMaps Tool that provides geospatial context for the user's query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GoogleMaps {
    /// Optional. Whether to return a widget context token in the GroundingMetadata of the
    /// response. Developers can use the widget context token to render a Google Maps widget with
    /// geospatial context related to the places that the model references in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_widget: Option<bool>,
}

/// Optional. GoogleSearch tool type. Tool to support Google Search in Model. Powered by
/// Google.
///
/// GoogleSearch tool type. Tool to support Google Search in Model. Powered by Google.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GoogleSearch {
    /// Optional. The set of search types to enable. If not set, web search is enabled by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_types: Option<SearchTypes>,
    /// Optional. Filter search results to a specific time range. If customers set a start time,
    /// they must set an end time (and vice versa).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range_filter: Option<Interval>,
}

/// Optional. The set of search types to enable. If not set, web search is enabled by
/// default.
///
/// Different types of search that can be enabled on the GoogleSearch tool.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct SearchTypes {
    /// Optional. Enables image search. Image bytes are returned.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_search: Option<serde_json::Map<String, serde_json::Value>>,
    /// Optional. Enables web search. Only text results are returned.
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Optional. Filter search results to a specific time range. If customers set a start time,
/// they must set an end time (and vice versa).
///
/// Represents a time interval, encoded as a Timestamp start (inclusive) and a Timestamp end
/// (exclusive). The start must be less than or equal to the end. When the start equals the
/// end, the interval is empty (matches no time). When both start and end are unspecified,
/// the interval matches any time.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Interval {
    /// Optional. Exclusive end of the interval. If specified, a Timestamp matching this interval
    /// will have to be before the end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// Optional. Inclusive start of the interval. If specified, a Timestamp matching this
    /// interval will have to be the same or after the start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
}

/// Optional. Retrieval tool that is powered by Google search.
///
/// Tool to retrieve public web data for grounding, powered by Google.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GoogleSearchRetrieval {
    /// Specifies the dynamic retrieval configuration for the given source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_retrieval_config: Option<DynamicRetrievalConfig>,
}

/// Specifies the dynamic retrieval configuration for the given source.
///
/// Describes the options to customize dynamic retrieval.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct DynamicRetrievalConfig {
    /// The threshold to be used in dynamic retrieval. If not set, a system default value is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_threshold: Option<f64>,
    /// The mode of the predictor to be used in dynamic retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<DynamicRetrievalConfigMode>,
}

/// The mode of the predictor to be used in dynamic retrieval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum DynamicRetrievalConfigMode {
    #[serde(rename = "MODE_DYNAMIC")]
    ModeDynamic,
    #[serde(rename = "MODE_UNSPECIFIED")]
    ModeUnspecified,
}

/// A MCPServer is a server that can be called by the model to perform actions. It is a
/// server that implements the MCP protocol. Next ID: 6
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct McpServer {
    /// The name of the MCPServer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A transport that can stream HTTP requests and responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streamable_http_transport: Option<StreamableHttpTransport>,
}

/// A transport that can stream HTTP requests and responses.
///
/// A transport that can stream HTTP requests and responses. Next ID: 6
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct StreamableHttpTransport {
    /// Optional: Fields for authentication headers, timeouts, etc., if needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Timeout for SSE read operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sse_read_timeout: Option<String>,
    /// Whether to close the client session when the transport closes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminate_on_close: Option<bool>,
    /// HTTP timeout for regular operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    /// The full URL for the MCPServer endpoint. Example: "https://api.example.com/mcp"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Response from the model supporting multiple candidate responses. Safety ratings and
/// content filtering are reported for both prompt in
/// `GenerateContentResponse.prompt_feedback` and for each candidate in `finish_reason` and
/// in `safety_ratings`. The API: - Returns either all requested candidates or none of them -
/// Returns no candidates at all only if there was something wrong with the prompt (check
/// `prompt_feedback`) - Reports feedback on each candidate in `finish_reason` and
/// `safety_ratings`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GenerateContentResponse {
    /// Candidate responses from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<Candidate>>,
    /// Output only. The current model status of this model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_status: Option<ModelStatus>,
    /// Output only. The model version used to generate the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
    /// Returns the prompt's feedback related to the content filters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_feedback: Option<PromptFeedback>,
    /// Output only. response_id is used to identify each response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    /// Output only. Metadata on the generation requests' token usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

/// A response candidate generated from the model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Candidate {
    /// Output only. Average log probability score of the candidate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_logprobs: Option<f64>,
    /// Output only. Citation information for model-generated candidate. This field may be
    /// populated with recitation information for any text included in the `content`. These are
    /// passages that are "recited" from copyrighted material in the foundational LLM's training
    /// data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_metadata: Option<CitationMetadata>,
    /// Output only. Generated content returned from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// Optional. Output only. Details the reason why the model stopped generating tokens. This
    /// is populated only when `finish_reason` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_message: Option<String>,
    /// Optional. Output only. The reason why the model stopped generating tokens. If empty, the
    /// model has not stopped generating tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    /// Output only. Attribution information for sources that contributed to a grounded answer.
    /// This field is populated for `GenerateAnswer` calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_attributions: Option<Vec<GroundingAttribution>>,
    /// Output only. Grounding metadata for the candidate. This field is populated for
    /// `GenerateContent` calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_metadata: Option<GroundingMetadata>,
    /// Output only. Index of the candidate in the list of response candidates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i64>,
    /// Output only. Log-likelihood scores for the response tokens and top tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs_result: Option<LogprobsResult>,
    /// List of ratings for the safety of a response candidate. There is at most one rating per
    /// category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
    /// Output only. Token count for this candidate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i64>,
    /// Output only. Metadata related to url context retrieval tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_context_metadata: Option<UrlContextMetadata>,
}

/// Output only. Citation information for model-generated candidate. This field may be
/// populated with recitation information for any text included in the `content`. These are
/// passages that are "recited" from copyrighted material in the foundational LLM's training
/// data.
///
/// A collection of source attributions for a piece of content.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct CitationMetadata {
    /// Citations to sources for a specific response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_sources: Option<Vec<CitationSource>>,
}

/// A citation to a source for a portion of a specific response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct CitationSource {
    /// Optional. End of the attributed segment, exclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i64>,
    /// Optional. License for the GitHub project that is attributed as a source for segment.
    /// License info is required for code citations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Optional. Start of segment of the response that is attributed to this source. Index
    /// indicates the start of the segment, measured in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i64>,
    /// Optional. URI that is attributed as a source for a portion of the text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Optional. Output only. The reason why the model stopped generating tokens. If empty, the
/// model has not stopped generating tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum FinishReason {
    Blocklist,
    Escalation,
    #[serde(rename = "FINISH_REASON_UNSPECIFIED")]
    FinishReasonUnspecified,
    #[serde(rename = "IMAGE_OTHER")]
    ImageOther,
    #[serde(rename = "IMAGE_PROHIBITED_CONTENT")]
    ImageProhibitedContent,
    #[serde(rename = "IMAGE_RECITATION")]
    ImageRecitation,
    #[serde(rename = "IMAGE_SAFETY")]
    ImageSafety,
    Language,
    #[serde(rename = "MALFORMED_FUNCTION_CALL")]
    MalformedFunctionCall,
    #[serde(rename = "MALFORMED_RESPONSE")]
    MalformedResponse,
    #[serde(rename = "MAX_TOKENS")]
    MaxTokens,
    #[serde(rename = "MISSING_THOUGHT_SIGNATURE")]
    MissingThoughtSignature,
    #[serde(rename = "NO_IMAGE")]
    NoImage,
    Other,
    #[serde(rename = "PROHIBITED_CONTENT")]
    ProhibitedContent,
    Recitation,
    Safety,
    Spii,
    Stop,
    #[serde(rename = "TOO_MANY_TOOL_CALLS")]
    TooManyToolCalls,
    #[serde(rename = "UNEXPECTED_TOOL_CALL")]
    UnexpectedToolCall,
}

/// Attribution for a source that contributed to an answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GroundingAttribution {
    /// Grounding source content that makes up this attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// Output only. Identifier for the source contributing to this attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<AttributionSourceId>,
}

/// Output only. Identifier for the source contributing to this attribution.
///
/// Identifier for the source contributing to this attribution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct AttributionSourceId {
    /// Identifier for an inline passage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_passage: Option<GroundingPassageId>,
    /// Identifier for a `Chunk` fetched via Semantic Retriever.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_retriever_chunk: Option<SemanticRetrieverChunk>,
}

/// Identifier for an inline passage.
///
/// Identifier for a part within a `GroundingPassage`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GroundingPassageId {
    /// Output only. Index of the part within the `GenerateAnswerRequest`'s
    /// `GroundingPassage.content`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_index: Option<i64>,
    /// Output only. ID of the passage matching the `GenerateAnswerRequest`'s
    /// `GroundingPassage.id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passage_id: Option<String>,
}

/// Identifier for a `Chunk` fetched via Semantic Retriever.
///
/// Identifier for a `Chunk` retrieved via Semantic Retriever specified in the
/// `GenerateAnswerRequest` using `SemanticRetrieverConfig`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct SemanticRetrieverChunk {
    /// Output only. Name of the `Chunk` containing the attributed text. Example:
    /// `corpora/123/documents/abc/chunks/xyz`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<String>,
    /// Output only. Name of the source matching the request's `SemanticRetrieverConfig.source`.
    /// Example: `corpora/123` or `corpora/123/documents/abc`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Output only. Grounding metadata for the candidate. This field is populated for
/// `GenerateContent` calls.
///
/// Metadata returned to client when grounding is enabled.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GroundingMetadata {
    /// Optional. Resource name of the Google Maps widget context token that can be used with the
    /// PlacesContextElement widget in order to render contextual data. Only populated in the
    /// case that grounding with Google Maps is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_maps_widget_context_token: Option<String>,
    /// List of supporting references retrieved from specified grounding source. When streaming,
    /// this only contains the grounding chunks that have not been included in the grounding
    /// metadata of previous responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_chunks: Option<Vec<GroundingChunk>>,
    /// List of grounding support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_supports: Option<Vec<GoogleAiGenerativelanguageV1BetaGroundingSupport>>,
    /// Image search queries used for grounding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_search_queries: Option<Vec<String>>,
    /// Metadata related to retrieval in the grounding flow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_metadata: Option<RetrievalMetadata>,
    /// Optional. Google search entry for the following-up web searches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_entry_point: Option<SearchEntryPoint>,
    /// Web search queries for the following-up web search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_queries: Option<Vec<String>>,
}

/// A `GroundingChunk` represents a segment of supporting evidence that grounds the model's
/// response. It can be a chunk from the web, a retrieved context from a file, or information
/// from Google Maps.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GroundingChunk {
    /// Optional. Grounding chunk from image search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    /// Optional. Grounding chunk from Google Maps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maps: Option<Maps>,
    /// Optional. Grounding chunk from context retrieved by the file search tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieved_context: Option<RetrievedContext>,
    /// Grounding chunk from the web.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<Web>,
}

/// Optional. Grounding chunk from image search.
///
/// Chunk from image search.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Image {
    /// The root domain of the web page that the image is from, e.g. "example.com".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// The image asset URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_uri: Option<String>,
    /// The web page URI for attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    /// The title of the web page that the image is from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Optional. Grounding chunk from Google Maps.
///
/// A grounding chunk from Google Maps. A Maps chunk corresponds to a single place.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct Maps {
    /// Sources that provide answers about the features of a given place in Google Maps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_answer_sources: Option<PlaceAnswerSources>,
    /// The ID of the place, in `places/{place_id}` format. A user can use this ID to look up
    /// that place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_id: Option<String>,
    /// Text description of the place answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Title of the place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// URI reference of the place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Sources that provide answers about the features of a given place in Google Maps.
///
/// Collection of sources that provide answers about the features of a given place in Google
/// Maps. Each PlaceAnswerSources message corresponds to a specific place in Google Maps. The
/// Google Maps tool used these sources in order to answer questions about features of the
/// place (e.g: "does Bar Foo have Wifi" or "is Foo Bar wheelchair accessible?"). Currently
/// we only support review snippets as sources.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct PlaceAnswerSources {
    /// Snippets of reviews that are used to generate answers about the features of a given place
    /// in Google Maps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_snippets: Option<Vec<ReviewSnippet>>,
}

/// Encapsulates a snippet of a user review that answers a question about the features of a
/// specific place in Google Maps.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ReviewSnippet {
    /// A link that corresponds to the user review on Google Maps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_maps_uri: Option<String>,
    /// The ID of the review snippet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_id: Option<String>,
    /// Title of the review.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Optional. Grounding chunk from context retrieved by the file search tool.
///
/// Chunk from context retrieved by the file search tool.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct RetrievedContext {
    /// Optional. User-provided metadata about the retrieved context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_metadata: Option<Vec<GroundingChunkCustomMetadata>>,
    /// Optional. Name of the `FileSearchStore` containing the document. Example:
    /// `fileSearchStores/123`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search_store: Option<String>,
    /// Optional. The media blob resource name for multimodal file search results. Format:
    /// fileSearchStores/{file_search_store_id}/media/{blob_id}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_id: Option<String>,
    /// Optional. Page number of the retrieved context, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i64>,
    /// Optional. Text of the chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Optional. Title of the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional. URI reference of the semantic retrieval document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// User provided metadata about the GroundingFact.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GroundingChunkCustomMetadata {
    /// The key of the metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Optional. The numeric value of the metadata. The expected range for this value depends on
    /// the specific `key` used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric_value: Option<f64>,
    /// Optional. A list of string values for the metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_list_value: Option<GroundingChunkStringList>,
    /// Optional. The string value of the metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
}

/// Optional. A list of string values for the metadata.
///
/// A list of string values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct GroundingChunkStringList {
    /// The string values of the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// Grounding chunk from the web.
///
/// Chunk from the web.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct Web {
    /// Output only. Title of the chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Output only. URI reference of the chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Grounding support.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GoogleAiGenerativelanguageV1BetaGroundingSupport {
    /// Optional. Confidence score of the support references. Ranges from 0 to 1. 1 is the most
    /// confident. This list must have the same size as the grounding_chunk_indices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_scores: Option<Vec<f64>>,
    /// Optional. A list of indices (into 'grounding_chunk' in
    /// `response.candidate.grounding_metadata`) specifying the citations associated with the
    /// claim. For instance [1,3,4] means that grounding_chunk[1], grounding_chunk[3],
    /// grounding_chunk[4] are the retrieved content attributed to the claim. If the response is
    /// streaming, the grounding_chunk_indices refer to the indices across all responses. It is
    /// the client's responsibility to accumulate the grounding chunks from all responses (while
    /// maintaining the same order).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_chunk_indices: Option<Vec<i64>>,
    /// Output only. Indices into the `parts` field of the candidate's content. These indices
    /// specify which rendered parts are associated with this support source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_parts: Option<Vec<i64>>,
    /// Segment of the content this support belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<GoogleAiGenerativelanguageV1BetaSegment>,
}

/// Segment of the content this support belongs to.
///
/// Segment of the content.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct GoogleAiGenerativelanguageV1BetaSegment {
    /// End index in the given Part, measured in bytes. Offset from the start of the Part,
    /// exclusive, starting at zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i64>,
    /// The index of a Part object within its parent Content object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_index: Option<i64>,
    /// Start index in the given Part, measured in bytes. Offset from the start of the Part,
    /// inclusive, starting at zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i64>,
    /// The text corresponding to the segment from the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Metadata related to retrieval in the grounding flow.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct RetrievalMetadata {
    /// Optional. Score indicating how likely information from google search could help answer
    /// the prompt. The score is in the range [0, 1], where 0 is the least likely and 1 is the
    /// most likely. This score is only populated when google search grounding and dynamic
    /// retrieval is enabled. It will be compared to the threshold to determine whether to
    /// trigger google search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_search_dynamic_retrieval_score: Option<f64>,
}

/// Optional. Google search entry for the following-up web searches.
///
/// Google search entry point.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct SearchEntryPoint {
    /// Optional. Web content snippet that can be embedded in a web page or an app webview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_content: Option<String>,
    /// Optional. Base64 encoded JSON representing array of tuple.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_blob: Option<String>,
}

/// Output only. Log-likelihood scores for the response tokens and top tokens
///
/// Logprobs Result
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct LogprobsResult {
    /// Length = total number of decoding steps. The chosen candidates may or may not be in
    /// top_candidates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chosen_candidates: Option<Vec<LogprobsResultCandidate>>,
    /// Sum of log probabilities for all tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_probability_sum: Option<f64>,
    /// Length = total number of decoding steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_candidates: Option<Vec<TopCandidates>>,
}

/// Candidate for the logprobs token and score.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct LogprobsResultCandidate {
    /// The candidate's log probability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_probability: Option<f64>,
    /// The candidate’s token string value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// The candidate’s token id value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<i64>,
}

/// Candidates with top log probabilities at each decoding step.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct TopCandidates {
    /// Sorted by log probability in descending order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<LogprobsResultCandidate>>,
}

/// Safety rating for a piece of content. The safety rating contains the category of harm and
/// the harm probability level in that category for a piece of content. Content is classified
/// for safety across a number of harm categories and the probability of the harm
/// classification is included here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "google/")]
pub struct SafetyRating {
    /// Was this content blocked because of this rating?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
    /// Required. The category for this rating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    /// Required. The probability of harm for this content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability: Option<Probability>,
}

/// Required. The probability of harm for this content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Probability {
    #[serde(rename = "HARM_PROBABILITY_UNSPECIFIED")]
    HarmProbabilityUnspecified,
    High,
    Low,
    Medium,
    Negligible,
}

/// Output only. Metadata related to url context retrieval tool.
///
/// Metadata related to url context retrieval tool.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct UrlContextMetadata {
    /// List of url context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_metadata: Option<Vec<UrlMetadata>>,
}

/// Context of the a single url retrieval.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct UrlMetadata {
    /// Retrieved url by the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieved_url: Option<String>,
    /// Status of the url retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_retrieval_status: Option<UrlRetrievalStatus>,
}

/// Status of the url retrieval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum UrlRetrievalStatus {
    #[serde(rename = "URL_RETRIEVAL_STATUS_ERROR")]
    UrlRetrievalStatusError,
    #[serde(rename = "URL_RETRIEVAL_STATUS_PAYWALL")]
    UrlRetrievalStatusPaywall,
    #[serde(rename = "URL_RETRIEVAL_STATUS_SUCCESS")]
    UrlRetrievalStatusSuccess,
    #[serde(rename = "URL_RETRIEVAL_STATUS_UNSAFE")]
    UrlRetrievalStatusUnsafe,
    #[serde(rename = "URL_RETRIEVAL_STATUS_UNSPECIFIED")]
    UrlRetrievalStatusUnspecified,
}

/// Output only. The current model status of this model.
///
/// The status of the underlying model. This is used to indicate the stage of the underlying
/// model and the retirement time if applicable.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ModelStatus {
    /// A message explaining the model status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The stage of the underlying model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_stage: Option<ModelStage>,
    /// The time at which the model will be retired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retirement_time: Option<String>,
}

/// The stage of the underlying model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum ModelStage {
    Deprecated,
    Experimental,
    Legacy,
    #[serde(rename = "MODEL_STAGE_UNSPECIFIED")]
    ModelStageUnspecified,
    Preview,
    Retired,
    Stable,
    #[serde(rename = "UNSTABLE_EXPERIMENTAL")]
    UnstableExperimental,
}

/// Returns the prompt's feedback related to the content filters.
///
/// A set of the feedback metadata the prompt specified in `GenerateContentRequest.content`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct PromptFeedback {
    /// Optional. If set, the prompt was blocked and no candidates are returned. Rephrase the
    /// prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<BlockReason>,
    /// Ratings for safety of the prompt. There is at most one rating per category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

/// Optional. If set, the prompt was blocked and no candidates are returned. Rephrase the
/// prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum BlockReason {
    #[serde(rename = "BLOCK_REASON_UNSPECIFIED")]
    BlockReasonUnspecified,
    Blocklist,
    #[serde(rename = "IMAGE_SAFETY")]
    ImageSafety,
    Other,
    #[serde(rename = "PROHIBITED_CONTENT")]
    ProhibitedContent,
    Safety,
}

/// Output only. Metadata on the generation requests' token usage.
///
/// Metadata on the generation request's token usage.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct UsageMetadata {
    /// Number of tokens in the cached part of the prompt (the cached content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content_token_count: Option<i64>,
    /// Output only. List of modalities of the cached content in the request input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_tokens_details: Option<Vec<ModalityTokenCount>>,
    /// Total number of tokens across all the generated response candidates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<i64>,
    /// Output only. List of modalities that were returned in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_tokens_details: Option<Vec<ModalityTokenCount>>,
    /// Number of tokens in the prompt. When `cached_content` is set, this is still the total
    /// effective prompt size meaning this includes the number of tokens in the cached content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<i64>,
    /// Output only. List of modalities that were processed in the request input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<Vec<ModalityTokenCount>>,
    /// Output only. Service tier of the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    /// Output only. Number of tokens of thoughts for thinking models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thoughts_token_count: Option<i64>,
    /// Output only. Number of tokens present in tool-use prompt(s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_prompt_token_count: Option<i64>,
    /// Output only. List of modalities that were processed for tool-use request inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_prompt_tokens_details: Option<Vec<ModalityTokenCount>>,
    /// Total token count for the generation request (prompt + thoughts + response candidates).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<i64>,
}

/// Represents token counting info for a single modality.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "google/")]
pub struct ModalityTokenCount {
    /// The modality associated with this token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<Modality>,
    /// Number of tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i64>,
}

/// The modality associated with this token count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "google/")]
pub enum Modality {
    Audio,
    Document,
    Image,
    #[serde(rename = "MODALITY_UNSPECIFIED")]
    ModalityUnspecified,
    Text,
    Video,
}
