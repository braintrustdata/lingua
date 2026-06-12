## Google provider type update notes

### Changes in generated.rs

1. **New field**: `translation_config: Option<TranslationConfig>` added to `GenerationConfig`.
2. **New struct**: `TranslationConfig` with `echo_target_language: Option<bool>` and `target_language_code: Option<String>`.

### Adapter impact

No hand-written code changes required. `translation_config` is a Google-specific real-time translation feature with no universal equivalent. It follows the same pattern as other Google-specific `GenerationConfig` fields (`speech_config`, `audio_timestamp`, `media_resolution`, `routing_config`) that are silently dropped during universal roundtrip.

The field is `Option<TranslationConfig>` and `GenerationConfig` derives `Default`, so all existing `..Default::default()` construction patterns compile correctly without modification.

### Items that do NOT need human clarification

- `translation_config` has no universal mapping and none is expected. It is correctly handled by being omitted from universal conversion (same as `speech_config`, `media_resolution`, etc.).
- No new enum variants were added to `FinishReason` or any other enum. No converter updates needed.
- Build, clippy, and all 194 Google-related tests pass.

### Pre-existing observation (not introduced by this diff)

The `Language` finish reason variant (already in the generated `FinishReason` enum before this diff) falls through to `FinishReason::Other` rather than `FinishReason::ContentFilter` in `convert.rs:1210`. With translation features becoming more prominent via `translation_config`, this variant may appear more often. Consider mapping it to `ContentFilter` if Google documents it as a blocking/safety reason.
