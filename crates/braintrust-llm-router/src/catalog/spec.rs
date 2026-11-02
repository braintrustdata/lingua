use lingua::ProviderFormat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub input_cache_write_cost_per_mil_tokens: Option<f64>,
    #[serde(default)]
    pub input_cache_write_5m_cost_per_mil_tokens: Option<f64>,
    #[serde(default)]
    pub input_cache_write_1h_cost_per_mil_tokens: Option<f64>,
    /// Verified tier prices keyed by resolved provider. Top-level price fields
    /// hold standard prices.
    #[serde(default)]
    pub pricing: Option<HashMap<String, TierPricing>>,
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

/// Rates per pricing tier for one provider. Tier names are pricing categories,
/// not provider API parameter values. A missing tier has no verified support
/// and does not fall back to standard prices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TierPricing {
    pub flex: Option<Vec<PricingRate>>,
    pub batch: Option<Vec<PricingRate>>,
    pub priority: Option<Vec<PricingRate>>,
}

/// Rates that apply only to the listed endpoints, regions, deployment, model
/// version, prompt length, and inclusive UTC date bounds. A missing price is
/// unknown, not zero.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PricingRate {
    pub input_cost_per_mil_tokens: Option<f64>,
    pub output_cost_per_mil_tokens: Option<f64>,
    pub input_cache_read_cost_per_mil_tokens: Option<f64>,
    pub input_cache_write_cost_per_mil_tokens: Option<f64>,
    pub input_cache_write_5m_cost_per_mil_tokens: Option<f64>,
    pub input_cache_write_1h_cost_per_mil_tokens: Option<f64>,
    pub endpoints: Option<Vec<String>>,
    pub supported_regions: Option<Vec<String>>,
    pub deployment_type: Option<String>,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    /// Inclusive lower bound on prompt length for these rates.
    pub min_input_tokens: Option<u32>,
    /// Inclusive upper bound on prompt length for these rates. This is not the
    /// model context window.
    pub max_input_tokens: Option<u32>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
}

fn default_true() -> bool {
    true
}

fn model_requires_responses_api(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    let normalized = lower
        .strip_prefix("@openai/")
        .or_else(|| lower.strip_prefix("braintrust/"))
        .or_else(|| lower.strip_prefix("openai."))
        .unwrap_or(lower.as_str());
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
    fn model_spec_parses_tier_pricing_and_ignores_unknown_tiers() {
        let spec: ModelSpec = serde_json::from_str(
            r#"{
                "model": "gpt-5.4",
                "format": "openai",
                "flavor": "chat",
                "input_cache_write_cost_per_mil_tokens": 3.0,
                "input_cache_write_5m_cost_per_mil_tokens": 3.5,
                "input_cache_write_1h_cost_per_mil_tokens": 5.0,
                "pricing": {
                    "openai": {
                        "flex": [
                            {
                                "input_cost_per_mil_tokens": 1.25,
                                "output_cost_per_mil_tokens": 7.5,
                                "input_cache_read_cost_per_mil_tokens": 0.125,
                                "input_cache_write_cost_per_mil_tokens": 1.5,
                                "input_cache_write_5m_cost_per_mil_tokens": 1.75,
                                "input_cache_write_1h_cost_per_mil_tokens": 2.5,
                                "endpoints": ["api.openai.com"],
                                "supported_regions": ["us-east-1"],
                                "deployment_type": "global",
                                "model_id": "gpt-5.4",
                                "model_version": "2026-03-05",
                                "min_input_tokens": 0,
                                "max_input_tokens": 272000,
                                "valid_from": "2026-03-05",
                                "valid_until": "2026-12-31"
                            },
                            {"min_input_tokens": 272001}
                        ],
                        "future_tier": {"unknown_field": true}
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(spec.input_cache_write_cost_per_mil_tokens, Some(3.0));
        assert_eq!(spec.input_cache_write_5m_cost_per_mil_tokens, Some(3.5));
        assert_eq!(spec.input_cache_write_1h_cost_per_mil_tokens, Some(5.0));
        assert_eq!(
            spec.pricing,
            Some(HashMap::from([(
                "openai".to_string(),
                TierPricing {
                    flex: Some(vec![
                        PricingRate {
                            input_cost_per_mil_tokens: Some(1.25),
                            output_cost_per_mil_tokens: Some(7.5),
                            input_cache_read_cost_per_mil_tokens: Some(0.125),
                            input_cache_write_cost_per_mil_tokens: Some(1.5),
                            input_cache_write_5m_cost_per_mil_tokens: Some(1.75),
                            input_cache_write_1h_cost_per_mil_tokens: Some(2.5),
                            endpoints: Some(vec!["api.openai.com".to_string()]),
                            supported_regions: Some(vec!["us-east-1".to_string()]),
                            deployment_type: Some("global".to_string()),
                            model_id: Some("gpt-5.4".to_string()),
                            model_version: Some("2026-03-05".to_string()),
                            min_input_tokens: Some(0),
                            max_input_tokens: Some(272000),
                            valid_from: Some("2026-03-05".to_string()),
                            valid_until: Some("2026-12-31".to_string()),
                        },
                        PricingRate {
                            min_input_tokens: Some(272001),
                            ..PricingRate::default()
                        },
                    ]),
                    ..TierPricing::default()
                },
            )]))
        );
    }

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
            "@openai/gpt-5.6-luna",
            "braintrust/gpt-5.6-luna",
            "braintrust/gpt-6-luna",
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
            "braintrust/gpt-5.2",
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
            input_cache_write_cost_per_mil_tokens: None,
            input_cache_write_5m_cost_per_mil_tokens: None,
            input_cache_write_1h_cost_per_mil_tokens: None,
            pricing: None,
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
