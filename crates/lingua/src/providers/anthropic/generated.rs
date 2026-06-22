// Generated Anthropic types using quicktype
// Essential types for Elmir Anthropic integration
#![allow(non_camel_case_types)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::doc_lazy_continuation)]

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::anthropic_schemas;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: anthropic_schemas = serde_json::from_str(&json).unwrap();
// }

use crate::serde_json;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct AnthropicSchemas {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<CreateMessageParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_response: Option<ErrorResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_event: Option<MessageStreamEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ErrorResponse {
    pub error: Error,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(rename = "type")]
    pub error_response_type: ErrorResponseType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Error {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: ErrorType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ErrorType {
    #[serde(rename = "api_error")]
    ApiError,
    #[serde(rename = "authentication_error")]
    AuthenticationError,
    #[serde(rename = "billing_error")]
    BillingError,
    #[serde(rename = "invalid_request_error")]
    InvalidRequestError,
    #[serde(rename = "not_found_error")]
    NotFoundError,
    #[serde(rename = "overloaded_error")]
    OverloadedError,
    #[serde(rename = "permission_error")]
    PermissionError,
    #[serde(rename = "rate_limit_error")]
    RateLimitError,
    #[serde(rename = "timeout_error")]
    TimeoutError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ErrorResponseType {
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct CreateMessageParams {
    /// Top-level cache control automatically applies a cache_control marker to the last
    /// cacheable block in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    /// Container identifier for reuse across requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    /// Specifies the geographic region for inference processing. If not specified, the
    /// workspace's `default_inference_geo` is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    /// The maximum number of tokens to generate before stopping.
    ///
    /// Note that our models may stop _before_ reaching this maximum. This parameter only
    /// specifies the absolute maximum number of tokens to generate.
    ///
    /// Set to `0` to populate the [prompt
    /// cache](https://docs.claude.com/en/docs/build-with-claude/prompt-caching#pre-warming-the-cache)
    /// without generating a response.
    ///
    /// Different models have different maximum values for this parameter.  See
    /// [models](https://docs.claude.com/en/docs/models-overview) for details.
    pub max_tokens: i64,
    /// Input messages.
    ///
    /// Our models are trained to operate on alternating `user` and `assistant` conversational
    /// turns. When creating a new `Message`, you specify the prior conversational turns with the
    /// `messages` parameter, and the model then generates the next `Message` in the
    /// conversation. Consecutive `user` or `assistant` turns in your request will be combined
    /// into a single turn.
    ///
    /// Each input message must be an object with a `role` and `content`. You can specify a
    /// single `user`-role message, or you can include multiple `user` and `assistant` messages.
    ///
    /// If the final message uses the `assistant` role, the response content will continue
    /// immediately from the content in that message. This can be used to constrain part of the
    /// model's response.
    ///
    /// Example with a single `user` message:
    ///
    /// ```json
    /// [{"role": "user", "content": "Hello, Claude"}]
    /// ```
    ///
    /// Example with multiple conversational turns:
    ///
    /// ```json
    /// [
    /// {"role": "user", "content": "Hello there."},
    /// {"role": "assistant", "content": "Hi, I'm Claude. How can I help you?"},
    /// {"role": "user", "content": "Can you explain LLMs in plain English?"},
    /// ]
    /// ```
    ///
    /// Example with a partially-filled response from Claude:
    ///
    /// ```json
    /// [
    /// {"role": "user", "content": "What's the Greek name for Sun? (A) Sol (B) Helios (C)
    /// Sun"},
    /// {"role": "assistant", "content": "The best answer is ("},
    /// ]
    /// ```
    ///
    /// Each input message `content` may be either a single `string` or an array of content
    /// blocks, where each block has a specific `type`. Using a `string` for `content` is
    /// shorthand for an array of one content block of type `"text"`. The following input
    /// messages are equivalent:
    ///
    /// ```json
    /// {"role": "user", "content": "Hello, Claude"}
    /// ```
    ///
    /// ```json
    /// {"role": "user", "content": [{"type": "text", "text": "Hello, Claude"}]}
    /// ```
    ///
    /// See [input examples](https://docs.claude.com/en/api/messages-examples).
    ///
    /// Note that if you want to include a [system
    /// prompt](https://docs.claude.com/en/docs/system-prompts), you can use the top-level
    /// `system` parameter — there is no `"system"` role for input messages in the Messages API.
    ///
    /// There is a limit of 100,000 messages in a single request.
    pub messages: Vec<InputMessage>,
    /// An object describing metadata about the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    pub model: String,
    /// Configuration options for the model's output, such as the output format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
    /// Determines whether to use priority capacity (if available) or standard capacity for this
    /// request.
    ///
    /// Anthropic offers different levels of service for your API requests. See
    /// [service-tiers](https://docs.claude.com/en/api/service-tiers) for details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTierEnum>,
    /// Custom text sequences that will cause the model to stop generating.
    ///
    /// Our models will normally stop when they have naturally completed their turn, which will
    /// result in a response `stop_reason` of `"end_turn"`.
    ///
    /// If you want the model to stop generating when it encounters custom strings of text, you
    /// can use the `stop_sequences` parameter. If the model encounters one of the custom
    /// sequences, the response `stop_reason` value will be `"stop_sequence"` and the response
    /// `stop_sequence` value will contain the matched stop sequence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Whether to incrementally stream the response using server-sent events.
    ///
    /// See [streaming](https://docs.claude.com/en/api/messages-streaming) for details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// System prompt.
    ///
    /// A system prompt is a way of providing context and instructions to Claude, such as
    /// specifying a particular goal or role. See our [guide to system
    /// prompts](https://docs.claude.com/en/docs/system-prompts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<System>,
    /// Amount of randomness injected into the response.
    ///
    /// Defaults to `1.0`. Ranges from `0.0` to `1.0`. Use `temperature` closer to `0.0` for
    /// analytical / multiple choice, and closer to `1.0` for creative and generative tasks.
    ///
    /// Note that even with `temperature` of `0.0`, the results will not be fully deterministic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Definitions of tools that the model may use.
    ///
    /// If you include `tools` in your API request, the model may return `tool_use` content
    /// blocks that represent the model's use of those tools. You can then run those tools using
    /// the tool input generated by the model and then optionally return results back to the
    /// model using `tool_result` content blocks.
    ///
    /// There are two types of tools: **client tools** and **server tools**. The behavior
    /// described below applies to client tools. For [server
    /// tools](https://docs.claude.com/en/docs/agents-and-tools/tool-use/overview#server-tools),
    /// see their individual documentation as each has its own behavior (e.g., the [web search
    /// tool](https://docs.claude.com/en/docs/agents-and-tools/tool-use/web-search-tool)).
    ///
    /// Each tool definition includes:
    ///
    /// * `name`: Name of the tool.
    /// * `description`: Optional, but strongly-recommended description of the tool.
    /// * `input_schema`: [JSON schema](https://json-schema.org/draft/2020-12) for the tool
    /// `input` shape that the model will produce in `tool_use` output content blocks.
    ///
    /// For example, if you defined `tools` as:
    ///
    /// ```json
    /// [
    /// {
    /// "name": "get_stock_price",
    /// "description": "Get the current stock price for a given ticker symbol.",
    /// "input_schema": {
    /// "type": "object",
    /// "properties": {
    /// "ticker": {
    /// "type": "string",
    /// "description": "The stock ticker symbol, e.g. AAPL for Apple Inc."
    /// }
    /// },
    /// "required": ["ticker"]
    /// }
    /// }
    /// ]
    /// ```
    ///
    /// And then asked the model "What's the S&P 500 at today?", the model might produce
    /// `tool_use` content blocks in the response like this:
    ///
    /// ```json
    /// [
    /// {
    /// "type": "tool_use",
    /// "id": "toolu_01D7FLrfh4GYq7yT1ULFeyMV",
    /// "name": "get_stock_price",
    /// "input": { "ticker": "^GSPC" }
    /// }
    /// ]
    /// ```
    ///
    /// You might then run your `get_stock_price` tool with `{"ticker": "^GSPC"}` as an input,
    /// and return the following back to the model in a subsequent `user` message:
    ///
    /// ```json
    /// [
    /// {
    /// "type": "tool_result",
    /// "tool_use_id": "toolu_01D7FLrfh4GYq7yT1ULFeyMV",
    /// "content": "259.75 USD"
    /// }
    /// ]
    /// ```
    ///
    /// Tools can be used for workflows that include running client-side tools and functions, or
    /// more generally whenever you want the model to produce a particular JSON structure of
    /// output.
    ///
    /// See our [guide](https://docs.claude.com/en/docs/tool-use) for more details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Only sample from the top K options for each subsequent token.
    ///
    /// Used to remove "long tail" low probability responses. [Learn more technical details
    /// here](https://towardsdatascience.com/how-to-sample-from-language-models-682bceb97277).
    ///
    /// Recommended for advanced use cases only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,
    /// Use nucleus sampling.
    ///
    /// In nucleus sampling, we compute the cumulative distribution over all the options for each
    /// subsequent token in decreasing probability order and cut it off once it reaches a
    /// particular probability specified by `top_p`.
    ///
    /// Recommended for advanced use cases only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct CacheControlEphemeral {
    /// The time-to-live for the cache control breakpoint.
    ///
    /// This may be one the following values:
    /// - `5m`: 5 minutes
    /// - `1h`: 1 hour
    ///
    /// Defaults to `5m`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<Ttl>,
    #[serde(rename = "type")]
    pub cache_control_ephemeral_type: CacheControlEphemeralType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum CacheControlEphemeralType {
    Ephemeral,
}

/// The time-to-live for the cache control breakpoint.
///
/// This may be one the following values:
/// - `5m`: 5 minutes
/// - `1h`: 1 hour
///
/// Defaults to `5m`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub enum Ttl {
    #[serde(rename = "1h")]
    The1H,
    #[serde(rename = "5m")]
    The5M,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "anthropic/")]
pub struct InputMessage {
    pub content: MessageContent,
    pub role: PurpleRole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum MessageContent {
    InputContentBlockArray(Vec<InputContentBlock>),
    String(String),
}

/// Regular text content.
///
/// Image content specified directly as base64 data or as a reference via a URL.
///
/// Document content, either specified directly as base64 data, as text, or as a reference
/// via a URL.
///
/// A search result block containing source, title, and content from search operations.
///
/// A block specifying internal thinking by the model.
///
/// A block specifying internal, redacted thinking by the model.
///
/// A block indicating a tool use by the model.
///
/// A block specifying the results of a tool use by the model.
///
/// A content block that represents a file to be uploaded to the container
/// Files uploaded via this block will be available in the container's input directory.
///
/// System instructions that appear mid-conversation.
///
/// Use this block to provide or update system-level instructions at a specific
/// point in the conversation, rather than only via the top-level `system` parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct InputContentBlock {
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Citations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub input_content_block_type: InputContentBlockType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// System instruction text blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<InputContentBlockContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<Caller>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
}

/// Tool invocation directly from the model.
///
/// Tool invocation generated by a server-side tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Caller {
    #[serde(rename = "type")]
    pub caller_type: AllowedCaller,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<String>,
}

/// Specifies who can invoke a tool.
///
/// Values:
/// direct: The model can call this tool directly.
/// code_execution_20250825: The tool can be called from the code execution environment
/// (v1).
/// code_execution_20260120: The tool can be called from the code execution environment (v2
/// with persistence).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum AllowedCaller {
    #[serde(rename = "code_execution_20250825")]
    CodeExecution20250825,
    #[serde(rename = "code_execution_20260120")]
    CodeExecution20260120,
    Direct,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum Citations {
    RequestCitationsConfig(RequestCitationsConfig),
    RequestLocationCitationArray(Vec<RequestLocationCitation>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RequestLocationCitation {
    /// The full text of the cited block range, concatenated.
    ///
    /// Always equals the contents of `content[start_block_index:end_block_index]` joined
    /// together. The text block is the minimal citable unit; this field is never a substring of
    /// a single block. Not counted toward output tokens, and not counted toward input tokens
    /// when sent back in subsequent turns.
    pub cited_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_char_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_char_index: Option<i64>,
    #[serde(rename = "type")]
    pub request_location_citation_type: CitationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_page_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_page_number: Option<i64>,
    /// Exclusive 0-based end index of the cited block range in the source's `content` array.
    ///
    /// Always greater than `start_block_index`; a single-block citation has `end_block_index =
    /// start_block_index + 1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_block_index: Option<i64>,
    /// 0-based index of the first cited block in the source's `content` array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_block_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 0-based index of the cited search result among all `search_result` content blocks in the
    /// request, in the order they appear across messages and tool results.
    ///
    /// Counted separately from `document_index`; server-side web search results are not included
    /// in this count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum CitationType {
    #[serde(rename = "char_location")]
    CharLocation,
    #[serde(rename = "content_block_location")]
    ContentBlockLocation,
    #[serde(rename = "page_location")]
    PageLocation,
    #[serde(rename = "search_result_location")]
    SearchResultLocation,
    #[serde(rename = "web_search_result_location")]
    WebSearchResultLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RequestCitationsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum InputContentBlockContent {
    BlockArray(Vec<Block>),
    RequestWebSearchToolResultError(RequestWebSearchToolResultError),
    String(String),
}

/// Regular text content.
///
/// Image content specified directly as base64 data or as a reference via a URL.
///
/// A search result block containing source, title, and content from search operations.
///
/// Document content, either specified directly as base64 data, as text, or as a reference
/// via a URL.
///
/// Tool reference block that can be included in tool_result content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Block {
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Citations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub block_type: WebSearchToolResultBlockItemType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<RequestTextBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_age: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum WebSearchToolResultBlockItemType {
    Document,
    Image,
    #[serde(rename = "search_result")]
    SearchResult,
    Text,
    #[serde(rename = "tool_reference")]
    ToolReference,
    #[serde(rename = "web_search_result")]
    WebSearchResult,
}

/// Regular text content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RequestTextBlock {
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<RequestLocationCitation>>,
    pub text: String,
    #[serde(rename = "type")]
    pub request_text_block_type: SystemType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum SystemType {
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum Source {
    SourceSource(SourceSource),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct SourceSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<FluffyMediaType>,
    #[serde(rename = "type")]
    pub source_type: FluffyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<SourceContent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum SourceContent {
    ContentBlockSourceContentItemArray(Vec<ContentBlockSourceContentItem>),
    String(String),
}

/// Regular text content.
///
/// Image content specified directly as base64 data or as a reference via a URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ContentBlockSourceContentItem {
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<RequestLocationCitation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub content_block_source_content_item_type: ContentBlockSourceContentItemType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceSourceClass>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ContentBlockSourceContentItemType {
    Image,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct SourceSourceClass {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<PurpleMediaType>,
    #[serde(rename = "type")]
    pub source_type: PurpleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub enum PurpleMediaType {
    #[serde(rename = "image/gif")]
    ImageGif,
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    ImagePng,
    #[serde(rename = "image/webp")]
    ImageWebp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum PurpleType {
    Base64,
    Url,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub enum FluffyMediaType {
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
    #[serde(rename = "image/gif")]
    ImageGif,
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    ImagePng,
    #[serde(rename = "image/webp")]
    ImageWebp,
    #[serde(rename = "text/plain")]
    TextPlain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum FluffyType {
    Base64,
    Content,
    Text,
    Url,
}

/// Code execution result with encrypted stdout for PFC + web_search results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RequestWebSearchToolResultError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ToolResultErrorCode>,
    #[serde(rename = "type")]
    pub request_web_search_tool_result_error_type: RequestWebSearchToolResultErrorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<RequestWebSearchToolResultErrorContent>,
    /// ISO 8601 timestamp when the content was retrieved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieved_at: Option<String>,
    /// Fetched content URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<FileType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_file_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_references: Option<Vec<RequestToolReferenceBlock>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum RequestWebSearchToolResultErrorContent {
    RequestCodeExecutionOutputBlockArray(Vec<RequestCodeExecutionOutputBlock>),
    RequestDocumentBlock(RequestDocumentBlock),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RequestCodeExecutionOutputBlock {
    pub file_id: String,
    #[serde(rename = "type")]
    pub request_code_execution_output_block_type: TentacledType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum TentacledType {
    #[serde(rename = "bash_code_execution_output")]
    BashCodeExecutionOutput,
    #[serde(rename = "code_execution_output")]
    CodeExecutionOutput,
}

/// Document content, either specified directly as base64 data, as text, or as a reference
/// via a URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RequestDocumentBlock {
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<RequestCitationsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub source: PurpleSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub request_document_block_type: StickyType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum StickyType {
    Document,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct PurpleSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<TentacledMediaType>,
    #[serde(rename = "type")]
    pub source_type: FluffyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<SourceContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub enum TentacledMediaType {
    #[serde(rename = "application/pdf")]
    ApplicationPdf,
    #[serde(rename = "text/plain")]
    TextPlain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ToolResultErrorCode {
    #[serde(rename = "execution_time_exceeded")]
    ExecutionTimeExceeded,
    #[serde(rename = "file_not_found")]
    FileNotFound,
    #[serde(rename = "invalid_tool_input")]
    InvalidToolInput,
    #[serde(rename = "max_uses_exceeded")]
    MaxUsesExceeded,
    #[serde(rename = "output_file_too_large")]
    OutputFileTooLarge,
    #[serde(rename = "query_too_long")]
    QueryTooLong,
    #[serde(rename = "request_too_large")]
    RequestTooLarge,
    #[serde(rename = "too_many_requests")]
    TooManyRequests,
    Unavailable,
    #[serde(rename = "unsupported_content_type")]
    UnsupportedContentType,
    #[serde(rename = "url_not_accessible")]
    UrlNotAccessible,
    #[serde(rename = "url_not_allowed")]
    UrlNotAllowed,
    #[serde(rename = "url_not_in_prior_context")]
    UrlNotInPriorContext,
    #[serde(rename = "url_too_long")]
    UrlTooLong,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum FileType {
    Image,
    Pdf,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum RequestWebSearchToolResultErrorType {
    #[serde(rename = "bash_code_execution_result")]
    BashCodeExecutionResult,
    #[serde(rename = "bash_code_execution_tool_result_error")]
    BashCodeExecutionToolResultError,
    #[serde(rename = "code_execution_result")]
    CodeExecutionResult,
    #[serde(rename = "code_execution_tool_result_error")]
    CodeExecutionToolResultError,
    #[serde(rename = "encrypted_code_execution_result")]
    EncryptedCodeExecutionResult,
    #[serde(rename = "text_editor_code_execution_create_result")]
    TextEditorCodeExecutionCreateResult,
    #[serde(rename = "text_editor_code_execution_str_replace_result")]
    TextEditorCodeExecutionStrReplaceResult,
    #[serde(rename = "text_editor_code_execution_tool_result_error")]
    TextEditorCodeExecutionToolResultError,
    #[serde(rename = "text_editor_code_execution_view_result")]
    TextEditorCodeExecutionViewResult,
    #[serde(rename = "tool_search_tool_result_error")]
    ToolSearchToolResultError,
    #[serde(rename = "tool_search_tool_search_result")]
    ToolSearchToolSearchResult,
    #[serde(rename = "web_fetch_result")]
    WebFetchResult,
    #[serde(rename = "web_fetch_tool_result_error")]
    WebFetchToolResultError,
    #[serde(rename = "web_search_tool_result_error")]
    WebSearchToolResultError,
}

/// Tool reference block that can be included in tool_result content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RequestToolReferenceBlock {
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    pub tool_name: String,
    #[serde(rename = "type")]
    pub request_tool_reference_block_type: ToolReferenceType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ToolReferenceType {
    #[serde(rename = "tool_reference")]
    ToolReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum InputContentBlockType {
    #[serde(rename = "bash_code_execution_tool_result")]
    BashCodeExecutionToolResult,
    #[serde(rename = "code_execution_tool_result")]
    CodeExecutionToolResult,
    #[serde(rename = "container_upload")]
    ContainerUpload,
    Document,
    Image,
    #[serde(rename = "mid_conv_system")]
    MidConvSystem,
    #[serde(rename = "redacted_thinking")]
    RedactedThinking,
    #[serde(rename = "search_result")]
    SearchResult,
    #[serde(rename = "server_tool_use")]
    ServerToolUse,
    Text,
    #[serde(rename = "text_editor_code_execution_tool_result")]
    TextEditorCodeExecutionToolResult,
    Thinking,
    #[serde(rename = "tool_result")]
    ToolResult,
    #[serde(rename = "tool_search_tool_result")]
    ToolSearchToolResult,
    #[serde(rename = "tool_use")]
    ToolUse,
    #[serde(rename = "web_fetch_tool_result")]
    WebFetchToolResult,
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum PurpleRole {
    Assistant,
    System,
    User,
}

/// An object describing metadata about the request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Metadata {
    /// An external identifier for the user who is associated with the request.
    ///
    /// This should be a uuid, hash value, or other opaque identifier. Anthropic may use this id
    /// to help detect abuse. Do not include any identifying information such as name, email
    /// address, or phone number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Configuration options for the model's output, such as the output format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct OutputConfig {
    /// How much effort the model should put into its response. Higher effort levels may result
    /// in more thorough analysis but take longer.
    ///
    /// Valid values are `low`, `medium`, `high`, `xhigh`, or `max`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<EffortLevel>,
    /// A schema to specify Claude's output format in responses. See [structured
    /// outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<JsonOutputFormat>,
}

/// All possible effort levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum EffortLevel {
    High,
    Low,
    Max,
    Medium,
    Xhigh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct JsonOutputFormat {
    /// The JSON schema of the format
    #[ts(type = "unknown")]
    pub schema: serde_json::Map<String, serde_json::Value>,
    #[serde(rename = "type")]
    pub json_output_format_type: JsonOutputFormatType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum JsonOutputFormatType {
    #[serde(rename = "json_schema")]
    JsonSchema,
}

/// Determines whether to use priority capacity (if available) or standard capacity for this
/// request.
///
/// Anthropic offers different levels of service for your API requests. See
/// [service-tiers](https://docs.claude.com/en/api/service-tiers) for details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ServiceTierEnum {
    Auto,
    #[serde(rename = "standard_only")]
    StandardOnly,
}

/// System prompt.
///
/// A system prompt is a way of providing context and instructions to Claude, such as
/// specifying a particular goal or role. See our [guide to system
/// prompts](https://docs.claude.com/en/docs/system-prompts).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum System {
    RequestTextBlockArray(Vec<RequestTextBlock>),
    String(String),
}

/// Configuration for enabling Claude's extended thinking.
///
/// When enabled, responses include `thinking` content blocks showing Claude's thinking
/// process before the final answer. Requires a minimum budget of 1,024 tokens and counts
/// towards your `max_tokens` limit.
///
/// See [extended
/// thinking](https://docs.claude.com/en/docs/build-with-claude/extended-thinking) for
/// details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Thinking {
    /// Determines how many tokens Claude can use for its internal reasoning process. Larger
    /// budgets can enable more thorough analysis for complex problems, improving response
    /// quality.
    ///
    /// Must be ≥1024 and less than `max_tokens`.
    ///
    /// See [extended
    /// thinking](https://docs.claude.com/en/docs/build-with-claude/extended-thinking) for
    /// details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<i64>,
    /// Controls how thinking content appears in the response. When set to `summarized`, thinking
    /// is returned normally. When set to `omitted`, thinking content is redacted but a signature
    /// is returned for multi-turn continuity. Defaults to `summarized`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<ThinkingDisplayMode>,
    #[serde(rename = "type")]
    pub thinking_type: ThinkingType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ThinkingDisplayMode {
    Omitted,
    Summarized,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ThinkingType {
    Adaptive,
    Disabled,
    Enabled,
}

/// How the model should use the provided tools. The model can use a specific tool, any
/// available tool, decide by itself, or not use tools at all.
///
/// The model will automatically decide whether to use tools.
///
/// The model will use any available tools.
///
/// The model will use the specified tool with `tool_choice.name`.
///
/// The model will not be allowed to use tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ToolChoice {
    /// Whether to disable parallel tool use.
    ///
    /// Defaults to `false`. If set to `true`, the model will output at most one tool use.
    ///
    /// Whether to disable parallel tool use.
    ///
    /// Defaults to `false`. If set to `true`, the model will output exactly one tool use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
    #[serde(rename = "type")]
    pub tool_choice_type: ToolChoiceType,
    /// The name of the tool to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ToolChoiceType {
    Any,
    Auto,
    None,
    Tool,
}

/// Code execution tool with REPL state persistence (daemon mode + gVisor checkpoint).
///
/// Web fetch tool with use_cache parameter for bypassing cached content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct CustomTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Description of what this tool does.
    ///
    /// Tool descriptions should be as detailed as possible. The more information that the model has about what the tool is and how to use it, the better it will perform. You can use natural language descriptions to reinforce important aspects of the tool input JSON schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Enable eager input streaming for this tool. When true, tool input parameters will be streamed incrementally as they are generated, and types will be inferred on-the-fly rather than buffering the full JSON output. When false, streaming is disabled for this tool even if the fine-grained-tool-streaming beta is active. When null (default), uses the default behavior based on beta headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eager_input_streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub input_examples: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// [JSON schema](https://json-schema.org/draft/2020-12) for this tool's input.
    ///
    /// This defines the shape of the `input` that your tool accepts and that the model will produce.
    #[ts(type = "unknown")]
    pub input_schema: serde_json::Value,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct BashTool20250124 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub input_examples: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct CodeExecutionTool20250522 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct CodeExecutionTool20250825 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Code execution tool with REPL state persistence (daemon mode + gVisor checkpoint).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct CodeExecutionTool20260120 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct MemoryTool20250818 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub input_examples: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct TextEditor20250124 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub input_examples: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct TextEditor20250429 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub input_examples: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct TextEditor20250728 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub input_examples: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Maximum number of characters to display when viewing a file. If not specified, defaults to displaying the full file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters: Option<i64>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct WebFetchTool20250910 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// List of domains to allow fetching from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// List of domains to block fetching from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// Citations configuration for fetched documents. Citations are disabled by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub citations: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Maximum number of tokens used by including web page text content in the context. The limit is approximate and does not apply to binary content such as PDFs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_tokens: Option<i64>,
    /// Maximum number of times the tool can be used in the API request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i64>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct WebFetchTool20260209 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// List of domains to allow fetching from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// List of domains to block fetching from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// Citations configuration for fetched documents. Citations are disabled by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub citations: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Maximum number of tokens used by including web page text content in the context. The limit is approximate and does not apply to binary content such as PDFs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_tokens: Option<i64>,
    /// Maximum number of times the tool can be used in the API request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i64>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Web fetch tool with use_cache parameter for bypassing cached content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct WebFetchTool20260309 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// List of domains to allow fetching from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// List of domains to block fetching from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// Citations configuration for fetched documents. Citations are disabled by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub citations: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Maximum number of tokens used by including web page text content in the context. The limit is approximate and does not apply to binary content such as PDFs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_tokens: Option<i64>,
    /// Maximum number of times the tool can be used in the API request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i64>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// Whether to use cached content. Set to false to bypass the cache and fetch fresh content. Only set to false when the user explicitly requests fresh content or when fetching rapidly-changing sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_cache: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct WebSearchTool20250305 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// If provided, only these domains will be included in results. Cannot be used alongside `blocked_domains`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// If provided, these domains will never appear in results. Cannot be used alongside `allowed_domains`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Maximum number of times the tool can be used in the API request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i64>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// Parameters for the user's location. Used to provide more relevant search results.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub user_location: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct WebSearchTool20260209 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    /// If provided, only these domains will be included in results. Cannot be used alongside `blocked_domains`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// If provided, these domains will never appear in results. Cannot be used alongside `allowed_domains`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    /// Create a cache control breakpoint at this content block.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub cache_control: Option<serde_json::Value>,
    /// If true, tool will not be included in initial system prompt. Only loaded when returned via tool_reference from tool search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    /// Maximum number of times the tool can be used in the API request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<i64>,
    /// Name of the tool.
    ///
    /// This is how the tool will be called by the model and in `tool_use` blocks.
    pub name: String,
    /// When true, guarantees schema validation on tool names and inputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// Parameters for the user's location. Used to provide more relevant search results.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub user_location: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export_to = "anthropic/")]
pub enum Tool {
    #[serde(rename = "bash_20250124")]
    Bash20250124(BashTool20250124),

    #[serde(rename = "code_execution_20250522")]
    CodeExecution20250522(CodeExecutionTool20250522),

    #[serde(rename = "code_execution_20250825")]
    CodeExecution20250825(CodeExecutionTool20250825),

    #[serde(rename = "code_execution_20260120")]
    CodeExecution20260120(CodeExecutionTool20260120),

    #[serde(rename = "memory_20250818")]
    Memory20250818(MemoryTool20250818),

    #[serde(rename = "text_editor_20250124")]
    TextEditor20250124(TextEditor20250124),

    #[serde(rename = "text_editor_20250429")]
    TextEditor20250429(TextEditor20250429),

    #[serde(rename = "text_editor_20250728")]
    TextEditor20250728(TextEditor20250728),

    #[serde(rename = "web_fetch_20250910")]
    WebFetch20250910(WebFetchTool20250910),

    #[serde(rename = "web_fetch_20260209")]
    WebFetch20260209(WebFetchTool20260209),

    #[serde(rename = "web_fetch_20260309")]
    WebFetch20260309(WebFetchTool20260309),

    #[serde(rename = "web_search_20250305")]
    WebSearch20250305(WebSearchTool20250305),

    #[serde(rename = "web_search_20260209")]
    WebSearch20260209(WebSearchTool20260209),

    #[serde(untagged)]
    Custom(CustomTool),
}

/// [JSON schema](https://json-schema.org/draft/2020-12) for this tool's input.
///
/// This defines the shape of the `input` that your tool accepts and that the model will
/// produce.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct InputSchema {
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub input_schema_type: InputSchemaType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum InputSchemaType {
    Object,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ToolType {
    #[serde(rename = "bash_20250124")]
    Bash20250124,
    #[serde(rename = "code_execution_20250522")]
    CodeExecution20250522,
    #[serde(rename = "code_execution_20250825")]
    CodeExecution20250825,
    #[serde(rename = "code_execution_20260120")]
    CodeExecution20260120,
    Custom,
    #[serde(rename = "memory_20250818")]
    Memory20250818,
    #[serde(rename = "text_editor_20250124")]
    TextEditor20250124,
    #[serde(rename = "text_editor_20250429")]
    TextEditor20250429,
    #[serde(rename = "text_editor_20250728")]
    TextEditor20250728,
    #[serde(rename = "tool_search_tool_bm25")]
    ToolSearchToolBm25,
    #[serde(rename = "tool_search_tool_bm25_20251119")]
    ToolSearchToolBm2520251119,
    #[serde(rename = "tool_search_tool_regex")]
    ToolSearchToolRegex,
    #[serde(rename = "tool_search_tool_regex_20251119")]
    ToolSearchToolRegex20251119,
    #[serde(rename = "web_fetch_20250910")]
    WebFetch20250910,
    #[serde(rename = "web_fetch_20260209")]
    WebFetch20260209,
    #[serde(rename = "web_fetch_20260309")]
    WebFetch20260309,
    #[serde(rename = "web_search_20250305")]
    WebSearch20250305,
    #[serde(rename = "web_search_20260209")]
    WebSearch20260209,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct UserLocation {
    /// The city of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// The two letter [ISO country code](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2) of
    /// the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// The region of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// The [IANA timezone](https://nodatime.org/TimeZones) of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(rename = "type")]
    pub user_location_type: UserLocationType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum UserLocationType {
    Approximate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Message {
    /// Information about the container used in this request.
    ///
    /// This will be non-null if a container tool (e.g. code execution) was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Container>,
    /// Content generated by the model.
    ///
    /// This is an array of content blocks, each of which has a `type` that determines its
    /// shape.
    ///
    /// Example:
    ///
    /// ```json
    /// [{"type": "text", "text": "Hi, I'm Claude."}]
    /// ```
    ///
    /// If the request input `messages` ended with an `assistant` turn, then the response
    /// `content` will continue directly from that last turn. You can use this to constrain the
    /// model's output.
    ///
    /// For example, if the input `messages` were:
    /// ```json
    /// [
    /// {"role": "user", "content": "What's the Greek name for Sun? (A) Sol (B) Helios (C)
    /// Sun"},
    /// {"role": "assistant", "content": "The best answer is ("}
    /// ]
    /// ```
    ///
    /// Then the response `content` might be:
    ///
    /// ```json
    /// [{"type": "text", "text": "B)"}]
    /// ```
    pub content: Vec<ContentBlock>,
    /// Unique object identifier.
    ///
    /// The format and length of IDs may change over time.
    pub id: String,
    pub model: String,
    /// Conversational role of the generated message.
    ///
    /// This will always be `"assistant"`.
    pub role: ResponseRole,
    /// Structured information about why model output stopped.
    ///
    /// This is `null` when the `stop_reason` has no additional detail to report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_details: Option<RefusalStopDetails>,
    /// The reason that we stopped.
    ///
    /// This may be one the following values:
    /// * `"end_turn"`: the model reached a natural stopping point
    /// * `"max_tokens"`: we exceeded the requested `max_tokens` or the model's maximum
    /// * `"stop_sequence"`: one of your provided custom `stop_sequences` was generated
    /// * `"tool_use"`: the model invoked one or more tools
    /// * `"pause_turn"`: we paused a long-running turn. You may provide the response back as-is
    /// in a subsequent request to let the model continue.
    /// * `"refusal"`: when streaming classifiers intervene to handle potential policy
    /// violations
    ///
    /// In non-streaming mode this value is always non-null. In streaming mode, it is null in the
    /// `message_start` event and non-null otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// Which custom stop sequence was generated, if any.
    ///
    /// This value will be a non-null string if one of your custom stop sequences was generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    /// Object type.
    ///
    /// For Messages, this is always `"message"`.
    #[serde(rename = "type")]
    pub message_type: ResponseType,
    /// Billing and rate-limit usage.
    ///
    /// Anthropic's API bills and rate-limits by token counts, as tokens represent the underlying
    /// cost to our systems.
    ///
    /// Under the hood, the API transforms requests into a format suitable for the model. The
    /// model's output then goes through a parsing stage before becoming an API response. As a
    /// result, the token counts in `usage` will not match one-to-one with the exact visible
    /// content of an API request or response.
    ///
    /// For example, `output_tokens` will be non-zero, even for an empty string response from
    /// Claude.
    ///
    /// Total input tokens in a request is the summation of `input_tokens`,
    /// `cache_creation_input_tokens`, and `cache_read_input_tokens`.
    pub usage: Usage,
}

/// Information about the container used in the request (for the code execution tool)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Container {
    /// The time at which the container will expire.
    pub expires_at: String,
    /// Identifier for the container used in this request
    pub id: String,
}

/// Response model for a file uploaded to the container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ContentBlock {
    /// Citations supporting the text block.
    ///
    /// The type of citation returned will depend on the type of document being cited. Citing a
    /// PDF results in `page_location`, plain text results in `char_location`, and content
    /// document results in `content_block_location`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<Citation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub content_block_type: ContentBlockType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<Caller>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[ts(type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentBlockContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Citation {
    /// The full text of the cited block range, concatenated.
    ///
    /// Always equals the contents of `content[start_block_index:end_block_index]` joined
    /// together. The text block is the minimal citable unit; this field is never a substring of
    /// a single block. Not counted toward output tokens, and not counted toward input tokens
    /// when sent back in subsequent turns.
    pub cited_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_char_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_char_index: Option<i64>,
    #[serde(rename = "type")]
    pub citation_type: CitationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_page_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_page_number: Option<i64>,
    /// Exclusive 0-based end index of the cited block range in the source's `content` array.
    ///
    /// Always greater than `start_block_index`; a single-block citation has `end_block_index =
    /// start_block_index + 1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_block_index: Option<i64>,
    /// 0-based index of the first cited block in the source's `content` array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_block_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 0-based index of the cited search result among all `search_result` content blocks in the
    /// request, in the order they appear across messages and tool results.
    ///
    /// Counted separately from `document_index`; server-side web search results are not included
    /// in this count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum ContentBlockContent {
    ResponseWebSearchResultBlockArray(Vec<ResponseWebSearchResultBlock>),
    ResponseWebSearchToolResultError(ResponseWebSearchToolResultError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ResponseWebSearchResultBlock {
    pub encrypted_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_age: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub response_web_search_result_block_type: IndigoType,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum IndigoType {
    #[serde(rename = "web_search_result")]
    WebSearchResult,
}

/// Code execution result with encrypted stdout for PFC + web_search results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ResponseWebSearchToolResultError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ToolResultErrorCode>,
    #[serde(rename = "type")]
    pub response_web_search_tool_result_error_type: RequestWebSearchToolResultErrorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ResponseWebSearchToolResultErrorContent>,
    /// ISO 8601 timestamp when the content was retrieved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieved_at: Option<String>,
    /// Fetched content URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<FileType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_file_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_lines: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_references: Option<Vec<ResponseToolReferenceBlock>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export_to = "anthropic/")]
pub enum ResponseWebSearchToolResultErrorContent {
    ResponseCodeExecutionOutputBlockArray(Vec<ResponseCodeExecutionOutputBlock>),
    ResponseDocumentBlock(ResponseDocumentBlock),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ResponseCodeExecutionOutputBlock {
    pub file_id: String,
    #[serde(rename = "type")]
    pub response_code_execution_output_block_type: TentacledType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ResponseDocumentBlock {
    /// Citation configuration for the document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<ResponseCitationsConfig>,
    pub source: FluffySource,
    /// The title of the document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub response_document_block_type: StickyType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ResponseCitationsConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct FluffySource {
    pub data: String,
    pub media_type: TentacledMediaType,
    #[serde(rename = "type")]
    pub source_type: IndecentType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum IndecentType {
    Base64,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ResponseToolReferenceBlock {
    pub tool_name: String,
    #[serde(rename = "type")]
    pub response_tool_reference_block_type: ToolReferenceType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ContentBlockType {
    #[serde(rename = "bash_code_execution_tool_result")]
    BashCodeExecutionToolResult,
    #[serde(rename = "code_execution_tool_result")]
    CodeExecutionToolResult,
    #[serde(rename = "container_upload")]
    ContainerUpload,
    #[serde(rename = "redacted_thinking")]
    RedactedThinking,
    #[serde(rename = "server_tool_use")]
    ServerToolUse,
    Text,
    #[serde(rename = "text_editor_code_execution_tool_result")]
    TextEditorCodeExecutionToolResult,
    Thinking,
    #[serde(rename = "tool_search_tool_result")]
    ToolSearchToolResult,
    #[serde(rename = "tool_use")]
    ToolUse,
    #[serde(rename = "web_fetch_tool_result")]
    WebFetchToolResult,
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ResponseType {
    Message,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ResponseRole {
    Assistant,
}

/// Structured information about a refusal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct RefusalStopDetails {
    /// The policy category that triggered the refusal.
    ///
    /// `null` when the refusal doesn't map to a named category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    /// Human-readable explanation of the refusal.
    ///
    /// This text is not guaranteed to be stable. `null` when no explanation is available for the
    /// category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(rename = "type")]
    pub refusal_stop_details_type: RefusalStopDetailsType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum Category {
    Bio,
    Cyber,
    #[serde(rename = "frontier_llm")]
    FrontierLlm,
    #[serde(rename = "reasoning_extraction")]
    ReasoningExtraction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum RefusalStopDetailsType {
    Refusal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum StopReason {
    #[serde(rename = "end_turn")]
    EndTurn,
    #[serde(rename = "max_tokens")]
    MaxTokens,
    #[serde(rename = "pause_turn")]
    PauseTurn,
    Refusal,
    #[serde(rename = "stop_sequence")]
    StopSequence,
    #[serde(rename = "tool_use")]
    ToolUse,
}

/// Billing and rate-limit usage.
///
/// Anthropic's API bills and rate-limits by token counts, as tokens represent the underlying
/// cost to our systems.
///
/// Under the hood, the API transforms requests into a format suitable for the model. The
/// model's output then goes through a parsing stage before becoming an API response. As a
/// result, the token counts in `usage` will not match one-to-one with the exact visible
/// content of an API request or response.
///
/// For example, `output_tokens` will be non-zero, even for an empty string response from
/// Claude.
///
/// Total input tokens in a request is the summation of `input_tokens`,
/// `cache_creation_input_tokens`, and `cache_read_input_tokens`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Usage {
    /// Breakdown of cached tokens by TTL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<CacheCreation>,
    /// The number of input tokens used to create the cache entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<i64>,
    /// The number of input tokens read from the cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i64>,
    /// The geographic region where inference was performed for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    /// The number of input tokens which were used.
    pub input_tokens: i64,
    /// The number of output tokens which were used.
    pub output_tokens: i64,
    /// Breakdown of output tokens by category.
    ///
    /// `output_tokens` remains the inclusive, authoritative total used for billing.
    /// This object provides a read-only decomposition for observability — for example,
    /// how many of the billed output tokens were spent on internal reasoning that may
    /// have been summarized before being returned to you.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// The number of server tool requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUsage>,
    /// If the request used the priority, standard, or batch tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTierServiceTier>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct CacheCreation {
    /// The number of input tokens used to create the 1 hour cache entry.
    #[serde(rename = "ephemeral_1h_input_tokens")]
    pub ephemeral_1_h_input_tokens: i64,
    /// The number of input tokens used to create the 5 minute cache entry.
    #[serde(rename = "ephemeral_5m_input_tokens")]
    pub ephemeral_5_m_input_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct OutputTokensDetails {
    /// Number of output tokens the model generated as internal reasoning, including
    /// the thinking-block delimiter tokens.
    ///
    /// Reflects the raw reasoning the model produced, not the (possibly shorter)
    /// summarized thinking text returned in the response body. Computed by
    /// re-tokenizing the raw reasoning text, so it may differ from the model's exact
    /// generation count by a small number of tokens. Always ≤ `output_tokens`;
    /// `output_tokens - thinking_tokens` approximates the non-reasoning output.
    pub thinking_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct ServerToolUsage {
    /// The number of web fetch tool requests.
    pub web_fetch_requests: i64,
    /// The number of web search tool requests.
    pub web_search_requests: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum ServiceTierServiceTier {
    Batch,
    Priority,
    Standard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct MessageStreamEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[serde(rename = "type")]
    pub message_stream_event_type: MessageStreamEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,
    /// Billing and rate-limit usage.
    ///
    /// Anthropic's API bills and rate-limits by token counts, as tokens represent the underlying
    /// cost to our systems.
    ///
    /// Under the hood, the API transforms requests into a format suitable for the model. The
    /// model's output then goes through a parsing stage before becoming an API response. As a
    /// result, the token counts in `usage` will not match one-to-one with the exact visible
    /// content of an API request or response.
    ///
    /// For example, `output_tokens` will be non-zero, even for an empty string response from
    /// Claude.
    ///
    /// Total input tokens in a request is the summation of `input_tokens`,
    /// `cache_creation_input_tokens`, and `cache_read_input_tokens`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<MessageDeltaUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_block: Option<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct Delta {
    /// Information about the container used in this request.
    ///
    /// This will be non-null if a container tool (e.g. code execution) was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Container>,
    /// Structured information about why model output stopped.
    ///
    /// This is `null` when the `stop_reason` has no additional detail to report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_details: Option<RefusalStopDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_type: Option<DeltaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<Citation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum DeltaType {
    #[serde(rename = "citations_delta")]
    CitationsDelta,
    #[serde(rename = "input_json_delta")]
    InputJsonDelta,
    #[serde(rename = "signature_delta")]
    SignatureDelta,
    #[serde(rename = "text_delta")]
    TextDelta,
    #[serde(rename = "thinking_delta")]
    ThinkingDelta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "anthropic/")]
pub enum MessageStreamEventType {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta,
    #[serde(rename = "content_block_start")]
    ContentBlockStart,
    #[serde(rename = "content_block_stop")]
    ContentBlockStop,
    #[serde(rename = "message_delta")]
    MessageDelta,
    #[serde(rename = "message_start")]
    MessageStart,
    #[serde(rename = "message_stop")]
    MessageStop,
}

/// Billing and rate-limit usage.
///
/// Anthropic's API bills and rate-limits by token counts, as tokens represent the underlying
/// cost to our systems.
///
/// Under the hood, the API transforms requests into a format suitable for the model. The
/// model's output then goes through a parsing stage before becoming an API response. As a
/// result, the token counts in `usage` will not match one-to-one with the exact visible
/// content of an API request or response.
///
/// For example, `output_tokens` will be non-zero, even for an empty string response from
/// Claude.
///
/// Total input tokens in a request is the summation of `input_tokens`,
/// `cache_creation_input_tokens`, and `cache_read_input_tokens`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "anthropic/")]
pub struct MessageDeltaUsage {
    /// The cumulative number of input tokens used to create the cache entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<i64>,
    /// The cumulative number of input tokens read from the cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i64>,
    /// The cumulative number of input tokens which were used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    /// The cumulative number of output tokens which were used.
    pub output_tokens: i64,
    /// Breakdown of output tokens by category.
    ///
    /// `output_tokens` remains the inclusive, authoritative total used for billing.
    /// This object provides a read-only decomposition for observability — for example,
    /// how many of the billed output tokens were spent on internal reasoning that may
    /// have been summarized before being returned to you.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// The number of server tool requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUsage>,
}

// Compatibility aliases for names used by Lingua's hand-written adapters.
pub type MessageRole = PurpleRole;
pub type ResponseLocationCitation = Citation;
