// Generated OpenAI types using quicktype
// Essential types for Elmir OpenAI integration
#![allow(clippy::large_enum_variant)]
#![allow(clippy::doc_lazy_continuation)]

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::openai_schemas;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: openai_schemas = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenaiSchemas {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_request: Option<CreateChatCompletionRequestClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_response: Option<CreateChatCompletionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_stream_response: Option<CreateChatCompletionStreamResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responses_request: Option<CreateResponseClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responses_response: Option<TheResponseObject>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionRequestClass {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Used by OpenAI to cache responses for similar requests to optimize your cache hit rates.
    /// Replaces the `user` field. [Learn
    /// more](https://platform.openai.com/docs/guides/prompt-caching).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    /// A stable identifier used to help detect users of your application that may be violating
    /// OpenAI's usage policies.
    /// The IDs should be a string that uniquely identifies each user. We recommend hashing their
    /// username or email address, in order to avoid sending us any identifying information.
    /// [Learn
    /// more](https://platform.openai.com/docs/guides/safety-best-practices#safety-identifiers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// An integer between 0 and 20 specifying the number of most likely tokens to
    /// return at each token position, each with an associated log probability.
    ///
    ///
    /// An integer between 0 and 20 specifying the number of most likely tokens to
    /// return at each token position, each with an associated log probability.
    /// `logprobs` must be set to `true` if this parameter is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// This field is being replaced by `safety_identifier` and `prompt_cache_key`. Use
    /// `prompt_cache_key` instead to maintain caching optimizations.
    /// A stable identifier for your end-users.
    /// Used to boost cache hit rates by better bucketing similar requests and  to help OpenAI
    /// detect and prevent abuse. [Learn
    /// more](https://platform.openai.com/docs/guides/safety-best-practices#safety-identifiers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Parameters for audio output. Required when audio output is requested with
    /// `modalities: ["audio"]`. [Learn more](https://platform.openai.com/docs/guides/audio).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<CreateChatCompletionRequestAudio>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on
    /// their existing frequency in the text so far, decreasing the model's
    /// likelihood to repeat the same line verbatim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Deprecated in favor of `tool_choice`.
    ///
    /// Controls which (if any) function is called by the model.
    ///
    /// `none` means the model will not call a function and instead generates a
    /// message.
    ///
    /// `auto` means the model can pick between generating a message or calling a
    /// function.
    ///
    /// Specifying a particular function via `{"name": "my_function"}` forces the
    /// model to call that function.
    ///
    /// `none` is the default when no functions are present. `auto` is the default
    /// if functions are present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallUnion>,
    /// Deprecated in favor of `tools`.
    ///
    /// A list of functions the model may generate JSON inputs for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<ChatCompletionFunctions>>,
    /// Modify the likelihood of specified tokens appearing in the completion.
    ///
    /// Accepts a JSON object that maps tokens (specified by their token ID in the
    /// tokenizer) to an associated bias value from -100 to 100. Mathematically,
    /// the bias is added to the logits generated by the model prior to sampling.
    /// The exact effect will vary per model, but values between -1 and 1 should
    /// decrease or increase likelihood of selection; values like -100 or 100
    /// should result in a ban or exclusive selection of the relevant token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, i64>>,
    /// Whether to return log probabilities of the output tokens or not. If true,
    /// returns the log probabilities of each output token returned in the
    /// `content` of `message`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// An upper bound for the number of tokens that can be generated for a completion, including
    /// visible output tokens and [reasoning
    /// tokens](https://platform.openai.com/docs/guides/reasoning).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<i64>,
    /// The maximum number of [tokens](/tokenizer) that can be generated in the
    /// chat completion. This value can be used to control
    /// [costs](https://openai.com/api/pricing/) for text generated via API.
    ///
    /// This value is now deprecated in favor of `max_completion_tokens`, and is
    /// not compatible with [o-series models](https://platform.openai.com/docs/guides/reasoning).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    /// A list of messages comprising the conversation so far. Depending on the
    /// [model](https://platform.openai.com/docs/models) you use, different message types
    /// (modalities) are
    /// supported, like [text](https://platform.openai.com/docs/guides/text-generation),
    /// [images](https://platform.openai.com/docs/guides/vision), and
    /// [audio](https://platform.openai.com/docs/guides/audio).
    pub messages: Vec<ChatCompletionRequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<ResponseModality>>,
    /// Model ID used to generate the response, like `gpt-4o` or `o3`. OpenAI
    /// offers a wide range of models with different capabilities, performance
    /// characteristics, and price points. Refer to the [model
    /// guide](https://platform.openai.com/docs/models)
    /// to browse and compare available models.
    pub model: String,
    /// How many chat completion choices to generate for each input message. Note that you will
    /// be charged based on the number of generated tokens across all of the choices. Keep `n` as
    /// `1` to minimize costs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Configuration for a [Predicted
    /// Output](https://platform.openai.com/docs/guides/predicted-outputs),
    /// which can greatly improve response times when large parts of the model
    /// response are known ahead of time. This is most common when you are
    /// regenerating a file with only minor changes to most of the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<StaticContent>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on
    /// whether they appear in the text so far, increasing the model's likelihood
    /// to talk about new topics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    /// An object specifying the format that the model must output.
    ///
    /// Setting to `{ "type": "json_schema", "json_schema": {...} }` enables
    /// Structured Outputs which ensures the model will match your supplied JSON
    /// schema. Learn more in the [Structured Outputs
    /// guide](https://platform.openai.com/docs/guides/structured-outputs).
    ///
    /// Setting to `{ "type": "json_object" }` enables the older JSON mode, which
    /// ensures the message the model generates is valid JSON. Using `json_schema`
    /// is preferred for models that support it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormatClass>,
    /// This feature is in Beta.
    /// If specified, our system will make a best effort to sample deterministically, such that
    /// repeated requests with the same `seed` and parameters should return the same result.
    /// Determinism is not guaranteed, and you should refer to the `system_fingerprint` response
    /// parameter to monitor changes in the backend.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopConfiguration>,
    /// Whether or not to store the output of this chat completion request for
    /// use in our [model distillation](https://platform.openai.com/docs/guides/distillation) or
    /// [evals](https://platform.openai.com/docs/guides/evals) products.
    ///
    /// Supports text and image inputs. Note: image inputs over 8MB will be dropped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    /// If set to true, the model response data will be streamed to the client
    /// as it is generated using [server-sent
    /// events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format).
    /// See the [Streaming section
    /// below](https://platform.openai.com/docs/api-reference/chat/streaming)
    /// for more information, along with the [streaming
    /// responses](https://platform.openai.com/docs/guides/streaming-responses)
    /// guide for more information on how to handle the streaming events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<ChatCompletionStreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,
    /// A list of tools the model may call. You can provide either
    /// [custom tools](https://platform.openai.com/docs/guides/function-calling#custom-tools) or
    /// [function tools](https://platform.openai.com/docs/guides/function-calling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolElement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<WebSearchContextSize>,
    /// This tool searches the web for relevant results to use in a response.
    /// Learn more about the [web search
    /// tool](https://platform.openai.com/docs/guides/tools-web-search?api-mode=chat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_options: Option<WebSearch>,
}

/// Parameters for audio output. Required when audio output is requested with
/// `modalities: ["audio"]`. [Learn more](https://platform.openai.com/docs/guides/audio).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionRequestAudio {
    /// Specifies the output audio format. Must be one of `wav`, `mp3`, `flac`,
    /// `opus`, or `pcm16`.
    pub format: AudioFormat,
    /// The voice the model uses to respond. Supported voices are
    /// `alloy`, `ash`, `ballad`, `coral`, `echo`, `fable`, `nova`, `onyx`, `sage`, and `shimmer`.
    pub voice: String,
}

/// Specifies the output audio format. Must be one of `wav`, `mp3`, `flac`,
/// `opus`, or `pcm16`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Aac,
    Flac,
    Mp3,
    Opus,
    Pcm16,
    Wav,
}

/// Deprecated in favor of `tool_choice`.
///
/// Controls which (if any) function is called by the model.
///
/// `none` means the model will not call a function and instead generates a
/// message.
///
/// `auto` means the model can pick between generating a message or calling a
/// function.
///
/// Specifying a particular function via `{"name": "my_function"}` forces the
/// model to call that function.
///
/// `none` is the default when no functions are present. `auto` is the default
/// if functions are present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionCallUnion {
    ChatCompletionFunctionCallOption(ChatCompletionFunctionCallOption),
    Enum(FunctionCallMode),
}

/// Specifying a particular function via `{"name": "my_function"}` forces the model to call
/// that function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionFunctionCallOption {
    /// The name of the function to call.
    pub name: String,
}

/// `none` means the model will not call a function and instead generates a message. `auto`
/// means the model can pick between generating a message or calling a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCallMode {
    Auto,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionFunctions {
    /// A description of what the function does, used by the model to choose when and how to call
    /// the function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The name of the function to be called. Must be a-z, A-Z, 0-9, or contain underscores and
    /// dashes, with a maximum length of 64.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, Option<serde_json::Value>>>,
}

/// Developer-provided instructions that the model should follow, regardless of
/// messages sent by the user. With o1 models and newer, `developer` messages
/// replace the previous `system` messages.
///
///
/// Developer-provided instructions that the model should follow, regardless of
/// messages sent by the user. With o1 models and newer, use `developer` messages
/// for this purpose instead.
///
///
/// Messages sent by an end user, containing prompts or additional context
/// information.
///
///
/// Messages sent by the model in response to user messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessage {
    /// The contents of the developer message.
    ///
    /// The contents of the system message.
    ///
    /// The contents of the user message.
    ///
    ///
    /// The contents of the tool message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ChatCompletionRequestMessageContent>,
    /// An optional name for the participant. Provides the model information to differentiate
    /// between participants of the same role.
    ///
    /// The name of the function to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The role of the messages author, in this case `developer`.
    ///
    /// The role of the messages author, in this case `system`.
    ///
    /// The role of the messages author, in this case `user`.
    ///
    /// The role of the messages author, in this case `assistant`.
    ///
    /// The role of the messages author, in this case `tool`.
    ///
    /// The role of the messages author, in this case `function`.
    pub role: ChatCompletionRequestMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionRequestMessageAudio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<ChatCompletionRequestMessageFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call that this message is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Data about a previous audio response from the model.
/// [Learn more](https://platform.openai.com/docs/guides/audio).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageAudio {
    /// Unique identifier for a previous audio response from the model.
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestMessageContent {
    ChatCompletionRequestMessageContentPartArray(Vec<ChatCompletionRequestMessageContentPart>),
    String(String),
}

/// An array of content parts with a defined type. For developer messages, only type `text`
/// is supported.
///
/// Learn about [text inputs](https://platform.openai.com/docs/guides/text-generation).
///
///
/// An array of content parts with a defined type. Supported options differ based on the
/// [model](https://platform.openai.com/docs/models) being used to generate the response. Can
/// contain text inputs.
///
/// An array of content parts with a defined type. For system messages, only type `text` is
/// supported.
///
/// An array of content parts with a defined type. For tool messages, only type `text` is
/// supported.
///
/// An array of content parts with a defined type. Supported options differ based on the
/// [model](https://platform.openai.com/docs/models) being used to generate the response. Can
/// contain text, image, or audio inputs.
///
/// Learn about [image inputs](https://platform.openai.com/docs/guides/vision).
///
///
/// Learn about [audio inputs](https://platform.openai.com/docs/guides/audio).
///
///
/// Learn about [file inputs](https://platform.openai.com/docs/guides/text) for text
/// generation.
///
///
/// An array of content parts with a defined type. Can be one or more of type `text`, or
/// exactly one of type `refusal`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPart {
    /// The text content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The type of the content part.
    ///
    /// The type of the content part. Always `input_audio`.
    ///
    /// The type of the content part. Always `file`.
    #[serde(rename = "type")]
    pub chat_completion_request_message_content_part_type: PurpleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<ArrayOfContentPartInputAudio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<File>,
    /// The refusal message generated by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// The type of the content part.
///
/// The type of the content part. Always `input_audio`.
///
/// The type of the content part. Always `file`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurpleType {
    File,
    #[serde(rename = "image_url")]
    ImageUrl,
    #[serde(rename = "input_audio")]
    InputAudio,
    Refusal,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    /// The base64 encoded file data, used when passing the file to the model
    /// as a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    /// The ID of an uploaded file to use as input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// The name of the file, used when passing the file to the model as a
    /// string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageUrl {
    /// Specifies the detail level of the image. Learn more in the [Vision
    /// guide](https://platform.openai.com/docs/guides/vision#low-or-high-fidelity-image-understanding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetail>,
    /// Either a URL of the image or the base64 encoded image data.
    pub url: String,
}

