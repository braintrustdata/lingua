# Preserve native audio requests and simplify preparation

## Root cause

The native Chat Completions audio preprocessor serializes a generated request type, dropping unmodeled provider extensions.

## Expected behavior

Use a typed compatibility view for `input_audio` that preserves all unrelated request, message, and content-part fields during serialization.

Preserve absent versus null fields and nested audio extensions; fetch only input_audio parts. Consolidate the shared conversion branch and remove the router's redundant media preparation pass and preliminary detection helper. Target remote_media.rs, providers/mod.rs, and router.rs. No captured provider artifacts should change: this is router preprocessing, not converter behavior.

## Tests

## Preserve source JSON-response metadata

The router recomputes metadata after preprocessing has converted the source payload to Converse, whose adapter does not recover the original JSON requirement. Promote the preprocessing flag to production and combine it with transformation metadata, including the native-audio path. Update remote_media.rs and router.rs; add a production preparation regression for JSON and plain-text requests, both streaming and non-streaming. This changes router metadata only, not provider converters or captured artifacts. First reproduce the failing regression, then run router tests, Clippy, formatting, and typed-boundary checks.

- Preserve an OpenRouter-style root `provider` extension while replacing an audio URL.
- Run focused remote-media and router tests.
- Compare the entire native request after inlining, including unknown fields at every level.
- Validate with cargo test -p braintrust-llm-router --lib, cargo clippy -p braintrust-llm-router --all-targets -- -D warnings, and make typed-boundary-check-branch BASE=main.
