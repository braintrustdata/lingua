# Import Cases: UI Span -> Rust Fixture Test

This folder holds fixtures for `import_messages_from_spans` Rust tests.

Each case uses:

- `<name>.spans.json`: required
- `<name>.assertions.json`: optional when bootstrapping

The test runner is:

- `crates/lingua/tests/import_fixtures.rs`

## Quick workflow

1. Copy a span object from the UI.
2. Save a fixture as `payloads/import-cases/<name>.spans.json`.
3. Optional: anonymize content, metadata, context, and output strings in the fixture.
4. Run the Rust test with `GENERATE_MISSING=1` to create missing assertions.
5. Review/edit the generated `*.assertions.json`.
6. Run the Rust test without flags for strict assertion mode.

## Anonymize fixture strings

Run from repo root:

```bash
pnpm --dir payloads anonymize -- import-cases/<name>.spans.json
```

Notes:

- Default mode anonymizes strings under `content`, `metadata*`, `context`, and `output` subtrees.
- It removes `metadata*.prompt` completely.
- It preserves structural keys (`role`, `type`) and keeps `metadata.model` unchanged.
- Use `--all-strings` to anonymize every string value in the file.

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

- You can provide either a single span object or an array of spans.
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
cargo test -p lingua --test import_fixtures -- --nocapture
```

Generate missing assertions:

```bash
GENERATE_MISSING=1 cargo test -p lingua --test import_fixtures -- --nocapture
```

Refresh all existing assertions (keeps existing `mustContainText` values):

```bash
ACCEPT=1 cargo test -p lingua --test import_fixtures -- --nocapture
```

Run only matching cases:

```bash
CASE_FILTER=simpsons cargo test -p lingua --test import_fixtures -- --nocapture
```

Notes:

- `GENERATE_MISSING` and `ACCEPT` are disabled in CI.
- Generated assertions infer message count and role order from current importer output.
