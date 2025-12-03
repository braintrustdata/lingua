/*!
Provider format enum - the authoritative source of truth for provider formats.

This enum represents the different LLM provider API formats that lingua can handle.
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
            _ => Ok(ProviderFormat::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
