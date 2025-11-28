/*!
Shared Lingua-level capability profiles.
*/

/// Capabilities that impact Lingua → Lingua transformations before converting to a
/// provider-specific format.
#[derive(Debug, Clone)]
pub struct UniversalCapabilities {
    /// Whether the downstream provider can receive dedicated `system` role messages.
    pub supports_system_messages: bool,
    /// Whether raw file attachments (binary payloads) are supported.
    pub supports_file_attachments: bool,
    /// Whether tool-specific messages (tool role) are supported directly.
    pub supports_tool_messages: bool,
    /// Whether arbitrary multimodal content (images/audio/video) is supported.
    pub supports_multimodal: bool,
    /// Optional character limit per text segment. Text exceeding the limit will
    /// be truncated before conversion.
    pub max_message_length: Option<usize>,
}

impl Default for UniversalCapabilities {
    fn default() -> Self {
        Self {
            supports_system_messages: true,
            supports_file_attachments: true,
            supports_tool_messages: true,
            supports_multimodal: true,
            max_message_length: None,
        }
    }
}

impl UniversalCapabilities {
    /// Convenience helper that returns capabilities for a known provider slug.
    pub fn for_provider(provider: &str) -> Self {
        match provider {
            // OpenAI-style APIs accept the full Lingua surface area.
            "openai" | "fireworks" | "azure" => Self::default(),
            // Anthropic does not allow system messages and has narrower multimodal support.
            "anthropic" => Self {
                supports_system_messages: false,
                supports_file_attachments: false,
                supports_tool_messages: false,
                supports_multimodal: false,
                max_message_length: Some(4096),
            },
            // Fallback: be conservative.
            _ => Self {
                supports_system_messages: false,
                supports_file_attachments: false,
                supports_tool_messages: false,
                supports_multimodal: false,
                max_message_length: Some(2048),
            },
        }
    }
}