/// Specifies the detail level of the image. Learn more in the [Vision
/// guide](https://platform.openai.com/docs/guides/vision#low-or-high-fidelity-image-understanding).
///
/// The detail level of the image to be sent to the model. One of `high`, `low`, or `auto`.
/// Defaults to `auto`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageDetail {
    Auto,
    High,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayOfContentPartInputAudio {
    /// Base64 encoded audio data.
    pub data: String,
    /// The format of the encoded audio data. Currently supports "wav" and "mp3".
    pub format: InputAudioFormat,
}

/// The format of the encoded audio data. Currently supports "wav" and "mp3".
///
///
/// The format of the audio data. Currently supported formats are `mp3` and
/// `wav`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputAudioFormat {
    Mp3,
    Wav,
}

/// Deprecated and replaced by `tool_calls`. The name and arguments of a function that should
/// be called, as generated by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageFunctionCall {
    /// The arguments to call the function with, as generated by the model in JSON format. Note
    /// that the model does not always generate valid JSON, and may hallucinate parameters not
    /// defined by your function schema. Validate the arguments in your code before calling your
    /// function.
    pub arguments: String,
    /// The name of the function to call.
    pub name: String,
}

/// The role of the messages author, in this case `developer`.
///
/// The role of the messages author, in this case `system`.
///
/// The role of the messages author, in this case `user`.
///
/// The role of the messages author, in this case `assistant`.
///
/// The role of the messages author, in this case `tool`.
///
/// The role of the messages author, in this case `function`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatCompletionRequestMessageRole {
    Assistant,
    Developer,
    Function,
    System,
    Tool,
    User,
}

/// The tool calls generated by the model, such as function calls.
///
/// A call to a function tool created by the model.
///
///
/// A call to a custom tool created by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// The function that the model called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<PurpleFunction>,
    /// The ID of the tool call.
    pub id: String,
    /// The type of the tool. Currently, only `function` is supported.
    ///
    /// The type of the tool. Always `custom`.
    #[serde(rename = "type")]
    pub tool_call_type: ToolType,
    /// The custom tool that the model called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<ToolCallCustom>,
}

/// The custom tool that the model called.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallCustom {
    /// The input for the custom tool call generated by the model.
    pub input: String,
    /// The name of the custom tool to call.
    pub name: String,
}

/// The function that the model called.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PurpleFunction {
    /// The arguments to call the function with, as generated by the model in JSON format. Note
    /// that the model does not always generate valid JSON, and may hallucinate parameters not
    /// defined by your function schema. Validate the arguments in your code before calling your
    /// function.
    pub arguments: String,
    /// The name of the function to call.
    pub name: String,
}

/// The type of the tool. Currently, only `function` is supported.
///
/// The type of the tool. Always `custom`.
///
/// The type of the custom tool. Always `custom`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Custom,
    Function,
}

/// Output types that you would like the model to generate.
/// Most models are capable of generating text, which is the default:
///
/// `["text"]`
///
/// The `gpt-4o-audio-preview` model can also be used to
/// [generate audio](https://platform.openai.com/docs/guides/audio). To request that this
/// model generate
/// both text and audio responses, you can use:
///
/// `["text", "audio"]`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseModality {
    Audio,
    Text,
}

/// Configuration for a [Predicted
/// Output](https://platform.openai.com/docs/guides/predicted-outputs),
/// which can greatly improve response times when large parts of the model
/// response are known ahead of time. This is most common when you are
/// regenerating a file with only minor changes to most of the content.
///
///
/// Static predicted output content, such as the content of a text file that is
/// being regenerated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticContent {
    /// The content that should be matched when generating a model response.
    /// If generated tokens would match this content, the entire model response
    /// can be returned much more quickly.
    pub content: PredictionContent,
    /// The type of the predicted content you want to provide. This type is
    /// currently always `content`.
    #[serde(rename = "type")]
    pub static_content_type: PredictionType,
}

/// The contents of the system message.
///
/// The contents of the tool message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PredictionContent {
    ContentPartArray(Vec<ContentPart>),
    String(String),
}

/// An array of content parts with a defined type. For developer messages, only type `text`
/// is supported.
///
/// Learn about [text inputs](https://platform.openai.com/docs/guides/text-generation).
///
///
/// An array of content parts with a defined type. Supported options differ based on the
/// [model](https://platform.openai.com/docs/models) being used to generate the response. Can
/// contain text inputs.
///
/// An array of content parts with a defined type. For system messages, only type `text` is
/// supported.
///
/// An array of content parts with a defined type. For tool messages, only type `text` is
/// supported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentPart {
    /// The text content.
    pub text: String,
    /// The type of the content part.
    #[serde(rename = "type")]
    pub content_part_type: FluffyType,
}

/// The type of the content part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FluffyType {
    Text,
}

/// The type of the predicted content you want to provide. This type is
/// currently always `content`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionType {
    Content,
}

/// Constrains effort on reasoning for
/// [reasoning models](https://platform.openai.com/docs/guides/reasoning).
/// Currently supported values are `minimal`, `low`, `medium`, and `high`. Reducing
/// reasoning effort can result in faster responses and fewer tokens used
/// on reasoning in a response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    High,
    Low,
    Medium,
    Minimal,
}

/// An object specifying the format that the model must output.
///
/// Setting to `{ "type": "json_schema", "json_schema": {...} }` enables
/// Structured Outputs which ensures the model will match your supplied JSON
/// schema. Learn more in the [Structured Outputs
/// guide](https://platform.openai.com/docs/guides/structured-outputs).
///
/// Setting to `{ "type": "json_object" }` enables the older JSON mode, which
/// ensures the message the model generates is valid JSON. Using `json_schema`
/// is preferred for models that support it.
///
///
/// Default response format. Used to generate text responses.
///
///
/// JSON Schema response format. Used to generate structured JSON responses.
/// Learn more about [Structured
/// Outputs](https://platform.openai.com/docs/guides/structured-outputs).
///
///
/// JSON object response format. An older method of generating JSON responses.
/// Using `json_schema` is recommended for models that support it. Note that the
/// model will not generate JSON without a system or user message instructing it
/// to do so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFormatClass {
    /// The type of response format being defined. Always `text`.
    ///
    /// The type of response format being defined. Always `json_schema`.
    ///
    /// The type of response format being defined. Always `json_object`.
    #[serde(rename = "type")]
    pub text_type: ResponseFormatType,
    /// Structured Outputs configuration options, including a JSON Schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonSchema>,
}

/// Structured Outputs configuration options, including a JSON Schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchema {
    /// A description of what the response format is for, used by the model to
    /// determine how to respond in the format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The name of the response format. Must be a-z, A-Z, 0-9, or contain
    /// underscores and dashes, with a maximum length of 64.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<HashMap<String, Option<serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// The type of response format being defined. Always `text`.
///
/// The type of response format being defined. Always `json_schema`.
///
/// The type of response format being defined. Always `json_object`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormatType {
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema,
    Text,
}

/// Specifies the processing type used for serving the request.
/// - If set to 'auto', then the request will be processed with the service tier configured
/// in the Project settings. Unless otherwise configured, the Project will use 'default'.
/// - If set to 'default', then the request will be processed with the standard pricing and
/// performance for the selected model.
/// - If set to '[flex](https://platform.openai.com/docs/guides/flex-processing)' or
/// '[priority](https://openai.com/api-priority-processing/)', then the request will be
/// processed with the corresponding service tier.
/// - When not set, the default behavior is 'auto'.
///
/// When the `service_tier` parameter is set, the response body will include the
/// `service_tier` value based on the processing mode actually used to serve the request.
/// This response value may be different from the value set in the parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Priority,
    Scale,
}

/// Not supported with latest reasoning models `o3` and `o4-mini`.
///
/// Up to 4 sequences where the API will stop generating further tokens. The
/// returned text will not contain the stop sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopConfiguration {
    String(String),
    StringArray(Vec<String>),
}

/// Options for streaming response. Only set this when you set `stream: true`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionStreamOptions {
    /// When true, stream obfuscation will be enabled. Stream obfuscation adds
    /// random characters to an `obfuscation` field on streaming delta events to
    /// normalize payload sizes as a mitigation to certain side-channel attacks.
    /// These obfuscation fields are included by default, but add a small amount
    /// of overhead to the data stream. You can set `include_obfuscation` to
    /// false to optimize for bandwidth if you trust the network links between
    /// your application and the OpenAI API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
    /// If set, an additional chunk will be streamed before the `data: [DONE]`
    /// message. The `usage` field on this chunk shows the token usage statistics
    /// for the entire request, and the `choices` field will always be an empty
    /// array.
    ///
    /// All other chunks will also include a `usage` field, but with a null
    /// value. **NOTE:** If the stream is interrupted, you may not receive the
    /// final usage chunk which contains the total token usage for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

/// Controls which (if any) tool is called by the model.
/// `none` means the model will not call any tool and instead generates a message.
/// `auto` means the model can pick between generating a message or calling one or more
/// tools.
/// `required` means the model must call one or more tools.
/// Specifying a particular tool via `{"type": "function", "function": {"name":
/// "my_function"}}` forces the model to call that tool.
///
/// `none` is the default when no tools are present. `auto` is the default if tools are
/// present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionToolChoiceOption {
    Enum(Auto),
    FunctionToolChoiceClass(FunctionToolChoiceClass),
}

/// Constrains the tools available to the model to a pre-defined set.
///
///
/// Specifies a tool the model should use. Use to force the model to call a specific
/// function.
///
/// Specifies a tool the model should use. Use to force the model to call a specific custom
/// tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionToolChoiceClass {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<AllowedTools>,
    /// Allowed tool configuration type. Always `allowed_tools`.
    ///
    /// For function calling, the type is always `function`.
    ///
    /// For custom tool calling, the type is always `custom`.
    #[serde(rename = "type")]
    pub allowed_tools_type: FunctionToolChoiceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<AllowedToolsFunction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<AllowedToolsCustom>,
}

/// Constrains the tools available to the model to a pre-defined set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowedTools {
    /// Constrains the tools available to the model to a pre-defined set.
    ///
    /// `auto` allows the model to pick from among the allowed tools and generate a
    /// message.
    ///
    /// `required` requires the model to call one or more of the allowed tools.
    pub mode: Mode,
    /// A list of tool definitions that the model should be allowed to call.
    ///
    /// For the Chat Completions API, the list of tool definitions might look like:
    /// ```json
    /// [
    /// { "type": "function", "function": { "name": "get_weather" } },
    /// { "type": "function", "function": { "name": "get_time" } }
    /// ]
    /// ```
    pub tools: Vec<HashMap<String, Option<serde_json::Value>>>,
}

/// Constrains the tools available to the model to a pre-defined set.
///
/// `auto` allows the model to pick from among the allowed tools and generate a
/// message.
///
/// `required` requires the model to call one or more of the allowed tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Auto,
    Required,
}

/// Allowed tool configuration type. Always `allowed_tools`.
///
/// For function calling, the type is always `function`.
///
/// For custom tool calling, the type is always `custom`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionToolChoiceType {
    #[serde(rename = "allowed_tools")]
    AllowedTools,
    Custom,
    Function,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowedToolsCustom {
    /// The name of the custom tool to call.
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowedToolsFunction {
    /// The name of the function to call.
    pub name: String,
}

/// `none` means the model will not call any tool and instead generates a message. `auto`
/// means the model can pick between generating a message or calling one or more tools.
/// `required` means the model must call one or more tools.
///
///
/// Controls which (if any) tool is called by the model.
///
/// `none` means the model will not call any tool and instead generates a message.
///
/// `auto` means the model can pick between generating a message or calling one or
/// more tools.
///
/// `required` means the model must call one or more tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Auto {
    Auto,
    None,
    Required,
}

/// A function tool that can be used to generate a response.
///
///
/// A custom tool that processes input using a specified format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionObject>,
    /// The type of the tool. Currently, only `function` is supported.
    ///
    /// The type of the custom tool. Always `custom`.
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    /// Properties of the custom tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<CustomToolProperties>,
}

/// Properties of the custom tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomToolProperties {
    /// Optional description of the custom tool, used to provide more context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The input format for the custom tool. Default is unconstrained text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<CustomFormat>,
    /// The name of the custom tool, used to identify it in tool calls.
    pub name: String,
}

/// The input format for the custom tool. Default is unconstrained text.
///
///
/// Unconstrained free-form text.
///
/// A grammar defined by the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomFormat {
    /// Unconstrained text format. Always `text`.
    ///
    /// Grammar format. Always `grammar`.
    #[serde(rename = "type")]
    pub format_type: FormatType,
    /// Your chosen grammar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar: Option<GrammarFormat>,
}

/// Unconstrained text format. Always `text`.
///
/// Grammar format. Always `grammar`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatType {
    Grammar,
    Text,
}

/// Your chosen grammar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrammarFormat {
    /// The grammar definition.
    pub definition: String,
    /// The syntax of the grammar definition. One of `lark` or `regex`.
    pub syntax: Syntax,
}

