# OpenAI Responses review fixes

## Root causes

- The Responses `InputContent` converters leave the newly generated
  `prompt_cache_breakpoint` field at its default `None` and discard it during
  import, so Lingua cache hints do not survive Responses routing.
- `tool_caller_from_provider` returns `Option<ToolCaller>` and uses `map` on the
  generated optional `caller_id`, silently converting an incomplete program
  caller into a direct call.

## Target files

- `payloads/cases/params.ts`
- `crates/lingua/src/providers/openai/convert.rs`
- Focused transform snapshots affected by Responses cache metadata

## Expected behavior

- Lingua user text parts with `cache_control` emit a Responses
  `prompt_cache_breakpoint` with mode `explicit`.
- Responses input-text breakpoints import as ephemeral Lingua cache control
  without inventing a TTL, including on assistant-role input messages.
- Responses `output_text` remains breakpoint-free because the OpenAI SDK does
  not permit this field on output content.
- A program caller without `caller_id` returns a clear missing-field conversion
  error; direct callers and complete program callers retain their behavior.

## Tests

- Update the GPT-5.6 prompt-cache payload case with an explicit input-text
  breakpoint.
- Add focused Rust tests for Responses user text round-trips, assistant-role
  imports, and output-text schema constraints.
- Add focused Rust tests for complete, direct, and incomplete callers.
- Re-capture the payload case and run payload, cross-provider, TypeScript, and
  typed-boundary checks.

## Expected diff impact

Responses request snapshots containing Lingua cache metadata gain
`prompt_cache_breakpoint: { mode: "explicit" }`. No expected-difference
exception or generated-file edit is needed.

## Validation commands

```bash
make capture FILTER=responsesGpt56PromptCacheOptionsParam
cargo test -p lingua responses_prompt_cache_breakpoint
cargo test -p lingua tool_caller_from_provider
make capture FILTER=responsesGpt56PromptCacheOptionsParam
make test-payloads
cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures
make typed-boundary-check
make typed-boundary-check-branch BASE=main
```
