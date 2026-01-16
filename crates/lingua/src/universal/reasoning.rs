/*!
Reasoning conversion utilities for cross-provider semantic translation.

This module provides heuristics for converting between different providers'
reasoning/thinking configurations:
- OpenAI Chat: `reasoning_effort` (low/medium/high)
- OpenAI Responses: `reasoning` object with `effort` and `summary` fields
- Anthropic: `thinking.budget_tokens`
- Google: `thinkingConfig.thinkingBudget`

## Design

The conversion uses documented, deterministic heuristics:
- `effort_to_budget`: Converts effort level to token budget using multipliers
- `budget_to_effort`: Converts token budget to effort level using thresholds

These heuristics match the existing proxy behavior for consistency.

## Usage

```ignore
use std::convert::TryInto;
use crate::capabilities::ProviderFormat;
use crate::universal::request::ReasoningConfig;

// FROM: Parse provider-specific value to universal config
let config: ReasoningConfig = (ProviderFormat::Anthropic, &raw_json).try_into()?;

// TO: Convert universal config to provider-specific value
// Note: max_tokens is passed explicitly for effort→budget conversion
let output = config.to_provider(ProviderFormat::Anthropic, Some(4096))?;
```
*/

use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::processing::transform::TransformError;
use crate::serde_json::{json, Map, Value};
use crate::universal::request::{ReasoningConfig, ReasoningEffort};
#[cfg(test)]
use crate::universal::request::SummaryMode;

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
pub fn effort_to_budget(effort: ReasoningEffort, max_tokens: Option<i64>) -> i64 {
    let max = max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
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
pub fn budget_to_effort(budget: i64, max_tokens: Option<i64>) -> ReasoningEffort {
    let max = max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
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
// TryFrom Implementation for FROM Conversions
// =============================================================================

impl<'a> TryFrom<(ProviderFormat, &'a Value)> for ReasoningConfig {
    type Error = TransformError;

    fn try_from((provider, value): (ProviderFormat, &'a Value)) -> Result<Self, Self::Error> {
        match provider {
            ProviderFormat::OpenAI => {
                // For OpenAI Chat, value is expected to be the reasoning_effort string
                if let Some(effort_str) = value.as_str() {
                    Ok(from_openai_chat_reasoning_effort(effort_str, value.clone()))
                } else {
                    Ok(Self::default())
                }
            }
            ProviderFormat::Responses => Ok(from_openai_responses(value)),
            ProviderFormat::Anthropic => Ok(from_anthropic(value)),
            ProviderFormat::Google => Ok(from_google(value)),
            _ => Ok(Self::default()),
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
            ProviderFormat::Google => Ok(to_google(self, max_tokens)),
            _ => Ok(None),
        }
    }
}

// =============================================================================
// Private Helper Functions - FROM Provider Formats
// =============================================================================

/// Parse OpenAI Chat `reasoning_effort` string into ReasoningConfig.
fn from_openai_chat_reasoning_effort(reasoning_effort: &str, raw_value: Value) -> ReasoningConfig {
    ReasoningConfig {
        enabled: Some(true),
        effort: reasoning_effort.parse().ok(),
        budget_tokens: None, // OpenAI Chat doesn't have budget
        summary: None,
        raw: Some(raw_value),
    }
}

/// Parse OpenAI Responses API `reasoning` object into ReasoningConfig.
fn from_openai_responses(reasoning: &Value) -> ReasoningConfig {
    let effort = reasoning
        .get("effort")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok());

    let summary = reasoning
        .get("summary")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok());

    ReasoningConfig {
        enabled: Some(true),
        effort,
        budget_tokens: None,
        summary,
        raw: Some(reasoning.clone()),
    }
}

/// Parse Anthropic `thinking` object into ReasoningConfig.
fn from_anthropic(thinking: &Value) -> ReasoningConfig {
    let enabled = thinking
        .get("type")
        .and_then(Value::as_str)
        .map(|t| t == "enabled");

    let budget_tokens = thinking.get("budget_tokens").and_then(Value::as_i64);

    ReasoningConfig {
        enabled,
        effort: None, // Anthropic doesn't have effort level
        budget_tokens,
        summary: None,
        raw: Some(thinking.clone()),
    }
}

/// Parse Google `thinkingConfig` object into ReasoningConfig.
fn from_google(config: &Value) -> ReasoningConfig {
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
        effort: None, // Google doesn't have effort level
        budget_tokens,
        summary: None,
        raw: Some(config.clone()),
    }
}