/// The syntax of the grammar definition. One of `lark` or `regex`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Syntax {
    Lark,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionObject {
    /// A description of what the function does, used by the model to choose when and how to call
    /// the function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The name of the function to be called. Must be a-z, A-Z, 0-9, or contain underscores and
    /// dashes, with a maximum length of 64.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, Option<serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Constrains the verbosity of the model's response. Lower values will result in
/// more concise responses, while higher values will result in more verbose responses.
/// Currently supported values are `low`, `medium`, and `high`.
///
///
/// High level guidance for the amount of context window space to use for the
/// search. One of `low`, `medium`, or `high`. `medium` is the default.
///
///
/// High level guidance for the amount of context window space to use for the search. One of
/// `low`, `medium`, or `high`. `medium` is the default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchContextSize {
    High,
    Low,
    Medium,
}

/// This tool searches the web for relevant results to use in a response.
/// Learn more about the [web search
/// tool](https://platform.openai.com/docs/guides/tools-web-search?api-mode=chat).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<WebSearchContextSize>,
    /// Approximate location parameters for the search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<UserLocation>,
}

/// Approximate location parameters for the search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserLocation {
    pub approximate: WebSearchLocation,
    /// The type of location approximation. Always `approximate`.
    #[serde(rename = "type")]
    pub user_location_type: UserLocationType,
}

/// Approximate location parameters for the search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchLocation {
    /// Free text input for the city of the user, e.g. `San Francisco`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// The two-letter
    /// [ISO country code](https://en.wikipedia.org/wiki/ISO_3166-1) of the user,
    /// e.g. `US`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Free text input for the region of the user, e.g. `California`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// The [IANA timezone](https://timeapi.io/documentation/iana-timezones)
    /// of the user, e.g. `America/Los_Angeles`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// The type of location approximation. Always `approximate`.
///
///
/// The type of location approximation. Always `approximate`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserLocationType {
    Approximate,
}

/// Represents a chat completion response returned by model, based on the provided input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionResponse {
    /// A list of chat completion choices. Can be more than one if `n` is greater than 1.
    pub choices: Vec<ChatResponseChoice>,
    /// The Unix timestamp (in seconds) of when the chat completion was created.
    pub created: i64,
    /// A unique identifier for the chat completion.
    pub id: String,
    /// The model used for the chat completion.
    pub model: String,
    /// The object type, which is always `chat.completion`.
    pub object: ChatResponseObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    /// This fingerprint represents the backend configuration that the model runs with.
    ///
    /// Can be used in conjunction with the `seed` request parameter to understand when backend
    /// changes have been made that might impact determinism.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<CompletionUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponseChoice {
    /// The reason the model stopped generating tokens. This will be `stop` if the model hit a
    /// natural stop point or a provided stop sequence,
    /// `length` if the maximum number of tokens specified in the request was reached,
    /// `content_filter` if content was omitted due to a flag from our content filters,
    /// `tool_calls` if the model called a tool, or `function_call` (deprecated) if the model
    /// called a function.
    pub finish_reason: FinishReason,
    /// The index of the choice in the list of choices.
    pub index: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<PurpleLogprobs>,
    pub message: ChatCompletionResponseMessage,
}

/// The reason the model stopped generating tokens. This will be `stop` if the model hit a
/// natural stop point or a provided stop sequence,
/// `length` if the maximum number of tokens specified in the request was reached,
/// `content_filter` if content was omitted due to a flag from our content filters,
/// `tool_calls` if the model called a tool, or `function_call` (deprecated) if the model
/// called a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    #[serde(rename = "content_filter")]
    ContentFilter,
    #[serde(rename = "function_call")]
    FunctionCall,
    Length,
    Stop,
    #[serde(rename = "tool_calls")]
    ToolCalls,
}

/// Log probability information for the choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PurpleLogprobs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ChatCompletionTokenLogprob>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Vec<ChatCompletionTokenLogprob>>,
}

/// A list of message content tokens with log probability information.
///
/// A list of message refusal tokens with log probability information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionTokenLogprob {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<i64>>,
    /// The log probability of this token, if it is within the top 20 most likely tokens.
    /// Otherwise, the value `-9999.0` is used to signify that the token is very unlikely.
    pub logprob: f64,
    /// The token.
    pub token: String,
    /// List of the most likely tokens and their log probability, at this token position. In rare
    /// cases, there may be fewer than the number of requested `top_logprobs` returned.
    pub top_logprobs: Vec<TopLogprob>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogprob {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<i64>>,
    /// The log probability of this token, if it is within the top 20 most likely tokens.
    /// Otherwise, the value `-9999.0` is used to signify that the token is very unlikely.
    pub logprob: f64,
    /// The token.
    pub token: String,
}

/// A chat completion message generated by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionResponseMessage {
    /// Annotations for the message, when applicable, as when using the
    /// [web search tool](https://platform.openai.com/docs/guides/tools-web-search?api-mode=chat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<AnnotationElement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<MessageAudio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Deprecated and replaced by `tool_calls`. The name and arguments of a function that should
    /// be called, as generated by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<MessageFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// The role of the author of this message.
    pub role: MessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// A URL citation when using web search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotationElement {
    /// The type of the URL citation. Always `url_citation`.
    #[serde(rename = "type")]
    pub annotation_type: AnnotationType,
    /// A URL citation when using web search.
    pub url_citation: UrlCitation,
}

/// The type of the URL citation. Always `url_citation`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationType {
    #[serde(rename = "url_citation")]
    UrlCitation,
}

/// A URL citation when using web search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlCitation {
    /// The index of the last character of the URL citation in the message.
    pub end_index: i64,
    /// The index of the first character of the URL citation in the message.
    pub start_index: i64,
    /// The title of the web resource.
    pub title: String,
    /// The URL of the web resource.
    pub url: String,
}

/// If the audio output modality is requested, this object contains data
/// about the audio response from the model. [Learn
/// more](https://platform.openai.com/docs/guides/audio).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageAudio {
    /// Base64 encoded audio bytes generated by the model, in the format
    /// specified in the request.
    pub data: String,
    /// The Unix timestamp (in seconds) for when this audio response will
    /// no longer be accessible on the server for use in multi-turn
    /// conversations.
    pub expires_at: i64,
    /// Unique identifier for this audio response.
    pub id: String,
    /// Transcript of the audio generated by the model.
    pub transcript: String,
}

/// Deprecated and replaced by `tool_calls`. The name and arguments of a function that should
/// be called, as generated by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageFunctionCall {
    /// The arguments to call the function with, as generated by the model in JSON format. Note
    /// that the model does not always generate valid JSON, and may hallucinate parameters not
    /// defined by your function schema. Validate the arguments in your code before calling your
    /// function.
    pub arguments: String,
    /// The name of the function to call.
    pub name: String,
}

/// The role of the author of this message.
///
/// The role of the output message. Always `assistant`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    Assistant,
}

/// The object type, which is always `chat.completion`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChatResponseObject {
    #[serde(rename = "chat.completion")]
    ChatCompletion,
}

/// Usage statistics for the completion request.
///
/// An optional field that will only be present when you set
/// `stream_options: {"include_usage": true}` in your request. When present, it
/// contains a null value **except for the last chunk** which contains the
/// token usage statistics for the entire request.
///
/// **NOTE:** If the stream is interrupted or cancelled, you may not
/// receive the final usage chunk which contains the total token usage for
/// the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionUsage {
    /// Number of tokens in the generated completion.
    pub completion_tokens: i64,
    /// Breakdown of tokens used in a completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    /// Number of tokens in the prompt.
    pub prompt_tokens: i64,
    /// Breakdown of tokens used in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// Total number of tokens used in the request (prompt + completion).
    pub total_tokens: i64,
}

/// Breakdown of tokens used in a completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    /// When using Predicted Outputs, the number of tokens in the
    /// prediction that appeared in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_prediction_tokens: Option<i64>,
    /// Audio input tokens generated by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<i64>,
    /// Tokens generated by the model for reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<i64>,
    /// When using Predicted Outputs, the number of tokens in the
    /// prediction that did not appear in the completion. However, like
    /// reasoning tokens, these tokens are still counted in the total
    /// completion tokens for purposes of billing, output, and context window
    /// limits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_prediction_tokens: Option<i64>,
}

/// Breakdown of tokens used in the prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    /// Audio input tokens present in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<i64>,
    /// Cached tokens present in the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<i64>,
}

/// Represents a streamed chunk of a chat completion response returned
/// by the model, based on the provided input.
/// [Learn more](https://platform.openai.com/docs/guides/streaming-responses).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionStreamResponse {
    /// A list of chat completion choices. Can contain more than one elements if `n` is greater
    /// than 1. Can also be empty for the
    /// last chunk if you set `stream_options: {"include_usage": true}`.
    pub choices: Vec<ChatStreamResponseChoice>,
    /// The Unix timestamp (in seconds) of when the chat completion was created. Each chunk has
    /// the same timestamp.
    pub created: i64,
    /// A unique identifier for the chat completion. Each chunk has the same ID.
    pub id: String,
    /// The model to generate the completion.
    pub model: String,
    /// The object type, which is always `chat.completion.chunk`.
    pub object: ChatStreamResponseObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    /// This fingerprint represents the backend configuration that the model runs with.
    /// Can be used in conjunction with the `seed` request parameter to understand when backend
    /// changes have been made that might impact determinism.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// An optional field that will only be present when you set
    /// `stream_options: {"include_usage": true}` in your request. When present, it
    /// contains a null value **except for the last chunk** which contains the
    /// token usage statistics for the entire request.
    ///
    /// **NOTE:** If the stream is interrupted or cancelled, you may not
    /// receive the final usage chunk which contains the total token usage for
    /// the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<CompletionUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatStreamResponseChoice {
    pub delta: ChatCompletionStreamResponseDelta,
    /// The reason the model stopped generating tokens. This will be `stop` if the model hit a
    /// natural stop point or a provided stop sequence,
    /// `length` if the maximum number of tokens specified in the request was reached,
    /// `content_filter` if content was omitted due to a flag from our content filters,
    /// `tool_calls` if the model called a tool, or `function_call` (deprecated) if the model
    /// called a function.
    pub finish_reason: FinishReason,
    /// The index of the choice in the list of choices.
    pub index: i64,
    /// Log probability information for the choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<FluffyLogprobs>,
}

/// A chat completion delta generated by streamed model responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionStreamResponseDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Deprecated and replaced by `tool_calls`. The name and arguments of a function that should
    /// be called, as generated by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<DeltaFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// The role of the author of this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<DeltaRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCallChunk>>,
}

/// Deprecated and replaced by `tool_calls`. The name and arguments of a function that should
/// be called, as generated by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaFunctionCall {
    /// The arguments to call the function with, as generated by the model in JSON format. Note
    /// that the model does not always generate valid JSON, and may hallucinate parameters not
    /// defined by your function schema. Validate the arguments in your code before calling your
    /// function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// The name of the function to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The role of the author of this message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaRole {
    Assistant,
    Developer,
    System,
    Tool,
    User,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageToolCallChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FluffyFunction>,
    /// The ID of the tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub index: i64,
    /// The type of the tool. Currently, only `function` is supported.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_completion_message_tool_call_chunk_type: Option<TentacledType>,
}

/// The type of the tool. Currently, only `function` is supported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TentacledType {
    Function,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FluffyFunction {
    /// The arguments to call the function with, as generated by the model in JSON format. Note
    /// that the model does not always generate valid JSON, and may hallucinate parameters not
    /// defined by your function schema. Validate the arguments in your code before calling your
    /// function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// The name of the function to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Log probability information for the choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FluffyLogprobs {
    /// A list of message content tokens with log probability information.
    pub content: Vec<ChatCompletionTokenLogprob>,
    /// A list of message refusal tokens with log probability information.
    pub refusal: Vec<ChatCompletionTokenLogprob>,
}

