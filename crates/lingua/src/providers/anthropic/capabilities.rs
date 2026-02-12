/*!
Anthropic-specific capability detection used by the transformation pipeline.
*/

/// Check if a model supports `output_config.effort` (vs legacy `thinking`).
///
/// Only Opus 4.5+ models support this. All models support `thinking` as fallback.
pub fn supports_output_config_effort(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.starts_with("claude-opus-4-5") || lower.starts_with("claude-opus-4-6")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_output_config_effort() {
        // Opus 4.5+ supports output_config.effort
        assert!(supports_output_config_effort("claude-opus-4-5-20250514"));
        assert!(supports_output_config_effort("claude-opus-4-6"));

        // Other models do not
        assert!(!supports_output_config_effort("claude-sonnet-4-5-20250929"));
        assert!(!supports_output_config_effort("claude-sonnet-4-20250514"));
        assert!(!supports_output_config_effort("claude-haiku-4-5-20251001"));
        assert!(!supports_output_config_effort("claude-3-5-sonnet-20241022"));
    }
}
