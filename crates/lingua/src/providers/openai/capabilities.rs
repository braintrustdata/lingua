/*!
OpenAI-specific capability detection used by the transformation pipeline.
*/
use crate::serde_json::{Map, Value};

/// Transforms required for specific model families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTransform {
    /// Strip temperature parameter (reasoning models don't support it)
    StripTemperature,
    /// Convert max_tokens to max_completion_tokens
    ForceMaxCompletionTokens,
    /// Convert max_completion_tokens to max_tokens
    ForceMaxTokens,
}

use ModelTransform::*;

/// Model prefixes and their required transforms.
/// Order matters - more specific prefixes must come first.
const MODEL_TRANSFORM_RULES: &[(&str, &[ModelTransform])] = &[
    ("o1", &[StripTemperature, ForceMaxCompletionTokens]),
    ("o3", &[StripTemperature, ForceMaxCompletionTokens]),
    ("o4", &[StripTemperature, ForceMaxCompletionTokens]),
    ("gpt-5", &[StripTemperature, ForceMaxCompletionTokens]),
    // TODO: would be nice if we could apply these rules by provider instead of model name, and
    // apply these to all Mistral models
    ("mistral", &[ForceMaxTokens]),
    ("magistral", &[ForceMaxTokens]),
    ("codestral", &[ForceMaxTokens]),
    ("pixstral", &[ForceMaxTokens]),
    ("devstral", &[ForceMaxTokens]),
    ("voxstral", &[ForceMaxTokens]),
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

/// Apply all transforms for a model to a request object.
pub fn apply_model_transforms(model: &str, obj: &mut Map<String, Value>) {
    for transform in get_model_transforms(model) {
        match transform {
            StripTemperature => {
                obj.remove("temperature");
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_get_model_transforms() {
        let cases = [
            ("o1", &[StripTemperature, ForceMaxCompletionTokens][..]),
            ("o1-mini", &[StripTemperature, ForceMaxCompletionTokens][..]),
            ("o3", &[StripTemperature, ForceMaxCompletionTokens][..]),
            (
                "o4-preview",
                &[StripTemperature, ForceMaxCompletionTokens][..],
            ),
            (
                "gpt-5-mini",
                &[StripTemperature, ForceMaxCompletionTokens][..],
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
    fn test_strip_temperature() {
        let reasoning_models = ["o1", "o1-mini", "o3", "gpt-5-mini"];
        let non_reasoning_models = ["gpt-4", "gpt-4o", "claude-3"];

        // Reasoning models: temperature should be stripped
        for model in reasoning_models {
            let mut obj = json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "temperature": 0.7
            })
            .as_object()
            .unwrap()
            .clone();
            apply_model_transforms(model, &mut obj);
            assert!(
                !obj.contains_key("temperature"),
                "{} should strip temperature",
                model
            );
        }

        // Non-reasoning models: temperature should be preserved
        for model in non_reasoning_models {
            let mut obj = json!({
                "model": model,
                "messages": [{"role": "user", "content": "Hello"}],
                "temperature": 0.7
            })
            .as_object()
            .unwrap()
            .clone();
            apply_model_transforms(model, &mut obj);
            assert!(
                obj.contains_key("temperature"),
                "{} should preserve temperature",
                model
            );
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
}
