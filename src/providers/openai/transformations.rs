/*!
OpenAI-specific request transformation utilities.

This module contains request preprocessing steps that mirror the legacy proxy
behavior. The transformations operate on generated OpenAI request types and
prepare payloads for downstream providers.
*/

use std::{cell::Cell, fmt};

use crate::{
    providers::openai::generated::{
        AllowedToolsFunction, ChatCompletionRequestMessageContent,
        ChatCompletionRequestMessageContentPart, ChatCompletionRequestMessageRole,
        ChatCompletionToolChoiceOption, CreateChatCompletionRequestClass, File, FunctionObject,
        FunctionToolChoiceClass, FunctionToolChoiceType, PurpleType, ResponseFormatType,
        ToolElement, ToolType,
    },
    serde_json::{Map, Value},
};
use thiserror::Error;

/// Result alias for transformation operations.
pub type TransformResult<T> = Result<T, TransformError>;

/// Errors that can occur during OpenAI payload transformations.
#[derive(Debug, Error)]
pub enum TransformError {
    /// A required field was missing in the request payload.
    #[error("missing required field: {field}")]
    MissingField { field: &'static str },
    /// The payload contained an invalid value that cannot be normalized.
    #[error("invalid value: {message}")]
    InvalidValue { message: String },
    /// The requested feature is not supported for the target provider.
    #[error("unsupported feature: {feature}")]
    Unsupported { feature: String },
    /// Wrapper for other kinds of transformation errors.
    #[error("{message}")]
    Other { message: String },
}

impl TransformError {
    /// Convenience for constructing a generic error from a string.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}

/// Target provider that will receive the translated OpenAI payload.
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

/// Context required to execute the transformation pipeline.
#[derive(Debug)]
pub struct TransformationConfig<'a> {
    pub target_provider: TargetProvider,
    pub provider_metadata: Option<&'a Map<String, Value>>,
    pub url_path: Option<&'a str>,
    pub supports_streaming: Option<bool>,
}

impl<'a> Default for TransformationConfig<'a> {
    fn default() -> Self {
        Self {
            target_provider: TargetProvider::OpenAI,
            provider_metadata: None,
            url_path: None,
            supports_streaming: None,
        }
    }
}

#[derive(Debug, Default)]
struct TransformerState {
    managed_structured_output: Cell<bool>,
    use_responses_api: Cell<bool>,
}

impl TransformerState {
    fn managed_structured_output(&self) -> bool {
        self.managed_structured_output.get()
    }

    fn mark_managed_structured_output(&self) {
        self.managed_structured_output.set(true);
    }

    fn use_responses_api(&self) -> bool {
        self.use_responses_api.get()
    }

    #[allow(dead_code)]
    fn set_use_responses_api(&self, value: bool) {
        self.use_responses_api.set(value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransformationStage {
    ProviderRouting,
    ProviderSanitization,
    Reasoning,
    MessageNormalization,
    ResponseFormat,
    StreamingOverrides,
    ModelRouting,
}

impl TransformationStage {
    fn ordered() -> &'static [Self] {
        &[
            Self::ProviderRouting,
            Self::ProviderSanitization,
            Self::Reasoning,
            Self::MessageNormalization,
            Self::ResponseFormat,
            Self::StreamingOverrides,
            Self::ModelRouting,
        ]
    }
}

struct CapabilityView<'a> {
    request: &'a CreateChatCompletionRequestClass,
}

impl CapabilityView<'_> {
    fn is_reasoning_model(&self) -> bool {
        is_reasoning_model(self.request)
    }

    fn has_response_format(&self) -> bool {
        self.request.response_format.is_some()
    }
}

struct TransformationContext<'a, 'cfg> {
    request: &'a mut CreateChatCompletionRequestClass,
    config: &'a TransformationConfig<'cfg>,
    state: &'a TransformerState,
}

impl TransformationContext<'_, '_> {
    fn supports_native_structured_output(&self) -> bool {
        supports_native_structured_output(self.request, self.config)
    }

    fn mark_managed_structured_output(&self) {
        self.state.mark_managed_structured_output();
    }
}

trait TransformationCapability: Sync {
    fn stage(&self) -> TransformationStage;
    fn applies(&self, view: &CapabilityView<'_>) -> bool {
        let _ = view;
        true
    }
    fn apply(&self, ctx: &mut TransformationContext<'_, '_>) -> TransformResult<()>;
}

/// Pipeline for applying OpenAI request transformations in a structured order.
pub struct OpenAIRequestTransformer<'a> {
    request: &'a mut CreateChatCompletionRequestClass,
    config: TransformationConfig<'a>,
    state: TransformerState,
}

