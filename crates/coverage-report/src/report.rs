/*!
Report generation for coverage-report.
*/

use std::collections::HashMap;

use lingua::processing::adapters::ProviderAdapter;

use crate::compact;
use crate::types::{
    CoverageSelection, FailureWithDiff, OutputFormat, PairResult, RoundtripDiff, TableOutput,
    TableStats,
};

pub fn format_cell(pair_result: &PairResult) -> String {
    // Working = passed + limitations (both represent successful translations)
    let working = pair_result.passed + pair_result.limitations;
    let total = working + pair_result.failed;
    if total == 0 {
        return "-".to_string();
    }

    let emoji = if pair_result.failed == 0 {
        "✅"
    } else {
        "❌"
    };
    format!("{} {}/{}", emoji, working, total)
}

/// Truncate a string to a maximum number of characters, adding "..." if truncated.
/// Uses character count, not byte count, to avoid UTF-8 panics on multi-byte characters.
fn truncate_display(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Render a simple link to the expected differences JSON file.
fn render_limitations_link(count: usize, transformation_type: &str) -> String {
    if count == 0 {
        return String::new();
    }

    // Map transformation type to JSON filename
    let json_file = match transformation_type {
        "Request" => "requests_expected_differences.json",
        "Response" => "responses_expected_differences.json",
        "Streaming" | "Streaming Response" => "streaming_expected_differences.json",
        _ => "expected_differences.json",
    };

    format!(
        "\n⚠️ {} tests have expected differences — [View {}](src/{})\n",
        count, json_file, json_file
    )
}

/// Generate a coverage table for a specific transformation type.
pub fn generate_table(
    results: &HashMap<(usize, usize), PairResult>,
    adapters: &[Box<dyn ProviderAdapter>],
    title: &str,
) -> TableOutput {
    let mut table = String::new();
    let mut stats = TableStats {
        passed: 0,
        failed: 0,
        limitations: 0,
    };
    let mut all_failures: Vec<FailureWithDiff> = Vec::new();
    let mut all_limitations: Vec<(String, String, String, Option<RoundtripDiff>)> = Vec::new();

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
            if let Some(pair_result) = results.get(&(source_idx, target_idx)) {
                table.push_str(&format!(" {} |", format_cell(pair_result)));

                stats.passed += pair_result.passed;
                stats.failed += pair_result.failed;
                stats.limitations += pair_result.limitations;

                for (test_case, error, diff) in &pair_result.failures {
                    all_failures.push((
                        format!("{} → {}", source.display_name(), target.display_name()),
                        test_case.clone(),
                        error.clone(),
                        diff.clone(),
                    ));
                }

                for (test_case, error, limitation_diff) in &pair_result.limitation_details {
                    all_limitations.push((
                        format!("{} → {}", source.display_name(), target.display_name()),
                        test_case.clone(),
                        error.clone(),
                        limitation_diff.clone(),
                    ));
                }
            } else {
                // Pair was filtered out
                table.push_str(" - |");
            }
        }
        table.push('\n');
    }

    TableOutput {
        table_markdown: table,
        stats,
        failures: all_failures,
        limitations: all_limitations,
    }
}

fn format_diff(diff: &Option<RoundtripDiff>) -> String {
    match diff {
        Some(d) if !d.is_empty() => {
            let mut output = String::new();
            if !d.lost_fields.is_empty() {
                output.push_str("\n        Lost: ");
                output.push_str(&d.lost_fields.join(", "));
            }
            if !d.added_fields.is_empty() {
                output.push_str("\n        Added: ");
                output.push_str(&d.added_fields.join(", "));
            }
            if !d.changed_fields.is_empty() {
                output.push_str("\n        Changed:");
                for (path, original, roundtripped) in &d.changed_fields {
                    let orig_display = truncate_display(original, 50);
                    let round_display = truncate_display(roundtripped, 50);
                    output.push_str(&format!(
                        "\n          - `{}`: {} → {}",
                        path, orig_display, round_display
                    ));
                }
            }
            output
        }
        _ => String::new(),
    }
}

pub fn generate_report(
    request_results: &HashMap<(usize, usize), PairResult>,
    response_results: &HashMap<(usize, usize), PairResult>,
    streaming_results: &HashMap<(usize, usize), PairResult>,
    adapters: &[Box<dyn ProviderAdapter>],
    selection: CoverageSelection,
    format: OutputFormat,
) -> String {
    match format {
        OutputFormat::Markdown => generate_markdown_report(
            request_results,
            response_results,
            streaming_results,
            adapters,
            selection,
        ),
        OutputFormat::Compact => generate_compact_report(
            request_results,
            response_results,
            streaming_results,
            adapters,
            selection,
        ),
    }
}

