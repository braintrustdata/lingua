/*!
OpenAI-specific capability detection used by the transformation pipeline.
*/
use crate::serde_json::{Map, Value};
use crate::universal::ReasoningEffort;

/// Transforms required for specific model families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTransform {
    /// Strip temperature parameter (reasoning models don't support it)
    StripTemperature,
    /// Strip top_p parameter (reasoning models don't support it)
    StripTopP,
    /// Convert max_tokens to max_completion_tokens
    ForceMaxCompletionTokens,
    /// Convert max_completion_tokens to max_tokens
    ForceMaxTokens,
    /// Strip stream_options parameter
    StripStreamOptions,
}

use ModelTransform::*;

/// Model prefixes and their required transforms.
/// Order matters - more specific prefixes must come first.
const MODEL_TRANSFORM_RULES: &[(&str, &[ModelTransform])] = &[
    (
        "o1",
        &[StripTemperature, StripTopP, ForceMaxCompletionTokens],
    ),
    (
        "o3",
        &[StripTemperature, StripTopP, ForceMaxCompletionTokens],
    ),
    (
        "o4",
        &[StripTemperature, StripTopP, ForceMaxCompletionTokens],
    ),
    (
        "gpt-5",
        &[StripTemperature, StripTopP, ForceMaxCompletionTokens],
    ),
    // TODO: would be nice if we could apply these rules by provider instead of model name, and
    // apply these to all Mistral models
    ("mistral", &[ForceMaxTokens]),
    ("magistral", &[ForceMaxTokens]),
    ("codestral", &[ForceMaxTokens]),
    ("pixstral", &[ForceMaxTokens]),
    ("devstral", &[ForceMaxTokens]),
    ("voxstral", &[ForceMaxTokens]),
    // Databricks models
    ("databricks-", &[ForceMaxTokens, StripStreamOptions]),
];

/// Get the transforms required for a model.
pub fn get_model_transforms(model: &str) -> &'static [ModelTransform] {
    let lower = model.to_ascii_lowercase();
    for (prefix, transforms) in MODEL_TRANSFORM_RULES {
        if lower.starts_with(prefix) {
            return transforms;
        }
    }
    &[]
}

