/*!
Reasoning conversion utilities for cross-provider semantic translation.

This module provides heuristics for converting between different providers'
reasoning/thinking configurations:
- OpenAI Chat: `reasoning_effort` (low/medium/high)
- OpenAI Responses: `reasoning` object with `effort` and `summary` fields
- Anthropic: `thinking.budget_tokens`
- Google: `thinkingConfig.thinkingBudget`

## Canonical Format

The universal representation uses `budget_tokens` as the single canonical field.
Adapters convert between provider-specific formats (like OpenAI's effort levels)
and the canonical token budget at the provider boundary.

## Design

The conversion uses documented, deterministic heuristics:
- `effort_to_budget`: Converts effort level to token budget using multipliers
- `budget_to_effort`: Converts token budget to effort level using thresholds

All conversions happen in adapter code via trait implementations, not in universal types.

## Usage

```ignore
use crate::universal::request::ReasoningConfig;
use crate::providers::openai::generated::ReasoningEffort as OpenAIEffort;

// FROM OpenAI: Use tuple-based From trait for context-aware conversion
let config: ReasoningConfig = (openai_effort, Some(max_tokens)).into();

// FROM OpenAI: Fallback without max_tokens (uses DEFAULT_MAX_TOKENS)
let config: ReasoningConfig = (&openai_reasoning).into();

// FROM Anthropic: Direct conversion (already uses budget_tokens)
let config: ReasoningConfig = (&anthropic_thinking).into();

// TO provider: Convert at adapter boundary
let output = config.to_provider(ProviderFormat::Anthropic, Some(4096))?;
```
*/

use crate::capabilities::ProviderFormat;
use crate::processing::transform::TransformError;
use crate::providers::anthropic::generated::{Thinking, ThinkingType};
use crate::providers::openai::generated::{
    Reasoning as OpenAIReasoning, ReasoningEffort as OpenAIReasoningEffort,
    Summary as OpenAISummary,
};
use crate::serde_json::{json, Map, Value};
#[cfg(test)]
use crate::universal::request::SummaryMode;
use crate::universal::request::{ReasoningConfig, ReasoningEffort};

// =============================================================================
// Heuristic Constants
// =============================================================================

/// Multiplier for "low" effort (25% of max_tokens)
pub const EFFORT_LOW_MULTIPLIER: f64 = 0.25;

/// Multiplier for "medium" effort (50% of max_tokens)
pub const EFFORT_MEDIUM_MULTIPLIER: f64 = 0.50;

/// Multiplier for "high" effort (75% of max_tokens)
pub const EFFORT_HIGH_MULTIPLIER: f64 = 0.75;

/// Threshold below which budget is considered "low" effort
pub const EFFORT_LOW_THRESHOLD: f64 = 0.35;

/// Threshold above which budget is considered "high" effort
pub const EFFORT_HIGH_THRESHOLD: f64 = 0.65;

/// Minimum thinking budget for Anthropic
pub const MIN_THINKING_BUDGET: i64 = 1024;

/// Default max_tokens to use when not specified
pub const DEFAULT_MAX_TOKENS: i64 = 4096;

/// Default reasoning effort when enabled but no budget specified
pub const DEFAULT_REASONING_EFFORT: ReasoningEffort = ReasoningEffort::Medium;

/// Required temperature for Anthropic when thinking is enabled
pub const ANTHROPIC_THINKING_TEMPERATURE: f64 = 1.0;

// =============================================================================
// Effort ↔ Budget Conversion
// =============================================================================

/// Convert effort level to token budget.
///
/// Uses multipliers applied to max_tokens:
/// - low: 25% of max_tokens
/// - medium: 50% of max_tokens
/// - high: 75% of max_tokens
///
/// Result is clamped to minimum of 1024 tokens (Anthropic requirement).
///
/// # Parameters
/// - `effort`: The reasoning effort level
/// - `max_tokens`: Maximum tokens (must be positive, uses DEFAULT_MAX_TOKENS if None/invalid)
///
/// # Validation
/// - If `max_tokens` is None, zero, or negative, uses `DEFAULT_MAX_TOKENS` (4096)
pub fn effort_to_budget(effort: ReasoningEffort, max_tokens: Option<i64>) -> i64 {
    // Validate max_tokens - must be strictly positive
    let max = match max_tokens {
        Some(value) if value > 0 => value,
        _ => DEFAULT_MAX_TOKENS, // Use default for None, zero, or negative
    };

    let multiplier = match effort {
        ReasoningEffort::Low => EFFORT_LOW_MULTIPLIER,
        ReasoningEffort::Medium => EFFORT_MEDIUM_MULTIPLIER,
        ReasoningEffort::High => EFFORT_HIGH_MULTIPLIER,
    };
    let budget = (max as f64 * multiplier).floor() as i64;
    budget.max(MIN_THINKING_BUDGET)
}

