/*!
Amazon Bedrock API provider types.

This module contains type definitions for Amazon Bedrock's Converse API
using the official AWS SDK types for maximum compatibility.
*/

pub mod adapter;
pub mod anthropic;
pub mod convert;
pub mod detect;
pub mod params;
pub mod request;
pub mod response;

// Re-export adapter
pub use adapter::BedrockAdapter;

// Re-export commonly used AWS SDK types (note: these don't have Serde by default)
pub use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, Message, SystemContentBlock,
};

// Re-export detection functions
pub use detect::{try_parse_bedrock, DetectionError};

// Re-export conversion functions
pub use convert::{bedrock_to_universal, universal_to_bedrock};

// Re-export our custom request/response wrappers
pub use request::{ConverseRequest, ConverseStreamRequest};
pub use response::{ConverseResponse, ConverseStreamResponse};

/// Returns true if the model ID represents a Bedrock-hosted Anthropic model
/// that supports the native Anthropic Messages API via the invoke endpoint.
///
/// These models have IDs starting with `anthropic.` or containing `.anthropic.`
/// (for cross-region inference profiles like `us.anthropic.claude-*`).
pub fn is_bedrock_anthropic_model(model: &str) -> bool {
    model.starts_with("anthropic.") || model.contains(".anthropic.")
}

/// Returns true if the given format + model combination targets a Bedrock
/// Anthropic invoke endpoint (format is Anthropic and model is a Bedrock Anthropic model).
pub fn is_bedrock_anthropic_target(
    format: crate::capabilities::ProviderFormat,
    model: Option<&str>,
) -> bool {
    format == crate::capabilities::ProviderFormat::Anthropic
        && model.is_some_and(is_bedrock_anthropic_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bedrock_anthropic_model() {
        // Anthropic models on Bedrock
        assert!(is_bedrock_anthropic_model(
            "anthropic.claude-sonnet-4-5-20250929-v1:0"
        ));
        assert!(is_bedrock_anthropic_model(
            "us.anthropic.claude-haiku-4-5-20251001-v1:0"
        ));
        assert!(is_bedrock_anthropic_model(
            "eu.anthropic.claude-sonnet-4-20250514-v1:0"
        ));

        // Non-Anthropic models on Bedrock
        assert!(!is_bedrock_anthropic_model("amazon.nova-pro-v1:0"));
        assert!(!is_bedrock_anthropic_model("meta.llama3-70b-instruct-v1:0"));

        // Direct Anthropic models (not Bedrock)
        assert!(!is_bedrock_anthropic_model("claude-sonnet-4-20250514"));
        assert!(!is_bedrock_anthropic_model("claude-haiku-4-5-20251001"));
    }
}
