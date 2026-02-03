/*!
Cross-provider transformation coverage testing library.

This library provides the core functionality for running cross-provider
transformation tests and generating coverage reports.

## Usage

```rust,ignore
use coverage_report::{run_all_tests, types::TestFilter};
use lingua::processing::adapters::adapters;

let adapters = adapters();
let filter = TestFilter::default();
let (requests, responses, streaming) = run_all_tests(adapters, &filter);
```
*/

pub mod compact;
pub mod discovery;
pub mod expected;
mod normalizers;
pub mod report;
pub mod runner;
pub mod types;

// Re-export commonly used items
pub use runner::run_all_tests;
pub use types::{PairResult, TestFilter, ValidationLevel};
