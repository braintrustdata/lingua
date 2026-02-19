/*!
Integration test for cross-provider transformations.

This test ensures that transformations between required providers have no
unexpected failures. Known limitations (documented in expected_differences.json)
are allowed, but regressions will cause this test to fail.
*/

use coverage_report::runner::{run_all_tests, truncate_error};
use coverage_report::types::TestFilter;
use lingua::capabilities::ProviderFormat;
use lingua::processing::adapters::adapters;

/// TODO: remove REQUIRED_PROVIDERS once all formats are fully supported by coverage-report.
/// this is temporary as we make incremental progress.
const REQUIRED_PROVIDERS: &[ProviderFormat] = &[
    ProviderFormat::Responses,
    ProviderFormat::ChatCompletions, // ChatCompletions
    ProviderFormat::Anthropic,
    ProviderFormat::BedrockAnthropic,
    ProviderFormat::Google,
];

#[test]
fn cross_provider_transformations_have_no_unexpected_failures() {
    let adapters = adapters();
    let filter = TestFilter {
        providers: Some(REQUIRED_PROVIDERS.to_vec()),
        ..Default::default()
    };

    let (request_results, response_results, streaming_results) = run_all_tests(adapters, &filter);

    let mut failures = Vec::new();

    // Collect failures from all result categories
    for (category, results) in [
        ("requests", &request_results),
        ("responses", &response_results),
        ("streaming", &streaming_results),
    ] {
        for ((src_idx, tgt_idx), pair_result) in results.iter() {
            if pair_result.failed > 0 {
                let src_format = adapters[*src_idx].format();
                let tgt_format = adapters[*tgt_idx].format();

                // Collect detailed failure messages
                for (test_case, error, _diff) in &pair_result.failures {
                    failures.push(format!(
                        "  [{category}] {:?} -> {:?}: {test_case}\n    Error: {}",
                        src_format,
                        tgt_format,
                        truncate_error(error.clone(), 1000)
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Unexpected cross-provider transformation failures:\n\n{}\n\n\
         These failures are NOT in the expected_differences.json whitelist.\n\
         Either fix the regression or add an entry to the appropriate *_expected_differences.json file.",
        failures.join("\n\n")
    );
}