/// The object type, which is always `chat.completion.chunk`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChatStreamResponseObject {
    #[serde(rename = "chat.completion.chunk")]
    ChatCompletionChunk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateResponseClass {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Used by OpenAI to cache responses for similar requests to optimize your cache hit rates.
    /// Replaces the `user` field. [Learn
    /// more](https://platform.openai.com/docs/guides/prompt-caching).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    /// A stable identifier used to help detect users of your application that may be violating
    /// OpenAI's usage policies.
    /// The IDs should be a string that uniquely identifies each user. We recommend hashing their
    /// username or email address, in order to avoid sending us any identifying information.
    /// [Learn
    /// more](https://platform.openai.com/docs/guides/safety-best-practices#safety-identifiers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// An integer between 0 and 20 specifying the number of most likely tokens to
    /// return at each token position, each with an associated log probability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// This field is being replaced by `safety_identifier` and `prompt_cache_key`. Use
    /// `prompt_cache_key` instead to maintain caching optimizations.
    /// A stable identifier for your end-users.
    /// Used to boost cache hit rates by better bucketing similar requests and  to help OpenAI
    /// detect and prevent abuse. [Learn
    /// more](https://platform.openai.com/docs/guides/safety-best-practices#safety-identifiers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_calls: Option<i64>,
    /// Model ID used to generate the response, like `gpt-4o` or `o3`. OpenAI
    /// offers a wide range of models with different capabilities, performance
    /// characteristics, and price points. Refer to the [model
    /// guide](https://platform.openai.com/docs/models)
    /// to browse and compare available models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Prompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    /// Configuration options for a text response from the model. Can be plain
    /// text or structured JSON data. Learn more:
    /// - [Text inputs and outputs](https://platform.openai.com/docs/guides/text)
    /// - [Structured Outputs](https://platform.openai.com/docs/guides/structured-outputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextClass>,
    /// How the model should select which tool (or tools) to use when generating
    /// a response. See the `tools` parameter to see how to specify which tools
    /// the model can call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<CreateResponseToolChoice>,
    /// An array of tools the model may call while generating a response. You
    /// can specify which tool to use by setting the `tool_choice` parameter.
    ///
    /// We support the following categories of tools:
    /// - **Built-in tools**: Tools that are provided by OpenAI that extend the
    /// model's capabilities, like [web
    /// search](https://platform.openai.com/docs/guides/tools-web-search)
    /// or [file search](https://platform.openai.com/docs/guides/tools-file-search). Learn more
    /// about
    /// [built-in tools](https://platform.openai.com/docs/guides/tools).
    /// - **MCP Tools**: Integrations with third-party systems via custom MCP servers
    /// or predefined connectors such as Google Drive and SharePoint. Learn more about
    /// [MCP Tools](https://platform.openai.com/docs/guides/tools-connectors-mcp).
    /// - **Function calls (custom tools)**: Functions that are defined by you,
    /// enabling the model to call your own code with strongly typed arguments
    /// and outputs. Learn more about
    /// [function calling](https://platform.openai.com/docs/guides/function-calling). You can
    /// also use
    /// custom tools to call your own code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<Truncation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ConversationUnion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<Includable>>,
    /// Text, image, or file inputs to the model, used to generate a response.
    ///
    /// Learn more:
    /// - [Text inputs and outputs](https://platform.openai.com/docs/guides/text)
    /// - [Image inputs](https://platform.openai.com/docs/guides/images)
    /// - [File inputs](https://platform.openai.com/docs/guides/pdf-files)
    /// - [Conversation state](https://platform.openai.com/docs/guides/conversation-state)
    /// - [Function calling](https://platform.openai.com/docs/guides/function-calling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Instructions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<ResponseStreamOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConversationUnion {
    ConversationObject(ConversationObject),
    String(String),
}

/// The conversation that this response belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationObject {
    /// The unique ID of the conversation.
    pub id: String,
}

/// Specify additional output data to include in the model response. Currently
/// supported values are:
/// - `web_search_call.action.sources`: Include the sources of the web search tool call.
/// - `code_interpreter_call.outputs`: Includes the outputs of python code execution
/// in code interpreter tool call items.
/// - `computer_call_output.output.image_url`: Include image urls from the computer call
/// output.
/// - `file_search_call.results`: Include the search results of
/// the file search tool call.
/// - `message.input_image.image_url`: Include image urls from the input message.
/// - `message.output_text.logprobs`: Include logprobs with assistant messages.
/// - `reasoning.encrypted_content`: Includes an encrypted version of reasoning
/// tokens in reasoning item outputs. This enables reasoning items to be used in
/// multi-turn conversations when using the Responses API statelessly (like
/// when the `store` parameter is set to `false`, or when an organization is
/// enrolled in the zero data retention program).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Includable {
    #[serde(rename = "code_interpreter_call.outputs")]
    CodeInterpreterCallOutputs,
    #[serde(rename = "computer_call_output.output.image_url")]
    ComputerCallOutputOutputImageUrl,
    #[serde(rename = "file_search_call.results")]
    FileSearchCallResults,
    #[serde(rename = "message.input_image.image_url")]
    MessageInputImageImageUrl,
    #[serde(rename = "message.output_text.logprobs")]
    MessageOutputTextLogprobs,
    #[serde(rename = "reasoning.encrypted_content")]
    ReasoningEncryptedContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Instructions {
    InputItemArray(Vec<InputItem>),
    String(String),
}

/// A list of one or many input items to the model, containing
/// different content types.
///
///
/// A message input to the model with a role indicating instruction following
/// hierarchy. Instructions given with the `developer` or `system` role take
/// precedence over instructions given with the `user` role. Messages with the
/// `assistant` role are presumed to have been generated by the model in previous
/// interactions.
///
///
/// An item representing part of the context for the response to be
/// generated by the model. Can contain text, images, and audio inputs,
/// as well as previous assistant responses and tool call outputs.
///
///
/// Content item used to generate a response.
///
///
/// A message input to the model with a role indicating instruction following
/// hierarchy. Instructions given with the `developer` or `system` role take
/// precedence over instructions given with the `user` role.
///
///
/// An output message from the model.
///
///
/// The results of a file search tool call. See the
/// [file search guide](https://platform.openai.com/docs/guides/tools-file-search) for more
/// information.
///
///
/// A tool call to a computer use tool. See the
/// [computer use guide](https://platform.openai.com/docs/guides/tools-computer-use) for more
/// information.
///
///
/// The output of a computer tool call.
///
/// The results of a web search tool call. See the
/// [web search guide](https://platform.openai.com/docs/guides/tools-web-search) for more
/// information.
///
///
/// A tool call to run a function. See the
/// [function calling guide](https://platform.openai.com/docs/guides/function-calling) for
/// more information.
///
///
/// The output of a function tool call.
///
/// A description of the chain of thought used by a reasoning model while generating
/// a response. Be sure to include these items in your `input` to the Responses API
/// for subsequent turns of a conversation if you are manually
/// [managing context](https://platform.openai.com/docs/guides/conversation-state).
///
///
/// An image generation request made by the model.
///
///
/// A tool call to run code.
///
///
/// A tool call to run a command on the local shell.
///
///
/// The output of a local shell tool call.
///
///
/// A list of tools available on an MCP server.
///
///
/// A request for human approval of a tool invocation.
///
///
/// A response to an MCP approval request.
///
///
/// An invocation of a tool on an MCP server.
///
///
/// The output of a custom tool call from your code, being sent back to the model.
///
///
/// A call to a custom tool created by the model.
///
///
/// An internal identifier for an item to reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputItem {
    /// Text, image, or audio input to the model, used to generate a response.
    /// Can also contain previous assistant responses.
    ///
    ///
    /// The content of the output message.
    ///
    ///
    /// Reasoning text content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<InputItemContent>,
    /// The role of the message input. One of `user`, `assistant`, `system`, or
    /// `developer`.
    ///
    ///
    /// The role of the message input. One of `user`, `system`, or `developer`.
    ///
    ///
    /// The role of the output message. Always `assistant`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<InputItemRole>,
    /// The type of the message input. Always `message`.
    ///
    ///
    /// The type of the message input. Always set to `message`.
    ///
    ///
    /// The type of the output message. Always `message`.
    ///
    ///
    /// The type of the file search tool call. Always `file_search_call`.
    ///
    ///
    /// The type of the computer call. Always `computer_call`.
    ///
    /// The type of the computer tool call output. Always `computer_call_output`.
    ///
    /// The type of the web search tool call. Always `web_search_call`.
    ///
    ///
    /// The type of the function tool call. Always `function_call`.
    ///
    ///
    /// The type of the function tool call output. Always `function_call_output`.
    ///
    /// The type of the object. Always `reasoning`.
    ///
    ///
    /// The type of the image generation call. Always `image_generation_call`.
    ///
    ///
    /// The type of the code interpreter tool call. Always `code_interpreter_call`.
    ///
    ///
    /// The type of the local shell call. Always `local_shell_call`.
    ///
    ///
    /// The type of the local shell tool call output. Always `local_shell_call_output`.
    ///
    ///
    /// The type of the item. Always `mcp_list_tools`.
    ///
    ///
    /// The type of the item. Always `mcp_approval_request`.
    ///
    ///
    /// The type of the item. Always `mcp_approval_response`.
    ///
    ///
    /// The type of the item. Always `mcp_call`.
    ///
    ///
    /// The type of the custom tool call output. Always `custom_tool_call_output`.
    ///
    ///
    /// The type of the custom tool call. Always `custom_tool_call`.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_item_type: Option<InputItemType>,
    /// The status of item. One of `in_progress`, `completed`, or
    /// `incomplete`. Populated when items are returned via API.
    ///
    ///
    /// The status of the message input. One of `in_progress`, `completed`, or
    /// `incomplete`. Populated when input items are returned via API.
    ///
    ///
    /// The status of the file search tool call. One of `in_progress`,
    /// `searching`, `incomplete` or `failed`,
    ///
    ///
    /// The status of the item. One of `in_progress`, `completed`, or
    /// `incomplete`. Populated when items are returned via API.
    ///
    ///
    /// The status of the web search tool call.
    ///
    ///
    /// The status of the image generation call.
    ///
    ///
    /// The status of the code interpreter tool call. Valid values are `in_progress`,
    /// `completed`, `incomplete`, `interpreting`, and `failed`.
    ///
    ///
    /// The status of the local shell call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<FunctionCallItemStatus>,
    /// The unique ID of the output message.
    ///
    ///
    /// The unique ID of the file search tool call.
    ///
    ///
    /// The unique ID of the computer call.
    ///
    /// The unique ID of the web search tool call.
    ///
    ///
    /// The unique ID of the function tool call.
    ///
    ///
    /// The unique identifier of the reasoning content.
    ///
    ///
    /// The unique ID of the image generation call.
    ///
    ///
    /// The unique ID of the code interpreter tool call.
    ///
    ///
    /// The unique ID of the local shell call.
    ///
    ///
    /// The unique ID of the local shell tool call generated by the model.
    ///
    ///
    /// The unique ID of the list.
    ///
    ///
    /// The unique ID of the approval request.
    ///
    ///
    /// The unique ID of the tool call.
    ///
    ///
    /// The unique ID of the custom tool call output in the OpenAI platform.
    ///
    ///
    /// The unique ID of the custom tool call in the OpenAI platform.
    ///
    ///
    /// The ID of the item to reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The queries used to search for files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Result>>,
    /// An object describing the specific action taken in this web search call.
    /// Includes details on how the model used the web (search, open_page, find).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ComputerAction>,
    /// An identifier used when responding to the tool call with output.
    ///
    ///
    /// The ID of the computer tool call that produced the output.
    ///
    /// The unique ID of the function tool call generated by the model.
    ///
    ///
    /// The unique ID of the function tool call generated by the model.
    ///
    /// The unique ID of the local shell tool call generated by the model.
    ///
    ///
    /// The call ID, used to map this custom tool call output to a custom tool call.
    ///
    ///
    /// An identifier used to map this custom tool call to a tool call output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<serde_json::Value>,
    /// The pending safety checks for the computer call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_safety_checks: Option<Vec<ComputerToolCallSafetyCheck>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_safety_checks: Option<Vec<ComputerCallSafetyCheckParam>>,
    /// A JSON string of the output of the local shell tool call.
    ///
    ///
    /// The output from the custom tool call generated by your code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Refusal>,
    /// A JSON string of the arguments to pass to the function.
    ///
    ///
    /// A JSON string of arguments for the tool.
    ///
    ///
    /// A JSON string of the arguments passed to the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// The name of the function to run.
    ///
    ///
    /// The name of the tool to run.
    ///
    ///
    /// The name of the tool that was run.
    ///
    ///
    /// The name of the custom tool being called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
    /// Reasoning summary content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Vec<SummaryText>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// The ID of the container used to run the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<CodeInterpreterOutput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The label of the MCP server.
    ///
    ///
    /// The label of the MCP server making the request.
    ///
    ///
    /// The label of the MCP server running the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,
    /// The tools available on the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<McpListToolsTool>>,
    /// The ID of the approval request being answered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_request_id: Option<String>,
    /// Whether the request was approved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approve: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<serde_json::Value>,
    /// The input for the custom tool call generated by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
}

