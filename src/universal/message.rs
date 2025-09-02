use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::citation::Citation;
use super::provider::ProviderMessagePartConfig;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    User {
        content: Vec<UserContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Assistant {
        content: Vec<AssistantContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Tool {
        content: Vec<ToolContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
}

// User can send: text, images, documents, audio
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContentPart {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        citations: Option<Vec<Citation>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Image {
        data: FileData,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        detail: Option<ImageDetail>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Document {
        data: FileData,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        citations: Option<Vec<Citation>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Audio {
        data: FileData,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        transcript: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
}

// Assistant can respond with: text, images, tool calls, thinking, search results, refusals
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContentPart {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        citations: Option<Vec<Citation>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Image {
        data: FileData,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        detail: Option<ImageDetail>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    ToolUse {
        id: String,
        name: String,
        #[ts(type = "any")]
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Thinking {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    RedactedThinking {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    SearchResult {
        query: String,
        results: Vec<SearchResultItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    WebSearchResult {
        query: String,
        results: Vec<WebSearchResultItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    ExecutableCode {
        language: String,
        code: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
    Refusal {
        refusal: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
}

// Tool messages contain only tool results
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContentPart {
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        content: Option<Vec<ToolResultContent>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        provider_config: Option<ProviderMessagePartConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ImageDetail {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContent {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        citations: Option<Vec<Citation>>,
    },
    Image {
        data: FileData,
    },
    Json {
        #[ts(type = "any")]
        json: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct SearchResultItem {
    pub title: Option<String>,
    pub url: Option<String>,
    pub content: Option<String>,
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<Citation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct WebSearchResultItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<Citation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileData {
    Url(String),
    Base64(Base64Data),
    FileId(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Base64Data {
    pub mime_type: String,
    pub data: Vec<u8>,
}
