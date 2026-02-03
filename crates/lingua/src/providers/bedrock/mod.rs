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
