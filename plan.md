# Gate Chat Completions audio by model capability

## Root cause

The Chat Completions converter emits universal audio as `input_audio` without checking whether the selected target model supports audio input. The existing Google-to-Chat-Completions audio capture targets `gpt-5-nano` and receives a 400 response.

## Target files

- `crates/lingua/src/providers/openai/capabilities.rs`
- `crates/lingua/src/providers/openai/adapter.rs`
- `payloads/transforms/transform_errors.json`

## Expected behavior

Chat Completions requests with audio fail with `UnsupportedMapping` unless the model is explicitly known to accept Chat Completions audio input. The adapter must reject before serializing an invalid provider request.

## Tests and validation

- Add capability and adapter tests for an unsupported model and a known audio-capable model.
- Re-run the affected Google-to-Chat-Completions payload capture and payload tests.
- Run the cross-provider coverage test and typed-boundary checks.
