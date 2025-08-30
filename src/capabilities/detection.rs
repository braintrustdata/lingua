/*!
Capability detection for different providers.
*/

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Capabilities supported by a provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Capabilities {
    /// Web search capability
    pub web_search: bool,
    /// Code execution capability
    pub code_execution: bool,
    /// File processing capability
    pub file_processing: bool,
    /// Image generation capability
    pub image_generation: bool,
    /// Reasoning/thinking capability
    pub reasoning: bool,
    /// Context caching capability
    pub context_caching: bool,
    /// Streaming support
    pub streaming: bool,
    /// Tool calling support
    pub tool_calling: bool,
}

/// Provider-specific capabilities information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProviderCapabilities {
    /// Provider name
    pub provider: String,
    /// Supported capabilities
    pub capabilities: Capabilities,
    /// Supported models
    pub models: Vec<String>,
}

/// Detect capabilities for a given provider
pub fn detect_capabilities(provider: &str) -> ProviderCapabilities {
    match provider {
        "openai" => ProviderCapabilities {
            provider: provider.to_string(),
            capabilities: Capabilities {
                web_search: true,
                code_execution: true,
                file_processing: true,
                image_generation: true,
                reasoning: true,
                context_caching: true,
                streaming: true,
                tool_calling: true,
            },
            models: vec!["gpt-4".to_string(), "gpt-4-turbo".to_string()],
        },
        "anthropic" => ProviderCapabilities {
            provider: provider.to_string(),
            capabilities: Capabilities {
                web_search: true,
                code_execution: true,
                file_processing: true,
                image_generation: false,
                reasoning: true,
                context_caching: true,
                streaming: true,
                tool_calling: true,
            },
            models: vec!["claude-3-5-sonnet-20241022".to_string()],
        },
        _ => ProviderCapabilities {
            provider: provider.to_string(),
            capabilities: Capabilities {
                web_search: false,
                code_execution: false,
                file_processing: false,
                image_generation: false,
                reasoning: false,
                context_caching: false,
                streaming: false,
                tool_calling: false,
            },
            models: vec![],
        },
    }
}