/// The safety checks reported by the API that have been acknowledged by the developer.
///
/// A pending safety check for the computer call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputerCallSafetyCheckParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// The ID of the pending safety check.
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// A click action.
///
///
/// A double click action.
///
///
/// A drag action.
///
///
/// A collection of keypresses the model would like to perform.
///
///
/// A mouse move action.
///
///
/// A screenshot action.
///
///
/// A scroll action.
///
///
/// An action to type in text.
///
///
/// A wait action.
///
///
/// An object describing the specific action taken in this web search call.
/// Includes details on how the model used the web (search, open_page, find).
///
///
/// Action type "search" - Performs a web search query.
///
///
/// Action type "open_page" - Opens a specific URL from search results.
///
///
/// Action type "find": Searches for a pattern within a loaded page.
///
///
/// Execute a shell command on the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputerAction {
    /// Indicates which mouse button was pressed during the click. One of `left`, `right`,
    /// `wheel`, `back`, or `forward`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<Button>,
    /// Specifies the event type. For a click action, this property is
    /// always set to `click`.
    ///
    ///
    /// Specifies the event type. For a double click action, this property is
    /// always set to `double_click`.
    ///
    ///
    /// Specifies the event type. For a drag action, this property is
    /// always set to `drag`.
    ///
    ///
    /// Specifies the event type. For a keypress action, this property is
    /// always set to `keypress`.
    ///
    ///
    /// Specifies the event type. For a move action, this property is
    /// always set to `move`.
    ///
    ///
    /// Specifies the event type. For a screenshot action, this property is
    /// always set to `screenshot`.
    ///
    ///
    /// Specifies the event type. For a scroll action, this property is
    /// always set to `scroll`.
    ///
    ///
    /// Specifies the event type. For a type action, this property is
    /// always set to `type`.
    ///
    ///
    /// Specifies the event type. For a wait action, this property is
    /// always set to `wait`.
    ///
    ///
    /// The action type.
    ///
    ///
    /// The type of the local shell action. Always `exec`.
    #[serde(rename = "type")]
    pub computer_action_type: ActionType,
    /// The x-coordinate where the click occurred.
    ///
    ///
    /// The x-coordinate where the double click occurred.
    ///
    ///
    /// The x-coordinate to move to.
    ///
    ///
    /// The x-coordinate where the scroll occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    /// The y-coordinate where the click occurred.
    ///
    ///
    /// The y-coordinate where the double click occurred.
    ///
    ///
    /// The y-coordinate to move to.
    ///
    ///
    /// The y-coordinate where the scroll occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
    /// An array of coordinates representing the path of the drag action. Coordinates will appear
    /// as an array
    /// of objects, eg
    /// ```json
    /// [
    /// { "x": 100, "y": 200 },
    /// { "x": 200, "y": 300 }
    /// ]
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<Coordinate>>,
    /// The combination of keys the model is requesting to be pressed. This is an
    /// array of strings, each representing a key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<String>>,
    /// The horizontal scroll distance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_x: Option<i64>,
    /// The vertical scroll distance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_y: Option<i64>,
    /// The text to type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The search query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The sources used in the search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<WebSearchSource>>,
    /// The URL opened by the model.
    ///
    ///
    /// The URL of the page searched for the pattern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The pattern or text to search for within the page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// The command to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    /// Environment variables to set for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
}

/// Indicates which mouse button was pressed during the click. One of `left`, `right`,
/// `wheel`, `back`, or `forward`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Back,
    Forward,
    Left,
    Right,
    Wheel,
}

/// Specifies the event type. For a click action, this property is
/// always set to `click`.
///
///
/// Specifies the event type. For a double click action, this property is
/// always set to `double_click`.
///
///
/// Specifies the event type. For a drag action, this property is
/// always set to `drag`.
///
///
/// Specifies the event type. For a keypress action, this property is
/// always set to `keypress`.
///
///
/// Specifies the event type. For a move action, this property is
/// always set to `move`.
///
///
/// Specifies the event type. For a screenshot action, this property is
/// always set to `screenshot`.
///
///
/// Specifies the event type. For a scroll action, this property is
/// always set to `scroll`.
///
///
/// Specifies the event type. For a type action, this property is
/// always set to `type`.
///
///
/// Specifies the event type. For a wait action, this property is
/// always set to `wait`.
///
///
/// The action type.
///
///
/// The type of the local shell action. Always `exec`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Click,
    #[serde(rename = "double_click")]
    DoubleClick,
    Drag,
    Exec,
    Find,
    Keypress,
    Move,
    #[serde(rename = "open_page")]
    OpenPage,
    Screenshot,
    Scroll,
    Search,
    Type,
    Wait,
}

/// A series of x/y coordinate pairs in the drag path.
///
///
/// An x/y coordinate pair, e.g. `{ x: 100, y: 200 }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    /// The x-coordinate.
    pub x: i64,
    /// The y-coordinate.
    pub y: i64,
}

/// A source used in the search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchSource {
    /// The type of source. Always `url`.
    #[serde(rename = "type")]
    pub web_search_source_type: SourceType,
    /// The URL of the source.
    pub url: String,
}

/// The type of source. Always `url`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Url,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputItemContent {
    InputContentArray(Vec<InputContent>),
    String(String),
}

/// A list of one or many input items to the model, containing different content
/// types.
///
///
/// A text input to the model.
///
/// An image input to the model. Learn about [image
/// inputs](https://platform.openai.com/docs/guides/vision).
///
/// A file input to the model.
///
/// An audio input to the model.
///
///
/// A text output from the model.
///
/// A refusal from the model.
///
/// Reasoning text from the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputContent {
    /// The text input to the model.
    ///
    /// The text output from the model.
    ///
    /// The reasoning text from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The type of the input item. Always `input_text`.
    ///
    /// The type of the input item. Always `input_image`.
    ///
    /// The type of the input item. Always `input_file`.
    ///
    /// The type of the input item. Always `input_audio`.
    ///
    ///
    /// The type of the output text. Always `output_text`.
    ///
    /// The type of the refusal. Always `refusal`.
    ///
    /// The type of the reasoning text. Always `reasoning_text`.
    #[serde(rename = "type")]
    pub input_content_type: InputItemContentListType,
    /// The detail level of the image to be sent to the model. One of `high`, `low`, or `auto`.
    /// Defaults to `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// The content of the file to be sent to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    /// The URL of the file to be sent to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    /// The name of the file to be sent to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<InputItemContentListInputAudio>,
    /// The annotations of the text output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<LogProbability>>,
    /// The refusal explanation from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// A citation to a file.
///
/// A citation for a web resource used to generate a model response.
///
/// A citation for a container file used to generate a model response.
///
/// A path to a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    /// The ID of the file.
    ///
    /// The ID of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// The filename of the file cited.
    ///
    /// The filename of the container file cited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// The index of the file in the list of files.
    ///
    /// The index of the file in the list of files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i64>,
    /// The type of the file citation. Always `file_citation`.
    ///
    /// The type of the URL citation. Always `url_citation`.
    ///
    /// The type of the container file citation. Always `container_file_citation`.
    ///
    /// The type of the file path. Always `file_path`.
    #[serde(rename = "type")]
    pub annotation_type: AnnotationTypeEnum,
    /// The index of the last character of the URL citation in the message.
    ///
    /// The index of the last character of the container file citation in the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i64>,
    /// The index of the first character of the URL citation in the message.
    ///
    /// The index of the first character of the container file citation in the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i64>,
    /// The title of the web resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The URL of the web resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The ID of the container file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
}

/// The type of the file citation. Always `file_citation`.
///
/// The type of the URL citation. Always `url_citation`.
///
/// The type of the container file citation. Always `container_file_citation`.
///
/// The type of the file path. Always `file_path`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationTypeEnum {
    #[serde(rename = "container_file_citation")]
    ContainerFileCitation,
    #[serde(rename = "file_citation")]
    FileCitation,
    #[serde(rename = "file_path")]
    FilePath,
    #[serde(rename = "url_citation")]
    UrlCitation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputItemContentListInputAudio {
    /// Base64-encoded audio data.
    pub data: String,
    /// The format of the audio data. Currently supported formats are `mp3` and
    /// `wav`.
    pub format: InputAudioFormat,
}

/// The type of the input item. Always `input_text`.
///
/// The type of the input item. Always `input_image`.
///
/// The type of the input item. Always `input_file`.
///
/// The type of the input item. Always `input_audio`.
///
///
/// The type of the output text. Always `output_text`.
///
/// The type of the refusal. Always `refusal`.
///
/// The type of the reasoning text. Always `reasoning_text`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputItemContentListType {
    #[serde(rename = "input_audio")]
    InputAudio,
    #[serde(rename = "input_file")]
    InputFile,
    #[serde(rename = "input_image")]
    InputImage,
    #[serde(rename = "input_text")]
    InputText,
    #[serde(rename = "output_text")]
    OutputText,
    #[serde(rename = "reasoning_text")]
    ReasoningText,
    Refusal,
}

/// The log probability of a token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogProbability {
    pub bytes: Vec<i64>,
    pub logprob: f64,
    pub token: String,
    pub top_logprobs: Vec<TopLogProbability>,
}

/// The top log probability of a token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogProbability {
    pub bytes: Vec<i64>,
    pub logprob: f64,
    pub token: String,
}

/// The type of the message input. Always `message`.
///
///
/// The type of the message input. Always set to `message`.
///
///
/// The type of the output message. Always `message`.
///
///
/// The type of the file search tool call. Always `file_search_call`.
///
///
/// The type of the computer call. Always `computer_call`.
///
/// The type of the computer tool call output. Always `computer_call_output`.
///
/// The type of the web search tool call. Always `web_search_call`.
///
///
/// The type of the function tool call. Always `function_call`.
///
///
/// The type of the function tool call output. Always `function_call_output`.
///
/// The type of the object. Always `reasoning`.
///
///
/// The type of the image generation call. Always `image_generation_call`.
///
///
/// The type of the code interpreter tool call. Always `code_interpreter_call`.
///
///
/// The type of the local shell call. Always `local_shell_call`.
///
///
/// The type of the local shell tool call output. Always `local_shell_call_output`.
///
///
/// The type of the item. Always `mcp_list_tools`.
///
///
/// The type of the item. Always `mcp_approval_request`.
///
///
/// The type of the item. Always `mcp_approval_response`.
///
///
/// The type of the item. Always `mcp_call`.
///
///
/// The type of the custom tool call output. Always `custom_tool_call_output`.
///
///
/// The type of the custom tool call. Always `custom_tool_call`.
///
///
/// The type of item to reference. Always `item_reference`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputItemType {
    #[serde(rename = "code_interpreter_call")]
    CodeInterpreterCall,
    #[serde(rename = "computer_call")]
    ComputerCall,
    #[serde(rename = "computer_call_output")]
    ComputerCallOutput,
    #[serde(rename = "custom_tool_call")]
    CustomToolCall,
    #[serde(rename = "custom_tool_call_output")]
    CustomToolCallOutput,
    #[serde(rename = "file_search_call")]
    FileSearchCall,
    #[serde(rename = "function_call")]
    FunctionCall,
    #[serde(rename = "function_call_output")]
    FunctionCallOutput,
    #[serde(rename = "image_generation_call")]
    ImageGenerationCall,
    #[serde(rename = "item_reference")]
    ItemReference,
    #[serde(rename = "local_shell_call")]
    LocalShellCall,
    #[serde(rename = "local_shell_call_output")]
    LocalShellCallOutput,
    #[serde(rename = "mcp_approval_request")]
    McpApprovalRequest,
    #[serde(rename = "mcp_approval_response")]
    McpApprovalResponse,
    #[serde(rename = "mcp_call")]
    McpCall,
    #[serde(rename = "mcp_list_tools")]
    McpListTools,
    Message,
    #[serde(rename = "reasoning")]
    Reasoning,
    #[serde(rename = "web_search_call")]
    WebSearchCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Refusal {
    ComputerScreenshotImage(ComputerScreenshotImage),
    String(String),
}

/// A computer screenshot image used with the computer use tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputerScreenshotImage {
    /// The identifier of an uploaded file that contains the screenshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// The URL of the screenshot image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Specifies the event type. For a computer screenshot, this property is
    /// always set to `computer_screenshot`.
    #[serde(rename = "type")]
    pub computer_screenshot_image_type: StickyType,
}

/// Specifies the event type. For a computer screenshot, this property is
/// always set to `computer_screenshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StickyType {
    #[serde(rename = "computer_screenshot")]
    ComputerScreenshot,
}

/// The outputs generated by the code interpreter, such as logs or images.
/// Can be null if no outputs are available.
///
///
/// The logs output from the code interpreter.
///
///
/// The image output from the code interpreter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeInterpreterOutput {
    /// The logs output from the code interpreter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
    /// The type of the output. Always 'logs'.
    ///
    /// The type of the output. Always 'image'.
    #[serde(rename = "type")]
    pub code_interpreter_output_type: IndigoType,
    /// The URL of the image output from the code interpreter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// The type of the output. Always 'logs'.
///
/// The type of the output. Always 'image'.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndigoType {
    Image,
    Logs,
}

/// A pending safety check for the computer call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputerToolCallSafetyCheck {
    /// The type of the pending safety check.
    pub code: String,
    /// The ID of the pending safety check.
    pub id: String,
    /// Details about the pending safety check.
    pub message: String,
}

