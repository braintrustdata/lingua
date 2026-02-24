/*!
Expected differences whitelist.

This module defines expected differences between providers that are NOT bugs,
but documented semantic differences due to provider limitations.

This is the SINGLE SOURCE OF TRUTH for all expected limitations, covering:
- Test case skips (entire tests that cannot transform between providers)
- Field differences during comparison (params that don't exist in target provider)
- Transform errors (features that fail transformation with expected errors)

# JSON files

Expected differences are split by test category:
- `requests_expected_differences.json` - for request transformation tests
- `responses_expected_differences.json` - for response transformation tests
- `streaming_expected_differences.json` - for streaming response tests

Each file uses a two-tier structure:

```json
{
  "global": [
    {
      "source": "*",
      "target": "Anthropic",
      "fields": [
        { "pattern": "params.top_k", "reason": "OpenAI doesn't support top_k" }
      ],
      "errors": [
        { "pattern": "does not support logprobs", "reason": "Anthropic doesn't support logprobs" }
      ]
    }
  ],
  "perTestCase": [
    {
      "testCase": "imageContentParam",
      "source": "*",
      "target": "Anthropic",
      "skip": true,
      "reason": "Anthropic assistant messages don't support image content"
    }
  ]
}
```

## Structure

- **global**: Rules that apply to ALL tests for a source→target pair
- **perTestCase**: Test-specific rules with explicit test case name
  - Can have `skip: true` to skip entire test
  - Can have `fields`/`errors` arrays for partial differences

## Matching behavior

- **Test case skip**: Exact match on testCase name
- **Fields**: Prefix matching (e.g., "params.response_format" matches "params.response_format.json_schema")
- **Errors**: Substring matching on error message

## Provider matching

`source` and `target` can be `"*"` to match any provider. This is useful for:
- Universal limitations (e.g., image media_type normalization)
- One-sided limitations (e.g., "any source → Anthropic" for frequency_penalty)
*/

use crate::types::ExpectedDifferences;
use std::sync::LazyLock;

/// The category of test this expected difference applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCategory {
    Requests,
    Responses,
    Streaming,
}

/// Parse JSON content into ExpectedDifferences.
fn parse_expected_differences(json: &str, filename: &str) -> ExpectedDifferences {
    big_serde_json::from_str(json).unwrap_or_else(|e| panic!("Failed to parse {}: {}", filename, e))
}

/// Expected differences for request transformations.
static EXPECTED_REQUESTS: LazyLock<ExpectedDifferences> = LazyLock::new(|| {
    let json = include_str!("requests_expected_differences.json");
    parse_expected_differences(json, "requests_expected_differences.json")
});

/// Expected differences for response transformations.
static EXPECTED_RESPONSES: LazyLock<ExpectedDifferences> = LazyLock::new(|| {
    let json = include_str!("responses_expected_differences.json");
    parse_expected_differences(json, "responses_expected_differences.json")
});

/// Expected differences for streaming response transformations.
static EXPECTED_STREAMING: LazyLock<ExpectedDifferences> = LazyLock::new(|| {
    let json = include_str!("streaming_expected_differences.json");
    parse_expected_differences(json, "streaming_expected_differences.json")
});

/// Get the expected differences for a given test category.
fn get_expected_differences(category: TestCategory) -> &'static ExpectedDifferences {
    match category {
        TestCategory::Requests => &EXPECTED_REQUESTS,
        TestCategory::Responses => &EXPECTED_RESPONSES,
        TestCategory::Streaming => &EXPECTED_STREAMING,
    }
}

/// Whether `provider` inherits expected-difference rules from `parent`.
///
/// Bedrock Anthropic delegates all conversion to `AnthropicAdapter`, so it
/// shares every known limitation with "Anthropic".
fn inherits_from(provider: &str, parent: &str) -> bool {
    matches!(
        (provider, parent),
        ("Bedrock Anthropic", "Anthropic") | ("Vertex Anthropic", "Anthropic")
    )
}

