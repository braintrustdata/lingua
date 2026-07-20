/*!
Anthropic-specific capability detection used by the transformation pipeline.
*/
use crate::serde_json::{Map, Value};
use regex::Regex;
use std::sync::LazyLock;

const OUTPUT_CONFIG_EFFORT_MODEL_PREFIXES: &[&str] = &["claude-opus-4-5", "claude-opus-4-6"];
static OPUS_4_7_OR_LATER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(^|[./:@])claude-(opus-(4[-.]([7-9]|[1-9]\d)|([5-9]|[1-9]\d)[-.]\d{1,2})|sonnet-([5-9]|[1-9]\d)([-.]\d{1,2})?|fable-[a-z0-9][a-z0-9.-]*)($|[-./:@])",
    )
    .expect("valid Opus 4.7+ / Sonnet 5+ / Fable model regex")
});
// Denylist of Claude models that do NOT support mid-conversation system messages.
// Verified 2026-07-16 against the live Messages API: opus 4.1-4.7, sonnet 4.x, and haiku 4.x reject them.
static UNSUPPORTED_MID_CONVERSATION_SYSTEM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(^|[./:@])claude-(?:opus-4[-.][1-7]|sonnet-4(?:[-.]\d{1,2})?|haiku-4(?:[-.]\d{1,2})?)($|[-./:@])",
    )
    .expect("valid unsupported mid-conversation system model regex")
});

// Fable 5 and Mythos 5 keep adaptive thinking always on; `thinking: {type: "disabled"}`
// is rejected, so a reasoning opt-out must omit `thinking` instead of emitting disabled.
static ALWAYS_ON_THINKING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(^|[./:@])claude-(fable-[a-z0-9][a-z0-9.-]*|mythos-[a-z0-9][a-z0-9.-]*)($|[-./:@])",
    )
    .expect("valid always-on thinking model regex")
});
/// Check if a model supports `output_config.effort` (vs legacy `thinking`).
///
/// Opus 4.5+ and Sonnet 5+ models support this. All models support `thinking` as fallback.
pub fn supports_output_config_effort(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    // Bedrock/Vertex model IDs wrap the Anthropic model token with provider-specific
    // separators and suffixes (e.g. `us.anthropic.<model>-v1:0`, `anthropic/<model>@...`).
    // Split on known separators for 4.5/4.6, and use a regex for 4.7+.
    lower
        .split(['.', '/', ':', '@'])
        .any(part_supports_output_config_effort)
        || is_opus_4_7_or_later(&lower)
}

fn part_supports_output_config_effort(model_part: &str) -> bool {
    OUTPUT_CONFIG_EFFORT_MODEL_PREFIXES
        .iter()
        .any(|prefix| model_part.starts_with(prefix))
}

/// Check if a model uses adaptive thinking instead of legacy enabled thinking.
pub fn supports_adaptive_thinking(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    is_opus_4_7_or_later(&lower)
}

/// Check if a model accepts `thinking: {type: "disabled"}`.
///
/// Fable 5 / Mythos 5 keep adaptive thinking always on and reject `disabled`, so an
/// explicit reasoning opt-out on those models must omit `thinking` instead of emitting it.
pub fn supports_disabling_thinking(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    !is_always_on_thinking(&lower)
}

/// Check if a Claude model supports mid-conversation system messages (`role: "system"` in `messages`).
///
/// Non-Claude models never support it; Claude models are supported unless listed in the denylist.
pub fn supports_mid_conversation_system_messages(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    is_anthropic_claude_model(&lower) && !is_unsupported_mid_conversation_system_model(&lower)
}

/// Whether the model id refers to a Claude model, in direct, Bedrock, or Vertex form.
fn is_anthropic_claude_model(model: &str) -> bool {
    model.contains("claude")
}

/// Transforms required for specific Anthropic model families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTransform {
    /// Strip deprecated sampling parameters (Opus 4.7+ doesn't support them)
    StripSamplingParams,
}

use ModelTransform::*;

const OPUS_4_7_OR_LATER_TRANSFORMS: &[ModelTransform] = &[StripSamplingParams];