fn generate_compact_report(
    request_results: &HashMap<(usize, usize), PairResult>,
    response_results: &HashMap<(usize, usize), PairResult>,
    streaming_results: &HashMap<(usize, usize), PairResult>,
    adapters: &[Box<dyn ProviderAdapter>],
    selection: CoverageSelection,
) -> String {
    let mut report = String::new();

    // Collect stats
    let req_stats = compact::collect_stats(request_results);
    let resp_stats = compact::collect_stats(response_results);
    let stream_stats = compact::collect_stats(streaming_results);

    // Header with stats
    report.push_str(&compact::generate_compact_header(
        &req_stats,
        &resp_stats,
        &stream_stats,
    ));

    // Collect all failures
    let mut all_failures = Vec::new();
    if selection.requests {
        all_failures.extend(compact::collect_failures(request_results, adapters));
    }
    if selection.responses {
        all_failures.extend(compact::collect_failures(response_results, adapters));
    }
    if selection.streaming {
        all_failures.extend(compact::collect_failures(streaming_results, adapters));
    }

    // Deduplicated failures section
    report.push_str(&compact::generate_compact_failures(&all_failures));

    // Collect all limitations
    let mut all_limitations = Vec::new();
    if selection.requests {
        all_limitations.extend(compact::collect_limitations(request_results, adapters));
    }
    if selection.responses {
        all_limitations.extend(compact::collect_limitations(response_results, adapters));
    }
    if selection.streaming {
        all_limitations.extend(compact::collect_limitations(streaming_results, adapters));
    }

    // Limitations section
    report.push_str(&compact::generate_compact_limitations(&all_limitations));

    report
}

