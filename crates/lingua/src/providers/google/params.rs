/*!
Typed parameter structs for Google GenerateContent API.

These structs use `#[serde(flatten)]` to automatically capture unknown fields,
eliminating the need for explicit KNOWN_KEYS arrays.
*/

use crate::providers::google::generated::{Content, GenerationConfig, Tool, ToolConfig};
use crate::serde_json::{self, Map, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Google GenerateContent API request parameters.
///
/// All known fields are explicitly typed. Unknown fields automatically
/// go into `extras` via `#[serde(flatten)]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleParams {
    // === Core fields ===
    pub model: Option<String>,
    pub contents: Option<Vec<Content>>,

    // === System prompt ===
    pub system_instruction: Option<Value>,

    // === Generation configuration ===
    pub generation_config: Option<GenerationConfig>,

    // === Safety settings ===
    pub safety_settings: Option<Value>,

    // === Tools and function calling ===
    pub tools: Option<Vec<Tool>>,
    pub tool_config: Option<ToolConfig>,

    // === Caching ===
    pub cached_content: Option<String>,

    /// Unknown fields - automatically captured by serde flatten.
    /// These are provider-specific fields not in the canonical set.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Value>,
}

/// Wire key under which Google-scoped extras carry the `generationConfig` fields that
/// have no `UniversalParams` representation.
///
/// The carried value is a partial Google `generationConfig`, so it merges straight back
/// into the exported request instead of needing an internal marker key.
pub const GENERATION_CONFIG_EXTRAS_KEY: &str = "generationConfig";

/// `generationConfig` wire keys that the Google adapter lifts into `UniversalParams`.
///
/// Everything outside this set has no universal representation and must be carried
/// through Google-scoped extras so `google -> universal -> google` stays lossless.
/// `canonical_generation_config_keys_are_complete` pins this list against the generated
/// type so a spec-side rename fails a test instead of silently dropping a field.
const CANONICAL_GENERATION_CONFIG_KEYS: &[&str] = &[
    "temperature",
    "topP",
    "topK",
    "maxOutputTokens",
    "stopSequences",
    "thinkingConfig",
    "responseMimeType",
    "responseJsonSchema",
    "responseSchema",
];

/// Returns the `generationConfig` fields that `UniversalParams` cannot represent.
///
/// The input is the typed generated struct, so this is a partition of a validated
/// provider type rather than an inspection of arbitrary JSON: keys lingua maps
/// canonically are removed and the remainder is carried verbatim.
pub fn unmapped_generation_config_fields(
    config: &GenerationConfig,
) -> Result<Map<String, Value>, serde_json::Error> {
    let Value::Object(mut fields) = serde_json::to_value(config)? else {
        return Ok(Map::new());
    };

    fields.retain(|key, value| {
        !value.is_null() && !CANONICAL_GENERATION_CONFIG_KEYS.contains(&key.as_str())
    });

    Ok(fields)
}

