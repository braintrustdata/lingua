# Import Cases: UI Span -> Rust Fixture Test

This folder holds fixtures for `import_messages_from_spans` Rust tests.

Each case uses:

- `<name>.spans.json`: required
- `<name>.assertions.json`: optional when bootstrapping

The test runner is:

- `crates/lingua/src/processing/import.rs`

## Quick workflow

1. Copy a span object from the UI.
2. Save a fixture as `payloads/import-cases/<name>.spans.json`.
3. Run the Rust test with `GENERATE_MISSING=1` to create missing assertions.
4. Review/edit the generated `*.assertions.json`.
5. Run the Rust test without flags for strict assertion mode.

## File formats

### `*.spans.json`

```json
[
  {
    "input": [...],
    "output": [...]
  }
]
```

Notes:

- Use an array even for a single span.
- Prefer stable fields (`input`, `output`) to reduce fixture churn.

### `*.assertions.json`

```json
{
  "expectedMessageCount": 2,
  "expectedRolesInOrder": ["user", "assistant"],
  "mustContainText": []
}
```

Supported keys:

- `expectedMessageCount` (number)
- `expectedRolesInOrder` (string array)
- `mustContainText` (string array)

## Test modes

Default mode (strict):

```bash
cargo test -p lingua processing::import::tests::test_import_cases_from_shared_fixtures -- --nocapture
```

Generate missing assertions:

```bash
GENERATE_MISSING=1 cargo test -p lingua processing::import::tests::test_import_cases_from_shared_fixtures -- --nocapture
```

Refresh all existing assertions (keeps existing `mustContainText` values):

```bash
ACCEPT=1 cargo test -p lingua processing::import::tests::test_import_cases_from_shared_fixtures -- --nocapture
```

Run only matching cases:

```bash
CASE_FILTER=simpsons cargo test -p lingua processing::import::tests::test_import_cases_from_shared_fixtures -- --nocapture
```

Notes:

- `GENERATE_MISSING` and `ACCEPT` are disabled in CI.
- Generated assertions infer message count and role order from current importer output.
