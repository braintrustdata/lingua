//! Regression tests for the Google generated-type update that renamed the
//! media-resolution types and added audio transcription config.
//!
//! Context for this update:
//!   * The Discovery schema id for the per-`Part` object changed from
//!     `MediaResolution` to `V1mainMediaResolution`, so the struct is now
//!     `V1MainMediaResolution`. Wire shape is unchanged (`{ "level": <enum> }`).
//!   * The inline `GenerationConfig.mediaResolution` string enum, previously
//!     named `MediaResolutionEnum`, is now named `MediaResolution`. Wire values
//!     are unchanged (`MEDIA_RESOLUTION_*`).
//!   * `GenerationConfig` gained a typed `audioTranscriptionConfig` field
//!     (`AudioTranscriptionConfig`) plus the `LanguageHints` type.
//!
//! These tests lock the wire formats so a future regeneration cannot silently
//! change them, and confirm still-valid provider payloads keep deserializing.

use crate::providers::google::generated::{
    AudioTranscriptionConfig, FunctionCallingConfig, FunctionCallingConfigMode, GenerationConfig,
    LanguageHints, Level, MediaResolution, Part, Schema, Type, V1MainMediaResolution,
};
use crate::serde_json::{self, json};

/// A `Part.mediaResolution` payload deserializes into the renamed
/// `V1MainMediaResolution` struct and round-trips with the same wire shape.
/// Also exercises the `MEDIA_RESOLUTION_ULTRA_HIGH` level, which only exists on
/// the per-`Part` `Level` enum (not on the `GenerationConfig` enum).
#[test]
fn part_media_resolution_v1main_roundtrips() {
    let payload = json!({
        "text": "hi",
        "mediaResolution": { "level": "MEDIA_RESOLUTION_ULTRA_HIGH" }
    });

    let part: Part = serde_json::from_value(payload.clone()).expect("Part should deserialize");
    let mr = part
        .media_resolution
        .as_ref()
        .expect("mediaResolution present");
    assert_eq!(mr.level, Some(Level::MediaResolutionUltraHigh));

    // Round-trip preserves the exact wire shape.
    let reserialized = serde_json::to_value(&part).expect("Part serializes");
    assert_eq!(
        reserialized["mediaResolution"]["level"],
        "MEDIA_RESOLUTION_ULTRA_HIGH"
    );

    // The struct wire shape is identical to a directly-constructed value.
    let direct = V1MainMediaResolution {
        level: Some(Level::MediaResolutionUltraHigh),
    };
    assert_eq!(
        serde_json::to_value(&direct).unwrap(),
        json!({ "level": "MEDIA_RESOLUTION_ULTRA_HIGH" })
    );
}

/// Every wire value of the `Level` enum (used by `V1MainMediaResolution`)
/// remains accepted.
#[test]
fn level_enum_accepts_all_wire_values() {
    let cases = [
        (
            "MEDIA_RESOLUTION_UNSPECIFIED",
            Level::MediaResolutionUnspecified,
        ),
        ("MEDIA_RESOLUTION_LOW", Level::MediaResolutionLow),
        ("MEDIA_RESOLUTION_MEDIUM", Level::MediaResolutionMedium),
        ("MEDIA_RESOLUTION_HIGH", Level::MediaResolutionHigh),
        (
            "MEDIA_RESOLUTION_ULTRA_HIGH",
            Level::MediaResolutionUltraHigh,
        ),
    ];
    for (wire, expected) in cases {
        let level: Level =
            serde_json::from_value(json!(wire)).unwrap_or_else(|e| panic!("{wire}: {e}"));
        assert_eq!(level, expected);
        assert_eq!(serde_json::to_value(&expected).unwrap(), json!(wire));
    }
}

/// `GenerationConfig.mediaResolution` uses the renamed `MediaResolution` enum
/// and every wire value stays valid.
#[test]
fn generation_config_media_resolution_enum_all_values() {
    let cases = [
        (
            "MEDIA_RESOLUTION_UNSPECIFIED",
            MediaResolution::MediaResolutionUnspecified,
        ),
        ("MEDIA_RESOLUTION_LOW", MediaResolution::MediaResolutionLow),
        (
            "MEDIA_RESOLUTION_MEDIUM",
            MediaResolution::MediaResolutionMedium,
        ),
        (
            "MEDIA_RESOLUTION_HIGH",
            MediaResolution::MediaResolutionHigh,
        ),
    ];
    for (wire, expected) in cases {
        let cfg: GenerationConfig =
            serde_json::from_value(json!({ "mediaResolution": wire })).unwrap();
        assert_eq!(cfg.media_resolution, Some(expected.clone()));
        let reserialized = serde_json::to_value(&cfg).unwrap();
        assert_eq!(reserialized["mediaResolution"], json!(wire));
    }
}