/// Typed view over Google-scoped extras.
///
/// Extras are read through this view instead of by plucking keys out of the raw map.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleExtrasView {
    /// `generationConfig` fields carried verbatim because lingua has no universal
    /// mapping for them (for example `audioTranscriptionConfig` or `speechConfig`).
    pub generation_config: Option<GenerationConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::google::generated::{
        AudioTranscriptionConfig, LanguageHints, ThinkingConfig,
    };
    use crate::serde_json;
    use crate::serde_json::json;

    #[test]
    fn test_google_params_known_fields() {
        let json = json!({
            "model": "gemini-pro",
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 1024
            }
        });

        let params: GoogleParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.model, Some("gemini-pro".to_string()));
        assert!(params.generation_config.is_some());
        assert!(params.extras.is_empty());
    }

    #[test]
    fn test_google_params_unknown_fields_go_to_extras() {
        let json = json!({
            "contents": [{"parts": [{"text": "Hello"}]}],
            "someFutureParam": "value"
        });

        let params: GoogleParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.extras.len(), 1);
        assert_eq!(
            params.extras.get("someFutureParam"),
            Some(&Value::String("value".to_string()))
        );
    }

    #[test]
    fn test_google_roundtrip_preserves_extras() {
        let json = json!({
            "contents": [],
            "customField": {"nested": "data"}
        });

        let params: GoogleParams = serde_json::from_value(json.clone()).unwrap();
        let back: Value = serde_json::to_value(&params).unwrap();

        // Custom field should be preserved
        assert_eq!(back.get("customField"), json.get("customField"));
    }

    #[test]
    fn test_google_params_types_audio_transcription_config() {
        let json = json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "generationConfig": {
                "audioTranscriptionConfig": {
                    "languageHints": {"languageCodes": ["en-US"]},
                    "diarization": true
                }
            }
        });

        let params: GoogleParams = serde_json::from_value(json).unwrap();

        // The field is captured by the typed GenerationConfig, not swallowed by the
        // top-level `#[serde(flatten)]` extras map.
        assert!(params.extras.is_empty());
        let config = params.generation_config.expect("generationConfig");
        let transcription = config
            .audio_transcription_config
            .expect("audioTranscriptionConfig");
        assert_eq!(transcription.diarization, Some(true));
        assert_eq!(
            transcription.language_hints,
            Some(LanguageHints {
                language_codes: Some(vec!["en-US".to_string()]),
            })
        );
    }

    #[test]
    fn canonical_generation_config_keys_are_complete() {
        // A config that sets only the fields lingua lifts into UniversalParams must
        // leave nothing to carry. This pins CANONICAL_GENERATION_CONFIG_KEYS against the
        // generated wire names: renaming any of them upstream fails here.
        let config = GenerationConfig {
            temperature: Some(0.5),
            top_p: Some(0.9),
            top_k: Some(20),
            max_output_tokens: Some(128),
            stop_sequences: Some(vec!["STOP".to_string()]),
            thinking_config: Some(ThinkingConfig {
                include_thoughts: Some(true),
                thinking_budget: Some(1024),
                thinking_level: None,
            }),
            response_mime_type: Some("application/json".to_string()),
            generation_config_response_json_schema: Some(json!({"type": "object"})),
            ..Default::default()
        };

        let serialized = serde_json::to_value(&config).unwrap();
        let Value::Object(serialized) = serialized else {
            panic!("GenerationConfig must serialize to an object");
        };
        for key in CANONICAL_GENERATION_CONFIG_KEYS {
            assert!(
                serialized.contains_key(*key),
                "canonical key '{key}' is not a GenerationConfig wire key anymore"
            );
        }

        assert!(unmapped_generation_config_fields(&config)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn unmapped_generation_config_fields_carries_unmapped_keys_only() {
        let config = GenerationConfig {
            temperature: Some(0.5),
            audio_transcription_config: Some(AudioTranscriptionConfig {
                adaptation_phrases: Some(vec!["Lingua".to_string()]),
                custom_vocabulary: Some(vec!["Gemini".to_string()]),
                ..Default::default()
            }),
            candidate_count: Some(2),
            seed: Some(7),
            ..Default::default()
        };

        let unmapped = unmapped_generation_config_fields(&config).unwrap();

        assert_eq!(
            unmapped.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["audioTranscriptionConfig", "candidateCount", "seed"]
        );
        assert_eq!(
            unmapped.get("audioTranscriptionConfig"),
            Some(&json!({
                "adaptationPhrases": ["Lingua"],
                "customVocabulary": ["Gemini"]
            }))
        );
    }

    #[test]
    fn google_extras_view_reads_carried_generation_config() {
        let extras = json!({
            "someFutureParam": "value",
            "generationConfig": {
                "audioTranscriptionConfig": {"wordTimestamp": true}
            }
        });

        let view: GoogleExtrasView = serde_json::from_value(extras).unwrap();
        let config = view.generation_config.expect("carried generationConfig");
        assert_eq!(
            config
                .audio_transcription_config
                .and_then(|c| c.word_timestamp),
            Some(true)
        );
    }
}
