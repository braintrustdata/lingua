# Google provider type update notes

## Changes applied

### `ImageProhibitedContent` and `ImageRecitation` finish reasons mapped to `ContentFilter`

These two `FinishReason` variants were falling through to `Other` in both the typed
(`From<&GoogleFinishReason>`) and string-based (`from_provider_string`) conversion paths.
They clearly parallel `ProhibitedContent` and `Recitation` which were already mapped to
`ContentFilter`, so both paths have been updated. Tests added in `convert.rs` and the
existing test in `response.rs` extended.

## No adapter changes needed

### `GenerationConfig.translation_config` (new field)

The generated `GenerationConfig` struct gained an optional `translation_config: Option<TranslationConfig>`
field. `TranslationConfig` contains `echo_target_language: Option<bool>` and
`target_language_code: Option<String>`.

This is a Google-specific translation feature with no universal equivalent. The field will
be silently dropped during Google-to-Universal-to-Google roundtrip, consistent with how
other Google-specific `GenerationConfig` sub-fields (e.g. `speech_config`, `audio_timestamp`,
`media_resolution`) are handled.

If the universal format should support translation configuration in the future, a new
universal param would need to be designed.

## Items needing human clarification

### `Language` finish reason

`GoogleFinishReason::Language` (wire value `"LANGUAGE"`) currently maps to `Other`.
Google documents this as "Token generation stopped because the content potentially contains
language that is not supported." This could be considered a content-filter scenario
(language policy restriction) or a capability limitation. Left as `Other` pending
clarification on the intended universal mapping.

### `ImageOther` finish reason

`GoogleFinishReason::ImageOther` (wire value `"IMAGE_OTHER"`) currently maps to `Other`.
This is a generic image-related finish reason. It is not clearly a content-filter scenario
(unlike `ImageSafety`, `ImageProhibitedContent`, `ImageRecitation`), so it was left as `Other`.
