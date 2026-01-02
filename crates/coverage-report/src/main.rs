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

mod discovery;
mod report;
mod runner;
mod types;

use lingua::processing::adapters::adapters;
use report::generate_report;
use runner::{run_all_tests, run_roundtrip_tests};

fn main() {
    let adapters = adapters();

    // Cross-provider transformation tests
    let (request_results, response_results, streaming_results) = run_all_tests(adapters);

    // Roundtrip transform tests (Provider → Universal → Provider)
    let roundtrip_results = run_roundtrip_tests(adapters);

    let report = generate_report(
        &request_results,
        &response_results,
        &streaming_results,
        &roundtrip_results,
        adapters,
    );
    println!("{}", report);
}
