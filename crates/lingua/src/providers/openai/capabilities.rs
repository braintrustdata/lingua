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
}

use ModelTransform::*;

/// Model prefixes and their required transforms.
/// Order matters - more specific prefixes must come first.
const MODEL_TRANSFORM_RULES: &[(&str, &[ModelTransform])] = &[
    ("o1", &[StripTemperature, ForceMaxCompletionTokens]),
    ("o3", &[StripTemperature, ForceMaxCompletionTokens]),
    ("o4", &[StripTemperature, ForceMaxCompletionTokens]),
    ("gpt-5", &[StripTemperature, ForceMaxCompletionTokens]),
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
                    return;
                }

                // (Chat Completions API) max_tokens is deprecated - convert to max_completion_tokens.
                if let Some(max_tokens) = obj.remove("max_tokens") {
                    obj.entry("max_completion_tokens").or_insert(max_tokens);
                }
            }
        }
    }
}
