/*!
Anthropic-specific capability detection used by the transformation pipeline.
*/

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
