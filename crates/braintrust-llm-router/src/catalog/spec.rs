use lingua::ProviderFormat;
use serde::{Deserialize, Serialize};

/// The API flavor/style a model uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFlavor {
    Chat,
    Completion,
    Embedding,
    Realtime,
    Live,
    /// Models using OpenAI's Responses API (e.g., o1-pro, o3-pro, gpt-5-pro, gpt-5-codex)
    Responses,
    /// Evaluation/judge models (e.g., TypeSafe's jev-*)
    Evaluation,
    /// Any flavor value the router does not yet model. Keeps catalog parsing
    /// resilient when the upstream `model_list.json` introduces a new flavor
    /// before the router adds first-class support, instead of failing the
    /// entire catalog parse.
    #[serde(other)]
    Unknown,
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
    #[serde(default)]
    pub available_providers: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn model_requires_responses_api(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    let normalized = lower.strip_prefix("openai.").unwrap_or(lower.as_str());
    let parse_version_component = |component: &str| {
        let digit_count = component.bytes().take_while(u8::is_ascii_digit).count();
        if digit_count == 0 {
            return None;
        }
        component[..digit_count].parse::<u32>().ok()
    };
    let gpt_version = normalized.strip_prefix("gpt-").and_then(|version| {
        let (major, minor) = version
            .split_once('.')
            .map_or((version, None), |(major, minor)| (major, Some(minor)));
        Some((
            parse_version_component(major)?,
            minor.and_then(parse_version_component),
        ))
    });
    normalized.starts_with("o1-pro")
        || normalized.starts_with("o3-pro")
        || normalized.starts_with("gpt-5-pro")
        || gpt_version.is_some_and(|(major, minor)| {
            major > 5 || (major == 5 && minor.is_some_and(|minor| minor >= 3))
        })
        || (normalized.starts_with("gpt-5") && normalized.contains("-codex"))
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
            "gpt-5.3",
            "gpt-5.3-chat-latest",
            "gpt-5.4",
            "gpt-5.5-chat-latest",
            "gpt-5-codex",
            "gpt-5.1-codex",
            "gpt-5.1-codex-mini",
            "gpt-6-astra",
            "gpt-7",
            "gpt-10.2-preview",
            "openai.gpt-5.4",
            "openai.gpt-5.5",
            "openai.gpt-6-astra",
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
        let not_required = [
            "gpt-5-mini",
            "gpt-5",
            "gpt-5.1",
            "gpt-5.2-chat-latest",
            "gpt-4o",
            "gpt-next",
            "claude-sonnet-4",
            "openai.gpt-oss-120b",
            "openai.gpt-oss-safeguard-120b",
        ];
        for model in not_required {
            assert!(
                !model_requires_responses_api(model),
                "expected non-Responses model: {model}"
            );
        }
    }

    #[test]
    fn model_requires_responses_api_applies_to_current_and_future_versions() {
        assert!(!model_requires_responses_api("gpt-5.2"));
        assert!(model_requires_responses_api("gpt-5.3"));
        assert!(model_requires_responses_api("gpt-5.10-preview"));
        assert!(model_requires_responses_api("gpt-6-astra"));
        assert!(model_requires_responses_api("gpt-10-preview"));
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
            available_providers: vec![],
        };
        assert!(spec.requires_responses_api());
    }
}
