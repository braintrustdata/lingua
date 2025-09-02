use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProviderMessagePartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<AnthropicMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai: Option<ExtraMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<ExtraMessagePartConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrock: Option<ExtraMessagePartConfig>,
    /// Other providers by name
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, any>")]
    pub other: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AnthropicMessagePartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "any")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExtraMessagePartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "any")]
    pub extra: Option<serde_json::Value>,
}

// Cache breakpoints are only used by Anthropic models.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CacheControlEphemeral {
    Ephemeral {
        #[serde(skip_serializing_if = "Option::is_none")]
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