/// The results of the file search tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Result {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, VectorStoreFileAttribute>>,
    /// The unique ID of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// The name of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// The relevance score of the file - a value between 0 and 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// The text that was retrieved from the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VectorStoreFileAttribute {
    Bool(bool),
    Double(f64),
    String(String),
}

/// The role of the message input. One of `user`, `assistant`, `system`, or
/// `developer`.
///
///
/// The role of the message input. One of `user`, `system`, or `developer`.
///
///
/// The role of the output message. Always `assistant`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputItemRole {
    Assistant,
    Developer,
    System,
    User,
}

/// The status of item. One of `in_progress`, `completed`, or
/// `incomplete`. Populated when items are returned via API.
///
///
/// The status of the message input. One of `in_progress`, `completed`, or
/// `incomplete`. Populated when input items are returned via API.
///
///
/// The status of the file search tool call. One of `in_progress`,
/// `searching`, `incomplete` or `failed`,
///
///
/// The status of the item. One of `in_progress`, `completed`, or
/// `incomplete`. Populated when items are returned via API.
///
///
/// The status of the message input. One of `in_progress`, `completed`, or `incomplete`.
/// Populated when input items are returned via API.
///
/// The status of the item. One of `in_progress`, `completed`, or `incomplete`. Populated
/// when items are returned via API.
///
/// The status of the web search tool call.
///
///
/// The status of the image generation call.
///
///
/// The status of the code interpreter tool call. Valid values are `in_progress`,
/// `completed`, `incomplete`, `interpreting`, and `failed`.
///
///
/// The status of the local shell call.
///
///
/// The status of the item. One of `in_progress`, `completed`, or `incomplete`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCallItemStatus {
    Completed,
    Failed,
    Generating,
    #[serde(rename = "in_progress")]
    InProgress,
    Incomplete,
    Interpreting,
    Searching,
}

/// A summary text from the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryText {
    /// A summary of the reasoning output from the model so far.
    pub text: String,
    /// The type of the object. Always `summary_text`.
    #[serde(rename = "type")]
    pub summary_text_type: SummaryType,
}

/// The type of the object. Always `summary_text`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryType {
    #[serde(rename = "summary_text")]
    SummaryText,
}

/// A tool available on an MCP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpListToolsTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, Option<serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The JSON schema describing the tool's input.
    pub input_schema: HashMap<String, Option<serde_json::Value>>,
    /// The name of the tool.
    pub name: String,
}

/// Reference to a prompt template and its variables.
/// [Learn
/// more](https://platform.openai.com/docs/guides/text?api-mode=responses#reusable-prompts).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prompt {
    /// The unique identifier of the prompt template to use.
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, PromptVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PromptVariable {
    Input(Input),
    String(String),
}

/// A text input to the model.
///
/// An image input to the model. Learn about [image
/// inputs](https://platform.openai.com/docs/guides/vision).
///
/// A file input to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    /// The text input to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The type of the input item. Always `input_text`.
    ///
    /// The type of the input item. Always `input_image`.
    ///
    /// The type of the input item. Always `input_file`.
    #[serde(rename = "type")]
    pub input_type: InputTextType,
    /// The detail level of the image to be sent to the model. One of `high`, `low`, or `auto`.
    /// Defaults to `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// The content of the file to be sent to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    /// The URL of the file to be sent to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    /// The name of the file to be sent to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// The type of the input item. Always `input_text`.
///
/// The type of the input item. Always `input_image`.
///
/// The type of the input item. Always `input_file`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputTextType {
    #[serde(rename = "input_file")]
    InputFile,
    #[serde(rename = "input_image")]
    InputImage,
    #[serde(rename = "input_text")]
    InputText,
}

/// **gpt-5 and o-series models only**
///
/// Configuration options for
/// [reasoning models](https://platform.openai.com/docs/guides/reasoning).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reasoning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_summary: Option<Summary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Summary>,
}

/// **Deprecated:** use `summary` instead.
///
/// A summary of the reasoning performed by the model. This can be
/// useful for debugging and understanding the model's reasoning process.
/// One of `auto`, `concise`, or `detailed`.
///
///
/// A summary of the reasoning performed by the model. This can be
/// useful for debugging and understanding the model's reasoning process.
/// One of `auto`, `concise`, or `detailed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Summary {
    Auto,
    Concise,
    Detailed,
}

/// Options for streaming responses. Only set this when you set `stream: true`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseStreamOptions {
    /// When true, stream obfuscation will be enabled. Stream obfuscation adds
    /// random characters to an `obfuscation` field on streaming delta events to
    /// normalize payload sizes as a mitigation to certain side-channel attacks.
    /// These obfuscation fields are included by default, but add a small amount
    /// of overhead to the data stream. You can set `include_obfuscation` to
    /// false to optimize for bandwidth if you trust the network links between
    /// your application and the OpenAI API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
}

/// Configuration options for a text response from the model. Can be plain
/// text or structured JSON data. Learn more:
/// - [Text inputs and outputs](https://platform.openai.com/docs/guides/text)
/// - [Structured Outputs](https://platform.openai.com/docs/guides/structured-outputs)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextClass {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<TextResponseFormatConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<WebSearchContextSize>,
}

/// An object specifying the format that the model must output.
///
/// Configuring `{ "type": "json_schema" }` enables Structured Outputs,
/// which ensures the model will match your supplied JSON schema. Learn more in the
/// [Structured Outputs guide](https://platform.openai.com/docs/guides/structured-outputs).
///
/// The default format is `{ "type": "text" }` with no additional options.
///
/// **Not recommended for gpt-4o and newer models:**
///
/// Setting to `{ "type": "json_object" }` enables the older JSON mode, which
/// ensures the message the model generates is valid JSON. Using `json_schema`
/// is preferred for models that support it.
///
///
/// Default response format. Used to generate text responses.
///
///
/// JSON Schema response format. Used to generate structured JSON responses.
/// Learn more about [Structured
/// Outputs](https://platform.openai.com/docs/guides/structured-outputs).
///
///
/// JSON object response format. An older method of generating JSON responses.
/// Using `json_schema` is recommended for models that support it. Note that the
/// model will not generate JSON without a system or user message instructing it
/// to do so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextResponseFormatConfiguration {
    /// The type of response format being defined. Always `text`.
    ///
    /// The type of response format being defined. Always `json_schema`.
    ///
    /// The type of response format being defined. Always `json_object`.
    #[serde(rename = "type")]
    pub text_response_format_configuration_type: ResponseFormatType,
    /// A description of what the response format is for, used by the model to
    /// determine how to respond in the format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The name of the response format. Must be a-z, A-Z, 0-9, or contain
    /// underscores and dashes, with a maximum length of 64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<HashMap<String, Option<serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// How the model should select which tool (or tools) to use when generating
/// a response. See the `tools` parameter to see how to specify which tools
/// the model can call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateResponseToolChoice {
    Enum(Auto),
    HostedToolClass(HostedToolClass),
}

/// Constrains the tools available to the model to a pre-defined set.
///
///
/// Indicates that the model should use a built-in tool to generate a response.
/// [Learn more about built-in tools](https://platform.openai.com/docs/guides/tools).
///
///
/// Use this option to force the model to call a specific function.
///
///
/// Use this option to force the model to call a specific tool on a remote MCP server.
///
///
/// Use this option to force the model to call a specific custom tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostedToolClass {
    /// Constrains the tools available to the model to a pre-defined set.
    ///
    /// `auto` allows the model to pick from among the allowed tools and generate a
    /// message.
    ///
    /// `required` requires the model to call one or more of the allowed tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    /// A list of tool definitions that the model should be allowed to call.
    ///
    /// For the Responses API, the list of tool definitions might look like:
    /// ```json
    /// [
    /// { "type": "function", "name": "get_weather" },
    /// { "type": "mcp", "server_label": "deepwiki" },
    /// { "type": "image_generation" }
    /// ]
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<HashMap<String, Option<serde_json::Value>>>>,
    /// Allowed tool configuration type. Always `allowed_tools`.
    ///
    /// The type of hosted tool the model should to use. Learn more about
    /// [built-in tools](https://platform.openai.com/docs/guides/tools).
    ///
    /// Allowed values are:
    /// - `file_search`
    /// - `web_search_preview`
    /// - `computer_use_preview`
    /// - `code_interpreter`
    /// - `image_generation`
    ///
    ///
    /// For function calling, the type is always `function`.
    ///
    /// For MCP tools, the type is always `mcp`.
    ///
    /// For custom tool calling, the type is always `custom`.
    #[serde(rename = "type")]
    pub allowed_tools_type: HostedToolType,
    /// The name of the function to call.
    ///
    /// The name of the custom tool to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The label of the MCP server to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,
}

/// Allowed tool configuration type. Always `allowed_tools`.
///
/// The type of hosted tool the model should to use. Learn more about
/// [built-in tools](https://platform.openai.com/docs/guides/tools).
///
/// Allowed values are:
/// - `file_search`
/// - `web_search_preview`
/// - `computer_use_preview`
/// - `code_interpreter`
/// - `image_generation`
///
///
/// For function calling, the type is always `function`.
///
/// For MCP tools, the type is always `mcp`.
///
/// For custom tool calling, the type is always `custom`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedToolType {
    #[serde(rename = "allowed_tools")]
    AllowedTools,
    #[serde(rename = "code_interpreter")]
    CodeInterpreter,
    #[serde(rename = "computer_use_preview")]
    ComputerUsePreview,
    Custom,
    #[serde(rename = "file_search")]
    FileSearch,
    Function,
    #[serde(rename = "image_generation")]
    ImageGeneration,
    Mcp,
    #[serde(rename = "web_search_preview")]
    WebSearchPreview,
    #[serde(rename = "web_search_preview_2025_03_11")]
    WebSearchPreview2025_03_11,
}

/// A tool that can be used to generate a response.
///
///
/// Defines a function in your own code the model can choose to call. Learn more about
/// [function calling](https://platform.openai.com/docs/guides/function-calling).
///
/// A tool that searches for relevant content from uploaded files. Learn more about the [file
/// search tool](https://platform.openai.com/docs/guides/tools-file-search).
///
/// A tool that controls a virtual computer. Learn more about the [computer
/// tool](https://platform.openai.com/docs/guides/tools-computer-use).
///
/// Search the Internet for sources related to the prompt. Learn more about the
/// [web search tool](https://platform.openai.com/docs/guides/tools-web-search).
///
///
/// Give the model access to additional tools via remote Model Context Protocol
/// (MCP) servers. [Learn more about
/// MCP](https://platform.openai.com/docs/guides/tools-remote-mcp).
///
///
/// A tool that runs Python code to help generate a response to a prompt.
///
///
/// A tool that generates images using a model like `gpt-image-1`.
///
///
/// A tool that allows the model to execute shell commands in a local environment.
///
///
/// A custom tool that processes input using a specified format. Learn more about
/// [custom tools](https://platform.openai.com/docs/guides/function-calling#custom-tools).
///
///
/// This tool searches the web for relevant results to use in a response. Learn more about
/// the [web search tool](https://platform.openai.com/docs/guides/tools-web-search).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// Optional description of the custom tool, used to provide more context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The name of the function to call.
    ///
    /// The name of the custom tool, used to identify it in tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, Option<serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// The type of the function tool. Always `function`.
    ///
    /// The type of the file search tool. Always `file_search`.
    ///
    /// The type of the computer use tool. Always `computer_use_preview`.
    ///
    /// The type of the web search tool. One of `web_search` or `web_search_2025_08_26`.
    ///
    /// The type of the MCP tool. Always `mcp`.
    ///
    /// The type of the code interpreter tool. Always `code_interpreter`.
    ///
    ///
    /// The type of the image generation tool. Always `image_generation`.
    ///
    ///
    /// The type of the local shell tool. Always `local_shell`.
    ///
    /// The type of the custom tool. Always `custom`.
    ///
    /// The type of the web search tool. One of `web_search_preview` or
    /// `web_search_preview_2025_03_11`.
    #[serde(rename = "type")]
    pub tool_type: ToolTypeEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<CompFilter>,
    /// The maximum number of results to return. This number should be between 1 and 50 inclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<i64>,
    /// Ranking options for search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<RankingOptions>,
    /// The IDs of the vector stores to search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_store_ids: Option<Vec<String>>,
    /// The height of the computer display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_height: Option<i64>,
    /// The width of the computer display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_width: Option<i64>,
    /// The type of computer environment to control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<ComputerEnvironment1>,
    /// High level guidance for the amount of context window space to use for the search. One of
    /// `low`, `medium`, or `high`. `medium` is the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<WebSearchContextSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<ApproximateLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<AllowedToolsUnion>,
    /// An OAuth access token that can be used with a remote MCP server, either
    /// with a custom MCP server URL or a service connector. Your application
    /// must handle the OAuth authorization flow and provide the token here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<String>,
    /// Identifier for service connectors, like those available in ChatGPT. One of
    /// `server_url` or `connector_id` must be provided. Learn more about service
    /// connectors [here](https://platform.openai.com/docs/guides/tools-remote-mcp#connectors).
    ///
    /// Currently supported `connector_id` values are:
    ///
    /// - Dropbox: `connector_dropbox`
    /// - Gmail: `connector_gmail`
    /// - Google Calendar: `connector_googlecalendar`
    /// - Google Drive: `connector_googledrive`
    /// - Microsoft Teams: `connector_microsoftteams`
    /// - Outlook Calendar: `connector_outlookcalendar`
    /// - Outlook Email: `connector_outlookemail`
    /// - SharePoint: `connector_sharepoint`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<ConnectorId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_approval: Option<RequireApproval>,
    /// Optional description of the MCP server, used to provide more context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_description: Option<String>,
    /// A label for this MCP server, used to identify it in tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,
    /// The URL for the MCP server. One of `server_url` or `connector_id` must be
    /// provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    /// The code interpreter container. Can be a container ID or an object that
    /// specifies uploaded file IDs to make available to your code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Container>,
    /// Background type for the generated image. One of `transparent`,
    /// `opaque`, or `auto`. Default: `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_fidelity: Option<ImageInputFidelity>,
    /// Optional mask for inpainting. Contains `image_url`
    /// (string, optional) and `file_id` (string, optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_image_mask: Option<InputImageMask>,
    /// The image generation model to use. Default: `gpt-image-1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    /// Moderation level for the generated image. Default: `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<Moderation>,
    /// Compression level for the output image. Default: 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<i64>,
    /// The output format of the generated image. One of `png`, `webp`, or
    /// `jpeg`. Default: `png`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,
    /// Number of partial images to generate in streaming mode, from 0 (default value) to 3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_images: Option<i64>,
    /// The quality of the generated image. One of `low`, `medium`, `high`,
    /// or `auto`. Default: `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,
    /// The size of the generated image. One of `1024x1024`, `1024x1536`,
    /// `1536x1024`, or `auto`. Default: `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
    /// The input format for the custom tool. Default is unconstrained text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ToolFormat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllowedToolsUnion {
    McpToolFilter(McpToolFilter),
    StringArray(Vec<String>),
}

