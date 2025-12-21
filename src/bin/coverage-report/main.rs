/*!
Cross-provider transformation coverage report generator.

This binary runs all cross-provider transformation tests and outputs a
markdown report showing which transformations succeed/fail.

Validates:
1. Transform doesn't error
2. Transformed output deserializes into target provider's Rust types (schema validation)
3. Key semantic fields are preserved (messages, model, tools, usage)

Usage:
    cargo run --bin generate-coverage-report
*/

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use lingua::capabilities::ProviderFormat;
use lingua::processing::transform::{transform_request, transform_response};
use lingua::providers::anthropic::generated::{CreateMessageParams, Message as AnthropicMessage};
use lingua::providers::openai::generated::{
    CreateChatCompletionRequestClass, CreateChatCompletionResponse,
};
use lingua::serde_json::{self, Value};

/// Provider definitions - ADD NEW PROVIDERS HERE
/// (directory_name, display_name, provider_format)
const PROVIDERS: &[(&str, &str, ProviderFormat)] = &[
    (
        "chat-completions",
        "ChatCompletions",
        ProviderFormat::OpenAI,
    ),
    ("anthropic", "Anthropic", ProviderFormat::Anthropic),
    ("responses", "Responses", ProviderFormat::Unknown),
    ("google", "Google", ProviderFormat::Google),
    ("bedrock", "Bedrock", ProviderFormat::Converse),
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum ValidationLevel {
    Pass,
    Fail,
}

/// Semantic fields check result
#[derive(Debug, Default)]
struct SemanticCheck {
    has_messages: bool,
    has_model: bool,
    has_tools: bool,
    has_content: bool, // for responses
    has_usage: bool,   // for responses
    has_tool_calls: bool,
}

#[derive(Debug)]
struct TransformResult {
    level: ValidationLevel,
    error: Option<String>,
}

#[derive(Debug, Default)]
struct PairResult {
    passed: usize,
    failed: usize,
    failures: Vec<(String, String)>,
}

/// Validate transformed request against target provider's schema
/// Note: Google and Bedrock types don't implement Deserialize (protobuf-generated),
/// so we skip schema validation and rely on semantic checks for those providers.
fn validate_request_schema(payload: &Value, target_format: ProviderFormat) -> Result<(), String> {
    match target_format {
        ProviderFormat::OpenAI => {
            serde_json::from_value::<CreateChatCompletionRequestClass>(payload.clone())
                .map(|_| ())
                .map_err(|e| format!("OpenAI schema: {}", e))
        }
        ProviderFormat::Anthropic => serde_json::from_value::<CreateMessageParams>(payload.clone())
            .map(|_| ())
            .map_err(|e| format!("Anthropic schema: {}", e)),
        ProviderFormat::Google | ProviderFormat::Converse => {
            // Types don't implement Deserialize - skip schema validation, use semantic checks
            Ok(())
        }
        ProviderFormat::Mistral => {
            // Mistral uses OpenAI-compatible format
            serde_json::from_value::<CreateChatCompletionRequestClass>(payload.clone())
                .map(|_| ())
                .map_err(|e| format!("Mistral schema: {}", e))
        }
        ProviderFormat::Unknown => Err("Unknown provider format".to_string()),
    }
}

/// Validate transformed response against target provider's schema
/// Note: Google and Bedrock types don't implement Deserialize, so we skip schema validation.
fn validate_response_schema(payload: &Value, target_format: ProviderFormat) -> Result<(), String> {
    match target_format {
        ProviderFormat::OpenAI => {
            serde_json::from_value::<CreateChatCompletionResponse>(payload.clone())
                .map(|_| ())
                .map_err(|e| format!("OpenAI response schema: {}", e))
        }
        ProviderFormat::Anthropic => serde_json::from_value::<AnthropicMessage>(payload.clone())
            .map(|_| ())
            .map_err(|e| format!("Anthropic response schema: {}", e)),
        ProviderFormat::Google | ProviderFormat::Converse => {
            // Types don't implement Deserialize - skip schema validation, use semantic checks
            Ok(())
        }
        ProviderFormat::Mistral => {
            serde_json::from_value::<CreateChatCompletionResponse>(payload.clone())
                .map(|_| ())
                .map_err(|e| format!("Mistral response schema: {}", e))
        }
        ProviderFormat::Unknown => Err("Unknown provider format".to_string()),
    }
}

/// Check semantic fields in a request payload (provider-agnostic)
fn check_request_semantics(payload: &Value, target_format: ProviderFormat) -> SemanticCheck {
    let mut check = SemanticCheck::default();

    match target_format {
        ProviderFormat::OpenAI | ProviderFormat::Mistral => {
            check.has_messages = payload.get("messages").map_or(false, |m| m.is_array());
            check.has_model = payload.get("model").map_or(false, |m| m.is_string());
            check.has_tools = payload.get("tools").map_or(false, |t| t.is_array());
        }
        ProviderFormat::Anthropic => {
            check.has_messages = payload.get("messages").map_or(false, |m| m.is_array());
            check.has_model = payload.get("model").map_or(false, |m| m.is_string());
            check.has_tools = payload.get("tools").map_or(false, |t| t.is_array());
        }
        ProviderFormat::Google => {
            check.has_messages = payload.get("contents").map_or(false, |c| c.is_array());
            check.has_model = true; // Model is in URL path for Google, not body
            check.has_tools = payload.get("tools").map_or(false, |t| t.is_array());
        }
        ProviderFormat::Converse => {
            check.has_messages = payload.get("messages").map_or(false, |m| m.is_array());
            check.has_model = true; // Model is in URL path for Bedrock
            check.has_tools = payload.get("toolConfig").is_some();
        }
        ProviderFormat::Unknown => {
            // Can't check semantics for unknown format
        }
    }

    check
}

/// Check semantic fields in a response payload (provider-agnostic)
fn check_response_semantics(payload: &Value, target_format: ProviderFormat) -> SemanticCheck {
    let mut check = SemanticCheck::default();

    match target_format {
        ProviderFormat::OpenAI | ProviderFormat::Mistral => {
            check.has_content = payload
                .get("choices")
                .and_then(|c| c.as_array())
                .map_or(false, |arr| !arr.is_empty());
            check.has_usage = payload.get("usage").is_some();
            check.has_tool_calls = payload
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("tool_calls"))
                .map_or(false, |t| t.is_array());
        }
        ProviderFormat::Anthropic => {
            check.has_content = payload.get("content").map_or(false, |c| c.is_array());
            check.has_usage = payload.get("usage").is_some();
            check.has_tool_calls =
                payload
                    .get("content")
                    .and_then(|c| c.as_array())
                    .map_or(false, |arr| {
                        arr.iter().any(|block| {
                            block.get("type") == Some(&Value::String("tool_use".to_string()))
                        })
                    });
        }
        ProviderFormat::Google => {
            check.has_content = payload
                .get("candidates")
                .and_then(|c| c.as_array())
                .map_or(false, |arr| !arr.is_empty());
            check.has_usage = payload.get("usageMetadata").is_some();
            check.has_tool_calls = payload
                .get("candidates")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array())
                .map_or(false, |arr| {
                    arr.iter().any(|p| p.get("functionCall").is_some())
                });
        }
        ProviderFormat::Converse => {
            check.has_content = payload.get("output").is_some();
            check.has_usage = payload.get("usage").is_some();
            check.has_tool_calls = payload
                .get("output")
                .and_then(|o| o.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
                .map_or(false, |arr| {
                    arr.iter().any(|block| block.get("toolUse").is_some())
                });
        }
        ProviderFormat::Unknown => {
            // Can't check semantics for unknown format
        }
    }

    check
}

