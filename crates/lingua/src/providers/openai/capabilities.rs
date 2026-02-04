/*!
OpenAI-specific capability detection used by the transformation pipeline.
*/

use crate::providers::openai::generated::CreateChatCompletionRequestClass;

/// Target provider that will receive a translated OpenAI payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProvider {
    OpenAI,
    Azure,
    Vertex,
    Fireworks,
    Mistral,
    Databricks,
    Lepton,
    Other,
}

impl std::str::FromStr for TargetProvider {
    type Err = std::convert::Infallible;

    fn from_str(provider: &str) -> Result<Self, Self::Err> {
        Ok(match provider {
            "openai" => Self::OpenAI,
            "azure" => Self::Azure,
            "vertex" => Self::Vertex,
            "fireworks" => Self::Fireworks,
            "mistral" => Self::Mistral,
            "databricks" => Self::Databricks,
            "lepton" => Self::Lepton,
            _ => Self::Other,
        })
    }
}

/// Capability view derived from a request/model combination.
#[derive(Debug, Clone)]
pub struct OpenAICapabilities {
    pub uses_reasoning_mode: bool,
    pub is_legacy_o1_model: bool,
    pub supports_native_structured_output: bool,

    // Provider-specific limitations
    pub supports_stream_options: bool,
    pub supports_parallel_tools: bool,
    pub supports_seed_field: bool,
    pub requires_model_normalization: bool,
}

impl OpenAICapabilities {
    pub fn detect(
        request: &CreateChatCompletionRequestClass,
        target: TargetProvider,
    ) -> OpenAICapabilities {
        let model = request.model.to_ascii_lowercase();
        let uses_reasoning_mode =
            request.reasoning_effort.is_some() || is_reasoning_model_name(&model);

        // Provider-specific capability detection
        let (
            supports_stream_options,
            supports_parallel_tools,
            supports_seed_field,
            requires_model_normalization,
        ) = match target {
            TargetProvider::Mistral => (false, false, true, false),
            TargetProvider::Fireworks => (false, true, true, false),
            TargetProvider::Databricks => (false, false, true, false),
            TargetProvider::Azure => (true, false, true, false),
            TargetProvider::Vertex => (true, true, true, true),
            TargetProvider::OpenAI | TargetProvider::Lepton | TargetProvider::Other => {
                (true, true, true, false)
            }
        };

        OpenAICapabilities {
            uses_reasoning_mode,
            is_legacy_o1_model: is_legacy_o1_model(&model),
            supports_native_structured_output: supports_native_structured_output(&model, target),
            supports_stream_options,
            supports_parallel_tools,
            supports_seed_field,
            requires_model_normalization,
        }
    }

    pub fn requires_reasoning_transforms(&self) -> bool {
        self.uses_reasoning_mode
    }

    /// Check if seed field should be removed for Azure with API version
    pub fn should_remove_seed_for_azure(
        &self,
        target: TargetProvider,
        has_api_version: bool,
    ) -> bool {
        matches!(target, TargetProvider::Azure) && has_api_version
    }
}

/// Model prefixes that support native structured output.
const STRUCTURED_OUTPUT_PREFIXES: &[&str] = &["gpt", "o1", "o3"];

/// Model prefixes that indicate reasoning models.
const REASONING_MODEL_PREFIXES: &[&str] = &["o1", "o2", "o3", "o4", "gpt-5"];

/// Legacy o1 models that need special handling.
const LEGACY_O1_MODELS: &[&str] = &["o1-preview", "o1-mini", "o1-preview-2024-09-12"];

fn supports_native_structured_output(model: &str, target: TargetProvider) -> bool {
    STRUCTURED_OUTPUT_PREFIXES
        .iter()
        .any(|prefix| model.starts_with(prefix))
        || matches!(target, TargetProvider::Fireworks)
}

/// Check if a model name indicates a reasoning model that requires special handling.
///
/// Reasoning models (o1, o2, o3, o4, gpt-5) require `max_completion_tokens` instead
/// of `max_tokens`, so passthrough must be disabled to ensure proper conversion.
pub fn is_reasoning_model_name(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    REASONING_MODEL_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn is_legacy_o1_model(model: &str) -> bool {
    LEGACY_O1_MODELS.contains(&model)
}