/// Helper function for source/target matching with wildcard support.
fn matches_source_target(rule_source: &str, rule_target: &str, source: &str, target: &str) -> bool {
    let source_matches =
        rule_source == "*" || rule_source == source || inherits_from(source, rule_source);
    let target_matches =
        rule_target == "*" || rule_target == target || inherits_from(target, rule_target);
    source_matches && target_matches
}

/// Check if a test case is expected to be skipped for the given source→target.
///
/// Returns the reason if expected to skip, None otherwise.
pub fn is_expected_test_case(
    category: TestCategory,
    source: &str,
    target: &str,
    test_case: &str,
) -> Option<String> {
    let diffs = get_expected_differences(category);

    // Check per-test-case rules
    for rule in &diffs.per_test_case {
        if rule.test_case == test_case
            && rule.skip
            && matches_source_target(&rule.source, &rule.target, source, target)
        {
            return rule.reason.clone();
        }
    }

    None
}

/// Check if a field difference is expected for the given source→target translation.
///
/// Returns the reason if the difference is expected, None if it's unexpected (a bug).
pub fn is_expected_field(
    category: TestCategory,
    source: &str,
    target: &str,
    test_case: Option<&str>,
    field: &str,
) -> Option<String> {
    let diffs = get_expected_differences(category);

    // Helper to check if a pattern matches (prefix matching with [*] wildcard)
    // Example: pattern "choices[*].delta.refusal" matches field "choices[0].delta.refusal"
    let pattern_matches = |pattern: &str| {
        if pattern.contains("[*]") {
            // Convert pattern to regex: replace [*] with \[\d+\]
            let regex_pattern = pattern
                .replace("[", "\\[")
                .replace("]", "\\]")
                .replace("\\[*\\]", "\\[\\d+\\]");
            regex::Regex::new(&format!("^{}", regex_pattern))
                .map(|re| re.is_match(field))
                .unwrap_or(false)
        } else {
            field.starts_with(pattern)
        }
    };

    // Check per-test-case rules first (if we have a test case)
    if let Some(test_name) = test_case {
        for rule in &diffs.per_test_case {
            if rule.test_case == test_name
                && matches_source_target(&rule.source, &rule.target, source, target)
            {
                if let Some(entry) = rule.fields.iter().find(|e| pattern_matches(&e.pattern)) {
                    return Some(entry.reason.clone());
                }
            }
        }
    }

    // Check global rules
    for rule in &diffs.global {
        if matches_source_target(&rule.source, &rule.target, source, target) {
            if let Some(entry) = rule.fields.iter().find(|e| pattern_matches(&e.pattern)) {
                return Some(entry.reason.clone());
            }
        }
    }

    None
}

