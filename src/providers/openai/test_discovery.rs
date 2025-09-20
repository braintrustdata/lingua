use std::fs;

pub fn discover_openai_test_cases() -> Result<Vec<String>, std::io::Error> {
    let snapshots_dir = "payloads/snapshots";
    let mut test_cases = Vec::new();

    for entry in fs::read_dir(snapshots_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with("openai-responses") && filename.ends_with(".json") {
                // Extract test case name (remove prefix and suffix)
                if let Some(case_name) = filename
                    .strip_prefix("openai-responses-")
                    .and_then(|s| s.strip_suffix(".json"))
                {
                    if !test_cases.contains(&case_name.to_string()) {
                        test_cases.push(case_name.to_string());
                    }
                }
            }
        }
    }

    test_cases.sort();
    Ok(test_cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_openai_cases() {
        match discover_openai_test_cases() {
            Ok(cases) => {
                println!("Found {} OpenAI test cases:", cases.len());
                for case in &cases {
                    println!("  - {}", case);
                }

                // Just verify we found some cases
                assert!(
                    !cases.is_empty(),
                    "Should find at least some openai-responses test cases"
                );
            }
            Err(e) => {
                println!("Failed to discover test cases: {}", e);
                panic!("Test case discovery failed");
            }
        }
    }
}
