/*!
Stop sequences conversion utilities for cross-provider semantic translation.

This module provides bidirectional conversion between different providers'
stop sequences configurations:
- OpenAI: `stop: string | string[]` (allows single string or array)
- Anthropic: `stop_sequences: string[]`
- Google: `generationConfig.stop_sequences: string[]`
- Bedrock: `inferenceConfig.stopSequences: string[]`

## Design

The conversion normalizes all inputs to a `Vec<String>` for cross-provider
compatibility while preserving the original value in `raw` for lossless
same-provider round-trips.

## Usage

```ignore
use std::convert::TryInto;
use crate::capabilities::ProviderFormat;
use crate::universal::request::StopConfig;

// FROM: Parse provider-specific value to universal config
let config: StopConfig = (ProviderFormat::OpenAI, &raw_json).try_into()?;

// TO: Convert universal config to provider-specific value
let output = config.to_provider(ProviderFormat::OpenAI)?;
```
*/

use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::processing::transform::TransformError;
use crate::serde_json::{json, Value};
use crate::universal::request::StopConfig;

// =============================================================================
// TryFrom Implementation for FROM Conversions
// =============================================================================

impl<'a> TryFrom<(ProviderFormat, &'a Value)> for StopConfig {
    type Error = TransformError;

    fn try_from((_provider, value): (ProviderFormat, &'a Value)) -> Result<Self, Self::Error> {
        // All providers use the same parsing logic - normalize to sequences array
        Ok(from_value(value))
    }
}

// =============================================================================
// to_provider Method for TO Conversions
// =============================================================================

impl StopConfig {
    /// Convert this config to a provider-specific value.
    ///
    /// # Arguments
    /// * `provider` - Target provider format
    ///
    /// # Returns
    /// `Ok(Some(value))` if conversion succeeded
    /// `Ok(None)` if sequences are empty
    /// `Err(_)` if conversion failed
    pub fn to_provider(
        &self,
        provider: ProviderFormat,
    ) -> Result<Option<Value>, TransformError> {
        match provider {
            ProviderFormat::OpenAI | ProviderFormat::Responses => Ok(to_openai(self)),
            ProviderFormat::Anthropic | ProviderFormat::Google | ProviderFormat::Converse => {
                Ok(to_array(self).map(|arr| Value::Array(arr.into_iter().map(Value::String).collect())))
            }
            _ => Ok(None),
        }
    }

    /// Get the sequences as a Vec<String> for providers that need arrays.
    ///
    /// This is a convenience method for providers that need the raw array.
    pub fn to_sequences_array(&self) -> Option<Vec<String>> {
        to_array(self)
    }
}

// =============================================================================
// Private Helper Functions - FROM Provider Formats
// =============================================================================

/// Parse stop sequences from any provider format.
///
/// Handles:
/// - `"single_string"` → `["single_string"]`
/// - `["arr", "of", "strings"]` → `["arr", "of", "strings"]`
/// - Other types → empty sequences with raw preserved
fn from_value(value: &Value) -> StopConfig {
    let sequences = match value {
        Value::String(s) => vec![s.clone()],
        Value::Array(arr) => arr
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect(),
        _ => vec![],
    };

    StopConfig {
        sequences,
        raw: Some(value.clone()),
    }
}

// =============================================================================
// Private Helper Functions - TO Provider Formats
// =============================================================================

/// Convert StopConfig to OpenAI format.
///
/// For lossless round-trip: returns the original `raw` value if available.
/// For cross-provider conversion: returns the appropriate format based on sequence count.
///
/// Output format:
/// - `"single_string"` (if single sequence and raw was a string)
/// - `["arr", "of", "strings"]` (if multiple sequences or raw was array)
fn to_openai(config: &StopConfig) -> Option<Value> {
    // For lossless round-trip, prefer raw
    if let Some(ref raw) = config.raw {
        return Some(raw.clone());
    }

    // Cross-provider conversion from sequences
    if config.sequences.is_empty() {
        return None;
    }

    // OpenAI accepts either string or array, prefer array for consistency
    Some(Value::Array(
        config.sequences.iter().map(|s| json!(s)).collect(),
    ))
}

/// Convert StopConfig to array format for providers that only accept arrays.
///
/// Used by: Anthropic, Google, Bedrock
///
/// For lossless round-trip: extracts array from raw if it was already an array.
/// For cross-provider conversion: returns sequences as array.
fn to_array(config: &StopConfig) -> Option<Vec<String>> {
    if config.sequences.is_empty() {
        // If we have raw, try to extract from it
        if let Some(ref raw) = config.raw {
            return match raw {
                Value::Array(arr) => {
                    let sequences: Vec<String> = arr
                        .iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect();
                    if sequences.is_empty() {
                        None
                    } else {
                        Some(sequences)
                    }
                }
                Value::String(s) => Some(vec![s.clone()]),
                _ => None,
            };
        }
        return None;
    }

    Some(config.sequences.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_from_string() {
        let value = json!("stop_word");
        let config: StopConfig = (ProviderFormat::OpenAI, &value).try_into().unwrap();
        assert_eq!(config.sequences, vec!["stop_word"]);
        assert!(config.raw.is_some());
    }

    #[test]
    fn test_from_array() {
        let value = json!(["stop1", "stop2", "stop3"]);
        let config: StopConfig = (ProviderFormat::OpenAI, &value).try_into().unwrap();
        assert_eq!(config.sequences, vec!["stop1", "stop2", "stop3"]);
    }

    #[test]
    fn test_to_openai_roundtrip_string() {
        let original = json!("single");
        let config: StopConfig = (ProviderFormat::OpenAI, &original).try_into().unwrap();
        let back = config.to_provider(ProviderFormat::OpenAI).unwrap().unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_to_openai_roundtrip_array() {
        let original = json!(["a", "b"]);
        let config: StopConfig = (ProviderFormat::OpenAI, &original).try_into().unwrap();
        let back = config.to_provider(ProviderFormat::OpenAI).unwrap().unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_to_sequences_array() {
        let config = StopConfig {
            sequences: vec!["stop1".into(), "stop2".into()],
            raw: None,
        };
        let arr = config.to_sequences_array().unwrap();
        assert_eq!(arr, vec!["stop1", "stop2"]);
    }

    #[test]
    fn test_cross_provider_string_to_array() {
        // OpenAI string input → Anthropic array output
        let openai_value = json!("single_stop");
        let config: StopConfig = (ProviderFormat::OpenAI, &openai_value).try_into().unwrap();
        let anthropic_arr = config.to_sequences_array().unwrap();
        assert_eq!(anthropic_arr, vec!["single_stop"]);
    }

    #[test]
    fn test_empty_sequences() {
        let config = StopConfig::default();
        assert!(config.to_provider(ProviderFormat::OpenAI).unwrap().is_none());
        assert!(config.to_sequences_array().is_none());
    }
}
