//! Google model-specific capability detection.
//!
//! This module provides capability detection for Google Gemini models,
//! following the same pattern as OpenAI's capabilities.rs.
//!
//! ## Thinking configuration
//!
//! Google has two different ways to configure thinking/reasoning:
//! - **Gemini 3+**: Uses `thinkingLevel` (LOW/MEDIUM/HIGH/MINIMAL) - effort-based
//! - **Gemini 2.5**: Uses `thinkingBudget` (integer token count) - budget-based
//!
//! Using `thinkingBudget` with Gemini 3 Pro may result in suboptimal performance.

use crate::providers::google::generated::ThinkingLevel;
use crate::universal::ReasoningEffort;

/// Google model thinking capability tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoogleThinkingStyle {
    /// Gemini 3+ models: use thinkingLevel (effort-based)
    ThinkingLevelBased,
    /// Gemini 2.5 models: use thinkingBudget (token-based)
    ThinkingBudget,
    /// Models without thinking support
    None,
}

/// Model prefixes that use thinkingLevel (Gemini 3+)
const THINKING_LEVEL_PREFIXES: &[&str] = &["gemini-3"];

/// Model prefixes that use thinkingBudget (Gemini 2.5)
const THINKING_BUDGET_PREFIXES: &[&str] = &["gemini-2.5", "gemini-2.0"];

/// Google model capabilities
pub struct GoogleCapabilities {
    pub thinking_style: GoogleThinkingStyle,
}

impl GoogleCapabilities {
    /// Detect capabilities from model name
    pub fn detect(model: Option<&str>) -> Self {
        let thinking_style = model
            .map(|m| {
                let m_lower = m.to_ascii_lowercase();
                if THINKING_LEVEL_PREFIXES
                    .iter()
                    .any(|p| m_lower.starts_with(p))
                {
                    GoogleThinkingStyle::ThinkingLevelBased
                } else if THINKING_BUDGET_PREFIXES
                    .iter()
                    .any(|p| m_lower.starts_with(p))
                {
                    GoogleThinkingStyle::ThinkingBudget
                } else {
                    // Default to ThinkingBudget for unknown models (safer/backwards compatible)
                    GoogleThinkingStyle::ThinkingBudget
                }
            })
            .unwrap_or(GoogleThinkingStyle::ThinkingBudget);

        Self { thinking_style }
    }
}

pub fn thinking_level_to_effort(level: &ThinkingLevel) -> ReasoningEffort {
    match level {
        ThinkingLevel::Low => ReasoningEffort::Low,
        ThinkingLevel::Medium => ReasoningEffort::Medium,
        ThinkingLevel::High => ReasoningEffort::High,
        ThinkingLevel::Minimal => ReasoningEffort::Low, // closest approximation
        ThinkingLevel::ThinkingLevelUnspecified => ReasoningEffort::High, // Google's default
    }
}

/// Convert ReasoningEffort to Google ThinkingLevel enum value
pub fn effort_to_thinking_level(effort: ReasoningEffort) -> ThinkingLevel {
    match effort {
        ReasoningEffort::Low => ThinkingLevel::Low,
        ReasoningEffort::Medium => ThinkingLevel::Medium,
        ReasoningEffort::High => ThinkingLevel::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_gemini_3_models() {
        assert_eq!(
            GoogleCapabilities::detect(Some("gemini-3-flash-preview")).thinking_style,
            GoogleThinkingStyle::ThinkingLevelBased
        );
        assert_eq!(
            GoogleCapabilities::detect(Some("gemini-3-pro")).thinking_style,
            GoogleThinkingStyle::ThinkingLevelBased
        );
        assert_eq!(
            GoogleCapabilities::detect(Some("Gemini-3-Flash")).thinking_style,
            GoogleThinkingStyle::ThinkingLevelBased
        );
    }

    #[test]
    fn test_detect_gemini_25_models() {
        assert_eq!(
            GoogleCapabilities::detect(Some("gemini-2.5-flash")).thinking_style,
            GoogleThinkingStyle::ThinkingBudget
        );
        assert_eq!(
            GoogleCapabilities::detect(Some("gemini-2.5-pro")).thinking_style,
            GoogleThinkingStyle::ThinkingBudget
        );
        assert_eq!(
            GoogleCapabilities::detect(Some("gemini-2.0-flash")).thinking_style,
            GoogleThinkingStyle::ThinkingBudget
        );
    }

    #[test]
    fn test_detect_unknown_models() {
        // Unknown models default to ThinkingBudget for backwards compatibility
        assert_eq!(
            GoogleCapabilities::detect(Some("gemini-1.5-pro")).thinking_style,
            GoogleThinkingStyle::ThinkingBudget
        );
        assert_eq!(
            GoogleCapabilities::detect(Some("some-other-model")).thinking_style,
            GoogleThinkingStyle::ThinkingBudget
        );
        assert_eq!(
            GoogleCapabilities::detect(None).thinking_style,
            GoogleThinkingStyle::ThinkingBudget
        );
    }

    #[test]
    fn test_thinking_level_to_effort() {
        assert_eq!(
            thinking_level_to_effort(&ThinkingLevel::Low),
            ReasoningEffort::Low
        );
        assert_eq!(
            thinking_level_to_effort(&ThinkingLevel::Medium),
            ReasoningEffort::Medium
        );
        assert_eq!(
            thinking_level_to_effort(&ThinkingLevel::High),
            ReasoningEffort::High
        );
        assert_eq!(
            thinking_level_to_effort(&ThinkingLevel::Minimal),
            ReasoningEffort::Low
        );
        assert_eq!(
            thinking_level_to_effort(&ThinkingLevel::ThinkingLevelUnspecified),
            ReasoningEffort::High
        );
    }

    #[test]
    fn test_effort_to_thinking_level() {
        assert_eq!(
            effort_to_thinking_level(ReasoningEffort::Low),
            ThinkingLevel::Low
        );
        assert_eq!(
            effort_to_thinking_level(ReasoningEffort::Medium),
            ThinkingLevel::Medium
        );
        assert_eq!(
            effort_to_thinking_level(ReasoningEffort::High),
            ThinkingLevel::High
        );
    }

    #[test]
    fn test_roundtrip_effort() {
        for level in [
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
        ] {
            let effort = thinking_level_to_effort(&level);
            let back = effort_to_thinking_level(effort);
            assert_eq!(back, level);
        }
    }
}
