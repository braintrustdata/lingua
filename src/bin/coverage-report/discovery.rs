/*!
Test case discovery for coverage-report.
*/

use lingua::serde_json::{self, Value};
use std::fs;
use std::path::PathBuf;

/// Discover all test case directories in payloads/snapshots
pub fn discover_test_cases() -> Vec<String> {
    let snapshots_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("payloads")
        .join("snapshots");

    let mut test_cases = Vec::new();

    if let Ok(entries) = fs::read_dir(&snapshots_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip hidden directories and transformations directory
                    if !name.starts_with('.') && name != "transformations" {
                        test_cases.push(name.to_string());
                    }
                }
            }
        }
    }

    test_cases.sort();
    test_cases
}

/// Load a JSON payload from a test case directory
pub fn load_payload(test_case: &str, dir_name: &str, filename: &str) -> Option<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("payloads")
        .join("snapshots")
        .join(test_case)
        .join(dir_name)
        .join(filename);

    if path.exists() {
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}
