/*!
Test case discovery for coverage-report.
*/

use bytes::Bytes;
use std::fs;
use std::path::PathBuf;

use crate::types::TestFilter;

/// Discover test case directories in payloads/snapshots, filtered by the provided filter.
pub fn discover_test_cases_filtered(filter: &TestFilter) -> Vec<String> {
    discover_all_test_cases()
        .into_iter()
        .filter(|name| filter.matches_test_case(name))
        .collect()
}

/// Discover all test case directories in payloads/snapshots (unfiltered)
fn discover_all_test_cases() -> Vec<String> {
    // Navigate from crates/coverage-report to workspace root
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let snapshots_dir = workspace_root.join("payloads").join("snapshots");

    let mut test_cases = Vec::new();

    if let Ok(entries) = fs::read_dir(&snapshots_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip hidden directories
                    if !name.starts_with('.') {
                        test_cases.push(name.to_string());
                    }
                }
            }
        }
    }

    test_cases.sort();
    test_cases
}

/// Load a JSON payload from a test case directory as bytes
pub fn load_payload(test_case: &str, dir_name: &str, filename: &str) -> Option<Bytes> {
    // Navigate from crates/coverage-report to workspace root
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let path = workspace_root
        .join("payloads")
        .join("snapshots")
        .join(test_case)
        .join(dir_name)
        .join(filename);

    if path.exists() {
        fs::read(&path).ok().map(Bytes::from)
    } else {
        None
    }
}