fn generate_markdown_report(
    request_results: &HashMap<(usize, usize), PairResult>,
    response_results: &HashMap<(usize, usize), PairResult>,
    streaming_results: &HashMap<(usize, usize), PairResult>,
    adapters: &[Box<dyn ProviderAdapter>],
    selection: CoverageSelection,
) -> String {
    let mut report = String::new();

    report.push_str("## Transformation Coverage\n\n");

    // Add explanatory paragraph about test semantics
    report.push_str("Tests format interoperability between providers. ");
    report.push_str(
        "Diagonal cells (e.g., ChatCompletions→ChatCompletions) test roundtrip fidelity. ",
    );
    report.push_str("Off-diagonal cells test cross-provider translation.\n\n");

    let mut req_stats = TableStats {
        passed: 0,
        failed: 0,
        limitations: 0,
    };
    let mut resp_stats = TableStats {
        passed: 0,
        failed: 0,
        limitations: 0,
    };
    let mut stream_stats = TableStats {
        passed: 0,
        failed: 0,
        limitations: 0,
    };

    let mut req_failures: Vec<FailureWithDiff> = Vec::new();
    let mut resp_failures: Vec<FailureWithDiff> = Vec::new();
    let mut stream_failures: Vec<FailureWithDiff> = Vec::new();

    let mut has_table = false;
    if selection.requests {
        let output = generate_table(request_results, adapters, "Request Transformations");
        report.push_str(&output.table_markdown);
        report.push_str(&render_limitations_link(
            output.stats.limitations,
            "Request",
        ));
        req_stats = output.stats;
        req_failures = output.failures;
        has_table = true;
    }

    if selection.responses {
        if has_table {
            report.push('\n');
        }
        let output = generate_table(response_results, adapters, "Response Transformations");
        report.push_str(&output.table_markdown);
        report.push_str(&render_limitations_link(
            output.stats.limitations,
            "Response",
        ));
        resp_stats = output.stats;
        resp_failures = output.failures;
        has_table = true;
    }

    if selection.streaming {
        if has_table {
            report.push('\n');
        }
        let output = generate_table(
            streaming_results,
            adapters,
            "Streaming Response Transformations",
        );
        report.push_str(&output.table_markdown);
        report.push_str(&render_limitations_link(
            output.stats.limitations,
            "Streaming",
        ));
        stream_stats = output.stats;
        stream_failures = output.failures;
    }

    let total_passed = req_stats.passed + resp_stats.passed + stream_stats.passed;
    let total_failed = req_stats.failed + resp_stats.failed + stream_stats.failed;
    let total_limitations =
        req_stats.limitations + resp_stats.limitations + stream_stats.limitations;

    // "Working" = passed + limitations (both represent successful translations)
    let total_working = total_passed + total_limitations;
    let working_total = total_working + total_failed;
    let working_percentage = if working_total > 0 {
        (total_working as f64 / working_total as f64) * 100.0
    } else {
        0.0
    };

    report.push_str("\n### Summary\n\n");
    report.push_str(&format!(
        "**{}/{} ({:.1}%) working** [{} full + {} limited] - {} failed\n",
        total_working,
        working_total,
        working_percentage,
        total_passed,
        total_limitations,
        total_failed
    ));

    if selection.requests {
        let req_working = req_stats.passed + req_stats.limitations;
        let req_total = req_working + req_stats.failed;
        report.push_str(&format!(
            "\n**Requests:** {}/{} working [{} full + {} limited], {} failed\n",
            req_working, req_total, req_stats.passed, req_stats.limitations, req_stats.failed
        ));
    }
    if selection.responses {
        let resp_working = resp_stats.passed + resp_stats.limitations;
        let resp_total = resp_working + resp_stats.failed;
        report.push_str(&format!(
            "**Responses:** {}/{} working [{} full + {} limited], {} failed\n",
            resp_working, resp_total, resp_stats.passed, resp_stats.limitations, resp_stats.failed
        ));
    }
    if selection.streaming {
        let stream_working = stream_stats.passed + stream_stats.limitations;
        let stream_total = stream_working + stream_stats.failed;
        report.push_str(&format!(
            "**Streaming:** {}/{} working [{} full + {} limited], {} failed\n",
            stream_working,
            stream_total,
            stream_stats.passed,
            stream_stats.limitations,
            stream_stats.failed
        ));
    }

    // Organize issues by source provider → request/response/streaming → target
    if !req_failures.is_empty() || !resp_failures.is_empty() || !stream_failures.is_empty() {
        report.push_str("\n### Issues by Source\n\n");

        // Group failures by source provider, keeping request/response/streaming separate
        let mut req_by_source: HashMap<String, Vec<FailureWithDiff>> = HashMap::new();
        let mut resp_by_source: HashMap<String, Vec<FailureWithDiff>> = HashMap::new();
        let mut stream_by_source: HashMap<String, Vec<FailureWithDiff>> = HashMap::new();

        for (direction, test_case, error, diff) in req_failures {
            let source = direction
                .split(" → ")
                .next()
                .unwrap_or(&direction)
                .to_string();
            req_by_source
                .entry(source)
                .or_default()
                .push((direction, test_case, error, diff));
        }

        for (direction, test_case, error, diff) in resp_failures {
            let source = direction
                .split(" → ")
                .next()
                .unwrap_or(&direction)
                .to_string();
            resp_by_source
                .entry(source)
                .or_default()
                .push((direction, test_case, error, diff));
        }

        for (direction, test_case, error, diff) in stream_failures {
            let source = direction
                .split(" → ")
                .next()
                .unwrap_or(&direction)
                .to_string();
            stream_by_source
                .entry(source)
                .or_default()
                .push((direction, test_case, error, diff));
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
                let mut by_target: HashMap<String, Vec<(String, String, Option<RoundtripDiff>)>> =
                    HashMap::new();
                for (direction, test_case, error, diff) in req_failures {
                    let target = direction
                        .split(" → ")
                        .nth(1)
                        .unwrap_or("Unknown")
                        .to_string();
                    by_target.entry(target).or_default().push((
                        test_case.clone(),
                        error.clone(),
                        diff.clone(),
                    ));
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

                    for (test_case, error, diff) in target_failures {
                        report.push_str(&format!(
                            "      - `{}` - {}{}\n",
                            test_case,
                            compact::truncate_str(&error, 200),
                            format_diff(&diff)
                        ));
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
                let mut by_target: HashMap<String, Vec<(String, String, Option<RoundtripDiff>)>> =
                    HashMap::new();
                for (direction, test_case, error, diff) in resp_failures {
                    let target = direction
                        .split(" → ")
                        .nth(1)
                        .unwrap_or("Unknown")
                        .to_string();
                    by_target.entry(target).or_default().push((
                        test_case.clone(),
                        error.clone(),
                        diff.clone(),
                    ));
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

                    for (test_case, error, diff) in target_failures {
                        report.push_str(&format!(
                            "      - `{}` - {}{}\n",
                            test_case,
                            compact::truncate_str(&error, 200),
                            format_diff(&diff)
                        ));
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
                let mut by_target: HashMap<String, Vec<(String, String, Option<RoundtripDiff>)>> =
                    HashMap::new();
                for (direction, test_case, error, diff) in stream_failures {
                    let target = direction
                        .split(" → ")
                        .nth(1)
                        .unwrap_or("Unknown")
                        .to_string();
                    by_target.entry(target).or_default().push((
                        test_case.clone(),
                        error.clone(),
                        diff.clone(),
                    ));
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

                    for (test_case, error, diff) in target_failures {
                        report.push_str(&format!(
                            "      - `{}` - {}{}\n",
                            test_case,
                            compact::truncate_str(&error, 200),
                            format_diff(&diff)
                        ));
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