impl<'a> OpenAIRequestTransformer<'a> {
    /// Create a new transformer for the provided request.
    pub fn new(request: &'a mut CreateChatCompletionRequestClass) -> Self {
        Self {
            request,
            config: TransformationConfig::default(),
            state: TransformerState::default(),
        }
    }

    /// Override the target provider that will receive the payload.
    pub fn with_target_provider(mut self, provider: TargetProvider) -> Self {
        self.config.target_provider = provider;
        self
    }

    /// Attach provider-specific metadata used during transformations.
    pub fn with_provider_metadata(mut self, metadata: Option<&'a Map<String, Value>>) -> Self {
        self.config.provider_metadata = metadata;
        self
    }

    /// Set the intended URL path (e.g. `/responses`, `/chat/completions`).
    pub fn with_url_path(mut self, url_path: Option<&'a str>) -> Self {
        self.config.url_path = url_path;
        self
    }

    /// Specify whether the downstream provider supports streaming responses.
    pub fn with_supports_streaming(mut self, supports_streaming: Option<bool>) -> Self {
        self.config.supports_streaming = supports_streaming;
        self
    }

    /// Execute the full transformation pipeline.
    pub fn transform(&mut self) -> TransformResult<()> {
        for stage in TransformationStage::ordered() {
            self.run_stage(*stage)?;
        }
        Ok(())
    }

    /// Indicates whether structured output is being managed by Lingua.
    pub fn managed_structured_output(&self) -> bool {
        self.state.managed_structured_output()
    }

    /// Indicates whether the request should be issued against the Responses API.
    pub fn use_responses_api(&self) -> bool {
        self.state.use_responses_api()
    }

    fn run_stage(&mut self, stage: TransformationStage) -> TransformResult<()> {
        for capability in capabilities()
            .iter()
            .filter(|capability| capability.stage() == stage)
        {
            let should_run = {
                let view = CapabilityView {
                    request: &*self.request,
                };
                capability.applies(&view)
            };

            if !should_run {
                continue;
            }

            let mut context = TransformationContext {
                request: &mut *self.request,
                config: &self.config,
                state: &self.state,
            };

            capability.apply(&mut context)?;
        }

        Ok(())
    }
}

struct ReasoningCapability;
struct MessageNormalizationCapability;
struct ResponseFormatCapability;

impl TransformationCapability for ReasoningCapability {
    fn stage(&self) -> TransformationStage {
        TransformationStage::Reasoning
    }

    fn applies(&self, view: &CapabilityView<'_>) -> bool {
        view.is_reasoning_model()
    }

    fn apply(&self, ctx: &mut TransformationContext<'_, '_>) -> TransformResult<()> {
        if let Some(max_tokens) = ctx.request.max_tokens.take() {
            ctx.request.max_completion_tokens = Some(max_tokens);
        }

        ctx.request.temperature = None;
        ctx.request.parallel_tool_calls = None;

        if is_legacy_o1_model(&ctx.request.model) {
            for message in &mut ctx.request.messages {
                if matches!(message.role, ChatCompletionRequestMessageRole::System) {
                    message.role = ChatCompletionRequestMessageRole::User;
                }
            }
        }

        Ok(())
    }
}

impl TransformationCapability for MessageNormalizationCapability {
    fn stage(&self) -> TransformationStage {
        TransformationStage::MessageNormalization
    }

