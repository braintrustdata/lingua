/*!
Anthropic API provider types.

This module contains type definitions for Anthropic's messages API
generated from the OpenAPI specification using quicktype.
*/

pub mod convert;
pub mod detect;
pub mod generated;

#[cfg(test)]
pub mod test_anthropic;

// Re-export detection functions and detector
pub use detect::{
    detect_payload_format, is_anthropic_format, is_anthropic_format_heuristic, AnthropicDetector,
    DetectionError, PayloadFormat,
};

// Re-export key generated types (automated approach with proper request/response separation)
// Temporarily disabled while testing generation
/*
pub use generated::{
    // Request types
    CreateMessageParams,
    InputMessage,

    // Response types
    Message,
    Usage,

    // Shared types
    StopReason,

    // Core error types with proper enum typing
    WebSearchToolResultErrorCode,
};
*/
