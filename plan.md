# Chat Completions cache round-trip fix

## Root causes

- The direct generated `ChatCompletionRequestMessageContentPart` converter
  ignores `prompt_cache_breakpoint`, while the wrapper converter preserves it.
- Chat Completions cache fixtures use either the legacy `cache_control` extension
  or the official breakpoint alone. Conversion canonicalizes both into Lingua
  cache control and emits both representations, so strict round-trip tests see
  an added field.

## Target files

- `payloads/cases/params.ts`
- `crates/lingua/src/providers/openai/convert.rs`
- Focused cache-control payload snapshots and transform snapshots

## Expected behavior

- Direct content-part conversion maps an explicit OpenAI breakpoint to ephemeral
  Lingua cache control without inventing a TTL.
- Cache fixtures include both the legacy cache metadata needed for TTL-preserving
  cross-provider tests and the official OpenAI breakpoint.
- Cache fixtures use a GPT-5.6 model that supports explicit breakpoints.
- Rust, TypeScript, and Python round trips preserve the original provider shape.

## Tests

- Add a focused Rust test for direct content-part breakpoint conversion.
- Run all generated `CacheControlParam` Rust round-trip cases.
- Run TypeScript and Python round-trip suites.
- Run payload, cross-provider, typed-boundary, and Clippy checks.

## Expected diff impact

The three focused Chat Completions cache fixtures and their captures gain
`prompt_cache_breakpoint: { mode: "explicit" }`; the official-breakpoint fixture
also records the corresponding legacy ephemeral cache extension. No generated
Rust source or broad expected-difference exception is needed.

## Validation commands

```bash
make capture FILTER=chatCompletionsAssistantCacheControlParam
make capture FILTER=chatCompletionsSystemCacheControlParam
make capture FILTER=chatCompletionsAnthropicCacheControlParam
cargo test -p lingua CacheControlParam -- --nocapture
make typescript
make test-python
make test-payloads
cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures
make typed-boundary-check
make typed-boundary-check-branch BASE=main
cargo clippy --all-targets --all-features -- -D warnings
```