// =============================================================================
// Private Helper Functions - TO Provider Formats
// =============================================================================

/// Convert ReasoningConfig to OpenAI Chat `reasoning_effort` string.
fn to_openai_chat(config: &ReasoningConfig, max_tokens: Option<i64>) -> Option<String> {
    // If we have effort, use it directly
    if let Some(effort) = config.effort {
        return Some(effort.to_string());
    }

    // If we have budget_tokens, derive effort from it
    if let Some(budget) = config.budget_tokens {
        let effort = budget_to_effort(budget, max_tokens);
        return Some(effort.to_string());
    }

    // If just enabled with no specifics, default to medium
    if config.enabled == Some(true) {
        return Some("medium".to_string());
    }

    None
}

/// Convert ReasoningConfig to OpenAI Responses API `reasoning` object.
fn to_openai_responses(config: &ReasoningConfig, max_tokens: Option<i64>) -> Option<Value> {
    if config.enabled != Some(true) {
        return None;
    }

    let mut obj = Map::new();

    // Effort
    let effort = config
        .effort
        .map(|e| e.to_string())
        .or_else(|| {
            config
                .budget_tokens
                .map(|b| budget_to_effort(b, max_tokens).to_string())
        })
        .unwrap_or_else(|| "medium".to_string());

    obj.insert("effort".into(), Value::String(effort));

    // Summary
    if let Some(summary) = config.summary {
        obj.insert("summary".into(), Value::String(summary.to_string()));
    }

    Some(Value::Object(obj))
}

/// Convert ReasoningConfig to Anthropic `thinking` object.
fn to_anthropic(config: &ReasoningConfig, max_tokens: Option<i64>) -> Option<Value> {
    if config.enabled != Some(true) {
        return None;
    }

    // Calculate budget_tokens
    let budget = config.budget_tokens.unwrap_or_else(|| {
        config
            .effort
            .map(|e| effort_to_budget(e, max_tokens))
            .unwrap_or(MIN_THINKING_BUDGET)
    });

    Some(json!({
        "type": "enabled",
        "budget_tokens": budget
    }))
}

/// Convert ReasoningConfig to Google `thinkingConfig` object.
fn to_google(config: &ReasoningConfig, max_tokens: Option<i64>) -> Option<Value> {
    if config.enabled != Some(true) {
        return None;
    }

    // Calculate thinkingBudget
    let budget = config.budget_tokens.unwrap_or_else(|| {
        config
            .effort
            .map(|e| effort_to_budget(e, max_tokens))
            .unwrap_or(MIN_THINKING_BUDGET)
    });

    Some(json!({
        "includeThoughts": true,
        "thinkingBudget": budget
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

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
        assert_eq!(budget_to_effort(4096, Some(8192)), ReasoningEffort::Medium); // 4096/8192 = 0.5
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
    fn test_from_anthropic() {
        let value = json!({
            "type": "enabled",
            "budget_tokens": 2048
        });
        let config: ReasoningConfig = (ProviderFormat::Anthropic, &value).try_into().unwrap();
        assert_eq!(config.enabled, Some(true));
        assert_eq!(config.budget_tokens, Some(2048));
    }

    #[test]
    fn test_to_anthropic_thinking() {
        let config = ReasoningConfig {
            enabled: Some(true),
            effort: Some(ReasoningEffort::Medium),
            budget_tokens: None,
            summary: None,
            raw: None,
        };

        let thinking = config.to_provider(ProviderFormat::Anthropic, Some(4096)).unwrap().unwrap();
        assert_eq!(thinking.get("type").unwrap(), "enabled");
        assert_eq!(thinking.get("budget_tokens").unwrap(), 2048);
    }

    #[test]
    fn test_to_openai_chat_reasoning() {
        let config = ReasoningConfig {
            enabled: Some(true),
            effort: None,
            budget_tokens: Some(2048),
            summary: None,
            raw: None,
        };

        let effort = config.to_provider(ProviderFormat::OpenAI, Some(4096)).unwrap().unwrap();
        assert_eq!(effort.as_str().unwrap(), "medium"); // 2048/4096 = 0.5 → medium
    }

    #[test]
    fn test_from_openai_responses() {
        let value = json!({
            "effort": "high",
            "summary": "detailed"
        });
        let config: ReasoningConfig = (ProviderFormat::Responses, &value).try_into().unwrap();
        assert_eq!(config.enabled, Some(true));
        assert_eq!(config.effort, Some(ReasoningEffort::High));
        assert_eq!(config.summary, Some(SummaryMode::Detailed));
    }
}
