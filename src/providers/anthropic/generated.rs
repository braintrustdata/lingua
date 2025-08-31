// Generated Anthropic types from unofficial OpenAPI spec
// Essential types for Elmir Anthropic messages integration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestTextBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<serde_json::Value>,
    pub text: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputContentBlock {
    #[serde(rename = "document")]
    Document(RequestDocumentBlock),
    #[serde(rename = "image")]
    Image(RequestImageBlock),
    #[serde(rename = "redacted_thinking")]
    RedactedThinking(RequestRedactedThinkingBlock),
    #[serde(rename = "search_result")]
    SearchResult(RequestSearchResultBlock),
    #[serde(rename = "server_tool_use")]
    ServerToolUse(RequestServerToolUseBlock),
    #[serde(rename = "text")]
    Text(RequestTextBlock),
    #[serde(rename = "thinking")]
    Thinking(RequestThinkingBlock),
    #[serde(rename = "tool_result")]
    ToolResult(RequestToolResultBlock),
    #[serde(rename = "tool_use")]
    ToolUse(RequestToolUseBlock),
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult(RequestWebSearchToolResultBlock),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCharLocationCitation {
    pub cited_text: String,
    pub document_index: i64,
    pub document_title: serde_json::Value,
    pub end_char_index: i64,
    pub file_id: serde_json::Value,
    pub start_char_index: i64,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Base64ImageSource {
    pub data: String,
    pub media_type: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTextBlock {
    pub citations: serde_json::Value,
    pub text: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestCharLocationCitation {
    pub cited_text: String,
    pub document_index: i64,
    pub document_title: serde_json::Value,
    pub end_char_index: i64,
    pub start_char_index: i64,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolChoiceTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "redacted_thinking")]
    RedactedThinking(ResponseRedactedThinkingBlock),
    #[serde(rename = "server_tool_use")]
    ServerToolUse(ResponseServerToolUseBlock),
    #[serde(rename = "text")]
    Text(ResponseTextBlock),
    #[serde(rename = "thinking")]
    Thinking(ResponseThinkingBlock),
    #[serde(rename = "tool_use")]
    ToolUse(ResponseToolUseBlock),
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult(ResponseWebSearchToolResultBlock),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseSearchResultLocationCitation {
    pub cited_text: String,
    pub end_block_index: i64,
    pub search_result_index: i64,
    pub source: String,
    pub start_block_index: i64,
    pub title: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchToolResultErrorCode {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchToolResultError {
    pub error_code: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestToolResultBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolChoiceNone {
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub r#type: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingConfigDisabled {
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponsePageLocationCitation {
    pub cited_text: String,
    pub document_index: i64,
    pub document_title: serde_json::Value,
    pub end_page_number: i64,
    pub file_id: serde_json::Value,
    pub start_page_number: i64,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolChoiceAuto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub max_tokens_to_sample: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub model: serde_json::Value,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct APIError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSearchTool_20250305 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<serde_json::Value>,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEditor_20250728 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters: Option<serde_json::Value>,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheControlEphemeral {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestServerToolUseBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    pub id: String,
    pub input: serde_json::Value,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentBlockSource {
    pub content: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestContentBlockLocationCitation {
    pub cited_text: String,
    pub document_index: i64,
    pub document_title: serde_json::Value,
    pub end_block_index: i64,
    pub start_block_index: i64,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestSearchResultBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<serde_json::Value>,
    pub content: Vec<serde_json::Value>,
    pub source: String,
    pub title: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestRedactedThinkingBlock {
    pub data: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEditor_20250124 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseThinkingBlock {
    pub signature: String,
    pub thinking: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseServerToolUseBlock {
    pub id: String,
    pub input: serde_json::Value,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchResultBlock {
    pub encrypted_content: String,
    pub page_age: serde_json::Value,
    pub title: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestDocumentBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    pub source: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchToolResultBlock {
    pub content: serde_json::Value,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct URLImageSource {
    #[serde(rename = "type")]
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateMessageParams {
    pub max_tokens: i64,
    pub messages: Vec<InputMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub model: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestWebSearchToolResultError {
    pub error_code: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchResultLocationCitation {
    pub cited_text: String,
    pub encrypted_index: String,
    pub title: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Base64PDFSource {
    pub data: String,
    pub media_type: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestToolUseBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    pub id: String,
    pub input: serde_json::Value,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestPageLocationCitation {
    pub cited_text: String,
    pub document_index: i64,
    pub document_title: serde_json::Value,
    pub end_page_number: i64,
    pub start_page_number: i64,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthenticationError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseToolUseBlock {
    pub id: String,
    pub input: serde_json::Value,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheCreation {
    pub ephemeral_1h_input_tokens: i64,
    pub ephemeral_5m_input_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub content: Vec<ContentBlock>,
    pub id: String,
    pub model: serde_json::Value,
    pub role: String,
    pub stop_reason: serde_json::Value,
    pub stop_sequence: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
    pub usage: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct URLPDFSource {
    #[serde(rename = "type")]
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BillingError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestImageBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    pub source: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMessage {
    pub content: Vec<InputContentBlock>,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestWebSearchResultLocationCitation {
    pub cited_text: String,
    pub encrypted_index: String,
    pub title: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestThinkingBlock {
    pub signature: String,
    pub thinking: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainTextSource {
    pub data: String,
    pub media_type: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub cache_creation: serde_json::Value,
    pub cache_creation_input_tokens: serde_json::Value,
    pub cache_read_input_tokens: serde_json::Value,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub server_tool_use: serde_json::Value,
    pub service_tier: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolChoiceAny {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvalidRequestError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverloadedError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopReason {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseRedactedThinkingBlock {
    pub data: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestCitationsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestSearchResultLocationCitation {
    pub cited_text: String,
    pub end_block_index: i64,
    pub search_result_index: i64,
    pub source: String,
    pub start_block_index: i64,
    pub title: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotFoundError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEditor_20250429 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestWebSearchResultBlock {
    pub encrypted_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_age: Option<serde_json::Value>,
    pub title: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub completion: String,
    pub id: String,
    pub model: serde_json::Value,
    pub stop_reason: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThinkingConfigParam {
    #[serde(rename = "disabled")]
    Disabled(ThinkingConfigDisabled),
    #[serde(rename = "enabled")]
    Enabled(ThinkingConfigEnabled),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolChoice {
    #[serde(rename = "any")]
    Any(ToolChoiceAny),
    #[serde(rename = "auto")]
    Auto(ToolChoiceAuto),
    #[serde(rename = "none")]
    None(ToolChoiceNone),
    #[serde(rename = "tool")]
    Tool(ToolChoiceTool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingConfigEnabled {
    pub budget_tokens: i64,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayTimeoutError {
    pub message: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BashTool_20250124 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseContentBlockLocationCitation {
    pub cited_text: String,
    pub document_index: i64,
    pub document_title: serde_json::Value,
    pub end_block_index: i64,
    pub file_id: serde_json::Value,
    pub start_block_index: i64,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestWebSearchToolResultBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    pub content: serde_json::Value,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerToolUsage {
    pub web_search_requests: i64,
}
