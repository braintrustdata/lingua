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

pub fn model_requires_responses_api(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.starts_with("o1-pro")
        || lower.starts_with("o3-pro")
        || lower.starts_with("gpt-5-pro")
        || (lower.starts_with("gpt-5") && lower.contains("-codex"))
}

impl ModelSpec {
    pub fn requires_responses_api(&self) -> bool {
        self.flavor == ModelFlavor::Responses || model_requires_responses_api(&self.model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_requires_responses_api_detects_required_families() {
        let required = [
            "o1-pro",
            "o3-pro",
            "gpt-5-pro",
            "gpt-5-pro-2025-10-06",
            "gpt-5-codex",
            "gpt-5.1-codex",
            "gpt-5.1-codex-mini",
        ];
        for model in required {
            assert!(
                model_requires_responses_api(model),
                "expected Responses-required model: {model}"
            );
        }
    }

    #[test]
    fn model_requires_responses_api_rejects_non_required_families() {
        let not_required = ["gpt-5-mini", "gpt-5", "gpt-4o", "claude-sonnet-4"];
        for model in not_required {
            assert!(
                !model_requires_responses_api(model),
                "expected non-Responses model: {model}"
            );
        }
    }

    #[test]
    fn model_spec_requires_responses_api_allows_flavor_override() {
        let spec = ModelSpec {
            model: "custom-model".to_string(),
            format: ProviderFormat::ChatCompletions,
            flavor: ModelFlavor::Responses,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: serde_json::Map::new(),
        };
        assert!(spec.requires_responses_api());
    }
}
