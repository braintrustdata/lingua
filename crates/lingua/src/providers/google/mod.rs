// Google AI Generative Language API types
// Generated from Discovery REST API spec

pub mod adapter;
pub mod capabilities;
pub mod convert;
pub mod detect;
pub mod generated;
pub mod params;

#[cfg(test)]
pub mod test_google;

// Re-export adapter
pub use adapter::GoogleAdapter;

// Re-export capabilities
pub use capabilities::{GoogleCapabilities, GoogleThinkingStyle};

// Re-export detection functions
pub use detect::{try_parse_google, DetectionError};

// Re-export conversion functions
pub use convert::{google_to_universal, universal_to_google};

// Re-export the most commonly used Google AI types for convenience
pub use generated::{
    Candidate, Content, FunctionDeclaration, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, Part, SafetySetting, Threshold, Tool,
};

// Type aliases for convenience
pub type SafetySettings = Vec<SafetySetting>;

/// Returns true if the model ID represents a Vertex AI model.
///
/// Vertex models use the `publishers/` prefix
/// (e.g. `publishers/google/models/gemini-2.5-flash-preview-04-17`).
pub fn is_vertex_model(model: &str) -> bool {
    model.starts_with("publishers/")
}

/// Returns true if the model is a Google model hosted on Vertex AI.
///
/// These models have IDs starting with `publishers/google/`
/// (e.g. `publishers/google/models/gemini-2.5-flash`).
pub fn is_vertex_google_model(model: &str) -> bool {
    model.starts_with("publishers/google/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_vertex_google_model_matches_publishers_google_prefix() {
        assert!(is_vertex_google_model(
            "publishers/google/models/gemini-2.5-flash"
        ));
        assert!(is_vertex_google_model(
            "publishers/google/models/gemini-pro"
        ));
        assert!(is_vertex_google_model("publishers/google/something-else"));
    }

    #[test]
    fn is_vertex_google_model_rejects_other_publishers() {
        assert!(!is_vertex_google_model(
            "publishers/anthropic/models/claude-haiku-4-5"
        ));
        assert!(!is_vertex_google_model("publishers/meta/models/llama3"));
        assert!(!is_vertex_google_model("gemini-2.5-flash"));
        assert!(!is_vertex_google_model("publishers/"));
        assert!(!is_vertex_google_model(""));
    }

    // ---- Generated provider-type wire-compatibility regressions ----
    //
    // The Google Discovery spec renamed the `Part.mediaResolution` schema to
    // `V1mainMediaResolution` and freed up the `MediaResolution` name for the inline
    // `GenerationConfig.mediaResolution` enum, without changing either wire format. The
    // generator pins the previous public identifiers (`MediaResolution` struct,
    // `MediaResolutionEnum` enum). These tests lock the wire contract so the two distinct
    // media-resolution shapes keep parsing and serializing exactly as before.
    use crate::providers::google::generated::{
        AudioTranscriptionConfig, FunctionCallingConfigMode, LanguageHints, Level, MediaResolution,
        MediaResolutionEnum, Type,
    };
    use crate::serde_json::{self, json};

    #[test]
    fn part_media_resolution_is_the_level_wrapper_struct() {
        // Part.mediaResolution is the object wrapper `{ "level": <enum> }`, and its Level
        // enum includes the uncommon-but-valid MEDIA_RESOLUTION_ULTRA_HIGH value.
        let wire = json!({
            "text": "hi",
            "mediaResolution": { "level": "MEDIA_RESOLUTION_ULTRA_HIGH" }
        });
        let part: Part = serde_json::from_value(wire.clone()).expect("Part should deserialize");
        let mr = part
            .media_resolution
            .as_ref()
            .expect("media_resolution present");
        assert_eq!(mr.level, Some(Level::MediaResolutionUltraHigh));
        // Round-trips back to the same wire shape.
        assert_eq!(serde_json::to_value(&part).unwrap(), wire);
    }

    #[test]
    fn media_resolution_struct_roundtrips_all_levels() {
        for level in [
            "MEDIA_RESOLUTION_LOW",
            "MEDIA_RESOLUTION_HIGH",
            "MEDIA_RESOLUTION_ULTRA_HIGH",
        ] {
            let wire = json!({ "level": level });
            let parsed: MediaResolution = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
        }
    }

    #[test]
    fn generation_config_media_resolution_is_the_string_enum() {
        // GenerationConfig.mediaResolution is the bare SCREAMING_SNAKE_CASE string enum
        // (MediaResolutionEnum), a distinct type from the Part wrapper. It has no
        // ULTRA_HIGH variant.
        let wire = json!({ "mediaResolution": "MEDIA_RESOLUTION_LOW" });
        let cfg: GenerationConfig = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            cfg.media_resolution,
            Some(MediaResolutionEnum::MediaResolutionLow)
        );
        // Typed round-trip (GenerationConfig always emits `responseSchema: null`, so compare
        // the reparsed struct rather than the raw wire object).
        let reparsed: GenerationConfig =
            serde_json::from_value(serde_json::to_value(&cfg).unwrap()).unwrap();
        assert_eq!(cfg, reparsed);
    }

    #[test]
    fn generation_config_accepts_new_audio_transcription_config() {
        // The regenerated spec adds GenerationConfig.audioTranscriptionConfig. Confirm the
        // new field and its nested LanguageHints parse and round-trip on the wire.
        let wire = json!({
            "audioTranscriptionConfig": {
                "diarization": true,
                "wordTimestamp": false,
                "languageHints": { "languageCodes": ["en-US", "fr-FR"] }
            }
        });
        let cfg: GenerationConfig = serde_json::from_value(wire.clone()).unwrap();
        let atc = cfg
            .audio_transcription_config
            .clone()
            .expect("audio_transcription_config present");
        assert_eq!(atc.diarization, Some(true));
        assert_eq!(atc.word_timestamp, Some(false));
        assert_eq!(
            atc.language_hints,
            Some(LanguageHints {
                language_codes: Some(vec!["en-US".to_string(), "fr-FR".to_string()])
            })
        );
        // Typed round-trip: the nested AudioTranscriptionConfig survives serialize/parse.
        let reparsed: GenerationConfig =
            serde_json::from_value(serde_json::to_value(&cfg).unwrap()).unwrap();
        assert_eq!(cfg, reparsed);
    }

    #[test]
    fn audio_transcription_config_defaults_are_all_optional() {
        // Every field is optional; an empty object is valid and serializes back to `{}`.
        let atc: AudioTranscriptionConfig = serde_json::from_value(json!({})).unwrap();
        assert_eq!(serde_json::to_value(&atc).unwrap(), json!({}));
    }

    #[test]
    fn schema_type_enum_accepts_both_google_and_json_schema_casing() {
        // Google native "STRING" and OpenAI JSON-Schema lowercase "string" both parse to the
        // same variant, and serialization stays SCREAMING_SNAKE_CASE.
        let from_upper: Type = serde_json::from_value(json!("STRING")).unwrap();
        let from_lower: Type = serde_json::from_value(json!("string")).unwrap();
        assert_eq!(from_upper, Type::String);
        assert_eq!(from_lower, Type::String);
        assert_eq!(serde_json::to_value(Type::String).unwrap(), json!("STRING"));
    }

    #[test]
    fn function_calling_config_mode_none_uses_screaming_wire_value() {
        assert_eq!(
            serde_json::to_value(FunctionCallingConfigMode::None).unwrap(),
            json!("NONE")
        );
        let parsed: FunctionCallingConfigMode = serde_json::from_value(json!("NONE")).unwrap();
        assert_eq!(parsed, FunctionCallingConfigMode::None);
    }
}
