/*!
Provider format enum - the authoritative source of truth for provider formats.

This enum represents the different LLM provider API formats that lingua can handle.
It is used by both lingua and downstream crates like braintrust-llm-router.
*/

use serde::{Deserialize, Serialize};

/// Represents the API format/protocol used by an LLM provider.
///
/// This enum is the single source of truth for provider formats across the ecosystem.
/// When adding a new provider format:
/// 1. Add a variant here
/// 2. Update detection heuristics in `processing/detect.rs`
/// 3. Add conversion logic in `providers/<name>/convert.rs` if needed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderFormat {
    /// OpenAI Chat Completions API format (also used by OpenAI-compatible providers)
    OpenAI,
    /// Anthropic Messages API format
    Anthropic,
    /// Google AI / Gemini GenerateContent API format
    Google,
    /// Mistral AI API format (similar to OpenAI with some differences)
    Mistral,
    /// AWS Bedrock Converse API format
    Converse,
    /// JavaScript/TypeScript SDK format (used for Braintrust SDK)
    Js,
    /// Unknown or undetectable format
    #[default]
    Unknown,
}

impl ProviderFormat {
    /// Returns the lowercase string representation of the format.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderFormat::OpenAI => "openai",
            ProviderFormat::Anthropic => "anthropic",
            ProviderFormat::Google => "google",
            ProviderFormat::Mistral => "mistral",
            ProviderFormat::Converse => "converse",
            ProviderFormat::Js => "js",
            ProviderFormat::Unknown => "unknown",
        }
    }

    /// Returns true if this format is a known, supported format.
    pub fn is_known(&self) -> bool {
        !matches!(self, ProviderFormat::Unknown)
    }
}

impl std::fmt::Display for ProviderFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ProviderFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderFormat::OpenAI),
            "anthropic" => Ok(ProviderFormat::Anthropic),
            "google" => Ok(ProviderFormat::Google),
            "mistral" => Ok(ProviderFormat::Mistral),
            "converse" | "bedrock" => Ok(ProviderFormat::Converse),
            "js" => Ok(ProviderFormat::Js),
            _ => Ok(ProviderFormat::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json;

    #[test]
    fn test_as_str() {
        assert_eq!(ProviderFormat::OpenAI.as_str(), "openai");
        assert_eq!(ProviderFormat::Anthropic.as_str(), "anthropic");
        assert_eq!(ProviderFormat::Google.as_str(), "google");
        assert_eq!(ProviderFormat::Converse.as_str(), "converse");
        assert_eq!(ProviderFormat::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_serde_roundtrip() {
        let formats = vec![
            ProviderFormat::OpenAI,
            ProviderFormat::Anthropic,
            ProviderFormat::Google,
            ProviderFormat::Mistral,
            ProviderFormat::Converse,
            ProviderFormat::Js,
            ProviderFormat::Unknown,
        ];

        for format in formats {
            let serialized = serde_json::to_string(&format).unwrap();
            let deserialized: ProviderFormat = serde_json::from_str(&serialized).unwrap();
            assert_eq!(format, deserialized);
        }
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "openai".parse::<ProviderFormat>().unwrap(),
            ProviderFormat::OpenAI
        );
        assert_eq!(
            "ANTHROPIC".parse::<ProviderFormat>().unwrap(),
            ProviderFormat::Anthropic
        );
        assert_eq!(
            "bedrock".parse::<ProviderFormat>().unwrap(),
            ProviderFormat::Converse
        );
        assert_eq!(
            "unknown_format".parse::<ProviderFormat>().unwrap(),
            ProviderFormat::Unknown
        );
    }

    #[test]
    fn test_is_known() {
        assert!(ProviderFormat::OpenAI.is_known());
        assert!(ProviderFormat::Anthropic.is_known());
        assert!(!ProviderFormat::Unknown.is_known());
    }
}
