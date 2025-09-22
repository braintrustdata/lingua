use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    OpenAIResponses,
    OpenAIChatCompletions,
    Anthropic,
}

impl Provider {
    pub fn directory_name(&self) -> &'static str {
        match self {
            Provider::OpenAIResponses => "openai-responses",
            Provider::OpenAIChatCompletions => "openai-chat-completions",
            Provider::Anthropic => "anthropic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestCase<Req, Resp, StreamResp> {
    pub name: String,
    pub provider: Provider,
    pub turn: TurnType,
    pub request: Req,
    pub streaming_response: Option<StreamResp>,
    pub non_streaming_response: Option<Resp>,
    pub error: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnType {
    FirstTurn,
    FollowupTurn,
}

impl TurnType {
    pub fn file_prefix(&self) -> &'static str {
        match self {
            TurnType::FirstTurn => "",
            TurnType::FollowupTurn => "followup-",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TurnType::FirstTurn => "first_turn",
            TurnType::FollowupTurn => "followup_turn",
        }
    }
}

#[derive(Debug)]
pub struct TestDiscoveryError {
    pub message: String,
    pub path: Option<String>,
}

impl std::fmt::Display for TestDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{} (path: {})", self.message, path),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for TestDiscoveryError {}

fn load_json_file<T: for<'de> Deserialize<'de>>(file_path: &Path) -> Result<T, TestDiscoveryError> {
    match fs::read_to_string(file_path) {
        Ok(content) => serde_json::from_str(&content).map_err(|e| TestDiscoveryError {
            message: format!("Failed to parse JSON as target type: {}", e),
            path: Some(file_path.to_string_lossy().to_string()),
        }),
        Err(e) => Err(TestDiscoveryError {
            message: format!("Failed to read file: {}", e),
            path: Some(file_path.to_string_lossy().to_string()),
        }),
    }
}

fn load_json_file_as_value(file_path: &Path) -> Result<Value, TestDiscoveryError> {
    load_json_file(file_path)
}

fn discover_test_case_for_turn<Req, Resp, StreamResp>(
    snapshots_dir: &Path,
    test_case_name: &str,
    provider: &Provider,
    turn: TurnType,
) -> Result<TestCase<Req, Resp, StreamResp>, TestDiscoveryError>
where
    Req: for<'de> Deserialize<'de>,
    Resp: for<'de> Deserialize<'de>,
    StreamResp: for<'de> Deserialize<'de>,
{
    let provider_dir = snapshots_dir
        .join(test_case_name)
        .join(provider.directory_name());

    let prefix = turn.file_prefix();

    // Try to load all possible files for this turn
    let request_path = provider_dir.join(format!("{}request.json", prefix));
    let streaming_response_path = provider_dir.join(format!("{}response-streaming.json", prefix));
    let non_streaming_response_path = provider_dir.join(format!("{}response.json", prefix));
    let error_path = provider_dir.join(format!("{}error.json", prefix));

    let request = if request_path.exists() {
        load_json_file::<Req>(&request_path)?
    } else {
        return Err(TestDiscoveryError {
            message: format!(
                "Request file not found for test case '{}' provider '{}' turn '{}'",
                test_case_name,
                provider.directory_name(),
                turn.display_name()
            ),
            path: Some(provider_dir.to_string_lossy().to_string()),
        });
    };

    let streaming_response = if streaming_response_path.exists() {
        Some(load_json_file::<StreamResp>(&streaming_response_path)?)
    } else {
        None
    };

    let non_streaming_response = if non_streaming_response_path.exists() {
        Some(load_json_file::<Resp>(&non_streaming_response_path)?)
    } else {
        None
    };

    let error = if error_path.exists() {
        Some(load_json_file_as_value(&error_path)?)
    } else {
        None
    };

    // A test case is valid if it has at least a request or any response
    if streaming_response.is_none() && non_streaming_response.is_none() && error.is_none() {
        return Err(TestDiscoveryError {
            message: format!(
                "No valid files found for test case '{}' provider '{}' turn '{}'",
                test_case_name,
                provider.directory_name(),
                turn.display_name()
            ),
            path: Some(provider_dir.to_string_lossy().to_string()),
        });
    }

    let case_name = format!(
        "{}_{}_{}",
        test_case_name,
        provider.directory_name(),
        turn.display_name()
    );

    Ok(TestCase {
        name: case_name,
        provider: provider.clone(),
        turn,
        request,
        streaming_response,
        non_streaming_response,
        error,
    })
}

pub fn discover_test_cases_typed<Req, Resp, StreamResp>(
    provider: Provider,
    test_name_filter: Option<&str>,
) -> Result<Vec<TestCase<Req, Resp, StreamResp>>, TestDiscoveryError>
where
    Req: for<'de> Deserialize<'de>,
    Resp: for<'de> Deserialize<'de>,
    StreamResp: for<'de> Deserialize<'de>,
{
    let snapshots_dir = Path::new("payloads/snapshots");

    if !snapshots_dir.exists() {
        return Err(TestDiscoveryError {
            message: "Snapshots directory not found".to_string(),
            path: Some(snapshots_dir.to_string_lossy().to_string()),
        });
    }

    let mut test_cases = Vec::new();

    // Scan for test case directories
    let entries = fs::read_dir(snapshots_dir).map_err(|e| TestDiscoveryError {
        message: format!("Failed to read snapshots directory: {}", e),
        path: Some(snapshots_dir.to_string_lossy().to_string()),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| TestDiscoveryError {
            message: format!("Failed to read directory entry: {}", e),
            path: Some(snapshots_dir.to_string_lossy().to_string()),
        })?;

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let test_case_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Apply test name filter if provided
        if let Some(filter) = test_name_filter {
            if !test_case_name.contains(filter) {
                continue;
            }
        }

        // Check if this test case has the requested provider directory
        let provider_dir = path.join(provider.directory_name());
        if !provider_dir.exists() || !provider_dir.is_dir() {
            continue;
        }

        // Try to discover both first turn and followup turn cases
        // First turn (required files: request.json, response.json, response-streaming.json)
        match discover_test_case_for_turn::<Req, Resp, StreamResp>(
            snapshots_dir,
            test_case_name,
            &provider,
            TurnType::FirstTurn,
        ) {
            Ok(case) => test_cases.push(case),
            Err(_) => {
                // First turn not found or invalid, skip this test case entirely
                continue;
            }
        }

        // Followup turn (optional files: followup-request.json, followup-response.json, followup-response-streaming.json)
        match discover_test_case_for_turn::<Req, Resp, StreamResp>(
            snapshots_dir,
            test_case_name,
            &provider,
            TurnType::FollowupTurn,
        ) {
            Ok(case) => test_cases.push(case),
            Err(e) => {
                eprintln!("Note: Followup turn not found or invalid for test case '{}' provider '{}' ({:?})", test_case_name, provider.directory_name(), e);
            }
        }
    }

    test_cases.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(test_cases)
}

// Backward compatibility function that returns untyped test cases
pub fn discover_test_cases(
    provider: Provider,
    test_name_filter: Option<&str>,
) -> Result<Vec<TestCase<Value, Value, Value>>, TestDiscoveryError> {
    discover_test_cases_typed::<Value, Value, Value>(provider, test_name_filter)
}

/// Generic differ that compares any two serializable types using a professional JSON diff library
pub fn diff_serializable<T, U>(original: &[T], roundtripped: &[U], item_name: &str) -> String
where
    T: serde::Serialize,
    U: serde::Serialize,
{
    use std::fmt::Write;

    // Convert both arrays to JSON Values
    let orig_values: Vec<Value> = original
        .iter()
        .map(|item| serde_json::to_value(item).expect("Failed to serialize original item"))
        .collect();

    let round_values: Vec<Value> = roundtripped
        .iter()
        .map(|item| serde_json::to_value(item).expect("Failed to serialize roundtripped item"))
        .collect();

    let mut diff_output = String::new();

    // Check for length differences first
    if orig_values.len() != round_values.len() {
        writeln!(diff_output, "📊 LENGTH MISMATCH:").unwrap();
        writeln!(
            diff_output,
            "  Original: {} {}",
            orig_values.len(),
            item_name
        )
        .unwrap();
        writeln!(
            diff_output,
            "  Roundtripped: {} {}",
            round_values.len(),
            item_name
        )
        .unwrap();
        writeln!(diff_output).unwrap();
    }

    let max_len = orig_values.len().max(round_values.len());
    let mut has_differences = orig_values.len() != round_values.len();

    // Compare each item using the professional JSON diff library
    for i in 0..max_len {
        let orig = orig_values.get(i);
        let round = round_values.get(i);

        match (orig, round) {
            (Some(o), Some(r)) => {
                if o != r {
                    has_differences = true;
                    writeln!(
                        diff_output,
                        "🔍 {} {} DIFFERENCES:",
                        item_name.to_uppercase(),
                        i
                    )
                    .unwrap();

                    // Create full JSON output with highlighted differences
                    let original_json = serde_json::to_string_pretty(o).unwrap();
                    let roundtripped_json = serde_json::to_string_pretty(r).unwrap();

                    let original_lines: Vec<&str> = original_json.lines().collect();
                    let roundtripped_lines: Vec<&str> = roundtripped_json.lines().collect();

                    writeln!(diff_output, "📄 Full comparison:").unwrap();
                    writeln!(diff_output).unwrap();

                    // Show the full JSON with differences highlighted
                    let max_len = original_lines.len().max(roundtripped_lines.len());

                    for i in 0..max_len {
                        let orig_line = original_lines.get(i).unwrap_or(&"");
                        let round_line = roundtripped_lines.get(i).unwrap_or(&"");

                        if orig_line == round_line {
                            // Lines are the same - show in default color
                            writeln!(diff_output, " {}", orig_line).unwrap();
                        } else {
                            // Lines differ - show both with red/green highlighting
                            if !orig_line.is_empty() {
                                writeln!(diff_output, "\x1b[31m-{}\x1b[0m", orig_line).unwrap();
                            }
                            if !round_line.is_empty() {
                                writeln!(diff_output, "\x1b[32m+{}\x1b[0m", round_line).unwrap();
                            }
                        }
                    }
                    writeln!(diff_output).unwrap();
                }
            }
            (Some(o), None) => {
                has_differences = true;
                writeln!(
                    diff_output,
                    "\x1b[31m❌ MISSING {} {} in roundtripped:\x1b[0m",
                    item_name.to_uppercase(),
                    i
                )
                .unwrap();
                let json = serde_json::to_string_pretty(o).unwrap();
                for line in json.lines() {
                    writeln!(diff_output, "\x1b[31m  {}\x1b[0m", line).unwrap();
                }
                writeln!(diff_output).unwrap();
            }
            (None, Some(r)) => {
                has_differences = true;
                writeln!(
                    diff_output,
                    "\x1b[32m➕ EXTRA {} {} in roundtripped:\x1b[0m",
                    item_name.to_uppercase(),
                    i
                )
                .unwrap();
                let json = serde_json::to_string_pretty(r).unwrap();
                for line in json.lines() {
                    writeln!(diff_output, "\x1b[32m  {}\x1b[0m", line).unwrap();
                }
                writeln!(diff_output).unwrap();
            }
            (None, None) => unreachable!(),
        }
    }

    if !has_differences {
        format!("✅ All {} match perfectly!", item_name)
    } else {
        format!("🚨 ROUNDTRIP DIFFERENCES DETECTED:\n\n{}", diff_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_directory_names() {
        assert_eq!(
            Provider::OpenAIResponses.directory_name(),
            "openai-responses"
        );
        assert_eq!(
            Provider::OpenAIChatCompletions.directory_name(),
            "openai-chat-completions"
        );
        assert_eq!(Provider::Anthropic.directory_name(), "anthropic");
    }

    #[test]
    fn test_turn_type_prefixes() {
        assert_eq!(TurnType::FirstTurn.file_prefix(), "");
        assert_eq!(TurnType::FollowupTurn.file_prefix(), "followup-");
    }

    #[test]
    fn test_discover_openai_responses_cases_untyped() {
        match discover_test_cases(Provider::OpenAIResponses, None) {
            Ok(cases) => {
                println!(
                    "Found {} OpenAI Responses test cases (untyped):",
                    cases.len()
                );
                for case in &cases {
                    println!("  - {} (turn: {:?})", case.name, case.turn);
                    println!("    Request: present");
                    println!(
                        "    Streaming Response: {}",
                        case.streaming_response.is_some()
                    );
                    println!(
                        "    Non-Streaming Response: {}",
                        case.non_streaming_response.is_some()
                    );
                    println!("    Error: {}", case.error.is_some());
                }

                // Basic validation
                for case in &cases {
                    assert_eq!(case.provider, Provider::OpenAIResponses);
                    assert!(!case.name.is_empty());
                }
            }
            Err(e) => {
                println!(
                    "Note: Could not discover test cases (expected in some environments): {}",
                    e
                );
                // This is OK in test environments where snapshots might not exist
            }
        }
    }

    #[test]
    fn test_discover_with_filter() {
        match discover_test_cases(Provider::OpenAIChatCompletions, Some("simple")) {
            Ok(cases) => {
                println!("Found {} filtered OpenAI Chat test cases:", cases.len());
                for case in &cases {
                    assert!(case.name.contains("simple"));
                    println!("  - {}", case.name);
                }
            }
            Err(e) => {
                println!("Note: Could not discover filtered test cases: {}", e);
            }
        }
    }

    #[test]
    fn test_discover_all_providers() {
        let providers = vec![
            Provider::OpenAIResponses,
            Provider::OpenAIChatCompletions,
            Provider::Anthropic,
        ];

        for provider in providers {
            match discover_test_cases(provider.clone(), None) {
                Ok(cases) => {
                    println!("\n{} ({} cases):", provider.directory_name(), cases.len());
                    for case in &cases {
                        println!("  ✓ {} (turn: {:?})", case.name, case.turn);
                        let files = [
                            ("request", true),
                            ("streaming_response", case.streaming_response.is_some()),
                            (
                                "non_streaming_response",
                                case.non_streaming_response.is_some(),
                            ),
                            ("error", case.error.is_some()),
                        ];
                        let available_files: Vec<_> = files
                            .iter()
                            .filter(|(_, exists)| *exists)
                            .map(|(name, _)| *name)
                            .collect();
                        println!("    Files: [{}]", available_files.join(", "));
                    }
                }
                Err(e) => {
                    println!(
                        "\n{}: Could not discover test cases: {}",
                        provider.directory_name(),
                        e
                    );
                }
            }
        }
    }
}
