/*!
Provider-specific API type definitions.
*/

#[cfg(feature = "anthropic")]
pub mod anthropic;

#[cfg(feature = "bedrock")]
pub mod bedrock;

#[cfg(feature = "google")]
pub mod google;

/// Mistral uses OpenAI-compatible format, so it requires the openai feature
#[cfg(feature = "openai")]
pub mod mistral;

#[cfg(feature = "openai")]
pub mod openai;