/// Check if a model requires any transforms.
pub fn model_needs_transforms(model: &str) -> bool {
    !get_model_transforms(model).is_empty()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffortFamily {
    NoneLowMediumHighXhigh,
    LowMediumHighXhigh,
    NoneLowMediumHigh,
    LowMediumHigh,
    MinimalLowMediumHigh,
}

impl EffortFamily {
    fn contains(self, effort: ReasoningEffort) -> bool {
        match self {
            EffortFamily::NoneLowMediumHighXhigh => matches!(
                effort,
                ReasoningEffort::None
                    | ReasoningEffort::Low
                    | ReasoningEffort::Medium
                    | ReasoningEffort::High
                    | ReasoningEffort::Xhigh
            ),
            EffortFamily::LowMediumHighXhigh => matches!(
                effort,
                ReasoningEffort::Low
                    | ReasoningEffort::Medium
                    | ReasoningEffort::High
                    | ReasoningEffort::Xhigh
            ),
            EffortFamily::NoneLowMediumHigh => matches!(
                effort,
                ReasoningEffort::None
                    | ReasoningEffort::Low
                    | ReasoningEffort::Medium
                    | ReasoningEffort::High
            ),
            EffortFamily::LowMediumHigh => matches!(
                effort,
                ReasoningEffort::Low | ReasoningEffort::Medium | ReasoningEffort::High
            ),
            EffortFamily::MinimalLowMediumHigh => matches!(
                effort,
                ReasoningEffort::Minimal
                    | ReasoningEffort::Low
                    | ReasoningEffort::Medium
                    | ReasoningEffort::High
            ),
        }
    }
}

fn normalize_openai_model_name(model: &str) -> String {
    let lower = model.to_ascii_lowercase();
    if let Some(stripped) = lower.strip_prefix("openai/") {
        stripped.to_string()
    } else {
        lower
    }
}

fn reasoning_effort_family_for_model(model: &str) -> Option<EffortFamily> {
    let model = normalize_openai_model_name(model);

    if model.starts_with("gpt-5.4") || model.starts_with("gpt-5.2") {
        if model.starts_with("gpt-5.2-codex") {
            Some(EffortFamily::LowMediumHighXhigh)
        } else {
            Some(EffortFamily::NoneLowMediumHighXhigh)
        }
    } else if model.starts_with("gpt-5.3-codex") {
        Some(EffortFamily::LowMediumHighXhigh)
    } else if model.starts_with("gpt-5.1-codex") {
        Some(EffortFamily::LowMediumHigh)
    } else if model.starts_with("gpt-5.1") {
        Some(EffortFamily::NoneLowMediumHigh)
    } else if model.starts_with("gpt-5-nano")
        || model.starts_with("gpt-5-mini")
        || model == "gpt-5"
        || model.starts_with("gpt-5-")
    {
        Some(EffortFamily::MinimalLowMediumHigh)
    } else {
        None
    }
}

/// Clamp a synthesized OpenAI reasoning effort to the values supported by the target model.
///
/// Same-provider raw OpenAI extras bypass this helper so exact user-supplied OpenAI
/// payloads can be preserved. This is for cross-provider or canonical emission.
pub fn clamp_reasoning_effort_for_model(model: &str, effort: ReasoningEffort) -> ReasoningEffort {
    let Some(family) = reasoning_effort_family_for_model(model) else {
        return effort;
    };

    if family.contains(effort) {
        return effort;
    }

    match family {
        EffortFamily::NoneLowMediumHighXhigh => match effort {
            ReasoningEffort::Minimal => ReasoningEffort::Low,
            _ => effort,
        },
        EffortFamily::LowMediumHighXhigh => match effort {
            ReasoningEffort::None | ReasoningEffort::Minimal => ReasoningEffort::Low,
            _ => effort,
        },
        EffortFamily::NoneLowMediumHigh => match effort {
            ReasoningEffort::Minimal => ReasoningEffort::Low,
            ReasoningEffort::Xhigh => ReasoningEffort::High,
            _ => effort,
        },
        EffortFamily::LowMediumHigh => match effort {
            ReasoningEffort::None | ReasoningEffort::Minimal => ReasoningEffort::Low,
            ReasoningEffort::Xhigh => ReasoningEffort::High,
            _ => effort,
        },
        EffortFamily::MinimalLowMediumHigh => match effort {
            ReasoningEffort::None => ReasoningEffort::Minimal,
            ReasoningEffort::Xhigh => ReasoningEffort::High,
            _ => effort,
        },
    }
}

/// Apply all transforms for a model to a request object.
pub fn apply_model_transforms(model: &str, obj: &mut Map<String, Value>) {
    for transform in get_model_transforms(model) {
        match transform {
            StripTemperature => {
                obj.remove("temperature");
            }
            StripTopP => {
                obj.remove("top_p");
            }
            ForceMaxCompletionTokens => {
                // (Responses API) max_output_tokens is valid.
                if obj.contains_key("max_output_tokens") {
                    continue;
                }

                // (Chat Completions API) max_tokens is deprecated - convert to max_completion_tokens.
                if let Some(max_tokens) = obj.remove("max_tokens") {
                    obj.entry("max_completion_tokens").or_insert(max_tokens);
                }
            }
            ForceMaxTokens => {
                // Mistral does not support max_completion_tokens yet, use max_tokens instead
                if let Some(max_tokens) = obj.remove("max_completion_tokens") {
                    obj.entry("max_tokens").or_insert(max_tokens);
                }
            }
            StripStreamOptions => {
                obj.remove("stream_options");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::{self, json};

    #[test]
    fn test_get_model_transforms() {
        let cases = [
            (
                "o1",
                &[StripTemperature, StripTopP, ForceMaxCompletionTokens][..],
            ),
            (
                "o1-mini",
                &[StripTemperature, StripTopP, ForceMaxCompletionTokens][..],
            ),
            (
                "o3",
                &[StripTemperature, StripTopP, ForceMaxCompletionTokens][..],
            ),
            (
                "o4-preview",
                &[StripTemperature, StripTopP, ForceMaxCompletionTokens][..],
            ),
            (
                "gpt-5-mini",
                &[StripTemperature, StripTopP, ForceMaxCompletionTokens][..],
            ),
            ("gpt-4", &[][..]),
            ("gpt-4o", &[][..]),
            ("claude-3", &[][..]),
        ];
        for (model, expected) in cases {
            assert_eq!(get_model_transforms(model), expected, "model: {}", model);
        }
    }

    #[test]
    fn test_model_needs_transforms() {
        let needs = ["o1", "o3", "gpt-5"];
        let no_needs = ["gpt-4", "gpt-4o", "claude-3"];
        for model in needs {
            assert!(model_needs_transforms(model), "should need: {}", model);
        }
        for model in no_needs {
            assert!(!model_needs_transforms(model), "should not need: {}", model);
        }
    }

    #[test]
    fn test_clamp_reasoning_effort_for_model() {
        let cases = [
            (
                "openai/gpt-5.4",
                ReasoningEffort::Minimal,
                ReasoningEffort::Low,
            ),
            ("gpt-5.4", ReasoningEffort::Xhigh, ReasoningEffort::Xhigh),
            ("gpt-5.3-codex", ReasoningEffort::None, ReasoningEffort::Low),
            (
                "gpt-5.2-codex",
                ReasoningEffort::Xhigh,
                ReasoningEffort::Xhigh,
            ),
            ("gpt-5.2", ReasoningEffort::None, ReasoningEffort::None),
            ("gpt-5.1", ReasoningEffort::Xhigh, ReasoningEffort::High),
            (
                "gpt-5.1-codex",
                ReasoningEffort::Minimal,
                ReasoningEffort::Low,
            ),
            ("gpt-5", ReasoningEffort::None, ReasoningEffort::Minimal),
            ("gpt-5-mini", ReasoningEffort::Xhigh, ReasoningEffort::High),
            (
                "gpt-5-nano",
                ReasoningEffort::Minimal,
                ReasoningEffort::Minimal,
            ),
            ("gpt-4o", ReasoningEffort::Xhigh, ReasoningEffort::Xhigh),
        ];

        for (model, effort, expected) in cases {
            assert_eq!(
                clamp_reasoning_effort_for_model(model, effort),
                expected,
                "model: {}",
                model
            );
        }
    }

    #[test]
    fn test_strip_temperature() {
        let reasoning_models = ["o1", "o1-mini", "o3", "gpt-5-mini"];
        let non_reasoning_models = ["gpt-4", "gpt-4o", "claude-3"];

        // Reasoning models: temperature should be stripped
        for model in reasoning_models {
            let mut obj: Map<String, Value> = serde_json::from_value(json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "temperature": 0.7
            }))
            .unwrap();
            apply_model_transforms(model, &mut obj);
            assert!(
                !obj.contains_key("temperature"),
                "{} should strip temperature",
                model
            );
        }

        // Non-reasoning models: temperature should be preserved
        for model in non_reasoning_models {
            let mut obj: Map<String, Value> = serde_json::from_value(json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "temperature": 0.7
            }))
            .unwrap();
            apply_model_transforms(model, &mut obj);
            assert!(
                obj.contains_key("temperature"),
                "{} should preserve temperature",
                model
            );
        }
    }

    #[test]
    fn test_strip_top_p() {
        let reasoning_models = ["o1", "o1-mini", "o3", "gpt-5-mini"];
        let non_reasoning_models = ["gpt-4", "gpt-4o", "claude-3"];

        for model in reasoning_models {
            let mut obj: Map<String, Value> = serde_json::from_value(json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "top_p": 0.9
            }))
            .unwrap();
            apply_model_transforms(model, &mut obj);
            assert!(!obj.contains_key("top_p"), "{} should strip top_p", model);
        }

        for model in non_reasoning_models {
            let mut obj: Map<String, Value> = serde_json::from_value(json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "top_p": 0.9
            }))
            .unwrap();
            apply_model_transforms(model, &mut obj);
            assert!(obj.contains_key("top_p"), "{} should preserve top_p", model);
        }
    }

    #[test]
    fn test_force_max_completion_tokens() {
        // Reasoning models: max_tokens → max_completion_tokens
        for model in ["o1", "gpt-5"] {
            let mut obj = json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "max_tokens": 100
            })
            .as_object()
            .unwrap()
            .clone();
            apply_model_transforms(model, &mut obj);
            assert!(
                obj.contains_key("max_completion_tokens"),
                "{} should add max_completion_tokens",
                model
            );
            assert!(
                !obj.contains_key("max_tokens"),
                "{} should remove max_tokens",
                model
            );
        }

        // Non-reasoning models: max_tokens stays as-is
        for model in ["gpt-4", "gpt-4o"] {
            let mut obj = json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "max_tokens": 100
            })
            .as_object()
            .unwrap()
            .clone();
            apply_model_transforms(model, &mut obj);
            assert!(
                !obj.contains_key("max_completion_tokens"),
                "{} should not add max_completion_tokens",
                model
            );
            assert!(
                obj.contains_key("max_tokens"),
                "{} should preserve max_tokens",
                model
            );
        }

        // max_output_tokens is valid for Responses API - not converted
        let mut obj = json!({
            "model": "o3",
            "input": [{"role": "user", "content": "Hello"}],
            "max_output_tokens": 100
        })
        .as_object()
        .unwrap()
        .clone();
        apply_model_transforms("o3", &mut obj);
        assert!(
            obj.contains_key("max_output_tokens"),
            "max_output_tokens should be preserved"
        );
        assert!(
            !obj.contains_key("max_completion_tokens"),
            "should not convert max_output_tokens"
        );
    }

    #[test]
    fn test_force_max_tokens() {
        for model in [
            "mistral",
            "magistral",
            "codestral",
            "pixstral",
            "devstral",
            "voxstral",
        ] {
            let mut obj = json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "max_completion_tokens": 100
            })
            .as_object()
            .unwrap()
            .clone();
            apply_model_transforms(model, &mut obj);
            assert!(
                obj.contains_key("max_tokens"),
                "{} should add max_tokens",
                model
            );
            assert!(
                !obj.contains_key("max_completion_tokens"),
                "{} should remove max_completion_tokens",
                model
            );
        }
    }

    #[test]
    fn test_strip_stream_options() {
        let mut obj: Map<String, Value> = serde_json::from_value(json!({
            "model": "databricks-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream_options": { "include_usage": true }
        }))
        .unwrap();
        apply_model_transforms("databricks-model", &mut obj);
        assert!(
            !obj.contains_key("stream_options"),
            "stream_options should be removed"
        );
    }
}