/// Convert token budget to effort level.
///
/// Uses ratio of budget/max_tokens with thresholds:
/// - ratio < 0.35: low
/// - 0.35 <= ratio < 0.65: medium
/// - ratio >= 0.65: high
///
/// # Parameters
/// - `budget`: Token budget (must be positive, returns default effort if <= 0)
/// - `max_tokens`: Maximum tokens (must be positive, uses DEFAULT_MAX_TOKENS if None/invalid)
///
/// # Validation
/// - If `max_tokens` is None, zero, or negative, uses `DEFAULT_MAX_TOKENS` (4096)
/// - If `budget` is zero or negative, returns `DEFAULT_REASONING_EFFORT` (Medium)
pub fn budget_to_effort(budget: i64, max_tokens: Option<i64>) -> ReasoningEffort {
    // Validate max_tokens - must be strictly positive
    let max = match max_tokens {
        Some(value) if value > 0 => value,
        _ => DEFAULT_MAX_TOKENS, // Use default for None, zero, or negative
    };

    // Validate budget - if invalid, return default effort
    if budget <= 0 {
        return DEFAULT_REASONING_EFFORT;
    }

    let ratio = budget as f64 / max as f64;

    if ratio < EFFORT_LOW_THRESHOLD {
        ReasoningEffort::Low
    } else if ratio < EFFORT_HIGH_THRESHOLD {
        ReasoningEffort::Medium
    } else {
        ReasoningEffort::High
    }
}

// =============================================================================
// Typed From Implementations for Provider-to-Universal Conversions
// =============================================================================

/// Convert Anthropic Thinking to ReasoningConfig.
///
/// Anthropic's thinking is already normalized on `budget_tokens`, so this is a direct mapping.
impl From<&Thinking> for ReasoningConfig {
    fn from(thinking: &Thinking) -> Self {
        ReasoningConfig {
            enabled: Some(matches!(thinking.thinking_type, ThinkingType::Enabled)),
            budget_tokens: thinking.budget_tokens,
            ..Default::default()
        }
    }
}

/// Convert OpenAI ReasoningEffort to ReasoningConfig with context (for Chat API).
///
/// Takes max_tokens as context to compute accurate budget_tokens.
/// Uses DEFAULT_MAX_TOKENS if max_tokens is None.
impl From<(OpenAIReasoningEffort, Option<i64>)> for ReasoningConfig {
    fn from((effort, max_tokens): (OpenAIReasoningEffort, Option<i64>)) -> Self {
        let universal_effort = match effort {
            OpenAIReasoningEffort::Low | OpenAIReasoningEffort::Minimal => ReasoningEffort::Low,
            OpenAIReasoningEffort::Medium => ReasoningEffort::Medium,
            OpenAIReasoningEffort::High => ReasoningEffort::High,
        };
        ReasoningConfig {
            enabled: Some(true),
            budget_tokens: Some(effort_to_budget(universal_effort, max_tokens)),
            ..Default::default()
        }
    }
}

/// Convert OpenAI Reasoning to ReasoningConfig (for Responses API) - fallback.
///
/// Uses DEFAULT_MAX_TOKENS for effort→budget conversion when max_tokens is not available.
/// For context-aware conversion, use the tuple-based From impl.
impl From<&OpenAIReasoning> for ReasoningConfig {
    fn from(reasoning: &OpenAIReasoning) -> Self {
        let budget_tokens = reasoning.effort.as_ref().map(|e| {
            let universal_effort = match e {
                OpenAIReasoningEffort::Low | OpenAIReasoningEffort::Minimal => ReasoningEffort::Low,
                OpenAIReasoningEffort::Medium => ReasoningEffort::Medium,
                OpenAIReasoningEffort::High => ReasoningEffort::High,
            };
            effort_to_budget(universal_effort, None) // Uses DEFAULT_MAX_TOKENS
        });

        let summary = reasoning
            .summary
            .as_ref()
            .or(reasoning.generate_summary.as_ref())
            .map(|s| match s {
                OpenAISummary::Auto => crate::universal::request::SummaryMode::Auto,
                OpenAISummary::Concise => crate::universal::request::SummaryMode::Auto, // Map concise to auto
                OpenAISummary::Detailed => crate::universal::request::SummaryMode::Detailed,
            });

        ReasoningConfig {
            enabled: Some(true),
            budget_tokens,
            summary,
        }
    }
}

