# Import Cases: UI Span -> Test Fixture

This folder holds fixtures for span-import testing.

Each test case uses two files:

- `<name>.spans.json`: array of spans
- `<name>.assertions.json`: expected outcomes

Current consumer:

- TypeScript test: `bindings/typescript/tests/two-step-conversion.test.ts`

Rust currently has one fixture-backed test in:

- `crates/lingua/src/processing/import.rs`

## Quick workflow

1. Copy a span object from the UI.
2. Keep only stable fields you want to test (usually `input` and `output`).
3. Wrap it in an array and save as `payloads/import-cases/<name>.spans.json`.
4. Add `payloads/import-cases/<name>.assertions.json`.
5. Run the TS import fixture test.

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
