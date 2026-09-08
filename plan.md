# Preserve audio semantics in system content and Google imports

## Root cause

- Google inline data labelled `audio/mp3` is an MP3 alias but currently imports as file content.
- System and developer content is reduced to text by Anthropic, Google, and Bedrock request construction. Audio in that content is therefore silently discarded.

## Target files

- `crates/lingua/src/providers/google/convert.rs`
- `crates/lingua/src/providers/google/adapter.rs`
- `crates/lingua/src/providers/anthropic/adapter.rs`
- `crates/lingua/src/providers/bedrock/convert.rs`

## Expected behavior

- `audio/mp3` and `audio/mpeg` both import as universal MP3 audio.
- A system or developer message containing universal audio returns an explicit unsupported-mapping error before a provider extracts or flattens its system prompt.

## Tests and validation

- Extend the Google inline-audio conversion test with `audio/mp3`.
- Add focused adapter/converter tests for system/developer audio rejection.
- Run focused Rust tests, payload transforms, and typed-boundary checks.
