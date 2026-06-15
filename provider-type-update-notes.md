## Google provider type update notes

### New types

- `TranslationConfig` struct added to `generated.rs` with fields `echo_target_language: Option<bool>` and `target_language_code: Option<String>`.
- `translation_config: Option<TranslationConfig>` field added to `GenerationConfig`.

### Adapter impact

No hand-written code changes required. All `GenerationConfig` construction in `adapter.rs` and `convert.rs` uses `..Default::default()`, so the new field safely defaults to `None`.

### Roundtrip behavior

`translation_config` is silently lost during Google → Universal → Google roundtrip. The universal format has no representation for translation configuration. This matches the existing behavior for other unmapped `GenerationConfig` sub-fields (`audio_timestamp`, `media_resolution`, `routing_config`, `speech_config`, etc.) which are also not preserved through the universal layer.

### Human clarification needed

If lossless roundtrip of `translation_config` is desired, a decision is needed on how to represent it in the universal format. Options include:

1. Add a dedicated `translation_config` field to `UniversalParams` (if other providers gain similar features).
2. Preserve it via the existing provider-specific extras mechanism (would require changes to how `GenerationConfig` sub-fields flow through extras, since currently only top-level request fields are captured by `GoogleParams.extras`).
3. Accept the current lossy behavior as intentional (translation is a Google-specific feature with no cross-provider equivalent).

### Validation

- `cargo check --all-features`: pass
- `cargo clippy --all-features -- -D warnings`: pass
- `cargo test -p lingua -- google`: 194 tests pass
- No credentials or external services needed for the above checks.
