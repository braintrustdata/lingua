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
}

impl OpenAICapabilities {
    pub fn detect(
        request: &CreateChatCompletionRequestClass,
        target: TargetProvider,
    ) -> OpenAICapabilities {
        let model = request.model.to_ascii_lowercase();
        let uses_reasoning_mode =
            request.reasoning_effort.is_some() || is_reasoning_model_name(&model);

        OpenAICapabilities {
            uses_reasoning_mode,
            is_legacy_o1_model: is_legacy_o1_model(&model),
            supports_native_structured_output: supports_native_structured_output(&model, target),
        }
    }

    pub fn requires_reasoning_transforms(&self) -> bool {
        self.uses_reasoning_mode
    }
}

fn supports_native_structured_output(model: &str, target: TargetProvider) -> bool {
    model.starts_with("gpt")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || matches!(target, TargetProvider::Fireworks)
}

fn is_reasoning_model_name(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.starts_with("o1")
        || lower.starts_with("o2")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
        || lower.starts_with("gpt-5")
}

fn is_legacy_o1_model(model: &str) -> bool {
    matches!(model, "o1-preview" | "o1-mini" | "o1-preview-2024-09-12")
}