/// Discover all test case directories in payloads/snapshots
fn discover_test_cases() -> Vec<String> {
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
fn load_payload(test_case: &str, dir_name: &str, filename: &str) -> Option<Value> {
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

fn test_request_transformation(
    test_case: &str,
    source_dir: &str,
    source_format: ProviderFormat,
    target_format: ProviderFormat,
    filename: &str,
) -> TransformResult {
    let payload = match load_payload(test_case, source_dir, filename) {
        Some(p) => p,
        None => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Source payload not found: {}", filename)),
            }
        }
    };

    match transform_request(&payload, target_format) {
        Ok(result) => {
            if result.is_pass_through() && source_format == target_format {
                return TransformResult {
                    level: ValidationLevel::Pass,
                    error: None,
                };
            }

            let transformed = result.payload_or_original(payload);

            if let Err(schema_error) = validate_request_schema(&transformed, target_format) {
                return TransformResult {
                    level: ValidationLevel::Fail,
                    error: Some(schema_error),
                };
            }

            let semantics = check_request_semantics(&transformed, target_format);
            let mut issues = vec![];

            if !semantics.has_messages {
                issues.push("missing messages");
            }
            if !semantics.has_model {
                issues.push("missing model");
            }

            if issues.is_empty() {
                TransformResult {
                    level: ValidationLevel::Pass,
                    error: None,
                }
            } else {
                TransformResult {
                    level: ValidationLevel::Fail,
                    error: Some(issues.join(", ")),
                }
            }
        }
        Err(e) => TransformResult {
            level: ValidationLevel::Fail,
            error: Some(format!("{}", e)),
        },
    }
}

