use lingua::ProviderFormat;
use serde::{Deserialize, Serialize};

/// The API flavor/style a model uses.
///
/// Note: The `Responses` variant must be kept in sync with lingua's
/// `requires_responses_api` detection in `capabilities.rs`. Models that
/// require the Responses API include: o1-pro*, o3-pro*, gpt-5-pro*, gpt-5-codex*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFlavor {
    Chat,
    Completion,
    Embedding,
    /// Models using OpenAI's Responses API (e.g., o1-pro, o3-pro, gpt-5-pro, gpt-5-codex)
    Responses,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    #[serde(default)]
    pub model: String,
    pub format: ProviderFormat,
    pub flavor: ModelFlavor,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub input_cost_per_mil_tokens: Option<f64>,
    #[serde(default)]
    pub output_cost_per_mil_tokens: Option<f64>,
    #[serde(default)]
    pub input_cache_read_cost_per_mil_tokens: Option<f64>,
    #[serde(default)]
    pub multimodal: Option<bool>,
    #[serde(default)]
    pub reasoning: Option<bool>,
    #[serde(default)]
    pub max_input_tokens: Option<u32>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    #[serde(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}
