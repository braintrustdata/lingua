# Remote media preparation fix plan

## Root cause

Limiting pre-transform remote-media preparation to audio URLs allows cross-format image and file URLs to be translated to target-native URL fields before inlining runs.

## Target files

- `crates/braintrust-llm-router/src/providers/remote_media.rs`
- `crates/braintrust-llm-router/src/router.rs`

## Expected behavior

- Preprocess cross-format requests for media-capable targets before target conversion.
- Retain same-format native passthrough when no remote audio requires special handling.
- Preserve the original source format in router metadata after preprocessing.

## Tests

- Verify a remote Chat Completions image is fetched and inlined before conversion to Bedrock.
- Run router library tests and typed-boundary checks.