/// Convert OpenAI Reasoning to ReasoningConfig with context (for Responses API).
///
/// Takes max_tokens as context to compute accurate budget_tokens.
/// Uses provided max_tokens or DEFAULT_MAX_TOKENS if None.
impl From<(&OpenAIReasoning, Option<i64>)> for ReasoningConfig {
    fn from((reasoning, max_tokens): (&OpenAIReasoning, Option<i64>)) -> Self {
        let budget_tokens = reasoning.effort.as_ref().map(|e| {
            let universal_effort = match e {
                OpenAIReasoningEffort::Low | OpenAIReasoningEffort::Minimal => ReasoningEffort::Low,
                OpenAIReasoningEffort::Medium => ReasoningEffort::Medium,
                OpenAIReasoningEffort::High => ReasoningEffort::High,
            };
            effort_to_budget(universal_effort, max_tokens)
        });

        let summary = reasoning
            .summary
            .as_ref()
            .or(reasoning.generate_summary.as_ref())
            .map(|s| match s {
                OpenAISummary::Auto => crate::universal::request::SummaryMode::Auto,
                OpenAISummary::Concise => crate::universal::request::SummaryMode::Auto, // Map concise to auto
                OpenAISummary::Detailed => crate::universal::request::SummaryMode::Detailed,
            });

        ReasoningConfig {
            enabled: Some(true),
            budget_tokens,
            summary,
        }
    }
}

// =============================================================================
// to_provider Method for TO Conversions
// =============================================================================

impl ReasoningConfig {
    /// Convert this config to a provider-specific value.
    ///
    /// # Arguments
    /// * `provider` - Target provider format
    /// * `max_tokens` - Max tokens for effort→budget conversion (for Anthropic/Google)
    ///
    /// # Returns
    /// `Ok(Some(value))` if conversion succeeded
    /// `Ok(None)` if reasoning is not enabled or no value should be set
    /// `Err(_)` if conversion failed
    pub fn to_provider(
        &self,
        provider: ProviderFormat,
        max_tokens: Option<i64>,
    ) -> Result<Option<Value>, TransformError> {
        match provider {
            ProviderFormat::OpenAI => Ok(to_openai_chat(self, max_tokens).map(Value::String)),
            ProviderFormat::Responses => Ok(to_openai_responses(self, max_tokens)),
            ProviderFormat::Anthropic => Ok(to_anthropic(self, max_tokens)),
            ProviderFormat::Converse => Ok(to_anthropic(self, max_tokens)), // Bedrock uses same format as Anthropic
            ProviderFormat::Google => Ok(to_google(self, max_tokens)),
            _ => Ok(None),
        }
    }
}

// =============================================================================
// Value-based Helper Functions - For Providers Without Typed Params
// =============================================================================

/// Parse Google `thinkingConfig` object into ReasoningConfig.
///
/// Google doesn't have typed params yet, so we still need Value-based parsing.
pub fn from_google(config: &Value) -> ReasoningConfig {
    let enabled = config
        .get("includeThoughts")
        .and_then(Value::as_bool)
        .or_else(|| {
            // If thinkingBudget > 0, thinking is enabled
            config
                .get("thinkingBudget")
                .and_then(Value::as_i64)
                .map(|b| b > 0)
        });

    let budget_tokens = config.get("thinkingBudget").and_then(Value::as_i64);

    ReasoningConfig {
        enabled,
        budget_tokens,
        ..Default::default()
    }
}

// =============================================================================
// Private Helper Functions - TO Provider Formats
// =============================================================================

/// Convert ReasoningConfig to OpenAI Chat `reasoning_effort` string.
fn to_openai_chat(config: &ReasoningConfig, max_tokens: Option<i64>) -> Option<String> {
    if config.enabled != Some(true) {
        return None;
    }

    // Convert budget_tokens → effort at adapter boundary
    if let Some(budget) = config.budget_tokens {
        let effort = budget_to_effort(budget, max_tokens);
        return Some(effort.to_string());
    }

    // If just enabled with no specifics, use default effort
    Some(DEFAULT_REASONING_EFFORT.to_string())
}

