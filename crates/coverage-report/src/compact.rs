/*!
Compact (token-optimized) report generation.

This module generates a condensed report format that minimizes token usage by:
- Using provider abbreviations (oai, ant, ggl, bed, rsp)
- Deduplicating errors by pattern and showing counts
- Removing HTML/markdown formatting overhead
- Using flat structure instead of nested sections
*/

use std::collections::HashMap;

use lingua::processing::adapters::ProviderAdapter;

use crate::types::{FailureWithDiff, PairResult, RoundtripDiff, TableStats};

/// Abbreviate provider name for compact output.
pub fn abbrev(name: &str) -> &'static str {
    match name {
        "Responses" => "rsp",
        "ChatCompletions" => "oai",
        "Anthropic" => "ant",
        "Google" => "ggl",
        "Bedrock" => "bed",
        _ => "???",
    }
}

/// Error pattern for deduplication.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ErrorPattern {
    pub pattern: String,
    pub category: char,
}

/// Group of test cases with the same error pattern.
#[derive(Debug)]
pub struct PatternGroup {
    pub pattern: ErrorPattern,
    pub by_direction: HashMap<String, Vec<String>>,
    pub total_count: usize,
}

/// Truncate a string to a maximum number of characters, adding "..." if truncated.
/// Uses character count, not byte count, to avoid UTF-8 panics.
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Normalize field path for grouping (collapse array indices, truncate long paths).
fn normalize_field_path(path: &str) -> String {
    // Replace array indices with [*]
    let mut result = String::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '[' {
            result.push('[');
            // Skip digits until ]
            let mut is_numeric = true;
            let mut inner = String::new();
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    break;
                }
                inner.push(chars.next().unwrap());
                if !inner.chars().last().unwrap().is_ascii_digit() {
                    is_numeric = false;
                }
            }
            if is_numeric && !inner.is_empty() {
                result.push('*');
            } else {
                result.push_str(&inner);
            }
        } else {
            result.push(c);
        }
    }

    // Truncate very long paths (character-safe)
    truncate_str(&result, 40)
}

/// Normalize field list for pattern matching.
fn normalize_field_list(fields: &[String]) -> String {
    if fields.len() <= 3 {
        fields
            .iter()
            .map(|f| normalize_field_path(f))
            .collect::<Vec<_>>()
            .join(",")
    } else {
        let first_two: Vec<_> = fields
            .iter()
            .take(2)
            .map(|f| normalize_field_path(f))
            .collect();
        format!("{}...(+{})", first_two.join(","), fields.len() - 2)
    }
}

/// Normalize error message for pattern grouping.
fn normalize_error_message(error: &str) -> String {
    truncate_str(error, 60)
}

/// Extract error pattern from a failure.
pub fn extract_pattern(error: &str, diff: &Option<RoundtripDiff>) -> ErrorPattern {
    if let Some(d) = diff {
        if !d.lost_fields.is_empty() {
            let fields = normalize_field_list(&d.lost_fields);
            return ErrorPattern {
                pattern: format!("L:{}", fields),
                category: 'L',
            };
        }
        if !d.added_fields.is_empty() {
            let fields = normalize_field_list(&d.added_fields);
            return ErrorPattern {
                pattern: format!("A:{}", fields),
                category: 'A',
            };
        }
        if !d.changed_fields.is_empty() {
            let fields: Vec<_> = d
                .changed_fields
                .iter()
                .map(|(path, _, _)| normalize_field_path(path))
                .collect();
            let fields_str = if fields.len() <= 3 {
                fields.join(",")
            } else {
                format!("{}...(+{})", fields[..2].join(","), fields.len() - 2)
            };
            return ErrorPattern {
                pattern: format!("C:{}", fields_str),
                category: 'C',
            };
        }
    }

    let normalized = normalize_error_message(error);
    ErrorPattern {
        pattern: normalized,
        category: 'E',
    }
}

/// Compact test case names using glob patterns where possible.
fn compact_test_names(names: &[String]) -> String {
    if names.len() <= 2 {
        return names.join(",");
    }
    format!("{}...(+{})", names[0], names.len() - 1)
}

/// Group failures by error pattern.
pub fn group_failures(failures: &[FailureWithDiff]) -> Vec<PatternGroup> {
    let mut groups: HashMap<ErrorPattern, PatternGroup> = HashMap::new();

    for (direction, test_case, error, diff) in failures {
        let pattern = extract_pattern(error, diff);

        let group = groups
            .entry(pattern.clone())
            .or_insert_with(|| PatternGroup {
                pattern,
                by_direction: HashMap::new(),
                total_count: 0,
            });

        group
            .by_direction
            .entry(direction.clone())
            .or_default()
            .push(test_case.clone());
        group.total_count += 1;
    }

    // Sort by count descending
    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by(|a, b| b.total_count.cmp(&a.total_count));
    groups
}

