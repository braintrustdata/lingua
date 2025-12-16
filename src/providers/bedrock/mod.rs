/*!
Amazon Bedrock API provider types.

This module contains type definitions for Amazon Bedrock's Converse API
using the official AWS SDK types for maximum compatibility.
*/

pub mod convert;
pub mod detect;
pub mod request;
pub mod response;

use crate::serde_json::Value;

// Re-export commonly used AWS SDK types (note: these don't have Serde by default)
pub use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, Message, SystemContentBlock,
};

// Re-export detection functions and detector
pub use detect::{
    is_bedrock_converse, is_bedrock_converse_value, try_parse_bedrock, ConverseDetector,
    DetectionError,
};

// Re-export conversion functions
pub use convert::{bedrock_to_universal, universal_to_bedrock};

// Re-export our custom request/response wrappers
pub use request::{ConverseRequest, ConverseStreamRequest};
pub use response::{ConverseResponse, ConverseStreamResponse};

/// Wrapper for AWS Bedrock Converse payloads used in format detection.
///
/// Since Bedrock's types are from the AWS SDK and don't have serde support by default,
/// this wrapper stores the validated raw JSON for Bedrock payloads, enabling a simpler
/// API for format detection and routing.
#[derive(Debug, Clone)]
pub struct BedrockPayload {
    /// The raw JSON payload (validated to be Bedrock Converse format)
    pub raw: Value,
    /// The model ID extracted from the payload
    pub model_id: Option<String>,
}

impl BedrockPayload {
    /// Create a new BedrockPayload from a validated JSON value.
    pub fn new(raw: Value) -> Self {
        let model_id = raw
            .get("modelId")
            .and_then(|v| v.as_str())
            .map(String::from);
        Self { raw, model_id }
    }

    /// Get the raw JSON value.
    pub fn into_value(self) -> Value {
        self.raw
    }
}