fn test_response_transformation(
    test_case: &str,
    source_dir: &str,
    target_format: ProviderFormat,
    filename: &str,
) -> TransformResult {
    let payload = match load_payload(test_case, source_dir, filename) {
        Some(p) => p,
        None => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Response payload not found: {}", filename)),
            }
        }
    };

    match transform_response(&payload, target_format) {
        Ok(result) => {
            let transformed = result.payload_or_original(payload);

            if let Err(schema_error) = validate_response_schema(&transformed, target_format) {
                return TransformResult {
                    level: ValidationLevel::Fail,
                    error: Some(schema_error),
                };
            }

            let semantics = check_response_semantics(&transformed, target_format);
            let mut issues = vec![];

            if !semantics.has_content {
                issues.push("missing content");
            }
            if !semantics.has_usage {
                issues.push("missing usage");
            }

            if issues.is_empty() {
                TransformResult {
                    level: ValidationLevel::Pass,
                    error: None,
                }
            } else {
                TransformResult {
                    level: ValidationLevel::Fail,
                    error: Some(issues.join(", ")),
                }
            }
        }
        Err(e) => TransformResult {
            level: ValidationLevel::Fail,
            error: Some(format!("{}", e)),
        },
    }
}

/// Run all cross-transformation tests and collect results
fn run_all_tests() -> (
    HashMap<(usize, usize), PairResult>,
    HashMap<(usize, usize), PairResult>,
) {
    let test_cases = discover_test_cases();
    let mut request_results: HashMap<(usize, usize), PairResult> = HashMap::new();
    let mut response_results: HashMap<(usize, usize), PairResult> = HashMap::new();

    // Initialize results for all pairs
    for source_idx in 0..PROVIDERS.len() {
        for target_idx in 0..PROVIDERS.len() {
            if source_idx != target_idx {
                request_results.insert((source_idx, target_idx), PairResult::default());
                response_results.insert((source_idx, target_idx), PairResult::default());
            }
        }
    }

    // Test each source→target pair for each test case
    for test_case in &test_cases {
        for source_idx in 0..PROVIDERS.len() {
            for target_idx in 0..PROVIDERS.len() {
                if source_idx == target_idx {
                    continue;
                }

                let (source_dir, _, source_format) = PROVIDERS[source_idx];
                let (_, _, target_format) = PROVIDERS[target_idx];

                // Test first turn request
                let result = test_request_transformation(
                    test_case,
                    source_dir,
                    source_format,
                    target_format,
                    "request.json",
                );
                let pair_result = request_results.get_mut(&(source_idx, target_idx)).unwrap();

                match result.level {
                    ValidationLevel::Pass => pair_result.passed += 1,
                    ValidationLevel::Fail => {
                        pair_result.failed += 1;
                        if let Some(error) = result.error {
                            pair_result
                                .failures
                                .push((format!("{} (request)", test_case), error));
                        }
                    }
                }

                // Test followup request if exists
                let followup_result = test_request_transformation(
                    test_case,
                    source_dir,
                    source_format,
                    target_format,
                    "followup-request.json",
                );
                if followup_result
                    .error
                    .as_ref()
                    .map_or(true, |e| !e.contains("not found"))
                {
                    match followup_result.level {
                        ValidationLevel::Pass => pair_result.passed += 1,
                        ValidationLevel::Fail => {
                            pair_result.failed += 1;
                            if let Some(error) = followup_result.error {
                                pair_result
                                    .failures
                                    .push((format!("{} (followup)", test_case), error));
                            }
                        }
                    }
                }

                // Test response transformation (source response transforms to target format)
                let response_result = test_response_transformation(
                    test_case,
                    source_dir,
                    target_format,
                    "response.json",
                );
                let resp_pair_result = response_results.get_mut(&(source_idx, target_idx)).unwrap();

                if response_result
                    .error
                    .as_ref()
                    .map_or(true, |e| !e.contains("not found"))
                {
                    match response_result.level {
                        ValidationLevel::Pass => resp_pair_result.passed += 1,
                        ValidationLevel::Fail => {
                            resp_pair_result.failed += 1;
                            if let Some(error) = response_result.error {
                                resp_pair_result
                                    .failures
                                    .push((format!("{} (response)", test_case), error));
                            }
                        }
                    }
                }
            }
        }
    }

    (request_results, response_results)
}