/// Generate compact report header with stats.
pub fn generate_compact_header(
    req_stats: &TableStats,
    resp_stats: &TableStats,
    stream_stats: &TableStats,
) -> String {
    let mut output = String::new();
    output.push_str("# Coverage (compact)\n");

    let total_passed = req_stats.passed + resp_stats.passed + stream_stats.passed;
    let total_failed = req_stats.failed + resp_stats.failed + stream_stats.failed;
    let total_lim = req_stats.limitations + resp_stats.limitations + stream_stats.limitations;
    let total_working = total_passed + total_lim;
    let total = total_working + total_failed;
    let pct = if total > 0 {
        (total_working as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    output.push_str(&format!(
        "Stats: {}/{} ({:.1}%) [{}+{}lim] {}fail\n",
        total_working, total, pct, total_passed, total_lim, total_failed
    ));

    // Per-type stats on one line
    let req_total = req_stats.passed + req_stats.failed + req_stats.limitations;
    let resp_total = resp_stats.passed + resp_stats.failed + resp_stats.limitations;
    let str_total = stream_stats.passed + stream_stats.failed + stream_stats.limitations;

    let mut type_stats = Vec::new();
    if req_total > 0 {
        type_stats.push(format!(
            "req:{}/{}",
            req_stats.passed + req_stats.limitations,
            req_total
        ));
    }
    if resp_total > 0 {
        type_stats.push(format!(
            "res:{}/{}",
            resp_stats.passed + resp_stats.limitations,
            resp_total
        ));
    }
    if str_total > 0 {
        type_stats.push(format!(
            "str:{}/{}",
            stream_stats.passed + stream_stats.limitations,
            str_total
        ));
    }

    if !type_stats.is_empty() {
        output.push_str(&type_stats.join(" "));
        output.push('\n');
    }

    output
}

/// Generate compact failures section.
pub fn generate_compact_failures(failures: &[FailureWithDiff]) -> String {
    let mut output = String::new();

    if failures.is_empty() {
        return output;
    }

    let groups = group_failures(failures);

    output.push_str(&format!(
        "\n## Failures ({} patterns, {} total)\n",
        groups.len(),
        failures.len()
    ));

    for (idx, group) in groups.iter().enumerate() {
        output.push_str(&format!(
            "\n[P{}] {} ({})\n",
            idx + 1,
            group.pattern.pattern,
            group.total_count
        ));

        // Sort directions by count descending
        let mut directions: Vec<_> = group.by_direction.iter().collect();
        directions.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        for (direction, test_cases) in directions {
            // Abbreviate direction
            let parts: Vec<_> = direction.split(" → ").collect();
            let abbrev_dir = if parts.len() == 2 {
                format!("{}→{}", abbrev(parts[0]), abbrev(parts[1]))
            } else {
                direction.clone()
            };

            let compact_names = compact_test_names(test_cases);
            output.push_str(&format!("  {}: {}\n", abbrev_dir, compact_names));
        }
    }

    output
}

/// Generate compact limitations section.
pub fn generate_compact_limitations(limitations: &[(String, String, String)]) -> String {
    let mut output = String::new();

    if limitations.is_empty() {
        return output;
    }

    // Group by direction
    let mut by_direction: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (direction, test_case, reason) in limitations {
        by_direction
            .entry(direction.clone())
            .or_default()
            .push((test_case.clone(), reason.clone()));
    }

    output.push_str(&format!("\n## Limitations ({})\n", limitations.len()));

    let mut directions: Vec<_> = by_direction.into_iter().collect();
    directions.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (direction, items) in directions {
        let parts: Vec<_> = direction.split(" → ").collect();
        let abbrev_dir = if parts.len() == 2 {
            format!("{}→{}", abbrev(parts[0]), abbrev(parts[1]))
        } else {
            direction.clone()
        };
        output.push_str(&format!(
            "{}: {}\n",
            abbrev_dir,
            compact_test_names(&items.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>())
        ));
    }

    output
}

/// Collect statistics from results.
pub fn collect_stats(results: &HashMap<(usize, usize), PairResult>) -> TableStats {
    let mut stats = TableStats {
        passed: 0,
        failed: 0,
        limitations: 0,
    };
    for pair_result in results.values() {
        stats.passed += pair_result.passed;
        stats.failed += pair_result.failed;
        stats.limitations += pair_result.limitations;
    }
    stats
}

/// Collect failures from results with direction info.
pub fn collect_failures(
    results: &HashMap<(usize, usize), PairResult>,
    adapters: &[Box<dyn ProviderAdapter>],
) -> Vec<FailureWithDiff> {
    let mut failures = Vec::new();
    for ((source_idx, target_idx), pair_result) in results {
        let direction = format!(
            "{} → {}",
            adapters[*source_idx].display_name(),
            adapters[*target_idx].display_name()
        );
        for (test_case, error, diff) in &pair_result.failures {
            failures.push((
                direction.clone(),
                test_case.clone(),
                error.clone(),
                diff.clone(),
            ));
        }
    }
    failures
}

/// Collect limitations from results with direction info.
pub fn collect_limitations(
    results: &HashMap<(usize, usize), PairResult>,
    adapters: &[Box<dyn ProviderAdapter>],
) -> Vec<(String, String, String)> {
    let mut limitations = Vec::new();
    for ((source_idx, target_idx), pair_result) in results {
        let direction = format!(
            "{} → {}",
            adapters[*source_idx].display_name(),
            adapters[*target_idx].display_name()
        );
        for (test_case, reason, _diff) in &pair_result.limitation_details {
            // Compact mode ignores diff - just pass through test_case and reason
            limitations.push((direction.clone(), test_case.clone(), reason.clone()));
        }
    }
    limitations
}
