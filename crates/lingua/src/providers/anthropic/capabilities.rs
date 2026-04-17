/*!
Anthropic-specific capability detection used by the transformation pipeline.
*/
use crate::serde_json::{Map, Value};

const OUTPUT_CONFIG_EFFORT_MODEL_PREFIXES: &[&str] = &["claude-opus-4-5", "claude-opus-4-6"];

/// Check if a model supports `output_config.effort` (vs legacy `thinking`).
///
/// Only Opus 4.5+ models support this. All models support `thinking` as fallback.
pub fn supports_output_config_effort(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    // Bedrock/Vertex model IDs wrap the Anthropic model token with provider-specific
    // separators and suffixes (e.g. `us.anthropic.<model>-v1:0`, `anthropic/<model>@...`).
    // Split on known separators and match hard-coded Anthropic model prefixes per part.
    lower
        .split(['.', '/', ':', '@'])
        .any(part_supports_output_config_effort)
}

fn part_supports_output_config_effort(model_part: &str) -> bool {
    OUTPUT_CONFIG_EFFORT_MODEL_PREFIXES
        .iter()
        .any(|prefix| model_part.starts_with(prefix))
}

/// Transforms required for specific Anthropic model families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTransform {
    /// Strip temperature parameter (Opus 4.7+ doesn't support it)
    StripTemperature,
}

use ModelTransform::*;

/// Model prefixes and their required transforms.
/// Order matters - more specific prefixes must come first.
const MODEL_TRANSFORM_RULES: &[(&str, &[ModelTransform])] =
    &[("claude-opus-4-7", &[StripTemperature])];

fn part_transforms(model_part: &str) -> Option<&'static [ModelTransform]> {
    for (prefix, transforms) in MODEL_TRANSFORM_RULES {
        if model_part.starts_with(prefix) {
            return Some(transforms);
        }
    }
    None
}

/// Get the transforms required for a model.
pub fn get_model_transforms(model: &str) -> &'static [ModelTransform] {
    let lower = model.to_ascii_lowercase();
    // Bedrock/Vertex model IDs wrap the Anthropic model token with provider-specific
    // separators and suffixes (e.g. `us.anthropic.<model>-v1:0`, `anthropic/<model>@...`).
    // Split on known separators and match hard-coded Anthropic model prefixes per part.
    lower
        .split(['.', '/', ':', '@'])
        .find_map(part_transforms)
        .unwrap_or(&[])
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_with_temperature() -> Map<String, Value> {
        let mut obj = Map::new();
        obj.insert("temperature".to_string(), Value::from(0.7));
        obj
    }

    #[test]
    fn test_supports_output_config_effort() {
        // Opus 4.5+ supports output_config.effort
        assert!(supports_output_config_effort("claude-opus-4-5-20250514"));
        assert!(supports_output_config_effort("claude-opus-4-6"));
        assert!(supports_output_config_effort("CLAUDE-OPUS-4-5"));
        assert!(supports_output_config_effort(
            "us.anthropic.claude-opus-4-6-v1:0"
        ));
        assert!(supports_output_config_effort(
            "anthropic.claude-opus-4-6-v1:0"
        ));
        assert!(supports_output_config_effort(
            "anthropic/claude-opus-4-5@20250514"
        ));

        // Other models do not
        assert!(!supports_output_config_effort("claude-opus-4-4"));
        assert!(!supports_output_config_effort("claude-opus-4-10"));
        assert!(!supports_output_config_effort("claude-opus-5-0"));
        assert!(!supports_output_config_effort(
            "us.anthropic.claude-sonnet-4-5-20250929-v1:0"
        ));
        assert!(!supports_output_config_effort(
            "anthropic/claude-opus-4-10@20260101"
        ));
        assert!(!supports_output_config_effort("claude-sonnet-4-5-20250929"));
        assert!(!supports_output_config_effort("claude-sonnet-4-20250514"));
        assert!(!supports_output_config_effort("claude-haiku-4-5-20251001"));
        assert!(!supports_output_config_effort("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_get_model_transforms() {
        let cases = [
            ("claude-opus-4-7", &[StripTemperature][..]),
            ("claude-opus-4-7-20260401", &[StripTemperature][..]),
            ("CLAUDE-OPUS-4-7", &[StripTemperature][..]),
            ("us.anthropic.claude-opus-4-7-v1:0", &[StripTemperature][..]),
            (
                "anthropic/claude-opus-4-7@20260401",
                &[StripTemperature][..],
            ),
            ("claude-opus-4-6", &[][..]),
            ("claude-opus-4-5-20250514", &[][..]),
            ("claude-sonnet-4-5-20250929", &[][..]),
            ("claude-3-5-sonnet-20241022", &[][..]),
        ];
        for (model, expected) in cases {
            assert_eq!(get_model_transforms(model), expected, "model: {}", model);
        }
    }

    #[test]
    fn test_model_needs_transforms() {
        let needs = [
            "claude-opus-4-7",
            "claude-opus-4-7-20260401",
            "us.anthropic.claude-opus-4-7-v1:0",
        ];
        let no_needs = [
            "claude-opus-4-6",
            "claude-opus-4-5",
            "claude-sonnet-4-5-20250929",
            "claude-3-5-sonnet-20241022",
        ];
        for model in needs {
            assert!(model_needs_transforms(model), "should need: {}", model);
        }
        for model in no_needs {
            assert!(!model_needs_transforms(model), "should not need: {}", model);
        }
    }

    #[test]
    fn test_strip_temperature() {
        let strip_models = [
            "claude-opus-4-7",
            "claude-opus-4-7-20260401",
            "us.anthropic.claude-opus-4-7-v1:0",
            "anthropic/claude-opus-4-7@20260401",
        ];
        let preserve_models = [
            "claude-opus-4-6",
            "claude-opus-4-5-20250514",
            "claude-sonnet-4-5-20250929",
            "claude-3-5-sonnet-20241022",
        ];

        for model in strip_models {
            let mut obj = object_with_temperature();
            apply_model_transforms(model, &mut obj);
            assert!(
                !obj.contains_key("temperature"),
                "{} should strip temperature",
                model
            );
        }

        for model in preserve_models {
            let mut obj = object_with_temperature();
            apply_model_transforms(model, &mut obj);
            assert!(
                obj.contains_key("temperature"),
                "{} should preserve temperature",
                model
            );
        }
    }
}
