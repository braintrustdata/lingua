#!/bin/bash

# Show the assistant-thinking + sibling-tool-calls import behavior.
#
# On a broken importer, this prints an assistant message with reasoning only and
# then fails because the sibling tool_call fields are absent. On a fixed importer,
# it prints one assistant message containing both the reasoning part and tool_call
# part.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_FILE="$REPO_ROOT/crates/lingua/tests/show_import_thinking_tool_calls_issue.rs"

cleanup() {
    rm -f "$TEST_FILE"
}
trap cleanup EXIT

if [ -f "$TEST_FILE" ]; then
    echo "Refusing to overwrite existing test file: $TEST_FILE" >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to run this script" >&2
    exit 1
fi

CARGO_CMD=(cargo)
if [ -n "${CARGO_TOOLCHAIN:-}" ]; then
    CARGO_CMD=(cargo "+$CARGO_TOOLCHAIN")
elif command -v rustup >/dev/null 2>&1 && rustup toolchain list | grep -q '^1\.95\.0'; then
    CARGO_CMD=(cargo +1.95.0)
fi

cat > "$TEST_FILE" <<'RS'
use lingua::processing::{import_messages_from_spans, Span};
use lingua::serde_json;

#[test]
fn show_import_thinking_tool_calls_issue() {
    let spans_json = r#"
[
  {
    "output": [
      {
        "role": "assistant",
        "content": [
          {
            "type": "thinking",
            "thinking": "Need to query structured data before answering.",
            "signature": "reasoning_signature_123"
          }
        ],
        "tool_calls": [
          {
            "id": "call_structured_data_123",
            "type": "function",
            "function": {
              "name": "StructuredDataQueryTool",
              "arguments": {
                "query": "select count(*) from anonymized_table",
                "limit": 10
              }
            }
          }
        ]
      }
    ]
  }
]
"#;

    let spans: Vec<Span> = serde_json::from_str(spans_json).expect("fixture should parse");
    let messages = import_messages_from_spans(spans);
    let pretty = serde_json::to_string_pretty(&messages).expect("messages should serialize");

    println!("\nImported Lingua messages:\n{}\n", pretty);

    for expected in [
        "Need to query structured data before answering.",
        "reasoning_signature_123",
        "call_structured_data_123",
        "StructuredDataQueryTool",
        "select count(*) from anonymized_table",
        "\"limit\": 10",
    ] {
        assert!(
            pretty.contains(expected),
            "missing expected text: {expected}\n\nOn the broken importer, sibling tool_calls are dropped before the UI sees them.\n\nImported messages:\n{pretty}"
        );
    }
}
RS

cd "$REPO_ROOT"
echo "Running: ${CARGO_CMD[*]} test -p lingua --test show_import_thinking_tool_calls_issue -- --nocapture"
"${CARGO_CMD[@]}" test -p lingua --test show_import_thinking_tool_calls_issue -- --nocapture
