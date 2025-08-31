/*!
Translation logic between Elmir universal format and provider-specific formats.
*/

pub mod openai;

use crate::universal::SimpleMessage;

/// Result type for translation operations
pub type TranslationResult<T> = anyhow::Result<T>;

/// Trait for bidirectional format translation
pub trait Translator<ProviderRequest, ProviderResponse> {
    /// Convert Elmir messages to provider request format
    fn to_provider_request(messages: Vec<SimpleMessage>) -> TranslationResult<ProviderRequest>;

    /// Convert provider response back to Elmir format
    fn from_provider_response(response: ProviderResponse) -> TranslationResult<Vec<SimpleMessage>>;
}

// Re-export convenience functions
pub use openai::{from_openai_response, to_openai_format};
