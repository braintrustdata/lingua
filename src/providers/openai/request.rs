/*!
OpenAI chat completion request types.

These types match the OpenAI TypeScript SDK exactly, extracted from the latest version.
All fields and nested types are preserved to ensure full API compatibility.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main chat completion request parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionCreateParams {
    /// A list of messages comprising the conversation
    pub messages: Vec<ChatCompletionMessageParam>,

    /// Model ID used to generate the response
    pub model: String,

    /// Parameters for audio output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionAudioParam>,

    /// Frequency penalty between -2.0 and 2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    /// Deprecated: use tool_choice instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<ChatCompletionFunctionCallOption>,

    /// Deprecated: use tools instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FunctionDefinition>>,

    /// Logit bias modification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f64>>,

    /// Whether to return log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,

    /// Maximum number of completion tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,

    /// Deprecated: use max_completion_tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Request metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Output modalities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<ChatCompletionModality>>,

    /// Number of completion choices to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    /// Whether to enable parallel function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,

    /// Static predicted output content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<ChatCompletionPredictionContent>,

    /// Presence penalty between -2.0 and 2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Prompt cache key for optimization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,

    /// Reasoning effort constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,

    /// Response format specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// Safety identifier for policy monitoring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,

    /// Random seed for deterministic output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,

    /// Service tier selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequences>,

    /// Whether to store the output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,

    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Streaming options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<ChatCompletionStreamOptions>,

    /// Sampling temperature between 0 and 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Tool choice option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,

    /// List of tools the model may call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,

    /// Number of most likely tokens to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,

    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Deprecated: use safety_identifier and prompt_cache_key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Verbosity constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<Verbosity>,

    /// Web search options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_options: Option<WebSearchOptions>,
}

/// Non-streaming chat completion parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionCreateParamsNonStreaming {
    #[serde(flatten)]
    pub base: ChatCompletionCreateParams,

    /// Stream must be false or null for non-streaming
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>, // false or null
}

/// Streaming chat completion parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionCreateParamsStreaming {
    #[serde(flatten)]
    pub base: ChatCompletionCreateParams,

    /// Stream must be true for streaming
    pub stream: bool, // true
}

/// Chat completion message parameters (union type)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum ChatCompletionMessageParam {
    #[serde(rename = "developer")]
    Developer(ChatCompletionDeveloperMessageParam),
    #[serde(rename = "system")]
    System(ChatCompletionSystemMessageParam),
    #[serde(rename = "user")]
    User(ChatCompletionUserMessageParam),
    #[serde(rename = "assistant")]
    Assistant(ChatCompletionAssistantMessageParam),
    #[serde(rename = "tool")]
    Tool(ChatCompletionToolMessageParam),
    #[serde(rename = "function")]
    Function(ChatCompletionFunctionMessageParam),
}

/// Developer message parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionDeveloperMessageParam {
    /// Contents of the developer message
    pub content: MessageContentTextOnly,
    /// Optional name for the participant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// System message parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionSystemMessageParam {
    /// Contents of the system message
    pub content: MessageContentTextOnly,
    /// Optional name for the participant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// User message parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionUserMessageParam {
    /// Contents of the user message
    pub content: MessageContentWithParts,
    /// Optional name for the participant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Assistant message parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAssistantMessageParam {
    /// Data about previous audio response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionAssistantAudio>,
    /// Contents of the assistant message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContentWithRefusal>,
    /// Deprecated function call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<AssistantFunctionCall>,
    /// Optional name for the participant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Refusal message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// Tool calls generated by the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCall>>,
}

/// Tool message parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionToolMessageParam {
    /// Contents of the tool message
    pub content: MessageContentTextOnly,
    /// Tool call ID this message responds to
    pub tool_call_id: String,
}

/// Function message parameters (deprecated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionFunctionMessageParam {
    /// Contents of the function message
    pub content: Option<String>,
    /// Name of the function
    pub name: String,
}

/// Message content variants for different message types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContentTextOnly {
    String(String),
    Parts(Vec<ChatCompletionContentPartText>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContentWithParts {
    String(String),
    Parts(Vec<ChatCompletionContentPart>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContentWithRefusal {
    String(String),
    Parts(Vec<ChatCompletionContentPartTextOrRefusal>),
}

/// Content part types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatCompletionContentPart {
    #[serde(rename = "text")]
    Text(ChatCompletionContentPartText),
    #[serde(rename = "image_url")]
    Image(ChatCompletionContentPartImage),
    #[serde(rename = "input_audio")]
    InputAudio(ChatCompletionContentPartInputAudio),
    #[serde(rename = "file")]
    File(ChatCompletionContentPartFile),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatCompletionContentPartTextOrRefusal {
    #[serde(rename = "text")]
    Text(ChatCompletionContentPartText),
    #[serde(rename = "refusal")]
    Refusal(ChatCompletionContentPartRefusal),
}

/// Text content part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionContentPartText {
    /// The text content
    pub text: String,
}

/// Image content part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionContentPartImage {
    /// Image URL information
    pub image_url: ImageUrl,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageUrl {
    /// URL or base64 encoded image data
    pub url: String,
    /// Detail level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetail>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Auto,
    Low,
    High,
}

/// Input audio content part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionContentPartInputAudio {
    /// Input audio information
    pub input_audio: InputAudio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputAudio {
    /// Base64 encoded audio data
    pub data: String,
    /// Audio format
    pub format: AudioFormat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Wav,
    Mp3,
}

/// File content part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionContentPartFile {
    /// File information
    pub file: FileReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileReference {
    /// Base64 encoded file data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    /// File ID of uploaded file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// Filename when passing as string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Refusal content part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionContentPartRefusal {
    /// The refusal message
    pub refusal: String,
}

/// Audio parameters for output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAudioParam {
    /// Output audio format
    pub format: OutputAudioFormat,
    /// Voice to use
    pub voice: Voice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputAudioFormat {
    Wav,
    Aac,
    Mp3,
    Flac,
    Opus,
    Pcm16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Voice {
    Alloy,
    Ash,
    Ballad,
    Coral,
    Echo,
    Sage,
    Shimmer,
    Verse,
}

/// Assistant audio information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAssistantAudio {
    /// Audio response ID
    pub id: String,
}

/// Assistant function call (deprecated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantFunctionCall {
    /// Function arguments
    pub arguments: String,
    /// Function name
    pub name: String,
}

/// Tool call from the model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatCompletionMessageToolCall {
    #[serde(rename = "function")]
    Function(ChatCompletionMessageFunctionToolCall),
    #[serde(rename = "custom")]
    Custom(ChatCompletionMessageCustomToolCall),
}

/// Function tool call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageFunctionToolCall {
    /// Tool call ID
    pub id: String,
    /// Function information
    pub function: FunctionCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function arguments in JSON format
    pub arguments: String,
    /// Function name
    pub name: String,
}

/// Custom tool call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageCustomToolCall {
    /// Tool call ID
    pub id: String,
    /// Custom tool information
    pub custom: CustomToolCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomToolCall {
    /// Input for the custom tool
    pub input: String,
    /// Name of the custom tool
    pub name: String,
}

/// Function call option (deprecated)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionFunctionCallOption {
    String(String), // "none" or "auto"
    Named(NamedFunctionCall),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedFunctionCall {
    /// Function name to call
    pub name: String,
}

/// Function definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,
    /// Function description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Function parameters as JSON Schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Whether to enforce strict schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Chat completion modality
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatCompletionModality {
    Text,
    Audio,
}

/// Prediction content for static output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionPredictionContent {
    /// Content to match
    pub content: MessageContentTextOnly,
}

/// Reasoning effort levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
}

/// Response format options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema(ResponseFormatJsonSchema),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFormatJsonSchema {
    /// JSON schema definition
    pub json_schema: JsonSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchema {
    /// Schema name
    pub name: String,
    /// Schema description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The JSON schema
    pub schema: serde_json::Value,
    /// Whether to enforce strict adherence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Service tier options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}

/// Stop sequences
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopSequences {
    String(String),
    Array(Vec<String>),
}

/// Streaming options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionStreamOptions {
    /// Whether to include obfuscation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
    /// Whether to include usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

/// Tool choice options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionToolChoiceOption {
    String(String), // "none", "auto", "required"
    AllowedTools(ChatCompletionAllowedToolChoice),
    Named(ChatCompletionNamedToolChoice),
    Custom(ChatCompletionNamedToolChoiceCustom),
}

/// Allowed tool choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAllowedToolChoice {
    /// Allowed tools configuration
    pub allowed_tools: ChatCompletionAllowedTools,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionAllowedTools {
    /// Mode for allowed tools
    pub mode: AllowedToolsMode,
    /// List of allowed tools
    pub tools: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AllowedToolsMode {
    Auto,
    Required,
}

/// Named tool choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionNamedToolChoice {
    /// Function information
    pub function: NamedToolChoiceFunction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedToolChoiceFunction {
    /// Function name
    pub name: String,
}

/// Named custom tool choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionNamedToolChoiceCustom {
    /// Custom tool information
    pub custom: NamedToolChoiceCustom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedToolChoiceCustom {
    /// Custom tool name
    pub name: String,
}

/// Tool definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatCompletionTool {
    #[serde(rename = "function")]
    Function(ChatCompletionFunctionTool),
    #[serde(rename = "custom")]
    Custom(ChatCompletionCustomTool),
}

/// Function tool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionFunctionTool {
    /// Function definition
    pub function: FunctionDefinition,
}

/// Custom tool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionCustomTool {
    /// Custom tool properties
    pub custom: CustomToolDefinition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<CustomToolInputFormat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CustomToolInputFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "grammar")]
    Grammar(GrammarFormat),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrammarFormat {
    /// Grammar definition
    pub grammar: GrammarDefinition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrammarDefinition {
    /// Grammar definition string
    pub definition: String,
    /// Grammar syntax
    pub syntax: GrammarSyntax,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrammarSyntax {
    Lark,
    Regex,
}

/// Verbosity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    Low,
    Medium,
    High,
}

/// Web search options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchOptions {
    /// Search context size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<SearchContextSize>,
    /// User location for search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<UserLocation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchContextSize {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserLocation {
    /// Approximate location
    pub approximate: ApproximateLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproximateLocation {
    /// City
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// Country code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Timezone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}