fn format_cell(pair_result: &PairResult) -> String {
    let total = pair_result.passed + pair_result.failed;
    if total == 0 {
        return "-".to_string();
    }

    let emoji = if pair_result.failed == 0 {
        "✅"
    } else {
        "❌"
    };
    format!("{} {}/{}", emoji, pair_result.passed, total)
}

struct TableStats {
    passed: usize,
    failed: usize,
}

fn generate_table(
    results: &HashMap<(usize, usize), PairResult>,
    title: &str,
) -> (String, TableStats, Vec<(String, String, String)>) {
    let mut table = String::new();
    let mut stats = TableStats {
        passed: 0,
        failed: 0,
    };
    let mut all_failures: Vec<(String, String, String)> = Vec::new();

    table.push_str(&format!("### {}\n\n", title));
    table.push_str("| Source ↓ / Target → |");
    for (_, display_name, _) in PROVIDERS {
        table.push_str(&format!(" {} |", display_name));
    }
    table.push_str("\n|---------------------|");
    for _ in PROVIDERS {
        table.push_str("-------------|");
    }
    table.push('\n');

    for source_idx in 0..PROVIDERS.len() {
        let (_, source_display, _) = PROVIDERS[source_idx];
        table.push_str(&format!("| {} |", source_display));
        for target_idx in 0..PROVIDERS.len() {
            if source_idx == target_idx {
                table.push_str(" - |");
            } else {
                let pair_result = results.get(&(source_idx, target_idx)).unwrap();
                table.push_str(&format!(" {} |", format_cell(pair_result)));

                stats.passed += pair_result.passed;
                stats.failed += pair_result.failed;

                let (_, target_display, _) = PROVIDERS[target_idx];
                for (test_case, error) in &pair_result.failures {
                    all_failures.push((
                        format!("{} → {}", source_display, target_display),
                        test_case.clone(),
                        error.clone(),
                    ));
                }
            }
        }
        table.push('\n');
    }

    (table, stats, all_failures)
}

