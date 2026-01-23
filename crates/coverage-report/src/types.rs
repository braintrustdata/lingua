/*!
Type definitions for coverage-report.
*/

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationLevel {
    Pass,
    Fail,
    /// Provider limitation - feature that can't be transformed (e.g., "has no OpenAI equivalent")
    Limitation,
    /// Missing test fixture - "Source payload not found"
    MissingFixture,
}

#[derive(Debug)]
pub struct TransformResult {
    pub level: ValidationLevel,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct PairResult {
    pub passed: usize,
    pub failed: usize,
    pub limitations: usize,
    pub missing_fixtures: usize,
    pub failures: Vec<(String, String)>,
    pub limitation_details: Vec<(String, String)>,
    pub missing_fixture_details: Vec<(String, String)>,
}

pub struct TableStats {
    pub passed: usize,
    pub failed: usize,
    pub limitations: usize,
    pub missing_fixtures: usize,
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
#[derive(Debug, Default)]
pub struct RoundtripDiff {
    pub lost_fields: Vec<String>,
    pub added_fields: Vec<String>,
    pub changed_fields: Vec<(String, String, String)>,
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

/// Per-provider aggregated roundtrip test results.
#[derive(Debug, Default)]
pub struct ProviderRoundtripResult {
    pub request_passed: usize,
    pub request_failed: usize,
    pub request_failures: Vec<(String, RoundtripResult)>,
    pub response_passed: usize,
    pub response_failed: usize,
    pub response_failures: Vec<(String, RoundtripResult)>,
}

impl ProviderRoundtripResult {
    #[allow(dead_code)]
    pub fn total_passed(&self) -> usize {
        self.request_passed + self.response_passed
    }

    pub fn total_failed(&self) -> usize {
        self.request_failed + self.response_failed
    }
}