/// A filter object to specify which tools are allowed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolFilter {
    /// Indicates whether or not a tool modifies data or is read-only. If an
    /// MCP server is [annotated with
    /// `readOnlyHint`](https://modelcontextprotocol.io/specification/2025-06-18/schema#toolannotations-readonlyhint),
    /// it will match this filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    /// List of allowed tool names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_names: Option<Vec<String>>,
}

/// Background type for the generated image. One of `transparent`,
/// `opaque`, or `auto`. Default: `auto`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    Auto,
    Opaque,
    Transparent,
}

/// Identifier for service connectors, like those available in ChatGPT. One of
/// `server_url` or `connector_id` must be provided. Learn more about service
/// connectors [here](https://platform.openai.com/docs/guides/tools-remote-mcp#connectors).
///
/// Currently supported `connector_id` values are:
///
/// - Dropbox: `connector_dropbox`
/// - Gmail: `connector_gmail`
/// - Google Calendar: `connector_googlecalendar`
/// - Google Drive: `connector_googledrive`
/// - Microsoft Teams: `connector_microsoftteams`
/// - Outlook Calendar: `connector_outlookcalendar`
/// - Outlook Email: `connector_outlookemail`
/// - SharePoint: `connector_sharepoint`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorId {
    #[serde(rename = "connector_dropbox")]
    ConnectorDropbox,
    #[serde(rename = "connector_gmail")]
    ConnectorGmail,
    #[serde(rename = "connector_googlecalendar")]
    ConnectorGooglecalendar,
    #[serde(rename = "connector_googledrive")]
    ConnectorGoogledrive,
    #[serde(rename = "connector_microsoftteams")]
    ConnectorMicrosoftteams,
    #[serde(rename = "connector_outlookcalendar")]
    ConnectorOutlookcalendar,
    #[serde(rename = "connector_outlookemail")]
    ConnectorOutlookemail,
    #[serde(rename = "connector_sharepoint")]
    ConnectorSharepoint,
}

/// The code interpreter container. Can be a container ID or an object that
/// specifies uploaded file IDs to make available to your code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Container {
    CodeInterpreterContainerAuto(CodeInterpreterContainerAuto),
    String(String),
}

/// Configuration for a code interpreter container. Optionally specify the IDs
/// of the files to run the code on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeInterpreterContainerAuto {
    /// An optional list of uploaded files to make available to your code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,
    /// Always `auto`.
    #[serde(rename = "type")]
    pub code_interpreter_container_auto_type: CodeInterpreterContainerAutoType,
}

/// Always `auto`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeInterpreterContainerAutoType {
    Auto,
}

/// The type of computer environment to control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerEnvironment1 {
    Browser,
    Linux,
    Mac,
    Ubuntu,
    Windows,
}

/// A filter used to compare a specified attribute key to a given value using a defined
/// comparison operation.
///
///
/// Combine multiple filters using `and` or `or`.
///
/// Filters for the search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompFilter {
    /// The key to compare against the value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Specifies the comparison operator: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`.
    /// - `eq`: equals
    /// - `ne`: not equal
    /// - `gt`: greater than
    /// - `gte`: greater than or equal
    /// - `lt`: less than
    /// - `lte`: less than or equal
    ///
    ///
    /// Type of operation: `and` or `or`.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comp_filter_type: Option<ComparisonFilterType>,
    /// The value to compare against the attribute key; supports string, number, or boolean types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Array of filters to combine. Items can be `ComparisonFilter` or `CompoundFilter`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<Option<serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
}

/// Specifies the comparison operator: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`.
/// - `eq`: equals
/// - `ne`: not equal
/// - `gt`: greater than
/// - `gte`: greater than or equal
/// - `lt`: less than
/// - `lte`: less than or equal
///
///
/// Type of operation: `and` or `or`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonFilterType {
    And,
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
    Ne,
    Or,
}

/// The value to compare against the attribute key; supports string, number, or boolean types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Double(f64),
    String(String),
}

/// The input format for the custom tool. Default is unconstrained text.
///
///
/// Unconstrained free-form text.
///
/// A grammar defined by the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFormat {
    /// Unconstrained text format. Always `text`.
    ///
    /// Grammar format. Always `grammar`.
    #[serde(rename = "type")]
    pub format_type: FormatType,
    /// The grammar definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    /// The syntax of the grammar definition. One of `lark` or `regex`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syntax: Option<Syntax>,
}

/// Control how much effort the model will exert to match the style and features,
/// especially facial features, of input images. This parameter is only supported
/// for `gpt-image-1`. Supports `high` and `low`. Defaults to `low`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageInputFidelity {
    High,
    Low,
}

/// Optional mask for inpainting. Contains `image_url`
/// (string, optional) and `file_id` (string, optional).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputImageMask {
    /// File ID for the mask image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// Base64-encoded mask image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

/// The image generation model to use. Default: `gpt-image-1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Model {
    #[serde(rename = "gpt-image-1")]
    GptImage1,
}

/// Moderation level for the generated image. Default: `auto`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Moderation {
    Auto,
    Low,
}

/// The output format of the generated image. One of `png`, `webp`, or
/// `jpeg`. Default: `png`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Jpeg,
    Png,
    Webp,
}

/// The quality of the generated image. One of `low`, `medium`, `high`,
/// or `auto`. Default: `auto`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Auto,
    High,
    Low,
    Medium,
}

/// Ranking options for search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingOptions {
    /// The ranker to use for the file search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranker: Option<RankerVersionType>,
    /// The score threshold for the file search, a number between 0 and 1. Numbers closer to 1
    /// will attempt to return only the most relevant results, but may return fewer results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
}

/// The ranker to use for the file search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RankerVersionType {
    Auto,
    #[serde(rename = "default-2024-11-15")]
    Default20241115,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequireApproval {
    Enum(McpToolApprovalSetting),
    McpToolApprovalFilter(McpToolApprovalFilter),
}

/// Specify which of the MCP server's tools require approval. Can be
/// `always`, `never`, or a filter object associated with tools
/// that require approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolApprovalFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always: Option<McpToolFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never: Option<McpToolFilter>,
}

/// Specify a single approval policy for all tools. One of `always` or
/// `never`. When set to `always`, all tools will require approval. When
/// set to `never`, all tools will not require approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolApprovalSetting {
    Always,
    Never,
}

/// The size of the generated image. One of `1024x1024`, `1024x1536`,
/// `1536x1024`, or `auto`. Default: `auto`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    Auto,
    #[serde(rename = "1024x1024")]
    The1024X1024,
    #[serde(rename = "1024x1536")]
    The1024X1536,
    #[serde(rename = "1536x1024")]
    The1536X1024,
}

/// The type of the function tool. Always `function`.
///
/// The type of the file search tool. Always `file_search`.
///
/// The type of the computer use tool. Always `computer_use_preview`.
///
/// The type of the web search tool. One of `web_search` or `web_search_2025_08_26`.
///
/// The type of the MCP tool. Always `mcp`.
///
/// The type of the code interpreter tool. Always `code_interpreter`.
///
///
/// The type of the image generation tool. Always `image_generation`.
///
///
/// The type of the local shell tool. Always `local_shell`.
///
/// The type of the custom tool. Always `custom`.
///
/// The type of the web search tool. One of `web_search_preview` or
/// `web_search_preview_2025_03_11`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolTypeEnum {
    #[serde(rename = "code_interpreter")]
    CodeInterpreter,
    #[serde(rename = "computer_use_preview")]
    ComputerUsePreview,
    Custom,
    #[serde(rename = "file_search")]
    FileSearch,
    Function,
    #[serde(rename = "image_generation")]
    ImageGeneration,
    #[serde(rename = "local_shell")]
    LocalShell,
    Mcp,
    #[serde(rename = "web_search")]
    WebSearch,
    #[serde(rename = "web_search_2025_08_26")]
    WebSearch2025_08_26,
    #[serde(rename = "web_search_preview")]
    WebSearchPreview,
    #[serde(rename = "web_search_preview_2025_03_11")]
    WebSearchPreview2025_03_11,
}

/// The approximate location of the user.
///
///
/// The user's location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproximateLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// The type of location approximation. Always `approximate`.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximate_location_type: Option<UserLocationType>,
}

/// The truncation strategy to use for the model response.
/// - `auto`: If the input to this Response exceeds
/// the model's context window size, the model will truncate the
/// response to fit the context window by dropping items from the beginning of the
/// conversation.
/// - `disabled` (default): If the input size will exceed the context window
/// size for a model, the request will fail with a 400 error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Truncation {
    Auto,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TheResponseObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Used by OpenAI to cache responses for similar requests to optimize your cache hit rates.
    /// Replaces the `user` field. [Learn
    /// more](https://platform.openai.com/docs/guides/prompt-caching).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    /// A stable identifier used to help detect users of your application that may be violating
    /// OpenAI's usage policies.
    /// The IDs should be a string that uniquely identifies each user. We recommend hashing their
    /// username or email address, in order to avoid sending us any identifying information.
    /// [Learn
    /// more](https://platform.openai.com/docs/guides/safety-best-practices#safety-identifiers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// This field is being replaced by `safety_identifier` and `prompt_cache_key`. Use
    /// `prompt_cache_key` instead to maintain caching optimizations.
    /// A stable identifier for your end-users.
    /// Used to boost cache hit rates by better bucketing similar requests and  to help OpenAI
    /// detect and prevent abuse. [Learn
    /// more](https://platform.openai.com/docs/guides/safety-best-practices#safety-identifiers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_calls: Option<i64>,
    /// Model ID used to generate the response, like `gpt-4o` or `o3`. OpenAI
    /// offers a wide range of models with different capabilities, performance
    /// characteristics, and price points. Refer to the [model
    /// guide](https://platform.openai.com/docs/models)
    /// to browse and compare available models.
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Prompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Reasoning>,
    /// Configuration options for a text response from the model. Can be plain
    /// text or structured JSON data. Learn more:
    /// - [Text inputs and outputs](https://platform.openai.com/docs/guides/text)
    /// - [Structured Outputs](https://platform.openai.com/docs/guides/structured-outputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextClass>,
    /// How the model should select which tool (or tools) to use when generating
    /// a response. See the `tools` parameter to see how to specify which tools
    /// the model can call.
    pub tool_choice: TheResponseObjectToolChoice,
    /// An array of tools the model may call while generating a response. You
    /// can specify which tool to use by setting the `tool_choice` parameter.
    ///
    /// We support the following categories of tools:
    /// - **Built-in tools**: Tools that are provided by OpenAI that extend the
    /// model's capabilities, like [web
    /// search](https://platform.openai.com/docs/guides/tools-web-search)
    /// or [file search](https://platform.openai.com/docs/guides/tools-file-search). Learn more
    /// about
    /// [built-in tools](https://platform.openai.com/docs/guides/tools).
    /// - **MCP Tools**: Integrations with third-party systems via custom MCP servers
    /// or predefined connectors such as Google Drive and SharePoint. Learn more about
    /// [MCP Tools](https://platform.openai.com/docs/guides/tools-connectors-mcp).
    /// - **Function calls (custom tools)**: Functions that are defined by you,
    /// enabling the model to call your own code with strongly typed arguments
    /// and outputs. Learn more about
    /// [function calling](https://platform.openai.com/docs/guides/function-calling). You can
    /// also use
    /// custom tools to call your own code.
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<Truncation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<Conversation>,
    /// Unix timestamp (in seconds) of when this Response was created.
    pub created_at: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
    /// Unique identifier for this Response.
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_details: Option<IncompleteDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Instructions>,
    /// The object type of this resource - always set to `response`.
    pub object: TheResponseObjectObject,
    /// An array of content items generated by the model.
    ///
    /// - The length and order of items in the `output` array is dependent
    /// on the model's response.
    /// - Rather than accessing the first item in the `output` array and
    /// assuming it's an `assistant` message with the content generated by
    /// the model, you might consider using the `output_text` property where
    /// supported in SDKs.
    pub output: Vec<OutputItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,
    /// Whether to allow the model to run tool calls in parallel.
    pub parallel_tool_calls: bool,
    /// The status of the response generation. One of `completed`, `failed`,
    /// `in_progress`, `cancelled`, `queued`, or `incomplete`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
}

