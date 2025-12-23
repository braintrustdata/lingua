/*!
Type definitions for coverage-report.
*/

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationLevel {
    Pass,
    Fail,
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
    pub failures: Vec<(String, String)>,
}

pub struct TableStats {
    pub passed: usize,
    pub failed: usize,
}