    fn apply(&self, ctx: &mut TransformationContext<'_, '_>) -> TransformResult<()> {
        for message in &mut ctx.request.messages {
            if matches!(message.role, ChatCompletionRequestMessageRole::User) {
                if let Some(ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) =
                    message.content.as_mut()
                {
                    for part in parts.iter_mut() {
                        normalize_content_part(part)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl TransformationCapability for ResponseFormatCapability {
    fn stage(&self) -> TransformationStage {
        TransformationStage::ResponseFormat
    }

    fn applies(&self, view: &CapabilityView<'_>) -> bool {
        view.has_response_format()
    }

    fn apply(&self, ctx: &mut TransformationContext<'_, '_>) -> TransformResult<()> {
        let Some(response_format) = ctx.request.response_format.take() else {
            return Ok(());
        };

        match response_format.text_type {
            ResponseFormatType::Text => Ok(()),
            ResponseFormatType::JsonSchema => {
                if ctx.supports_native_structured_output() {
                    ctx.request.response_format = Some(response_format);
                    return Ok(());
                }

                if ctx
                    .request
                    .tools
                    .as_ref()
                    .is_some_and(|tools| !tools.is_empty())
                    || ctx.request.function_call.is_some()
                    || ctx.request.tool_choice.is_some()
                {
                    return Err(TransformError::Unsupported {
                        feature: "tools_with_structured_output".to_string(),
                    });
                }

                match response_format.json_schema {
                    Some(schema) => {
                        ctx.request.tools = Some(vec![ToolElement {
                            function: Some(FunctionObject {
                                description: Some("Output the result in JSON format".to_string()),
                                name: "json".to_string(),
                                parameters: schema.schema.clone(),
                                strict: schema.strict,
                            }),
                            tool_type: ToolType::Function,
                            custom: None,
                        }]);

                        ctx.request.tool_choice =
                            Some(ChatCompletionToolChoiceOption::FunctionToolChoiceClass(
                                FunctionToolChoiceClass {
                                    allowed_tools: None,
                                    allowed_tools_type: FunctionToolChoiceType::Function,
                                    function: Some(AllowedToolsFunction {
                                        name: "json".to_string(),
                                    }),
                                    custom: None,
                                },
                            ));

                        ctx.mark_managed_structured_output();
                        Ok(())
                    }
                    None => Err(TransformError::InvalidValue {
                        message: "json_schema response_format is missing schema".to_string(),
                    }),
                }
            }
            ResponseFormatType::JsonObject => {
                ctx.request.response_format = Some(response_format);
                Ok(())
            }
        }
    }
}

static REASONING_CAPABILITY: ReasoningCapability = ReasoningCapability;
static MESSAGE_NORMALIZATION_CAPABILITY: MessageNormalizationCapability =
    MessageNormalizationCapability;
static RESPONSE_FORMAT_CAPABILITY: ResponseFormatCapability = ResponseFormatCapability;

static CAPABILITIES: [&'static dyn TransformationCapability; 3] = [
    &REASONING_CAPABILITY,
    &MESSAGE_NORMALIZATION_CAPABILITY,
    &RESPONSE_FORMAT_CAPABILITY,
];

fn capabilities() -> &'static [&'static dyn TransformationCapability] {
    &CAPABILITIES
}

fn is_reasoning_model(request: &CreateChatCompletionRequestClass) -> bool {
    request.reasoning_effort.is_some() || is_reasoning_model_name(&request.model)
}

fn supports_native_structured_output(
    request: &CreateChatCompletionRequestClass,
    config: &TransformationConfig<'_>,
) -> bool {
    let model = request.model.to_ascii_lowercase();
    model.starts_with("gpt")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || matches!(config.target_provider, TargetProvider::Fireworks)
}

fn normalize_content_part(
    part: &mut ChatCompletionRequestMessageContentPart,
) -> TransformResult<()> {
    match part.chat_completion_request_message_content_part_type {
        PurpleType::ImageUrl => normalize_image_part(part),
        _ => Ok(()),
    }
}

fn normalize_image_part(part: &mut ChatCompletionRequestMessageContentPart) -> TransformResult<()> {
    let Some(image_url_value) = part
        .image_url
        .as_ref()
        .map(|image_url| image_url.url.clone())
    else {
        return Ok(());
    };

    if let Some(data_url) = DataUrl::parse(&image_url_value) {
        if !data_url.media_type.starts_with("image/") {
            convert_image_part_to_file(part, image_url_value.clone(), data_url.media_type);
        }
    }

    Ok(())
}

fn convert_image_part_to_file(
    part: &mut ChatCompletionRequestMessageContentPart,
    url: String,
    media_type: &str,
) {
    part.chat_completion_request_message_content_part_type = PurpleType::File;
    part.image_url = None;
    part.file = Some(File {
        file_data: Some(url),
        file_id: None,
        filename: Some(if media_type == "application/pdf" {
            "file_from_base64.pdf".to_string()
        } else {
            "file_from_base64".to_string()
        }),
    });
}

struct DataUrl<'a> {
    media_type: &'a str,
}

impl<'a> DataUrl<'a> {
    fn parse(input: &'a str) -> Option<Self> {
        if !input.starts_with("data:") {
            return None;
        }
        let without_prefix = &input["data:".len()..];
        let (meta, payload) = without_prefix.split_once(',')?;
        if payload.is_empty() {
            return None;
        }
        let (media_type, encoding) = meta.split_once(';')?;
        if encoding != "base64" {
            return None;
        }
        Some(Self { media_type })
    }
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

impl fmt::Debug for OpenAIRequestTransformer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAIRequestTransformer")
            .field("config", &self.config)
            .field(
                "managed_structured_output",
                &self.state.managed_structured_output(),
            )
            .field("use_responses_api", &self.state.use_responses_api())
            .finish()
    }
}
