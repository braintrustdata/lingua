/*!
Report generation for coverage-report.
*/

use std::collections::HashMap;

use lingua::processing::adapters::ProviderAdapter;

use crate::runner::RoundtripResults;
use crate::types::{PairResult, RoundtripResult, TableStats};

pub fn format_cell(pair_result: &PairResult) -> String {
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

pub fn generate_table(
    results: &HashMap<(usize, usize), PairResult>,
    adapters: &[Box<dyn ProviderAdapter>],
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
    for adapter in adapters {
        table.push_str(&format!(" {} |", adapter.display_name()));
    }
    table.push_str("\n|---------------------|");
    for _ in adapters {
        table.push_str("-------------|");
    }
    table.push('\n');

    for (source_idx, source) in adapters.iter().enumerate() {
        table.push_str(&format!("| {} |", source.display_name()));
        for (target_idx, target) in adapters.iter().enumerate() {
            if source_idx == target_idx {
                table.push_str(" - |");
            } else {
                let pair_result = results.get(&(source_idx, target_idx)).unwrap();
                table.push_str(&format!(" {} |", format_cell(pair_result)));

                stats.passed += pair_result.passed;
                stats.failed += pair_result.failed;

                for (test_case, error) in &pair_result.failures {
                    all_failures.push((
                        format!("{} → {}", source.display_name(), target.display_name()),
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

// ============================================================================
// Roundtrip report section
// ============================================================================

fn format_roundtrip_cell(passed: usize, failed: usize) -> String {
    let total = passed + failed;
    if total == 0 {
        return "-".to_string();
    }
    let emoji = if failed == 0 { "✅" } else { "❌" };
    format!("{} {}/{}", emoji, passed, total)
}

fn format_roundtrip_diff(result: &RoundtripResult) -> String {
    let mut output = String::new();

    if let Some(error) = &result.error {
        output.push_str(&format!("{}\n", error));
    }

    if let Some(diff) = &result.diff {
        if !diff.lost_fields.is_empty() {
            output.push_str("        Lost: ");
            output.push_str(&diff.lost_fields.join(", "));
            output.push('\n');
        }
        if !diff.added_fields.is_empty() {
            output.push_str("        Added: ");
            output.push_str(&diff.added_fields.join(", "));
            output.push('\n');
        }
        if !diff.changed_fields.is_empty() {
            output.push_str("        Changed:\n");
            for (path, original, roundtripped) in &diff.changed_fields {
                // Truncate long values
                let orig_display = if original.len() > 50 {
                    format!("{}...", &original[..47])
                } else {
                    original.clone()
                };
                let round_display = if roundtripped.len() > 50 {
                    format!("{}...", &roundtripped[..47])
                } else {
                    roundtripped.clone()
                };
                output.push_str(&format!(
                    "          - `{}`: {} → {}\n",
                    path, orig_display, round_display
                ));
            }
        }
    }

    output
}

/// Generate the roundtrip transform coverage section of the report.
pub fn generate_roundtrip_section(
    roundtrip_results: &RoundtripResults,
    adapters: &[Box<dyn ProviderAdapter>],
) -> String {
    let mut report = String::new();

    report.push_str("## Roundtrip Transform Coverage\n\n");
    report.push_str("Tests Provider → Universal → Provider fidelity.\n\n");

    // Summary table
    report.push_str("### Summary\n\n");
    report.push_str("| Provider | Requests | Responses |\n");
    report.push_str("|----------|----------|----------|\n");

    let mut total_req_passed = 0;
    let mut total_req_failed = 0;
    let mut total_resp_passed = 0;
    let mut total_resp_failed = 0;

    for (adapter_idx, adapter) in adapters.iter().enumerate() {
        if let Some(result) = roundtrip_results.get(&adapter_idx) {
            let req_cell = format_roundtrip_cell(result.request_passed, result.request_failed);
            let resp_cell = format_roundtrip_cell(result.response_passed, result.response_failed);
            report.push_str(&format!(
                "| {} | {} | {} |\n",
                adapter.display_name(),
                req_cell,
                resp_cell
            ));

            total_req_passed += result.request_passed;
            total_req_failed += result.request_failed;
            total_resp_passed += result.response_passed;
            total_resp_failed += result.response_failed;
        }
    }

    let total_passed = total_req_passed + total_resp_passed;
    let total_failed = total_req_failed + total_resp_failed;
    let total = total_passed + total_failed;
    let pass_percentage = if total > 0 {
        (total_passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    report.push_str(&format!(
        "\n**{}/{} ({:.1}%)** - {} failed\n",
        total_passed, total, pass_percentage, total_failed
    ));

    // Issues by provider
    let has_failures = roundtrip_results.values().any(|r| r.total_failed() > 0);

    if has_failures {
        report.push_str("\n### Issues by Provider\n\n");

        // Sort providers by failure count
        let mut providers_with_failures: Vec<_> = adapters
            .iter()
            .enumerate()
            .filter_map(|(idx, adapter)| {
                roundtrip_results
                    .get(&idx)
                    .filter(|r| r.total_failed() > 0)
                    .map(|r| (adapter, r))
            })
            .collect();
        providers_with_failures.sort_by(|a, b| b.1.total_failed().cmp(&a.1.total_failed()));

        for (adapter, result) in providers_with_failures {
            let total_issues = result.total_failed();
            report.push_str("<details>\n");
            report.push_str(&format!(
                "<summary>❌ {} ({} issues)</summary>\n\n",
                adapter.display_name(),
                total_issues
            ));

            // Request roundtrip issues
            if !result.request_failures.is_empty() {
                report.push_str(&format!(
                    "**Request roundtrip issues ({}):**\n\n",
                    result.request_failures.len()
                ));
                for (test_case, roundtrip_result) in &result.request_failures {
                    report.push_str(&format!("- `{}`\n", test_case));
                    let diff_output = format_roundtrip_diff(roundtrip_result);
                    if !diff_output.is_empty() {
                        report.push_str(&diff_output);
                    }
                }
                report.push('\n');
            }

            // Response roundtrip issues
            if !result.response_failures.is_empty() {
                report.push_str(&format!(
                    "**Response roundtrip issues ({}):**\n\n",
                    result.response_failures.len()
                ));
                for (test_case, roundtrip_result) in &result.response_failures {
                    report.push_str(&format!("- `{}`\n", test_case));
                    let diff_output = format_roundtrip_diff(roundtrip_result);
                    if !diff_output.is_empty() {
                        report.push_str(&diff_output);
                    }
                }
                report.push('\n');
            }

            report.push_str("</details>\n\n");
        }
    }

    report
}

pub fn generate_report(
    request_results: &HashMap<(usize, usize), PairResult>,
    response_results: &HashMap<(usize, usize), PairResult>,
    streaming_results: &HashMap<(usize, usize), PairResult>,
    roundtrip_results: &RoundtripResults,
    adapters: &[Box<dyn ProviderAdapter>],
) -> String {
    let mut report = String::new();

    report.push_str("## Cross-Provider Transformation Coverage\n\n");

    let (req_table, req_stats, req_failures) =
        generate_table(request_results, adapters, "Request Transformations");
    report.push_str(&req_table);

    report.push('\n');
    let (resp_table, resp_stats, resp_failures) =
        generate_table(response_results, adapters, "Response Transformations");
    report.push_str(&resp_table);

    report.push('\n');
    let (stream_table, stream_stats, stream_failures) = generate_table(
        streaming_results,
        adapters,
        "Streaming Response Transformations",
    );
    report.push_str(&stream_table);

    let total_passed = req_stats.passed + resp_stats.passed + stream_stats.passed;
    let total_failed = req_stats.failed + resp_stats.failed + stream_stats.failed;
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
    let stream_total = stream_stats.passed + stream_stats.failed;

    report.push_str(&format!(
        "\n**Requests:** {}/{} passed, {} failed\n",
        req_stats.passed, req_total, req_stats.failed
    ));
    report.push_str(&format!(
        "**Responses:** {}/{} passed, {} failed\n",
        resp_stats.passed, resp_total, resp_stats.failed
    ));
    report.push_str(&format!(
        "**Streaming:** {}/{} passed, {} failed\n",
        stream_stats.passed, stream_total, stream_stats.failed
    ));

    // Organize issues by source provider → request/response/streaming → target
    if !req_failures.is_empty() || !resp_failures.is_empty() || !stream_failures.is_empty() {
        report.push_str("\n### Issues by Source\n\n");

        // Group failures by source provider, keeping request/response/streaming separate
        let mut req_by_source: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
        let mut resp_by_source: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
        let mut stream_by_source: HashMap<String, Vec<(String, String, String)>> = HashMap::new();

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

        for (direction, test_case, error) in stream_failures {
            let source = direction
                .split(" → ")
                .next()
                .unwrap_or(&direction)
                .to_string();
            stream_by_source
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
        for (source, failures) in &stream_by_source {
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

            // Streaming transformation issues for this source
            if let Some(stream_failures) = stream_by_source.get(&source) {
                report.push_str("<details>\n");
                report.push_str(&format!(
                    "<summary>  🌊 Streaming transformations ({})</summary>\n\n",
                    stream_failures.len()
                ));

                // Group by target
                let mut by_target: HashMap<String, Vec<(String, String)>> = HashMap::new();
                for (direction, test_case, error) in stream_failures {
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

    // Add roundtrip section
    report.push('\n');
    report.push_str(&generate_roundtrip_section(roundtrip_results, adapters));

    report
}