fn is_opus_4_7_or_later(model: &str) -> bool {
    OPUS_4_7_OR_LATER_RE.is_match(model)
}

fn is_always_on_thinking(model: &str) -> bool {
    ALWAYS_ON_THINKING_RE.is_match(model)
}

fn is_unsupported_mid_conversation_system_model(model: &str) -> bool {
    UNSUPPORTED_MID_CONVERSATION_SYSTEM_RE.is_match(model)
}

/// Get the transforms required for a model.
pub fn get_model_transforms(model: &str) -> &'static [ModelTransform] {
    let lower = model.to_ascii_lowercase();
    if is_opus_4_7_or_later(&lower) {
        return OPUS_4_7_OR_LATER_TRANSFORMS;
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
            StripSamplingParams => {
                obj.remove("temperature");
                obj.remove("top_p");
                obj.remove("top_k");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_with_sampling_params() -> Map<String, Value> {
        let mut obj = Map::new();
        obj.insert("temperature".to_string(), Value::from(0.7));
        obj.insert("top_p".to_string(), Value::from(0.9));
        obj.insert("top_k".to_string(), Value::from(40));
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
        assert!(supports_output_config_effort("claude-opus-4-7"));
        assert!(supports_output_config_effort("claude-opus-4-7-20260401"));
        assert!(supports_output_config_effort(
            "us.anthropic.claude-opus-4-7-v1:0"
        ));
        assert!(supports_output_config_effort(
            "anthropic/claude-opus-4-7@20260401"
        ));
        assert!(supports_output_config_effort("claude-opus-4-8"));
        assert!(supports_output_config_effort("claude-opus-4-8-20260528"));
        assert!(supports_output_config_effort(
            "anthropic/claude-opus-4-8@20260528"
        ));
        assert!(supports_output_config_effort("claude-opus-4-10"));
        assert!(supports_output_config_effort(
            "anthropic/claude-opus-4-10@20260601"
        ));
        assert!(supports_output_config_effort("claude-opus-5-0"));
        assert!(supports_output_config_effort("claude-opus-5.0"));
        assert!(supports_output_config_effort("claude-opus-5-1-20260701"));
        assert!(supports_output_config_effort(
            "anthropic/claude-opus-5-0@20260701"
        ));
        assert!(supports_output_config_effort("claude-sonnet-5"));
        assert!(supports_output_config_effort("claude-sonnet-5-20260701"));
        assert!(supports_output_config_effort("CLAUDE-SONNET-5"));
        assert!(supports_output_config_effort(
            "us.anthropic.claude-sonnet-5-v1:0"
        ));
        assert!(supports_output_config_effort(
            "anthropic/claude-sonnet-5@20260701"
        ));
        assert!(supports_output_config_effort("claude-sonnet-5.0"));
        assert!(supports_output_config_effort("claude-sonnet-5-1-20260715"));

        // Other models do not
        assert!(!supports_output_config_effort("claude-opus-4-4"));
        assert!(!supports_output_config_effort("claude-opus-4-20250514"));
        assert!(!supports_output_config_effort(
            "us.anthropic.claude-sonnet-4-5-20250929-v1:0"
        ));
        assert!(!supports_output_config_effort("claude-sonnet-4-5-20250929"));
        assert!(!supports_output_config_effort("claude-sonnet-4-20250514"));
        assert!(!supports_output_config_effort("claude-haiku-4-5-20251001"));
        assert!(!supports_output_config_effort("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_supports_adaptive_thinking() {
        let adaptive_models = [
            "claude-opus-4-7",
            "claude-opus-4-7-20260401",
            "CLAUDE-OPUS-4-7",
            "us.anthropic.claude-opus-4-7-v1:0",
            "anthropic/claude-opus-4-7@20260401",
            "claude-opus-4-8",
            "claude-opus-4-8-20260528",
            "anthropic/claude-opus-4-8@20260528",
            "claude-opus-4-10",
            "anthropic/claude-opus-4-10@20260601",
            "claude-opus-5-0",
            "claude-opus-5.0",
            "claude-opus-5-1-20260701",
            "anthropic/claude-opus-5-0@20260701",
            "claude-sonnet-5",
            "claude-sonnet-5-20260701",
            "CLAUDE-SONNET-5",
            "us.anthropic.claude-sonnet-5-v1:0",
            "anthropic/claude-sonnet-5@20260701",
            "claude-sonnet-5.0",
        ];
        let legacy_models = [
            "claude-opus-4-20250514",
            "claude-opus-4-6",
            "claude-opus-4-5-20250514",
            "claude-sonnet-4-5-20250929",
            "claude-3-5-sonnet-20241022",
        ];

        for model in adaptive_models {
            assert!(supports_adaptive_thinking(model), "model: {}", model);
        }
        for model in legacy_models {
            assert!(!supports_adaptive_thinking(model), "model: {}", model);
        }
    }

    #[test]
    fn test_supports_disabling_thinking() {
        let disableable = [
            "claude-sonnet-5",
            "claude-opus-4-7",
            "claude-opus-4-8",
            "claude-sonnet-4-5-20250929",
            "claude-opus-4-6",
        ];
        let always_on = [
            "claude-fable-5",
            "claude-fable-5-20260601",
            "CLAUDE-FABLE-5",
            "claude-fable-6",
            "us.anthropic.claude-fable-5-v1:0",
            "anthropic/claude-fable-5@20260601",
            "claude-mythos-5",
            "us.anthropic.claude-mythos-5-v1:0",
        ];
        for model in disableable {
            assert!(
                supports_disabling_thinking(model),
                "should support disabling: {}",
                model
            );
        }
        for model in always_on {
            assert!(
                !supports_disabling_thinking(model),
                "should not support disabling: {}",
                model
            );
        }
    }

    #[test]
    fn test_supports_mid_conversation_system_messages() {
        // Verified 2026-07-16 against the live Messages API: these accept mid-conversation system messages (HTTP 200).
        for model in [
            "claude-opus-4-8",
            "claude-opus-4-8-20260528",
            "claude-opus-4.8",
            "claude-sonnet-5",
            "claude-fable-5",
            "claude-fable-5-20260601",
            "CLAUDE-SONNET-5",
        ] {
            assert!(
                supports_mid_conversation_system_messages(model),
                "expected supported (verified live): {model}"
            );
        }

        // Verified 2026-07-16 against the live Messages API: these reject mid-conversation system messages (HTTP 400).
        for model in [
            "claude-opus-4-1",
            "claude-opus-4-1-20250805",
            "claude-opus-4-5",
            "claude-opus-4-5-20251101",
            "claude-opus-4-6",
            "claude-opus-4-7",
            "claude-sonnet-4-5",
            "claude-sonnet-4-5-20250929",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
            "claude-haiku-4-5-20251001",
            "us.anthropic.claude-sonnet-4-5-v1:0",
            "anthropic/claude-opus-4-7@20260401",
            "us.anthropic.claude-haiku-4-5-v1:0",
        ] {
            assert!(
                !supports_mid_conversation_system_messages(model),
                "expected unsupported (verified live): {model}"
            );
        }

        // Models not in the denylist default to supported. Not individually verified.
        for model in [
            "claude-opus-4-10",
            "claude-opus-4-10-20260601",
            "claude-opus-5-0",
            "claude-opus-5.0",
            "claude-sonnet-6",
            "claude-haiku-5",
            "claude-fable-6",
            "us.anthropic.claude-opus-4-8-v1:0",
            "anthropic/claude-opus-4-8@20260528",
        ] {
            assert!(
                supports_mid_conversation_system_messages(model),
                "expected default-supported (denylist miss): {model}"
            );
        }

        // Non-Claude models never support this Anthropic-only feature.
        for model in ["gpt-5.5", "gpt-5-mini", "gemini-2.5-pro", "grok-4"] {
            assert!(
                !supports_mid_conversation_system_messages(model),
                "expected unsupported (non-Anthropic): {model}"
            );
        }
    }

    #[test]
    fn test_get_model_transforms() {
        let cases = [
            ("claude-opus-4-7", &[StripSamplingParams][..]),
            ("claude-opus-4-7-20260401", &[StripSamplingParams][..]),
            ("CLAUDE-OPUS-4-7", &[StripSamplingParams][..]),
            (
                "us.anthropic.claude-opus-4-7-v1:0",
                &[StripSamplingParams][..],
            ),
            (
                "anthropic/claude-opus-4-7@20260401",
                &[StripSamplingParams][..],
            ),
            ("claude-opus-4-8", &[StripSamplingParams][..]),
            ("claude-opus-4-8-20260528", &[StripSamplingParams][..]),
            (
                "anthropic/claude-opus-4-8@20260528",
                &[StripSamplingParams][..],
            ),
            ("claude-opus-4-10", &[StripSamplingParams][..]),
            (
                "anthropic/claude-opus-4-10@20260601",
                &[StripSamplingParams][..],
            ),
            ("claude-opus-5-0", &[StripSamplingParams][..]),
            ("claude-opus-5.0", &[StripSamplingParams][..]),
            ("claude-opus-5-1-20260701", &[StripSamplingParams][..]),
            (
                "anthropic/claude-opus-5-0@20260701",
                &[StripSamplingParams][..],
            ),
            ("claude-sonnet-5", &[StripSamplingParams][..]),
            ("claude-sonnet-5-20260701", &[StripSamplingParams][..]),
            ("CLAUDE-SONNET-5", &[StripSamplingParams][..]),
            (
                "us.anthropic.claude-sonnet-5-v1:0",
                &[StripSamplingParams][..],
            ),
            (
                "anthropic/claude-sonnet-5@20260701",
                &[StripSamplingParams][..],
            ),
            ("claude-sonnet-5.0", &[StripSamplingParams][..]),
            ("claude-sonnet-5-1-20260715", &[StripSamplingParams][..]),
            ("claude-fable-5", &[StripSamplingParams][..]),
            ("claude-fable-5-20260601", &[StripSamplingParams][..]),
            ("CLAUDE-FABLE-5", &[StripSamplingParams][..]),
            ("claude-fable-6", &[StripSamplingParams][..]),
            (
                "us.anthropic.claude-fable-5-v1:0",
                &[StripSamplingParams][..],
            ),
            (
                "anthropic/claude-fable-5@20260601",
                &[StripSamplingParams][..],
            ),
            ("claude-opus-4-6", &[][..]),
            ("claude-opus-4-20250514", &[][..]),
            ("claude-opus-4-5-20250514", &[][..]),
            ("claude-sonnet-4-5-20250929", &[][..]),
            ("claude-3-5-sonnet-20241022", &[][..]),
            ("claude-fable", &[][..]),
            ("not-claude-fable-5", &[][..]),
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
            "claude-opus-4-8",
            "claude-opus-4-8-20260528",
            "claude-opus-4-10",
            "claude-opus-5-0",
            "claude-opus-5.0",
            "claude-opus-5-1-20260701",
            "claude-sonnet-5",
            "claude-sonnet-5-20260701",
            "CLAUDE-SONNET-5",
            "us.anthropic.claude-sonnet-5-v1:0",
            "anthropic/claude-sonnet-5@20260701",
            "claude-sonnet-5.0",
            "claude-fable-5",
            "claude-fable-5-20260601",
            "CLAUDE-FABLE-5",
            "claude-fable-6",
            "us.anthropic.claude-fable-5-v1:0",
            "anthropic/claude-fable-5@20260601",
        ];
        let no_needs = [
            "claude-opus-4-20250514",
            "claude-opus-4-6",
            "claude-opus-4-5",
            "claude-sonnet-4-5-20250929",
            "claude-3-5-sonnet-20241022",
            "claude-fable",
            "not-claude-fable-5",
        ];
        for model in needs {
            assert!(model_needs_transforms(model), "should need: {}", model);
        }
        for model in no_needs {
            assert!(!model_needs_transforms(model), "should not need: {}", model);
        }
    }

    #[test]
    fn test_strip_sampling_params() {
        let strip_models = [
            "claude-opus-4-7",
            "claude-opus-4-7-20260401",
            "us.anthropic.claude-opus-4-7-v1:0",
            "anthropic/claude-opus-4-7@20260401",
            "claude-opus-4-8",
            "claude-opus-4-8-20260528",
            "anthropic/claude-opus-4-8@20260528",
            "claude-opus-4-10",
            "anthropic/claude-opus-4-10@20260601",
            "claude-opus-5-0",
            "claude-opus-5.0",
            "claude-opus-5-1-20260701",
            "anthropic/claude-opus-5-0@20260701",
            "claude-sonnet-5",
            "claude-sonnet-5-20260701",
            "CLAUDE-SONNET-5",
            "us.anthropic.claude-sonnet-5-v1:0",
            "anthropic/claude-sonnet-5@20260701",
            "claude-sonnet-5.0",
        ];
        let preserve_models = [
            "claude-opus-4-20250514",
            "claude-opus-4-6",
            "claude-opus-4-5-20250514",
            "claude-sonnet-4-5-20250929",
            "claude-3-5-sonnet-20241022",
        ];

        for model in strip_models {
            let mut obj = object_with_sampling_params();
            apply_model_transforms(model, &mut obj);
            assert!(
                !obj.contains_key("temperature"),
                "{} should strip temperature",
                model
            );
            assert!(!obj.contains_key("top_p"), "{} should strip top_p", model);
            assert!(!obj.contains_key("top_k"), "{} should strip top_k", model);
        }

        for model in preserve_models {
            let mut obj = object_with_sampling_params();
            apply_model_transforms(model, &mut obj);
            assert!(
                obj.contains_key("temperature"),
                "{} should preserve temperature",
                model
            );
            assert!(obj.contains_key("top_p"), "{} should preserve top_p", model);
            assert!(obj.contains_key("top_k"), "{} should preserve top_k", model);
        }
    }

    #[test]
    fn test_fable_strips_sampling_params() {
        for model in [
            "claude-fable-5",
            "claude-fable-5-20260601",
            "CLAUDE-FABLE-5",
            "claude-fable-6",
            "us.anthropic.claude-fable-5-v1:0",
            "anthropic/claude-fable-5@20260601",
        ] {
            let mut obj = object_with_sampling_params();
            apply_model_transforms(model, &mut obj);
            assert!(
                !obj.contains_key("temperature"),
                "{} should strip temperature",
                model
            );
            assert!(!obj.contains_key("top_p"), "{} should strip top_p", model);
            assert!(!obj.contains_key("top_k"), "{} should strip top_k", model);
        }
    }

    #[test]
    fn test_sonnet_5_strips_sampling_params() {
        for model in [
            "claude-sonnet-5",
            "claude-sonnet-5-20260701",
            "CLAUDE-SONNET-5",
            "claude-sonnet-5.0",
            "claude-sonnet-5-1-20260715",
            "us.anthropic.claude-sonnet-5-v1:0",
            "anthropic/claude-sonnet-5@20260701",
        ] {
            let mut obj = object_with_sampling_params();
            apply_model_transforms(model, &mut obj);
            assert!(
                !obj.contains_key("temperature"),
                "{} should strip temperature",
                model
            );
            assert!(!obj.contains_key("top_p"), "{} should strip top_p", model);
            assert!(!obj.contains_key("top_k"), "{} should strip top_k", model);
        }

        // Sonnet 4 and earlier must keep sampling params.
        for model in [
            "claude-sonnet-4-5-20250929",
            "claude-sonnet-4-20250514",
            "claude-3-5-sonnet-20241022",
        ] {
            let mut obj = object_with_sampling_params();
            apply_model_transforms(model, &mut obj);
            assert!(
                obj.contains_key("temperature"),
                "{} should preserve temperature",
                model
            );
            assert!(obj.contains_key("top_p"), "{} should preserve top_p", model);
            assert!(obj.contains_key("top_k"), "{} should preserve top_k", model);
        }
    }
}
