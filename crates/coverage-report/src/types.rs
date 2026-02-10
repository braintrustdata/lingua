/*!
Type definitions for coverage-report.
*/

use lingua::capabilities::ProviderFormat;

/// Output format for the coverage report.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OutputFormat {
    #[default]
    Markdown,
    Compact,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "compact" | "c" | "token" | "t" => Ok(OutputFormat::Compact),
            "markdown" | "md" | "full" => Ok(OutputFormat::Markdown),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

/// Filter configuration for granular test selection.
#[derive(Debug, Clone, Default)]
pub struct TestFilter {
    /// Glob patterns to match test case names (empty = match all)
    pub test_case_patterns: Vec<String>,
    /// Filter both source AND target to this set of providers
    pub providers: Option<Vec<ProviderFormat>>,
    /// Explicit source provider filter
    pub sources: Option<Vec<ProviderFormat>>,
    /// Explicit target provider filter
    pub targets: Option<Vec<ProviderFormat>>,
}

impl TestFilter {
    /// Check if a test case name matches the filter patterns.
    /// If no patterns specified, matches all test cases.
    pub fn matches_test_case(&self, name: &str) -> bool {
        if self.test_case_patterns.is_empty() {
            return true;
        }
        self.test_case_patterns
            .iter()
            .any(|pattern| glob_match(pattern, name))
    }

    /// Check if a provider pair matches the filter.
    /// Logic:
    /// - If `providers` is set: both source AND target must be in the list
    /// - If `sources` is set: source must be in the list
    /// - If `targets` is set: target must be in the list
    /// - Filters combine with AND logic
    pub fn matches_provider_pair(&self, source: ProviderFormat, target: ProviderFormat) -> bool {
        // Check providers filter (both must match)
        if let Some(ref providers) = self.providers {
            if !providers.contains(&source) || !providers.contains(&target) {
                return false;
            }
        }

        // Check explicit source filter
        if let Some(ref sources) = self.sources {
            if !sources.contains(&source) {
                return false;
            }
        }

        // Check explicit target filter
        if let Some(ref targets) = self.targets {
            if !targets.contains(&target) {
                return false;
            }
        }

        true
    }
}

