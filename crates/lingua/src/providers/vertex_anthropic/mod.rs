#[cfg(feature = "anthropic")]
pub mod adapter;

#[cfg(feature = "anthropic")]
pub use adapter::VertexAnthropicAdapter;

/// Returns true if the model ID represents a Vertex AI-hosted Anthropic model.
///
/// These models have IDs starting with `publishers/anthropic/`
/// (e.g. `publishers/anthropic/models/claude-haiku-4-5`).
pub fn is_vertex_anthropic_model(model: &str) -> bool {
    model.starts_with("publishers/anthropic/")
}

/// Returns true if the given format + model combination targets a Vertex
/// Anthropic rawPredict endpoint.
pub fn is_vertex_anthropic_target(
    format: crate::capabilities::ProviderFormat,
    model: Option<&str>,
) -> bool {
    matches!(
        format,
        crate::capabilities::ProviderFormat::Anthropic
            | crate::capabilities::ProviderFormat::VertexAnthropic
    ) && model.is_some_and(is_vertex_anthropic_model)
}
