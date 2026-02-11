#[cfg(feature = "anthropic")]
pub mod adapter;

#[cfg(feature = "anthropic")]
pub use adapter::BedrockAnthropicAdapter;

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
    matches!(
        format,
        crate::capabilities::ProviderFormat::Anthropic
            | crate::capabilities::ProviderFormat::BedrockAnthropic
    ) && model.is_some_and(is_bedrock_anthropic_model)
}