/// The conversation that this response belongs to. Input items and output items from this
/// response are automatically added to this conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    /// The unique ID of the conversation.
    pub id: String,
}

/// An error object returned when the model fails to generate a Response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: ResponseErrorCode,
    /// A human-readable description of the error.
    pub message: String,
}

/// The error code for the response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseErrorCode {
    #[serde(rename = "empty_image_file")]
    EmptyImageFile,
    #[serde(rename = "failed_to_download_image")]
    FailedToDownloadImage,
    #[serde(rename = "image_content_policy_violation")]
    ImageContentPolicyViolation,
    #[serde(rename = "image_file_not_found")]
    ImageFileNotFound,
    #[serde(rename = "image_file_too_large")]
    ImageFileTooLarge,
    #[serde(rename = "image_parse_error")]
    ImageParseError,
    #[serde(rename = "image_too_large")]
    ImageTooLarge,
    #[serde(rename = "image_too_small")]
    ImageTooSmall,
    #[serde(rename = "invalid_base64_image")]
    InvalidBase64Image,
    #[serde(rename = "invalid_image")]
    InvalidImage,
    #[serde(rename = "invalid_image_format")]
    InvalidImageFormat,
    #[serde(rename = "invalid_image_mode")]
    InvalidImageMode,
    #[serde(rename = "invalid_image_url")]
    InvalidImageUrl,
    #[serde(rename = "invalid_prompt")]
    InvalidPrompt,
    #[serde(rename = "rate_limit_exceeded")]
    RateLimitExceeded,
    #[serde(rename = "server_error")]
    ServerError,
    #[serde(rename = "unsupported_image_media_type")]
    UnsupportedImageMediaType,
    #[serde(rename = "vector_store_timeout")]
    VectorStoreTimeout,
}

/// Details about why the response is incomplete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncompleteDetails {
    /// The reason why the response is incomplete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
}

/// The reason why the response is incomplete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    #[serde(rename = "content_filter")]
    ContentFilter,
    #[serde(rename = "max_output_tokens")]
    MaxOutputTokens,
}

/// The object type of this resource - always set to `response`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TheResponseObjectObject {
    Response,
}

/// An output message from the model.
///
///
/// The results of a file search tool call. See the
/// [file search guide](https://platform.openai.com/docs/guides/tools-file-search) for more
/// information.
///
///
/// A tool call to run a function. See the
/// [function calling guide](https://platform.openai.com/docs/guides/function-calling) for
/// more information.
///
///
/// The results of a web search tool call. See the
/// [web search guide](https://platform.openai.com/docs/guides/tools-web-search) for more
/// information.
///
///
/// A tool call to a computer use tool. See the
/// [computer use guide](https://platform.openai.com/docs/guides/tools-computer-use) for more
/// information.
///
///
/// A description of the chain of thought used by a reasoning model while generating
/// a response. Be sure to include these items in your `input` to the Responses API
/// for subsequent turns of a conversation if you are manually
/// [managing context](https://platform.openai.com/docs/guides/conversation-state).
///
///
/// An image generation request made by the model.
///
///
/// A tool call to run code.
///
///
/// A tool call to run a command on the local shell.
///
///
/// An invocation of a tool on an MCP server.
///
///
/// A list of tools available on an MCP server.
///
///
/// A request for human approval of a tool invocation.
///
///
/// A call to a custom tool created by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputItem {
    /// The content of the output message.
    ///
    ///
    /// Reasoning text content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<OutputMessageContent>>,
    /// The unique ID of the output message.
    ///
    ///
    /// The unique ID of the file search tool call.
    ///
    ///
    /// The unique ID of the function tool call.
    ///
    ///
    /// The unique ID of the web search tool call.
    ///
    ///
    /// The unique ID of the computer call.
    ///
    /// The unique identifier of the reasoning content.
    ///
    ///
    /// The unique ID of the image generation call.
    ///
    ///
    /// The unique ID of the code interpreter tool call.
    ///
    ///
    /// The unique ID of the local shell call.
    ///
    ///
    /// The unique ID of the tool call.
    ///
    ///
    /// The unique ID of the list.
    ///
    ///
    /// The unique ID of the approval request.
    ///
    ///
    /// The unique ID of the custom tool call in the OpenAI platform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The role of the output message. Always `assistant`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MessageRole>,
    /// The status of the message input. One of `in_progress`, `completed`, or
    /// `incomplete`. Populated when input items are returned via API.
    ///
    ///
    /// The status of the file search tool call. One of `in_progress`,
    /// `searching`, `incomplete` or `failed`,
    ///
    ///
    /// The status of the item. One of `in_progress`, `completed`, or
    /// `incomplete`. Populated when items are returned via API.
    ///
    ///
    /// The status of the web search tool call.
    ///
    ///
    /// The status of the image generation call.
    ///
    ///
    /// The status of the code interpreter tool call. Valid values are `in_progress`,
    /// `completed`, `incomplete`, `interpreting`, and `failed`.
    ///
    ///
    /// The status of the local shell call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<FunctionCallItemStatus>,
    /// The type of the output message. Always `message`.
    ///
    ///
    /// The type of the file search tool call. Always `file_search_call`.
    ///
    ///
    /// The type of the function tool call. Always `function_call`.
    ///
    ///
    /// The type of the web search tool call. Always `web_search_call`.
    ///
    ///
    /// The type of the computer call. Always `computer_call`.
    ///
    /// The type of the object. Always `reasoning`.
    ///
    ///
    /// The type of the image generation call. Always `image_generation_call`.
    ///
    ///
    /// The type of the code interpreter tool call. Always `code_interpreter_call`.
    ///
    ///
    /// The type of the local shell call. Always `local_shell_call`.
    ///
    ///
    /// The type of the item. Always `mcp_call`.
    ///
    ///
    /// The type of the item. Always `mcp_list_tools`.
    ///
    ///
    /// The type of the item. Always `mcp_approval_request`.
    ///
    ///
    /// The type of the custom tool call. Always `custom_tool_call`.
    #[serde(rename = "type")]
    pub output_item_type: OutputItemType,
    /// The queries used to search for files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Result>>,
    /// A JSON string of the arguments to pass to the function.
    ///
    ///
    /// A JSON string of the arguments passed to the tool.
    ///
    ///
    /// A JSON string of arguments for the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    /// The unique ID of the function tool call generated by the model.
    ///
    ///
    /// An identifier used when responding to the tool call with output.
    ///
    ///
    /// The unique ID of the local shell tool call generated by the model.
    ///
    ///
    /// An identifier used to map this custom tool call to a tool call output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    /// The name of the function to run.
    ///
    ///
    /// The name of the tool that was run.
    ///
    ///
    /// The name of the tool to run.
    ///
    ///
    /// The name of the custom tool being called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// An object describing the specific action taken in this web search call.
    /// Includes details on how the model used the web (search, open_page, find).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ComputerAction>,
    /// The pending safety checks for the computer call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_safety_checks: Option<Vec<ComputerToolCallSafetyCheck>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
    /// Reasoning summary content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Vec<SummaryText>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// The ID of the container used to run the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<CodeInterpreterOutput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// The label of the MCP server running the tool.
    ///
    ///
    /// The label of the MCP server.
    ///
    ///
    /// The label of the MCP server making the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,
    /// The tools available on the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<McpListToolsTool>>,
    /// The input for the custom tool call generated by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
}

/// A text output from the model.
///
/// A refusal from the model.
///
/// Reasoning text from the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputMessageContent {
    /// The annotations of the text output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<LogProbability>>,
    /// The text output from the model.
    ///
    /// The reasoning text from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The type of the output text. Always `output_text`.
    ///
    /// The type of the refusal. Always `refusal`.
    ///
    /// The type of the reasoning text. Always `reasoning_text`.
    #[serde(rename = "type")]
    pub output_message_content_type: ContentType,
    /// The refusal explanation from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// The type of the output text. Always `output_text`.
///
/// The type of the refusal. Always `refusal`.
///
/// The type of the reasoning text. Always `reasoning_text`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    #[serde(rename = "output_text")]
    OutputText,
    #[serde(rename = "reasoning_text")]
    ReasoningText,
    Refusal,
}

/// The type of the output message. Always `message`.
///
///
/// The type of the file search tool call. Always `file_search_call`.
///
///
/// The type of the function tool call. Always `function_call`.
///
///
/// The type of the web search tool call. Always `web_search_call`.
///
///
/// The type of the computer call. Always `computer_call`.
///
/// The type of the object. Always `reasoning`.
///
///
/// The type of the image generation call. Always `image_generation_call`.
///
///
/// The type of the code interpreter tool call. Always `code_interpreter_call`.
///
///
/// The type of the local shell call. Always `local_shell_call`.
///
///
/// The type of the item. Always `mcp_call`.
///
///
/// The type of the item. Always `mcp_list_tools`.
///
///
/// The type of the item. Always `mcp_approval_request`.
///
///
/// The type of the custom tool call. Always `custom_tool_call`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputItemType {
    #[serde(rename = "code_interpreter_call")]
    CodeInterpreterCall,
    #[serde(rename = "computer_call")]
    ComputerCall,
    #[serde(rename = "custom_tool_call")]
    CustomToolCall,
    #[serde(rename = "file_search_call")]
    FileSearchCall,
    #[serde(rename = "function_call")]
    FunctionCall,
    #[serde(rename = "image_generation_call")]
    ImageGenerationCall,
    #[serde(rename = "local_shell_call")]
    LocalShellCall,
    #[serde(rename = "mcp_approval_request")]
    McpApprovalRequest,
    #[serde(rename = "mcp_call")]
    McpCall,
    #[serde(rename = "mcp_list_tools")]
    McpListTools,
    Message,
    #[serde(rename = "reasoning")]
    Reasoning,
    #[serde(rename = "web_search_call")]
    WebSearchCall,
}

/// The status of the response generation. One of `completed`, `failed`,
/// `in_progress`, `cancelled`, `queued`, or `incomplete`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Cancelled,
    Completed,
    Failed,
    #[serde(rename = "in_progress")]
    InProgress,
    Incomplete,
    Queued,
}

/// How the model should select which tool (or tools) to use when generating
/// a response. See the `tools` parameter to see how to specify which tools
/// the model can call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TheResponseObjectToolChoice {
    Enum(Auto),
    HostedToolClass(HostedToolClass),
}

/// Represents token usage details including input tokens, output tokens,
/// a breakdown of output tokens, and the total tokens used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseUsage {
    /// The number of input tokens.
    pub input_tokens: i64,
    /// A detailed breakdown of the input tokens.
    pub input_tokens_details: InputTokensDetails,
    /// The number of output tokens.
    pub output_tokens: i64,
    /// A detailed breakdown of the output tokens.
    pub output_tokens_details: OutputTokensDetails,
    /// The total number of tokens used.
    pub total_tokens: i64,
}

/// A detailed breakdown of the input tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputTokensDetails {
    /// The number of tokens that were retrieved from the cache.
    /// [More on prompt caching](https://platform.openai.com/docs/guides/prompt-caching).
    pub cached_tokens: i64,
}

/// A detailed breakdown of the output tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputTokensDetails {
    /// The number of reasoning tokens.
    pub reasoning_tokens: i64,
}
