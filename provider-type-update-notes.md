# Provider type update notes: Google

## Changes in this update

### New `TranslationConfig` struct and `GenerationConfig.translation_config` field

**Generated diff**: `crates/lingua/src/providers/google/generated.rs`

A new optional `translation_config: Option<TranslationConfig>` field was added to `GenerationConfig`. The `TranslationConfig` struct has two fields:
- `echo_target_language: Option<bool>` - controls audio parroting for target language
- `target_language_code: Option<String>` - BCP-47 language code for translation target

**Universal mapping**: None. This is a Google-specific translation feature (Gemini live translation) with no equivalent in the universal request format.

**Adapter impact**: No hand-written code changes required.
- `request_to_universal` (adapter.rs): does not read `translation_config`; the field is ignored during conversion to universal, consistent with other unmapped `GenerationConfig` fields (`speech_config`, `image_config`, `media_resolution`, etc.).
- `request_from_universal` (adapter.rs): constructs `GenerationConfig` with `..Default::default()`, so `translation_config` defaults to `None`.
- `ResponseFormatConfig` conversions (convert.rs): unaffected; only reads/writes response-format-related fields.
- `GoogleParams` (params.rs): automatically deserializes `translation_config` as part of the typed `GenerationConfig` struct. No change needed.

**Roundtrip behavior**: `translation_config` is lost during Google-to-universal-to-Google roundtrips. This is consistent with how other unmapped `GenerationConfig` fields are handled.

**Human clarification needed**: If translation config should survive cross-provider transforms or Google roundtrips, a universal representation would need to be designed. No workaround has been added.

## Validation

- `cargo check -p lingua`: pass
- `cargo clippy -p lingua -- -D warnings`: pass
- `cargo test -p lingua -- google`: 194 tests pass
