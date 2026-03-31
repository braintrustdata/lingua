/*!
Amazon Bedrock API provider types.

This module contains type definitions for Amazon Bedrock's Converse API
using the official AWS SDK types for maximum compatibility.
*/

pub mod adapter;
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

#[cfg(feature = "anthropic")]
pub use crate::providers::bedrock_anthropic::{
    is_bedrock_anthropic_model, is_bedrock_anthropic_target,
};

#[cfg(all(test, feature = "anthropic"))]
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
