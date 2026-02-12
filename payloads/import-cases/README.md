# Import Cases: UI Span -> Test Fixture

This folder holds fixtures for span-import testing.

Each test case uses two files:

- `<name>.spans.json`: array of spans
- `<name>.assertions.json`: expected outcomes

Current consumers:

- TypeScript test: `bindings/typescript/tests/two-step-conversion.test.ts`
- Rust test: `crates/lingua/src/processing/import.rs`

## Quick workflow

1. Copy a span object from the UI.
2. Generate fixture files with `pnpm new-import-case`.
3. Review and adjust assertions.
4. Run TS and/or Rust fixture tests.

## Fixture generator

From `payloads/`:

```bash
pnpm new-import-case --name simpsons-cancelation --from /tmp/span.json
```

Or from pasted JSON on stdin:

```bash
pbpaste | pnpm new-import-case --name simpsons-cancelation
```

The generator creates:

- `payloads/import-cases/<name>.spans.json`
- `payloads/import-cases/<name>.assertions.json`

Default behavior trims spans to stable fields (`input` and `output`).

Useful flags:

- `--keep-full-span`: keep all span keys from UI
- `--force`: overwrite existing files
- `--dry-run`: print generated files without writing

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

- You can paste the full span from UI, but trimming to `input`/`output` keeps fixtures stable.
- Use an array even for a single span.

### `*.assertions.json`

```json
{
  "expectedMessageCount": 2,
  "expectedRolesInOrder": ["user", "assistant"],
  "mustContainText": ["magic 8-ball"]
}
```

Supported keys:

- `expectedMessageCount` (number)
- `expectedRolesInOrder` (string array)
- `mustContainText` (string array)

Generator note:

- `expectedMessageCount` and `expectedRolesInOrder` are inferred with simple role scanning.
- Always review inferred assertions before committing.

## Using a copied UI span

Given a large UI span object, extract:

- `input`
- `output`

and create:

```json
[
  {
    "input": [
      { "role": "user", "content": "will they cancel the Simpsons show soon?", "type": "message" }
    ],
    "output": [
      {
        "content": [
          {
            "type": "output_text",
            "text": "I consulted my trusty magic 8-ball..."
          }
        ],
        "role": "assistant",
        "type": "message"
      }
    ]
  }
]
```

If you want an automated trim from a file containing one span object:

```bash
jq '[{input, output}]' raw-span.json > payloads/import-cases/<name>.spans.json
```

## Run tests

Run the fixture-driven TS test:

```bash
cd bindings/typescript
pnpm vitest run tests/two-step-conversion.test.ts
```

Run Rust import tests:

```bash
cargo test -p lingua processing::import::tests
```
