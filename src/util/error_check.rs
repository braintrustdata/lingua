#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    /// Test that ensures no error.json files exist in the project
    /// These files are typically created when there are typos or issues in test data
    #[test]
    fn test_no_error_json_files() {
        let error_files = find_error_json_files(".");

        if !error_files.is_empty() {
            let error_list = error_files
                .iter()
                .map(|path| format!("  - {}", path))
                .collect::<Vec<_>>()
                .join("\n");

            panic!(
                "Found {} error.json file(s) that need to be fixed:\n{}\n\nThese files are typically created due to typos or issues in test data. Please investigate and fix the underlying issues.",
                error_files.len(),
                error_list
            );
        }
    }

    /// Recursively search for error.json files starting from the given directory
    fn find_error_json_files(start_dir: &str) -> Vec<String> {
        let mut error_files = Vec::new();

        if let Ok(entries) = fs::read_dir(start_dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    // Skip common directories that shouldn't contain test data
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if matches!(dir_name, "target" | ".git" | "node_modules" | ".cargo") {
                            continue;
                        }
                    }

                    // Recursively search subdirectories
                    if let Some(path_str) = path.to_str() {
                        error_files.extend(find_error_json_files(path_str));
                    }
                } else if path.is_file() {
                    // Check if this is an error.json file
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name == "error.json" {
                            if let Some(path_str) = path.to_str() {
                                error_files.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        error_files
    }

    /// Helper test to show where test data directories are located
    /// This can help identify where error.json files might be expected to appear
    #[test]
    fn test_show_test_data_locations() {
        let test_dirs = find_test_data_directories(".");

        if !test_dirs.is_empty() {
            println!("Test data directories found:");
            for dir in &test_dirs {
                println!("  - {}", dir);
            }
        } else {
            println!("No test data directories found.");
        }

        // This test always passes - it's just for information
        assert!(true);
    }

    /// Find directories that likely contain test data
    fn find_test_data_directories(start_dir: &str) -> Vec<String> {
        let mut test_dirs = Vec::new();

        if let Ok(entries) = fs::read_dir(start_dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        // Skip system directories
                        if matches!(dir_name, "target" | ".git" | "node_modules" | ".cargo") {
                            continue;
                        }

                        // Check if this looks like a test data directory
                        if matches!(
                            dir_name,
                            "tests" | "test_data" | "testdata" | "fixtures" | "snapshots"
                        ) {
                            if let Some(path_str) = path.to_str() {
                                test_dirs.push(path_str.to_string());
                            }
                        }

                        // Also check for provider-specific test directories
                        if dir_name.contains("test") || dir_name.contains("case") {
                            if let Some(path_str) = path.to_str() {
                                // Check if this directory contains JSON files
                                if directory_contains_json_files(&path) {
                                    test_dirs.push(path_str.to_string());
                                }
                            }
                        }
                    }

                    // Recursively search subdirectories (but not too deep to avoid performance issues)
                    if let Some(path_str) = path.to_str() {
                        if path_str.split('/').count() < 5 {
                            // Limit recursion depth
                            test_dirs.extend(find_test_data_directories(path_str));
                        }
                    }
                }
            }
        }

        test_dirs
    }

    /// Check if a directory contains JSON files
    fn directory_contains_json_files(dir: &Path) -> bool {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "json" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
