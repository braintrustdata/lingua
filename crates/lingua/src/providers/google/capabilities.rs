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

/// First Gemini major version that uses thinkingLevel
const FIRST_THINKING_LEVEL_MAJOR_VERSION: u32 = 3;

/// Model prefixes that use thinkingBudget (Gemini 2.5)
const THINKING_BUDGET_PREFIXES: &[&str] = &["gemini-2.5", "gemini-2.0"];

/// Google model capabilities
pub struct GoogleCapabilities {
    pub thinking_style: GoogleThinkingStyle,
    pub requires_thought_signature_for_function_call_history: bool,
}

impl GoogleCapabilities {
    /// Detect capabilities from model name
    pub fn detect(model: Option<&str>) -> Self {
        let thinking_style = model
            .map(|m| {
                let m_lower = normalize_google_model_id(m).to_ascii_lowercase();
                if gemini_major_version(&m_lower)
                    .is_some_and(|major| major >= FIRST_THINKING_LEVEL_MAJOR_VERSION)
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

        let requires_thought_signature_for_function_call_history =
            thinking_style == GoogleThinkingStyle::ThinkingLevelBased;

        Self {
            thinking_style,
            requires_thought_signature_for_function_call_history,
        }
    }
}

/// Whether the model accepts `thinkingBudget: 0` to turn thinking off.
///
/// Only Gemini 2.5 Flash / Flash-Lite accept 0. Gemini 2.5 Pro requires a
/// minimum budget of 128 and rejects 0, so it cannot be disabled. Flash
/// multimodal variants (image / TTS / audio) have no thinkingConfig and error
/// if one is sent, so they are excluded.
pub fn supports_disabling_thinking(model: Option<&str>) -> bool {
    let Some(model) = model else {
        return false;
    };
    let model = normalize_google_model_id(model).to_ascii_lowercase();
    if !model.starts_with("gemini-2.5-flash") {
        return false;
    }
    !["image", "tts", "audio"]
        .iter()
        .any(|kw| model.contains(kw))
}

fn normalize_google_model_id(model: &str) -> &str {
    model
        .strip_prefix("publishers/google/models/")
        .or_else(|| model.strip_prefix("models/"))
        .unwrap_or(model)
}

fn gemini_major_version(model: &str) -> Option<u32> {
    let version = model.strip_prefix("gemini-")?;
    let digits_end = version
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(version.len());
    version[..digits_end].parse().ok()
}

/// Thinking level for an absolute token budget, using the bands Gemini 4
/// applies to thinkingBudget on the Gemini API.
pub fn budget_to_thinking_level(budget: i64) -> ThinkingLevel {
    if budget < 4096 {
        ThinkingLevel::Low
    } else if budget < 16384 {
        ThinkingLevel::Medium
    } else {
        ThinkingLevel::High
    }
}

pub fn thinking_level_to_effort(level: &ThinkingLevel) -> ReasoningEffort {
    match level {
        ThinkingLevel::Minimal => ReasoningEffort::Minimal,
        ThinkingLevel::Low => ReasoningEffort::Low,
        ThinkingLevel::Medium => ReasoningEffort::Medium,
        ThinkingLevel::High => ReasoningEffort::High,
        ThinkingLevel::ThinkingLevelUnspecified => ReasoningEffort::High, // Google's default
    }
}

/// Convert ReasoningEffort to Google ThinkingLevel enum value.
///
/// Gemini 3+ cannot turn thinking off, and several models (3.1 Pro, 3.8 Flash,
/// 4) reject MINIMAL, so none and minimal map to LOW.
pub fn effort_to_thinking_level(effort: ReasoningEffort) -> ThinkingLevel {
    match effort {
        ReasoningEffort::None | ReasoningEffort::Minimal | ReasoningEffort::Low => {
            ThinkingLevel::Low
        }
        ReasoningEffort::Medium => ThinkingLevel::Medium,
        ReasoningEffort::High => ThinkingLevel::High,
        ReasoningEffort::Xhigh | ReasoningEffort::Max => ThinkingLevel::High,
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

        let vertex_capabilities =
            GoogleCapabilities::detect(Some("publishers/google/models/gemini-3.5-flash"));
        assert_eq!(
            vertex_capabilities.thinking_style,
            GoogleThinkingStyle::ThinkingLevelBased
        );
        assert!(vertex_capabilities.requires_thought_signature_for_function_call_history);

        let gemini_api_capabilities = GoogleCapabilities::detect(Some("models/gemini-3.5-flash"));
        assert_eq!(
            gemini_api_capabilities.thinking_style,
            GoogleThinkingStyle::ThinkingLevelBased
        );
        assert!(gemini_api_capabilities.requires_thought_signature_for_function_call_history);
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
    fn test_supports_disabling_thinking() {
        // 2.5 Flash / Flash-Lite accept thinkingBudget: 0.
        assert!(supports_disabling_thinking(Some("gemini-2.5-flash")));
        assert!(supports_disabling_thinking(Some("gemini-2.5-flash-lite")));
        assert!(supports_disabling_thinking(Some(
            "gemini-2.5-flash-preview-05-20"
        )));
        assert!(supports_disabling_thinking(Some("Gemini-2.5-Flash")));
        assert!(supports_disabling_thinking(Some(
            "publishers/google/models/gemini-2.5-flash"
        )));
        assert!(supports_disabling_thinking(Some("models/gemini-2.5-flash")));

        // 2.5 Pro requires a minimum budget of 128 and cannot be disabled.
        assert!(!supports_disabling_thinking(Some("gemini-2.5-pro")));
        // 2.0 has no thinking config; Gemini 3 uses thinkingLevel.
        assert!(!supports_disabling_thinking(Some("gemini-2.0-flash")));
        assert!(!supports_disabling_thinking(Some("gemini-3-flash-preview")));
        assert!(!supports_disabling_thinking(None));
        // Flash multimodal variants have no thinking config.
        assert!(!supports_disabling_thinking(Some("gemini-2.5-flash-image")));
        assert!(!supports_disabling_thinking(Some(
            "gemini-2.5-flash-image-preview"
        )));
        assert!(!supports_disabling_thinking(Some(
            "gemini-2.5-flash-preview-tts"
        )));
        assert!(!supports_disabling_thinking(Some(
            "gemini-2.5-flash-preview-native-audio-dialog"
        )));
    }

    #[test]
    fn test_detect_gemini_4_and_later_models() {
        for model in [
            "gemini-4-argon",
            "publishers/google/models/gemini-4-argon",
            "models/gemini-4-argon",
            "Gemini-4-Argon",
            "gemini-5-pro",
            "gemini-10-flash",
        ] {
            let caps = GoogleCapabilities::detect(Some(model));
            assert_eq!(
                caps.thinking_style,
                GoogleThinkingStyle::ThinkingLevelBased,
                "{model}"
            );
            assert!(
                caps.requires_thought_signature_for_function_call_history,
                "{model}"
            );
        }
    }

    #[test]
    fn test_detect_non_versioned_gemini_models_use_budget() {
        for model in ["gemini-pro", "gemini-embedding-001", "gemini-exp-1206"] {
            assert_eq!(
                GoogleCapabilities::detect(Some(model)).thinking_style,
                GoogleThinkingStyle::ThinkingBudget,
                "{model}"
            );
        }
    }

    #[test]
    fn test_budget_to_thinking_level() {
        assert_eq!(budget_to_thinking_level(1), ThinkingLevel::Low);
        assert_eq!(budget_to_thinking_level(1024), ThinkingLevel::Low);
        assert_eq!(budget_to_thinking_level(4095), ThinkingLevel::Low);
        assert_eq!(budget_to_thinking_level(4096), ThinkingLevel::Medium);
        assert_eq!(budget_to_thinking_level(8000), ThinkingLevel::Medium);
        assert_eq!(budget_to_thinking_level(16384), ThinkingLevel::High);
        assert_eq!(budget_to_thinking_level(24576), ThinkingLevel::High);
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
            ReasoningEffort::Minimal
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
        assert_eq!(
            effort_to_thinking_level(ReasoningEffort::Minimal),
            ThinkingLevel::Low
        );
        assert_eq!(
            effort_to_thinking_level(ReasoningEffort::None),
            ThinkingLevel::Low
        );
        assert_eq!(
            effort_to_thinking_level(ReasoningEffort::Xhigh),
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