/// The new `audioTranscriptionConfig` field deserializes into the typed
/// `AudioTranscriptionConfig` struct (including nested `LanguageHints`) rather
/// than falling through to untyped extras, and round-trips losslessly.
#[test]
fn generation_config_audio_transcription_config_roundtrips() {
    let payload = json!({
        "audioTranscriptionConfig": {
            "customVocabulary": ["Lingua", "quicktype"],
            "diarization": true,
            "wordTimestamp": false,
            "languageHints": { "languageCodes": ["en-US", "fr-FR"] }
        }
    });

    let cfg: GenerationConfig =
        serde_json::from_value(payload.clone()).expect("GenerationConfig deserializes");
    let atc = cfg
        .audio_transcription_config
        .as_ref()
        .expect("audioTranscriptionConfig present");
    assert_eq!(
        atc.custom_vocabulary,
        Some(vec!["Lingua".to_string(), "quicktype".to_string()])
    );
    assert_eq!(atc.diarization, Some(true));
    assert_eq!(atc.word_timestamp, Some(false));
    assert_eq!(
        atc.language_hints
            .as_ref()
            .and_then(|h| h.language_codes.clone()),
        Some(vec!["en-US".to_string(), "fr-FR".to_string()])
    );

    // The audio-transcription subtree round-trips exactly. (We compare the
    // subtree rather than the whole object because unrelated generated fields
    // such as `responseSchema` serialize unconditionally.)
    let reserialized = serde_json::to_value(&cfg).expect("serializes");
    assert_eq!(
        reserialized["audioTranscriptionConfig"],
        payload["audioTranscriptionConfig"]
    );
}

/// A directly-constructed `AudioTranscriptionConfig` emits camelCase field
/// names and omits unset optionals.
#[test]
fn audio_transcription_config_emits_camel_case() {
    let atc = AudioTranscriptionConfig {
        language_hints: Some(LanguageHints {
            language_codes: Some(vec!["es-ES".to_string()]),
        }),
        ..Default::default()
    };
    let value = serde_json::to_value(&atc).unwrap();
    assert_eq!(
        value,
        json!({ "languageHints": { "languageCodes": ["es-ES"] } })
    );
}

/// `Schema.type == "STRING"` remains valid, and the lowercase `"string"` alias
/// (used by some JSON-schema style payloads) is still accepted. The serialized
/// wire value is always the canonical `"STRING"`.
#[test]
fn schema_type_string_wire_value_and_alias() {
    let from_canonical: Schema = serde_json::from_value(json!({ "type": "STRING" })).unwrap();
    assert_eq!(from_canonical.schema_type, Some(Type::String));

    let from_alias: Schema = serde_json::from_value(json!({ "type": "string" })).unwrap();
    assert_eq!(from_alias.schema_type, Some(Type::String));

    let reserialized = serde_json::to_value(&from_alias).unwrap();
    assert_eq!(reserialized["type"], "STRING");

    // Unknown type values are rejected (typed boundary, no silent coercion).
    assert!(serde_json::from_value::<Schema>(json!({ "type": "STRINGY" })).is_err());
}

/// `FunctionCallingConfigMode::None` serializes to and deserializes from the
/// canonical `"NONE"` wire value.
#[test]
fn function_calling_config_mode_none_wire_value() {
    let cfg: FunctionCallingConfig = serde_json::from_value(json!({ "mode": "NONE" })).unwrap();
    assert_eq!(cfg.mode, Some(FunctionCallingConfigMode::None));
    assert_eq!(serde_json::to_value(&cfg).unwrap()["mode"], json!("NONE"));

    // Sanity-check the other still-valid modes remain accepted.
    for wire in ["ANY", "AUTO", "MODE_UNSPECIFIED", "VALIDATED"] {
        let parsed: FunctionCallingConfig = serde_json::from_value(json!({ "mode": wire }))
            .unwrap_or_else(|e| panic!("{wire}: {e}"));
        assert_eq!(serde_json::to_value(&parsed).unwrap()["mode"], json!(wire));
    }
}
