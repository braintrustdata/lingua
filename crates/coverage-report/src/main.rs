/*!
Cross-provider transformation coverage report generator.

This binary runs all cross-provider transformation tests and outputs a
markdown report showing which transformations succeed/fail.

Validates:
1. Transform doesn't error
2. Transformed output deserializes into target provider's Rust types (schema validation)
3. Key semantic fields are preserved (messages, model, tools, usage)

Usage:
    cargo run --bin coverage-report
    cargo run --bin coverage-report -- --coverage requests,responses
    cargo run --bin coverage-report -- --test-cases seedParam,toolCallRequest
    cargo run --bin coverage-report -- --providers responses,anthropic
    cargo run --bin coverage-report -- --source responses --target anthropic
*/

use std::str::FromStr;

use coverage_report::report::generate_report;
use coverage_report::runner::run_all_tests;
use coverage_report::types::{parse_provider, CoverageSelection, OutputFormat, TestFilter};
use lingua::processing::adapters::adapters;

struct CliArgs {
    selection: CoverageSelection,
    filter: TestFilter,
    format: OutputFormat,
}

fn parse_cli_args() -> Result<CliArgs, String> {
    let mut selection_arg: Option<String> = None;
    let mut test_cases_arg: Option<String> = None;
    let mut providers_arg: Option<String> = None;
    let mut source_arg: Option<String> = None;
    let mut target_arg: Option<String> = None;
    let mut format_arg: Option<String> = None;

    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--coverage" => {
                selection_arg = args.next();
                if selection_arg.is_none() {
                    return Err("Missing value for --coverage".to_string());
                }
            }
            "--test-cases" | "-t" => {
                test_cases_arg = args.next();
                if test_cases_arg.is_none() {
                    return Err("Missing value for --test-cases".to_string());
                }
            }
            "--providers" | "-p" => {
                providers_arg = args.next();
                if providers_arg.is_none() {
                    return Err("Missing value for --providers".to_string());
                }
            }
            "--source" => {
                source_arg = args.next();
                if source_arg.is_none() {
                    return Err("Missing value for --source".to_string());
                }
            }
            "--target" => {
                target_arg = args.next();
                if target_arg.is_none() {
                    return Err("Missing value for --target".to_string());
                }
            }
            "--format" | "-f" => {
                format_arg = args.next();
                if format_arg.is_none() {
                    return Err("Missing value for --format".to_string());
                }
            }
            _ if arg.starts_with("--coverage=") => {
                selection_arg = Some(arg.strip_prefix("--coverage=").unwrap().to_string());
            }
            _ if arg.starts_with("--test-cases=") || arg.starts_with("-t=") => {
                let prefix = if arg.starts_with("--test-cases=") {
                    "--test-cases="
                } else {
                    "-t="
                };
                test_cases_arg = Some(arg.strip_prefix(prefix).unwrap().to_string());
            }
            _ if arg.starts_with("--providers=") || arg.starts_with("-p=") => {
                let prefix = if arg.starts_with("--providers=") {
                    "--providers="
                } else {
                    "-p="
                };
                providers_arg = Some(arg.strip_prefix(prefix).unwrap().to_string());
            }
            _ if arg.starts_with("--source=") => {
                source_arg = Some(arg.strip_prefix("--source=").unwrap().to_string());
            }
            _ if arg.starts_with("--target=") => {
                target_arg = Some(arg.strip_prefix("--target=").unwrap().to_string());
            }
            _ if arg.starts_with("--format=") || arg.starts_with("-f=") => {
                let prefix = if arg.starts_with("--format=") {
                    "--format="
                } else {
                    "-f="
                };
                format_arg = Some(arg.strip_prefix(prefix).unwrap().to_string());
            }
            _ => {
                return Err(format!("Unknown argument: {}", arg));
            }
        }
    }

    // Parse coverage selection
    let selection = match selection_arg {
        Some(value) => CoverageSelection::from_list(&value)?,
        None => CoverageSelection::all(),
    };

    // Parse output format
    let format = match format_arg {
        Some(value) => OutputFormat::from_str(&value)?,
        None => OutputFormat::default(),
    };

    // Parse test filter
    let mut filter = TestFilter::default();

    // Parse test case patterns
    if let Some(value) = test_cases_arg {
        filter.test_case_patterns = value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // Parse providers filter (both source AND target)
    if let Some(value) = providers_arg {
        let providers: Result<Vec<_>, _> =
            value.split(',').map(|s| parse_provider(s.trim())).collect();
        filter.providers = Some(providers?);
    }

    // Parse explicit source filter
    if let Some(value) = source_arg {
        let sources: Result<Vec<_>, _> =
            value.split(',').map(|s| parse_provider(s.trim())).collect();
        filter.sources = Some(sources?);
    }

    // Parse explicit target filter
    if let Some(value) = target_arg {
        let targets: Result<Vec<_>, _> =
            value.split(',').map(|s| parse_provider(s.trim())).collect();
        filter.targets = Some(targets?);
    }

    Ok(CliArgs {
        selection,
        filter,
        format,
    })
}

fn print_usage() {
    eprintln!("Usage: coverage-report [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --coverage <types>       Coverage types: requests,responses,streaming,all");
    eprintln!(
        "  -t, --test-cases <patterns>  Test case patterns (glob: seedParam, reasoning*, *Param)"
    );
    eprintln!(
        "  -p, --providers <names>  Filter provider pairs (both source AND target must match)"
    );
    eprintln!("  --source <names>         Filter source providers");
    eprintln!("  --target <names>         Filter target providers");
    eprintln!("  -f, --format <format>    Output format: markdown (default), compact");
    eprintln!();
    eprintln!("Provider names: responses, chat-completions, anthropic, google, bedrock");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  coverage-report                                    # Run all tests");
    eprintln!(
        "  coverage-report --coverage requests                # Only request transformations"
    );
    eprintln!("  coverage-report -t seedParam                       # Only seedParam test case");
    eprintln!("  coverage-report -t \"reasoning*\"                    # All reasoning test cases");
    eprintln!("  coverage-report -p chat-completions                # Roundtrip tests for ChatCompletions");
    eprintln!("  coverage-report -p responses,anthropic             # Only Responses↔Anthropic");
    eprintln!(
        "  coverage-report --source responses --target anthropic  # Only Responses→Anthropic"
    );
    eprintln!("  coverage-report -f compact                         # Token-optimized output");
}

fn main() {
    let CliArgs {
        selection,
        filter,
        format,
    } = match parse_cli_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("Error: {}", error);
            eprintln!();
            print_usage();
            std::process::exit(2);
        }
    };

    let adapters = adapters();

    // Run all transformation tests (including roundtrip when source == target)
    let (request_results, response_results, streaming_results) = run_all_tests(adapters, &filter);

    let report = generate_report(
        &request_results,
        &response_results,
        &streaming_results,
        adapters,
        selection,
        format,
    );
    println!("{}", report);
}
