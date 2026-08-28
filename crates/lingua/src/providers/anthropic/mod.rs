/*!
Anthropic API provider types.

This module contains type definitions for Anthropic's messages API
generated from the OpenAPI specification using quicktype.
*/

pub mod adapter;
pub mod capabilities;
pub mod convert;
pub mod detect;
pub mod generated;
pub mod params;
pub(crate) mod tool_discovery;

pub(crate) fn deserialize_required_nullable<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    <Option<T> as serde::Deserialize>::deserialize(deserializer)
}

pub(crate) fn deserialize_optional_non_null<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(test)]
pub mod test_anthropic;

// Re-export adapter
pub use adapter::AnthropicAdapter;

// Re-export detection functions
pub use detect::{try_parse_anthropic, DetectionError};

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
