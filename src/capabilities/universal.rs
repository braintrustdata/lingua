/*!
Shared Lingua-level capability profiles.

This module defines capabilities that control universal transformations
applied to messages before converting to provider-specific formats.
*/

/// Capabilities that control universal message transformations.
///
/// These capabilities determine which preprocessing steps are applied
/// to messages before they are converted to provider-specific formats.
#[derive(Debug, Clone, Default)]
pub struct UniversalCapabilities {
    /// Whether consecutive messages of the same role should be merged.
    /// Required for: Anthropic, Google, Bedrock. Not for: OpenAI.
    pub requires_message_flattening: bool,
    /// Whether system messages should be extracted to a separate parameter.
    /// Required for: Anthropic, Google, Bedrock. Not for: OpenAI.
    pub system_messages_separate: bool,
}

impl UniversalCapabilities {
    /// Returns capabilities for a known provider.
    ///
    /// # Example
    ///
    /// ```
    /// use lingua::capabilities::universal::UniversalCapabilities;
    ///
    /// let caps = UniversalCapabilities::for_provider("anthropic");
    /// assert!(caps.requires_message_flattening);
    /// assert!(caps.system_messages_separate);
    ///
    /// let caps = UniversalCapabilities::for_provider("openai");
    /// assert!(!caps.requires_message_flattening);
    /// assert!(!caps.system_messages_separate);
    /// ```
    pub fn for_provider(provider: &str) -> Self {
        match provider {
            // OpenAI-compatible providers handle messages inline
            "openai" | "azure" | "fireworks" | "mistral" | "databricks" | "lepton" | "cerebras" => {
                Self::openai_compatible()
            }
            // These providers require message flattening and separate system messages
            "anthropic" | "google" | "bedrock" | "vertex" => Self::requires_preprocessing(),
            // Conservative default: apply preprocessing
            _ => Self::requires_preprocessing(),
        }
    }

    /// Capabilities for OpenAI-compatible providers.
    /// No preprocessing needed - messages can be sent as-is.
    fn openai_compatible() -> Self {
        Self {
            requires_message_flattening: false,
            system_messages_separate: false,
        }
    }

    /// Capabilities for providers that require message preprocessing.
    /// These providers need consecutive messages merged and system messages extracted.
    fn requires_preprocessing() -> Self {
        Self {
            requires_message_flattening: true,
            system_messages_separate: true,
        }
    }
}
