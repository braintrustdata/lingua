// Generated OpenAI Decisions API types using quicktype
// Source: specs/openai/decisions.yml
#![allow(clippy::large_enum_variant)]
#![allow(clippy::doc_lazy_continuation)]

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::OpenaiDecisionsSchemas;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: OpenaiDecisionsSchemas = serde_json::from_str(&json).unwrap();
// }

use crate::serde_json;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct OpenaiDecisionsSchemas {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_request: Option<DecisionRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_response: Option<DecisionResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct DecisionRequest {
    pub input: DecisionInput,
    /// The model to use. Only `gpt-6-luna` is supported.
    pub model: String,
    /// Partition prompt cache reuse by caller-provided key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    /// Options for prompt caching. Supported for `gpt-5.6` and later models. By default, OpenAI
    /// automatically chooses one implicit cache breakpoint. You can add explicit breakpoints to
    /// content blocks with `prompt_cache_breakpoint`. Each request can write up to four
    /// breakpoints. For cache matching, OpenAI considers up to the latest 80 breakpoints in the
    /// conversation, without a content-block lookback limit. Set `mode` to `explicit` to disable
    /// the implicit breakpoint. The `ttl` defaults to `30m`, which is currently the only
    /// supported value. See the [prompt caching
    /// guide](https://developers.openai.com/api/docs/guides/prompt-caching) for current details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_options: Option<PromptCacheOptions>,
    /// Ask 1–400 questions about the input. Each question can be a predicate, choice, or score.
    /// Answers come back in the same order.
    pub questions: Vec<QuestionParam>,
    /// Opaque caller-provided end-user identifier, scoped by the verified org. Match Responses'
    /// limit; this is never the authenticated user identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
}

/// The input to evaluate for every question. Provide a text string or an ordered array of
/// messages, function calls and outputs, or custom tool calls and outputs. Each tool output
/// must identify a matching call by call ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum DecisionInput {
    DecisionInputMessageArray(Vec<DecisionInputMessage>),
    PurpleString(String),
}

/// A user message containing input text, images, or JSON.
///
/// A developer message containing input text.
///
/// A system message containing input text.
///
/// An assistant message containing output text or refusals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct DecisionInputMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<DecisionInputContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_input_message_type: Option<DecisionInputType>,
    /// The phase of an assistant message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<MessagePhaseParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<FunctionCallStatusParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<DecisionInputToolOutputContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum DecisionInputContent {
    DecisionInputContentPartElementArray(Vec<DecisionInputContentPartElement>),
    PurpleString(String),
}