/// Convert ReasoningConfig to OpenAI Responses API `reasoning` object.
fn to_openai_responses(config: &ReasoningConfig, max_tokens: Option<i64>) -> Option<Value> {
    if config.enabled != Some(true) {
        return None;
    }

    let mut obj = Map::new();

    // Convert budget_tokens → effort at adapter boundary
    let effort = if let Some(budget) = config.budget_tokens {
        budget_to_effort(budget, max_tokens).to_string()
    } else {
        DEFAULT_REASONING_EFFORT.to_string() // Default if only enabled=true
    };

    obj.insert("effort".into(), Value::String(effort));

    // Summary
    if let Some(summary) = config.summary {
        obj.insert("summary".into(), Value::String(summary.to_string()));
    }

    Some(Value::Object(obj))
}

/// Convert ReasoningConfig to Anthropic `thinking` object.
fn to_anthropic(config: &ReasoningConfig, _max_tokens: Option<i64>) -> Option<Value> {
    if config.enabled != Some(true) {
        return None;
    }

    // Use budget_tokens or default minimum
    let budget = config.budget_tokens.unwrap_or(MIN_THINKING_BUDGET);

    Some(json!({
        "type": "enabled",
        "budget_tokens": budget
    }))
}

/// Convert ReasoningConfig to Google `thinkingConfig` object.
fn to_google(config: &ReasoningConfig, _max_tokens: Option<i64>) -> Option<Value> {
    if config.enabled != Some(true) {
        return None;
    }

    // Use budget_tokens or default minimum
    let budget = config.budget_tokens.unwrap_or(MIN_THINKING_BUDGET);

    Some(json!({
        "includeThoughts": true,
        "thinkingBudget": budget
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effort_to_budget() {
        // With default max_tokens (4096)
        assert_eq!(effort_to_budget(ReasoningEffort::Low, None), 1024); // 4096 * 0.25 = 1024
        assert_eq!(effort_to_budget(ReasoningEffort::Medium, None), 2048); // 4096 * 0.50 = 2048
        assert_eq!(effort_to_budget(ReasoningEffort::High, None), 3072); // 4096 * 0.75 = 3072

        // With custom max_tokens
        assert_eq!(effort_to_budget(ReasoningEffort::Medium, Some(8192)), 4096);

        // Minimum budget enforced
        assert_eq!(effort_to_budget(ReasoningEffort::Low, Some(1000)), 1024); // Would be 250, clamped to 1024
    }

    #[test]
    fn test_budget_to_effort() {
        // With default max_tokens (4096)
        assert_eq!(budget_to_effort(500, None), ReasoningEffort::Low); // 500/4096 = 0.12 < 0.35
        assert_eq!(budget_to_effort(2000, None), ReasoningEffort::Medium); // 2000/4096 = 0.49
        assert_eq!(budget_to_effort(3000, None), ReasoningEffort::High); // 3000/4096 = 0.73 >= 0.65

        // With custom max_tokens
        assert_eq!(budget_to_effort(4096, Some(8192)), ReasoningEffort::Medium);
        // 4096/8192 = 0.5
    }

    #[test]
    fn test_roundtrip_effort() {
        // effort → budget → effort should preserve the original level
        for effort in [
            ReasoningEffort::Low,
            ReasoningEffort::Medium,
            ReasoningEffort::High,
        ] {
            let budget = effort_to_budget(effort, Some(4096));
            let back = budget_to_effort(budget, Some(4096));
            assert_eq!(effort, back, "Roundtrip failed for {:?}", effort);
        }
    }

    #[test]
    fn test_from_anthropic_thinking() {
        let thinking = Thinking {
            thinking_type: ThinkingType::Enabled,
            budget_tokens: Some(2048),
        };
        let config = ReasoningConfig::from(&thinking);
        assert_eq!(config.enabled, Some(true));
        assert_eq!(config.budget_tokens, Some(2048));
    }

    #[test]
    fn test_to_anthropic_thinking() {
        let config = ReasoningConfig {
            enabled: Some(true),
            budget_tokens: Some(2048),
            ..Default::default()
        };

        let thinking = config
            .to_provider(ProviderFormat::Anthropic, Some(4096))
            .unwrap()
            .unwrap();
        assert_eq!(thinking.get("type").unwrap(), "enabled");
        assert_eq!(thinking.get("budget_tokens").unwrap(), 2048);
    }

    #[test]
    fn test_to_openai_chat_reasoning() {
        let config = ReasoningConfig {
            enabled: Some(true),
            budget_tokens: Some(2048),
            ..Default::default()
        };

        let effort = config
            .to_provider(ProviderFormat::OpenAI, Some(4096))
            .unwrap()
            .unwrap();
        assert_eq!(effort.as_str().unwrap(), "medium"); // 2048/4096 = 0.5 → medium
    }

    #[test]
    fn test_from_openai_reasoning_effort() {
        // Test tuple-based conversion with max_tokens
        let config = ReasoningConfig::from((OpenAIReasoningEffort::High, Some(4096)));
        assert_eq!(config.enabled, Some(true));
        assert_eq!(config.budget_tokens, Some(3072)); // 75% of 4096
    }

    #[test]
    fn test_from_openai_responses_reasoning() {
        let reasoning = OpenAIReasoning {
            effort: Some(OpenAIReasoningEffort::High),
            summary: Some(OpenAISummary::Detailed),
            generate_summary: None,
        };

        // Test fallback conversion (uses DEFAULT_MAX_TOKENS)
        let config_fallback = ReasoningConfig::from(&reasoning);
        assert_eq!(config_fallback.enabled, Some(true));
        assert_eq!(config_fallback.budget_tokens, Some(3072)); // 75% of DEFAULT_MAX_TOKENS (4096)
        assert_eq!(config_fallback.summary, Some(SummaryMode::Detailed));

        // Test context-aware conversion with custom max_tokens
        let config_context = ReasoningConfig::from((&reasoning, Some(8192)));
        assert_eq!(config_context.enabled, Some(true));
        assert_eq!(config_context.budget_tokens, Some(6144)); // 75% of 8192
        assert_eq!(config_context.summary, Some(SummaryMode::Detailed));
    }

    #[test]
    fn test_to_bedrock_thinking() {
        // Bedrock uses the same format as Anthropic for Claude models
        let config = ReasoningConfig {
            enabled: Some(true),
            budget_tokens: Some(3072),
            ..Default::default()
        };

        let thinking = config
            .to_provider(ProviderFormat::Converse, Some(4096))
            .unwrap()
            .unwrap();
        assert_eq!(thinking.get("type").unwrap(), "enabled");
        assert_eq!(thinking.get("budget_tokens").unwrap(), 3072);
    }

    #[test]
    fn test_to_bedrock_thinking_with_budget() {
        // When budget_tokens is explicitly set, it should be used directly
        let config = ReasoningConfig {
            enabled: Some(true),
            budget_tokens: Some(5000),
            ..Default::default()
        };

        let thinking = config
            .to_provider(ProviderFormat::Converse, Some(8192))
            .unwrap()
            .unwrap();
        assert_eq!(thinking.get("type").unwrap(), "enabled");
        assert_eq!(thinking.get("budget_tokens").unwrap(), 5000);
    }

    #[test]
    fn test_to_bedrock_thinking_disabled() {
        let config = ReasoningConfig {
            enabled: Some(false),
            ..Default::default()
        };

        let result = config
            .to_provider(ProviderFormat::Converse, Some(4096))
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_budget_to_effort_edge_cases() {
        let test_cases = vec![
            // (budget, max_tokens, expected_effort, description)
            (
                2048,
                Some(0),
                ReasoningEffort::Medium,
                "zero max_tokens uses DEFAULT",
            ),
            (
                1200,
                Some(-100),
                ReasoningEffort::Low,
                "negative max_tokens uses DEFAULT (1200/4096=0.29<0.35)",
            ),
            (
                0,
                Some(4096),
                DEFAULT_REASONING_EFFORT,
                "zero budget returns default",
            ),
            (
                -1000,
                Some(4096),
                DEFAULT_REASONING_EFFORT,
                "negative budget returns default",
            ),
            (
                -500,
                Some(-200),
                DEFAULT_REASONING_EFFORT,
                "both negative returns default",
            ),
        ];

        for (budget, max_tokens, expected, description) in test_cases {
            assert_eq!(
                budget_to_effort(budget, max_tokens),
                expected,
                "Failed: {}",
                description
            );
        }
    }

    #[test]
    fn test_effort_to_budget_edge_cases() {
        let test_cases = vec![
            // (effort, max_tokens, expected_budget, description)
            (
                ReasoningEffort::Medium,
                Some(0),
                2048,
                "zero max_tokens uses DEFAULT (4096*0.5)",
            ),
            (
                ReasoningEffort::High,
                Some(-1000),
                3072,
                "negative max_tokens uses DEFAULT (4096*0.75)",
            ),
            (
                ReasoningEffort::Low,
                Some(-50),
                1024,
                "negative max_tokens clamped to minimum",
            ),
        ];

        for (effort, max_tokens, expected, description) in test_cases {
            assert_eq!(
                effort_to_budget(effort, max_tokens),
                expected,
                "Failed: {}",
                description
            );
        }
    }
}