/// Simple glob pattern matching.
/// Supports `*` (match any sequence) and `?` (match single char).
fn glob_match(pattern: &str, text: &str) -> bool {
    // Convert glob pattern to regex
    let regex_pattern = pattern
        .chars()
        .map(|c| match c {
            '*' => ".*".to_string(),
            '?' => ".".to_string(),
            // Escape regex special chars
            '.' | '+' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                format!("\\{}", c)
            }
            _ => c.to_string(),
        })
        .collect::<String>();

    // Anchor the pattern to match the entire string
    let full_pattern = format!("^{}$", regex_pattern);

    regex::Regex::new(&full_pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Parse a provider name string into a ProviderFormat.
pub fn parse_provider(name: &str) -> Result<ProviderFormat, String> {
    match name.to_lowercase().as_str() {
        "responses" | "response" | "openai-responses" => Ok(ProviderFormat::Responses),
        "chat-completions" | "chatcompletions" | "completions" | "openai" => {
            Ok(ProviderFormat::ChatCompletions)
        }
        "anthropic" => Ok(ProviderFormat::Anthropic),
        "google" | "gemini" => Ok(ProviderFormat::Google),
        "bedrock" | "converse" => Ok(ProviderFormat::Converse),
        _ => Err(format!("Unknown provider: {}", name)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationLevel {
    Pass,
    Fail,
    /// Provider limitation - feature that can't be transformed (e.g., "has no OpenAI equivalent")
    Limitation,
    /// Test skipped (e.g., payload file not found)
    Skipped,
}

#[derive(Debug)]
pub struct TransformResult {
    pub level: ValidationLevel,
    pub error: Option<String>,
    pub diff: Option<RoundtripDiff>,
    /// Human-readable reason from expected.rs whitelist (for limitations only)
    pub limitation_reason: Option<String>,
}

#[derive(Debug, Default)]
pub struct PairResult {
    pub passed: usize,
    pub failed: usize,
    pub limitations: usize,
    /// (test_case, error_message, optional_diff)
    pub failures: Vec<(String, String, Option<RoundtripDiff>)>,
    /// (test_case, reason, optional_diff)
    pub limitation_details: Vec<(String, String, Option<RoundtripDiff>)>,
}

pub struct TableStats {
    pub passed: usize,
    pub failed: usize,
    pub limitations: usize,
}

/// Failure with diff info: (direction, test_case, error, optional_diff)
pub type FailureWithDiff = (String, String, String, Option<RoundtripDiff>);

/// Output from generate_table function containing table markdown and statistics.
pub struct TableOutput {
    pub table_markdown: String,
    pub stats: TableStats,
    pub failures: Vec<FailureWithDiff>,
    /// (direction, test_case, reason, optional_diff)
    pub limitations: Vec<(String, String, String, Option<RoundtripDiff>)>,
}

#[derive(Debug, Clone, Copy)]
pub struct CoverageSelection {
    pub requests: bool,
    pub responses: bool,
    pub streaming: bool,
}

impl CoverageSelection {
    pub fn all() -> Self {
        Self {
            requests: true,
            responses: true,
            streaming: true,
        }
    }

    pub fn from_list(value: &str) -> Result<Self, String> {
        let mut selection = Self {
            requests: false,
            responses: false,
            streaming: false,
        };

        for raw in value.split(',') {
            let token = raw.trim().to_lowercase();
            if token.is_empty() {
                continue;
            }

            match token.as_str() {
                "all" => return Ok(Self::all()),
                "requests" | "request" => selection.requests = true,
                "responses" | "response" => selection.responses = true,
                "streaming" | "stream" => selection.streaming = true,
                _ => return Err(format!("Unknown coverage section: {}", token)),
            }
        }

        if !selection.requests && !selection.responses && !selection.streaming {
            return Err("No valid coverage sections provided".to_string());
        }

        Ok(selection)
    }
}

/// An issue entry: (direction, test_case, error_message)
pub type IssueEntry = (String, String, String);

/// Result from generating a coverage table.
pub struct TableResult {
    pub markdown: String,
    pub stats: TableStats,
    pub failures: Vec<IssueEntry>,
    pub limitations: Vec<IssueEntry>,
    pub missing_fixtures: Vec<IssueEntry>,
}

// ============================================================================
// Roundtrip testing types
// ============================================================================

/// Structured diff showing what changed during roundtrip transformation.
///
/// Tracks three categories of differences:
/// - `lost_fields`: Fields present in original but missing after roundtrip
/// - `added_fields`: Fields added during roundtrip (not in original)
/// - `changed_fields`: Fields where values changed (path, original, roundtripped)
/// - `expected_diffs`: Fields that differed but are whitelisted limitations (field_path, before, after, reason)
#[derive(Debug, Default, Clone)]
pub struct RoundtripDiff {
    pub lost_fields: Vec<String>,
    pub added_fields: Vec<String>,
    pub changed_fields: Vec<(String, String, String)>,
    pub expected_diffs: Vec<(String, String, String, String)>,
}

impl RoundtripDiff {
    pub fn is_empty(&self) -> bool {
        self.lost_fields.is_empty()
            && self.added_fields.is_empty()
            && self.changed_fields.is_empty()
    }

    #[allow(dead_code)]
    pub fn total_issues(&self) -> usize {
        self.lost_fields.len() + self.added_fields.len() + self.changed_fields.len()
    }
}

/// Result of a single roundtrip test (Provider → Universal → Provider).
#[derive(Debug)]
pub struct RoundtripResult {
    pub level: ValidationLevel,
    pub error: Option<String>,
    pub diff: Option<RoundtripDiff>,
}

// ============================================================================
// Expected differences types
// ============================================================================

use serde::{Deserialize, Serialize};

/// Root structure for expected differences with two-tier organization.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedDifferences {
    /// Global rules that apply to all test cases for a source→target pair
    #[serde(default)]
    pub global: Vec<GlobalRule>,
    /// Per-test-case rules that only apply to specific tests
    #[serde(default, rename = "perTestCase")]
    pub per_test_case: Vec<PerTestCaseRule>,
}

/// Global rule that applies to all tests for a source→target transformation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalRule {
    pub source: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<DifferenceEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<DifferenceEntry>,
}

/// Per-test-case rule that applies only to a specific test.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerTestCaseRule {
    #[serde(rename = "testCase")]
    pub test_case: String,
    pub source: String,
    pub target: String,
    /// If true, entire test should be skipped
    #[serde(default)]
    pub skip: bool,
    /// Reason for the skip or differences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Field differences expected for this test
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<DifferenceEntry>,
    /// Error patterns expected for this test
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<DifferenceEntry>,
}

/// A single field or error pattern with explanation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DifferenceEntry {
    pub pattern: String,
    pub reason: String,
}
