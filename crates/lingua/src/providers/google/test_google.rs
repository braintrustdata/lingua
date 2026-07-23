use crate::providers::google::generated::{
    Content as GoogleContent, GenerateContentRequest, GenerateContentResponse,
};
use crate::serde_json::Value;
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::run_roundtrip_test;
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};

pub type GoogleTestCase = TestCase<GenerateContentRequest, GenerateContentResponse, Value>;

pub fn discover_google_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<GoogleTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<GenerateContentRequest, GenerateContentResponse, Value>(
        Provider::Google,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        let cases = discover_google_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        run_roundtrip_test(
            case,
            // Extract messages from request
            |request: &GenerateContentRequest| {
                Ok(request.contents.as_ref().expect("missing contents").clone())
            },
            // Convert to universal
            |messages: &Vec<GoogleContent>| {
                <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(messages.clone())
                    .map_err(|e| format!("Failed to convert to universal format: {}", e))
            },
            // Convert from universal
            |messages: Vec<Message>| {
                <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed roundtrip conversion: {}", e))
            },
            // Extract response content (candidate contents)
            |response: &GenerateContentResponse| {
                Ok(response
                    .candidates
                    .as_ref()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|candidate| candidate.content.clone())
                    .collect())
            },
            // Convert response to universal
            |contents: &Vec<GoogleContent>| {
                <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(contents.clone())
                    .map_err(|e| format!("Failed to convert response to universal format: {}", e))
            },
            // Convert universal to response content
            |messages: Vec<Message>| {
                <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))
            },
        )
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_google_tests.rs"));
    }

    // Regression coverage for the Google provider type update that renamed the
    // `Part.mediaResolution` referent from `MediaResolution` to `V1MainMediaResolution`,
    // reclaimed the `MediaResolution` name for the `GenerationConfig.mediaResolution`
    // string enum (previously `MediaResolutionEnum`), and added the additive
    // `GenerationConfig.audioTranscriptionConfig` field. Wire formats are unchanged, so
    // previously-valid payloads must continue to deserialize and round-trip losslessly.
    mod provider_type_update_regression {
        use crate::providers::google::generated::{
            AudioTranscriptionConfig, GenerationConfig, LanguageHints, Level, MediaResolution,
            Part, V1MainMediaResolution,
        };
        use serde_json::json;

        // Part.mediaResolution is now the V1MainMediaResolution object. The uncommon but
        // valid MEDIA_RESOLUTION_ULTRA_HIGH level must still deserialize and round-trip.
        #[test]
        fn test_part_media_resolution_v1main_ultra_high_roundtrips() {
            let payload = json!({
                "text": "hello",
                "mediaResolution": { "level": "MEDIA_RESOLUTION_ULTRA_HIGH" }
            });

            let part: Part = serde_json::from_value(payload.clone())
                .expect("Part with V1MainMediaResolution must deserialize");

            assert_eq!(
                part.media_resolution,
                Some(V1MainMediaResolution {
                    level: Some(Level::MediaResolutionUltraHigh),
                }),
            );

            let reserialized = serde_json::to_value(&part).expect("Part must serialize");
            assert_eq!(
                reserialized, payload,
                "Part media resolution must round-trip"
            );
        }

        // GenerationConfig.mediaResolution is the string enum formerly named
        // MediaResolutionEnum. Its wire values are unchanged and must still deserialize.
        #[test]
        fn test_generation_config_media_resolution_enum_roundtrips() {
            for (wire, variant) in [
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
            ] {
                let payload = json!({ "mediaResolution": wire });
                let config: GenerationConfig = serde_json::from_value(payload.clone())
                    .unwrap_or_else(|e| panic!("GenerationConfig {wire} must deserialize: {e}"));
                assert_eq!(config.media_resolution, Some(variant));

                // Round-trip at the struct level: GenerationConfig always emits an
                // unrelated `responseSchema: null` (Box<Option<Schema>> cannot use
                // skip_serializing_if), so compare parsed structs rather than raw JSON.
                let reserialized =
                    serde_json::to_value(&config).expect("GenerationConfig must serialize");
                let reparsed: GenerationConfig = serde_json::from_value(reserialized)
                    .expect("re-serialized GenerationConfig must deserialize");
                assert_eq!(config, reparsed, "media resolution enum must round-trip");
            }
        }

        // Additive audioTranscriptionConfig field, including the nested LanguageHints
        // object, must deserialize and round-trip losslessly.
        #[test]
        fn test_generation_config_audio_transcription_config_roundtrips() {
            let payload = json!({
                "audioTranscriptionConfig": {
                    "languageHints": { "languageCodes": ["en-US", "es-ES"] },
                    "diarization": true,
                    "wordTimestamp": false,
                    "customVocabulary": ["Lingua", "Gemini"]
                }
            });

            let config: GenerationConfig = serde_json::from_value(payload.clone())
                .expect("GenerationConfig with audioTranscriptionConfig must deserialize");

            assert_eq!(
                config.audio_transcription_config,
                Some(AudioTranscriptionConfig {
                    language_hints: Some(LanguageHints {
                        language_codes: Some(vec!["en-US".to_string(), "es-ES".to_string()]),
                    }),
                    diarization: Some(true),
                    word_timestamp: Some(false),
                    custom_vocabulary: Some(vec!["Lingua".to_string(), "Gemini".to_string()]),
                    ..Default::default()
                }),
            );

            let reserialized =
                serde_json::to_value(&config).expect("GenerationConfig must serialize");
            let reparsed: GenerationConfig = serde_json::from_value(reserialized)
                .expect("re-serialized GenerationConfig must deserialize");
            assert_eq!(
                config, reparsed,
                "audio transcription config must round-trip"
            );
        }
    }
}
