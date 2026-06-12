# Provider type update notes

## Google: `TranslationConfig` added to `GenerationConfig`

**New generated types:**
- `TranslationConfig` struct with `echo_target_language: Option<bool>` and `target_language_code: Option<String>`
- `translation_config: Option<TranslationConfig>` field on `GenerationConfig`

**Universal representation:** None. The universal format has no translation concept.

**Current behavior:** The field is optional and defaults to `None`. When a Google request includes `translationConfig`, the adapter deserializes it into the typed `GenerationConfig` struct but does not read or propagate it. During Google-to-universal conversion it is silently dropped. During universal-to-Google conversion it is omitted (defaults to `None`).

**What needs human clarification:** Should `translationConfig` be preserved during Google round-trips? Options:
1. **No action needed** if translation is out of scope for Lingua's universal format (current behavior: silent drop).
2. **Preserve via extras** if Google-to-Google round-trip fidelity matters: the adapter could stash `translationConfig` in provider extras and restore it when building the Google request. This would require hand-written code in the adapter but no universal type changes.
3. **Model in universal types** if translation is a cross-provider concept that should be representable universally.

**No other semantic changes** in this update. The `specs/google/discovery.json` diff is primarily JSON key reordering. No new enum variants, renamed wire values, or required field changes were introduced.
