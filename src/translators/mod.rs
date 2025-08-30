/*!
Translation logic between LLMIR universal format and provider-specific formats.

Each translator module handles bidirectional conversion:
- Universal → Provider (for requests)
- Provider → Universal (for responses)
*/

pub mod openai;
pub mod anthropic; 
pub mod google;

use crate::universal::SimpleMessage;

/// Result type for translation operations
pub type TranslationResult<T> = anyhow::Result<T>;

/// Trait for bidirectional format translation
pub trait Translator<ProviderRequest, ProviderResponse> {
    /// Convert LLMIR messages to provider request format
    fn to_provider_request(messages: Vec<SimpleMessage>) -> TranslationResult<ProviderRequest>;
    
    /// Convert provider response back to LLMIR format
    fn from_provider_response(response: ProviderResponse) -> TranslationResult<Vec<SimpleMessage>>;
}