fn generate_report(
    request_results: &HashMap<(usize, usize), PairResult>,
    response_results: &HashMap<(usize, usize), PairResult>,
) -> String {
    let mut report = String::new();

    report.push_str("## Cross-Provider Transformation Coverage\n\n");

    let (req_table, req_stats, req_failures) =
        generate_table(request_results, "Request Transformations");
    report.push_str(&req_table);

    report.push('\n');
    let (resp_table, resp_stats, resp_failures) =
        generate_table(response_results, "Response Transformations");
    report.push_str(&resp_table);

    let total_passed = req_stats.passed + resp_stats.passed;
    let total_failed = req_stats.failed + resp_stats.failed;
    let total = total_passed + total_failed;

    let pass_percentage = if total > 0 {
        (total_passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    report.push_str("\n### Summary\n\n");
    report.push_str(&format!(
        "**{}/{} ({:.1}%)** - {} failed\n",
        total_passed, total, pass_percentage, total_failed
    ));

    let req_total = req_stats.passed + req_stats.failed;
    let resp_total = resp_stats.passed + resp_stats.failed;

    report.push_str(&format!(
        "\n**Requests:** {}/{} passed, {} failed\n",
        req_stats.passed, req_total, req_stats.failed
    ));
    report.push_str(&format!(
        "**Responses:** {}/{} passed, {} failed\n",
        resp_stats.passed, resp_total, resp_stats.failed
    ));

    // Organize issues by source provider → request/response → target
    if !req_failures.is_empty() || !resp_failures.is_empty() {
        report.push_str("\n### Issues by Source\n\n");

        // Group failures by source provider, keeping request/response separate
        let mut req_by_source: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
        let mut resp_by_source: HashMap<String, Vec<(String, String, String)>> = HashMap::new();

        for (direction, test_case, error) in req_failures {
            let source = direction
                .split(" → ")
                .next()
                .unwrap_or(&direction)
                .to_string();
            req_by_source
                .entry(source)
                .or_default()
                .push((direction, test_case, error));
        }

        for (direction, test_case, error) in resp_failures {
            let source = direction
                .split(" → ")
                .next()
                .unwrap_or(&direction)
                .to_string();
            resp_by_source
                .entry(source)
                .or_default()
                .push((direction, test_case, error));
        }

        // Get all unique sources and sort by total failure count
        let mut all_sources: HashMap<String, usize> = HashMap::new();
        for (source, failures) in &req_by_source {
            *all_sources.entry(source.clone()).or_default() += failures.len();
        }
        for (source, failures) in &resp_by_source {
            *all_sources.entry(source.clone()).or_default() += failures.len();
        }

        let mut sources: Vec<_> = all_sources.into_iter().collect();
        sources.sort_by(|a, b| b.1.cmp(&a.1));

        for (source, total_count) in sources {
            report.push_str("<details>\n");
            report.push_str(&format!(
                "<summary>❌ {} ({} issues)</summary>\n\n",
                source, total_count
            ));

            // Request transformation issues for this source
            if let Some(req_failures) = req_by_source.get(&source) {
                report.push_str("<details>\n");
                report.push_str(&format!(
                    "<summary>  📤 Request transformations ({})</summary>\n\n",
                    req_failures.len()
                ));

                // Group by target
                let mut by_target: HashMap<String, Vec<(String, String)>> = HashMap::new();
                for (direction, test_case, error) in req_failures {
                    let target = direction
                        .split(" → ")
                        .nth(1)
                        .unwrap_or("Unknown")
                        .to_string();
                    by_target
                        .entry(target)
                        .or_default()
                        .push((test_case.clone(), error.clone()));
                }

                let mut targets: Vec<_> = by_target.into_iter().collect();
                targets.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                for (target, target_failures) in targets {
                    report.push_str("<details>\n");
                    report.push_str(&format!(
                        "<summary>    → {} ({})</summary>\n\n",
                        target,
                        target_failures.len()
                    ));

                    for (test_case, error) in target_failures {
                        report.push_str(&format!("      - `{}` - {}\n", test_case, error));
                    }

                    report.push_str("\n</details>\n\n");
                }

                report.push_str("</details>\n\n");
            }

            // Response transformation issues for this source
            if let Some(resp_failures) = resp_by_source.get(&source) {
                report.push_str("<details>\n");
                report.push_str(&format!(
                    "<summary>  📥 Response transformations ({})</summary>\n\n",
                    resp_failures.len()
                ));

                // Group by target
                let mut by_target: HashMap<String, Vec<(String, String)>> = HashMap::new();
                for (direction, test_case, error) in resp_failures {
                    let target = direction
                        .split(" → ")
                        .nth(1)
                        .unwrap_or("Unknown")
                        .to_string();
                    by_target
                        .entry(target)
                        .or_default()
                        .push((test_case.clone(), error.clone()));
                }

                let mut targets: Vec<_> = by_target.into_iter().collect();
                targets.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                for (target, target_failures) in targets {
                    report.push_str("<details>\n");
                    report.push_str(&format!(
                        "<summary>    → {} ({})</summary>\n\n",
                        target,
                        target_failures.len()
                    ));

                    for (test_case, error) in target_failures {
                        report.push_str(&format!("      - `{}` - {}\n", test_case, error));
                    }

                    report.push_str("\n</details>\n\n");
                }

                report.push_str("</details>\n\n");
            }

            report.push_str("</details>\n\n");
        }
    }

    report
}

fn main() {
    let (request_results, response_results) = run_all_tests();
    let report = generate_report(&request_results, &response_results);
    println!("{}", report);
}