/// An image provided as a base64 data URL or a publicly accessible HTTP(S) URL. File IDs are
/// not supported.
///
/// A JSON object or an array of objects rendered as text. Nested values may be any JSON
/// value. Limited to 32 container levels, 10,000 values, and 256 KiB of compact UTF-8 JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct DecisionInputContentPartElement {
    /// Marks the exact end of a reusable prompt prefix. The breakpoint inherits its TTL from the
    /// request's `prompt_cache_options.ttl`; the boundary is not rounded to a token block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub decision_input_type: DecisionInputContentPartType,
    /// The image detail level, using the selected model's image profile. Defaults to auto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetailParam>,
    /// A base64-encoded image in a data URL or a publicly accessible HTTP(S) image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum Data {
    #[ts(type = "Record<string, unknown>")]
    AnythingMap(serde_json::Map<String, serde_json::Value>),
    #[ts(type = "Array<Record<string, unknown>>")]
    AnythingMapArray(Vec<serde_json::Map<String, serde_json::Value>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum DecisionInputContentPartType {
    #[serde(rename = "input_image")]
    InputImage,
    #[serde(rename = "input_json")]
    InputJson,
    #[serde(rename = "input_text")]
    InputText,
    #[serde(rename = "output_text")]
    OutputText,
    Refusal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum ImageDetailParam {
    Auto,
    High,
    Low,
    Original,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct PromptCacheBreakpoint {
    /// The breakpoint mode. Always `explicit`.
    pub mode: PromptCacheBreakpointMode,
}

/// The breakpoint mode. Always `explicit`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum PromptCacheBreakpointMode {
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum DecisionInputType {
    #[serde(rename = "custom_tool_call")]
    CustomToolCall,
    #[serde(rename = "custom_tool_call_output")]
    CustomToolCallOutput,
    #[serde(rename = "function_call")]
    FunctionCall,
    #[serde(rename = "function_call_output")]
    FunctionCallOutput,
    Message,
}

/// Text evidence or an ordered list of text and image parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum DecisionInputToolOutputContent {
    DecisionInputToolOutputContentPartElementArray(Vec<DecisionInputToolOutputContentPartElement>),
    PurpleString(String),
}

/// An image provided as a base64 data URL or a publicly accessible HTTP(S) URL. File IDs are
/// not supported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct DecisionInputToolOutputContentPartElement {
    /// Marks the exact end of a reusable prompt prefix. The breakpoint inherits its TTL from the
    /// request's `prompt_cache_options.ttl`; the boundary is not rounded to a token block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub decision_input_type: DecisionInputToolOutputContentPartType,
    /// The image detail level, using the selected model's image profile. Defaults to auto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetailParam>,
    /// A base64-encoded image in a data URL or a publicly accessible HTTP(S) image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum DecisionInputToolOutputContentPartType {
    #[serde(rename = "input_image")]
    InputImage,
    #[serde(rename = "input_text")]
    InputText,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum MessagePhaseParam {
    Commentary,
    #[serde(rename = "final_answer")]
    FinalAnswer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum Role {
    Assistant,
    Developer,
    System,
    User,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum FunctionCallStatusParam {
    Completed,
    #[serde(rename = "in_progress")]
    InProgress,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct PromptCacheOptions {
    /// Controls whether OpenAI automatically creates an implicit cache breakpoint. Defaults to
    /// `implicit`. With `implicit`, OpenAI creates one implicit breakpoint and writes up to the
    /// latest three explicit breakpoints in the request. With `explicit`, OpenAI does not create
    /// an implicit breakpoint and writes up to the latest four explicit breakpoints. If there
    /// are no explicit breakpoints, the request does not use prompt caching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PromptCacheOptionsMode>,
    /// The minimum lifetime applied to every implicit and explicit cache breakpoint written by
    /// the request. Defaults to `30m`, which is currently the only supported value. The backend
    /// may retain cache entries for longer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<Ttl>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum PromptCacheOptionsMode {
    Explicit,
    Implicit,
}

/// The minimum lifetime applied to every implicit and explicit cache breakpoint written by
/// the request. Defaults to `30m`, which is currently the only supported value. The backend
/// may retain cache entries for longer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub enum Ttl {
    #[serde(rename = "30m")]
    The30M,
}

/// A question about the request's input, with an optional correlation name.
///
/// Estimate how likely it is that a statement about the input is true.
///
/// Choose from the supplied options based on the input.
///
/// Rate the input against the supplied ordered levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct QuestionParam {
    pub instructions: Instructions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The type of the object. Always `predicate`.
    ///
    /// The type of the object. Always `choice`.
    ///
    /// The type of the object. Always `score`.
    #[serde(rename = "type")]
    pub question_param_type: QuestionParamType,
    /// Provide between 2 and 255 choices. Each choice must be unique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ChoiceOptionParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<Vec<ScoreLevelParam>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct ChoiceOptionParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub value: ChoiceValueParam,
}

/// Choice values are typed: a string and a boolean with the same text are distinct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum ChoiceValueParam {
    Bool(bool),
    PurpleString(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum Instructions {
    DecisionQuestionInputPartArray(Vec<DecisionQuestionInputPart>),
    PurpleString(String),
}

/// A text, image, or JSON content part in a question instruction.
///
/// An image provided as a base64 data URL or a publicly accessible HTTP(S) URL. File IDs are
/// not supported.
///
/// A JSON object or an array of objects rendered as text. Nested values may be any JSON
/// value. Limited to 32 container levels, 10,000 values, and 256 KiB of compact UTF-8 JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct DecisionQuestionInputPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub decision_question_input_part_type: DecisionQuestionInputPartType,
    /// The image detail level, using the selected model's image profile. Defaults to auto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetailParam>,
    /// A base64-encoded image in a data URL or a publicly accessible HTTP(S) image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum DecisionQuestionInputPartType {
    #[serde(rename = "input_image")]
    InputImage,
    #[serde(rename = "input_json")]
    InputJson,
    #[serde(rename = "input_text")]
    InputText,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct ScoreLevelParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub label: String,
}

/// The type of the object. Always `predicate`.
///
/// The type of the object. Always `choice`.
///
/// The type of the object. Always `score`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum QuestionParamType {
    Choice,
    Predicate,
    Score,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct DecisionResponse {
    /// Answers in question order. Any question can return a refusal instead of an answer; other
    /// questions can still receive answers. A refusal has `type: "refusal"` and the question's
    /// `name`, or `null` if it was unnamed.
    pub answers: Vec<AnswerResource>,
    /// The model used.
    pub model: String,
    /// Token usage for the request.
    pub usage: ResponseUsageResource,
}

/// A completed question always includes its name, including null when unnamed.
///
/// The model declined to answer this question. Other questions in the same request can still
/// receive answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct AnswerResource {
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability: Option<f64>,
    /// The type of the object. Always `predicate`.
    ///
    /// The type of the object. Always `choice`.
    ///
    /// The type of the object. Always `score`.
    ///
    /// The type of the object. Always `refusal`.
    #[serde(rename = "type")]
    pub answer_resource_type: AnswerResourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choice: Option<ChoiceValueResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probabilities: Option<Vec<EProbabilityResource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

/// The type of the object. Always `predicate`.
///
/// The type of the object. Always `choice`.
///
/// The type of the object. Always `score`.
///
/// The type of the object. Always `refusal`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "openai/")]
pub enum AnswerResourceType {
    Choice,
    Predicate,
    Refusal,
    Score,
}

/// Choice values are typed: a string and a boolean with the same text are distinct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum ChoiceValueResource {
    Bool(bool),
    PurpleString(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct EProbabilityResource {
    pub probability: f64,
    pub value: ValueUnion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "openai/")]
pub enum ValueUnion {
    Bool(bool),
    Integer(i64),
    PurpleString(String),
}

/// Token usage for the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct ResponseUsageResource {
    pub input_tokens: i64,
    pub input_tokens_details: InputTokensDetailsResource,
    pub output_tokens: i64,
    pub output_tokens_details: OutputTokensDetailsResource,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct InputTokensDetailsResource {
    pub cache_write_tokens: i64,
    pub cached_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "openai/")]
pub struct OutputTokensDetailsResource {
    pub reasoning_tokens: i64,
}