/// Check if a transform error is expected for the given source→target translation.
///
/// Returns the reason if the error is expected (limitation), None if it's unexpected (a bug).
pub fn is_expected_error(
    category: TestCategory,
    source: &str,
    target: &str,
    test_case: Option<&str>,
    error_msg: &str,
) -> Option<String> {
    let diffs = get_expected_differences(category);

    // Helper to check if error pattern matches (substring matching)
    let pattern_matches = |pattern: &str| error_msg.contains(pattern);

    // Check per-test-case rules first
    if let Some(test_name) = test_case {
        for rule in &diffs.per_test_case {
            if rule.test_case == test_name
                && matches_source_target(&rule.source, &rule.target, source, target)
            {
                if let Some(entry) = rule.errors.iter().find(|e| pattern_matches(&e.pattern)) {
                    return Some(entry.reason.clone());
                }
            }
        }
    }

    // Check global rules
    for rule in &diffs.global {
        if matches_source_target(&rule.source, &rule.target, source, target) {
            if let Some(entry) = rule.errors.iter().find(|e| pattern_matches(&e.pattern)) {
                return Some(entry.reason.clone());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Test case level tests
    // =========================================================================

    #[test]
    fn test_test_case_exact_match() {
        assert!(is_expected_test_case(
            TestCategory::Requests,
            "ChatCompletions",
            "Anthropic",
            "imageContentParam"
        )
        .is_some());
    }

    #[test]
    fn test_test_case_any_source_match() {
        // imageContentParam is configured with source=None, so any source should match
        assert!(is_expected_test_case(
            TestCategory::Requests,
            "ChatCompletions",
            "Anthropic",
            "imageContentParam"
        )
        .is_some());
        assert!(is_expected_test_case(
            TestCategory::Requests,
            "Responses",
            "Anthropic",
            "imageContentParam"
        )
        .is_some());
    }

    #[test]
    fn test_test_case_specific_source_match() {
        // codeInterpreterToolParam is configured with source=Responses
        assert!(is_expected_test_case(
            TestCategory::Requests,
            "Responses",
            "Anthropic",
            "codeInterpreterToolParam"
        )
        .is_some());
        // Should NOT match with ChatCompletions source
        assert!(is_expected_test_case(
            TestCategory::Requests,
            "ChatCompletions",
            "Anthropic",
            "codeInterpreterToolParam"
        )
        .is_none());
    }

    #[test]
    fn test_test_case_no_match() {
        // Unknown test case should not match
        assert!(is_expected_test_case(
            TestCategory::Requests,
            "Responses",
            "Anthropic",
            "unknownTestCase"
        )
        .is_none());
        // Known test case but wrong target
        assert!(is_expected_test_case(
            TestCategory::Requests,
            "ChatCompletions",
            "ChatCompletions",
            "imageContentParam"
        )
        .is_none());
    }

    // =========================================================================
    // Field level tests
    // =========================================================================

    #[test]
    fn test_field_exact_match() {
        assert!(is_expected_field(
            TestCategory::Requests,
            "Anthropic",
            "Responses",
            None,
            "params.reasoning.budget_tokens"
        )
        .is_some());
    }

    #[test]
    fn test_field_any_source_match() {
        // Use params.metadata which legitimately differs (not validated)
        assert!(is_expected_field(
            TestCategory::Requests,
            "ChatCompletions",
            "Anthropic",
            None,
            "params.metadata"
        )
        .is_some());
        assert!(is_expected_field(
            TestCategory::Requests,
            "Responses",
            "Anthropic",
            None,
            "params.metadata"
        )
        .is_some());
    }

    #[test]
    fn test_field_prefix_match() {
        assert!(is_expected_field(
            TestCategory::Requests,
            "ChatCompletions",
            "Anthropic",
            None,
            "params.response_format.json_schema.schema"
        )
        .is_some());
    }

    #[test]
    fn test_field_no_match() {
        assert!(is_expected_field(
            TestCategory::Requests,
            "ChatCompletions",
            "Responses",
            None,
            "messages[0].content"
        )
        .is_none());
    }

    #[test]
    fn test_error_match() {
        assert!(is_expected_error(
            TestCategory::Requests,
            "Anthropic",
            "ChatCompletions",
            None,
            "Tool 'bash' of type 'bash_20250124' is not supported by OpenAI Chat Completions"
        )
        .is_some());
    }

    #[test]
    fn test_error_missing_is_failure() {
        // Errors with "missing" should always be failures, not limitations
        assert!(is_expected_error(
            TestCategory::Requests,
            "Anthropic",
            "ChatCompletions",
            None,
            "missing model"
        )
        .is_none());
    }

    #[test]
    fn test_error_no_match() {
        assert!(is_expected_error(
            TestCategory::Requests,
            "Anthropic",
            "ChatCompletions",
            None,
            "some random error"
        )
        .is_none());
    }

    // =========================================================================
    // Category isolation tests
    // =========================================================================

    #[test]
    fn test_category_isolation() {
        // Requests category should find entries in requests file
        // Use params.metadata which legitimately differs (not validated)
        assert!(is_expected_field(
            TestCategory::Requests,
            "ChatCompletions",
            "Anthropic",
            None,
            "params.metadata"
        )
        .is_some());

        // Responses and Streaming categories have empty files, should not find anything
        assert!(is_expected_field(
            TestCategory::Responses,
            "ChatCompletions",
            "Anthropic",
            None,
            "params.metadata"
        )
        .is_none());
        assert!(is_expected_field(
            TestCategory::Streaming,
            "ChatCompletions",
            "Anthropic",
            None,
            "params.metadata"
        )
        .is_none());
    }
}
