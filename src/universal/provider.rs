use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct ProviderMessagePartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<AnthropicMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai: Option<ExtraMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<ExtraMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<ExtraMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, Record<string, unknown>>")]
    pub other: Option<BTreeMap<String, BTreeMap<String, serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct AnthropicMessagePartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub extra: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct ExtraMessagePartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub extra: Option<serde_json::Value>,
}

// Cache breakpoints are only used by Anthropic models.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CacheControlEphemeral {
    Ephemeral {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        ttl: Option<CacheTtl>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum CacheTtl {
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "1h")]
    OneHour,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct ProviderFileMessagePartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<AnthropicFileMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai: Option<OpenAIFileMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<ExtraMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<ExtraMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, Record<string, unknown>>")]
    pub other: Option<BTreeMap<String, BTreeMap<String, serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct AnthropicFileMessagePartConfig {
    pub cache_control: Option<CacheControlEphemeral>,

    // These fields apply to documents
    pub citations: Option<AnthropicCitationsConfig>,
    pub context: Option<String>,
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub extra: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct AnthropicCitationsConfig {
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, optional_fields)]
pub struct OpenAIFileMessagePartConfig {
    filename: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<OpenAIImageDetail>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub extra: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum OpenAIImageDetail {
    Low,
    Medium,
    High,
}
