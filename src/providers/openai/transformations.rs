/*!
OpenAI-specific request transformation utilities.

This module contains request preprocessing steps that mirror the legacy proxy
behavior. The transformations operate on generated OpenAI request types and
prepare payloads for downstream providers.
*/

use std::fmt;

use crate::{
    providers::openai::capabilities::OpenAICapabilities,
    providers::openai::generated::{
        AllowedToolsFunction, ChatCompletionRequestMessageContent,
        ChatCompletionRequestMessageContentPart, ChatCompletionRequestMessageRole,
        ChatCompletionToolChoiceOption, CreateChatCompletionRequestClass, File, FunctionObject,
        FunctionToolChoiceClass, FunctionToolChoiceType, ImageUrl, PurpleType, ResponseFormatType,
        ToolElement, ToolType,
    },
    serde_json::{Map, Value},
    util::media::{
        fetch_url_to_base64, is_localhost_url, media_block_to_url, parse_base64_data_url,
        parse_file_metadata_from_url,
    },
};

pub use crate::providers::openai::capabilities::TargetProvider;
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

/// Context required to execute the transformation pipeline.
#[derive(Debug)]
pub struct TransformationConfig<'a> {
    pub target_provider: TargetProvider,
    pub provider_metadata: Option<&'a Map<String, Value>>,
    pub url_path: Option<&'a str>,
    pub supports_streaming: Option<bool>,
}

impl Default for TransformationConfig<'_> {
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
    managed_structured_output: bool,
    use_responses_api: bool,
}

impl TransformerState {
    fn managed_structured_output(&self) -> bool {
        self.managed_structured_output
    }

    fn mark_managed_structured_output(&mut self) {
        self.managed_structured_output = true;
    }

    fn use_responses_api(&self) -> bool {
        self.use_responses_api
    }

    #[allow(dead_code)]
    fn set_use_responses_api(&mut self, value: bool) {
        self.use_responses_api = value;
    }
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

    /// Execute the full transformation pipeline (synchronous version).
    ///
    /// This version does not fetch localhost or PDF URLs. Use `transform_async`
    /// if you need URL fetching support.
    pub fn transform(&mut self) -> TransformResult<()> {
        let capabilities = OpenAICapabilities::detect(self.request, self.config.target_provider);

        // Apply reasoning model transformations
        if capabilities.requires_reasoning_transforms() {
            self.apply_reasoning_transforms(&capabilities)?;
        }

        // Apply provider-specific field sanitization
        self.apply_provider_sanitization(&capabilities)?;

        // Apply model name normalization if needed
        if capabilities.requires_model_normalization {
            self.apply_model_normalization()?;
        }

        // Check for Responses API routing
        self.check_responses_api_routing(&capabilities);

        // Continue with existing transformations (synchronous)
        self.normalize_user_messages_sync()?;
        self.apply_response_format(&capabilities)?;

        Ok(())
    }

    /// Execute the full transformation pipeline (asynchronous version).
    ///
    /// This version fetches localhost URLs and PDF URLs, converting them to
    /// inline base64 data URLs. This mirrors the proxy's behavior in
    /// `normalizeOpenAIContent`.
    pub async fn transform_async(&mut self) -> TransformResult<()> {
        let capabilities = OpenAICapabilities::detect(self.request, self.config.target_provider);

        // Apply reasoning model transformations
        if capabilities.requires_reasoning_transforms() {
            self.apply_reasoning_transforms(&capabilities)?;
        }

        // Apply provider-specific field sanitization
        self.apply_provider_sanitization(&capabilities)?;

        // Apply model name normalization if needed
        if capabilities.requires_model_normalization {
            self.apply_model_normalization()?;
        }

        // Check for Responses API routing
        self.check_responses_api_routing(&capabilities);

        // Continue with existing transformations (async for URL fetching)
        self.normalize_user_messages_async().await?;
        self.apply_response_format(&capabilities)?;

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

    fn apply_reasoning_transforms(
        &mut self,
        capabilities: &OpenAICapabilities,
    ) -> TransformResult<()> {
        if let Some(max_tokens) = self.request.max_tokens.take() {
            self.request.max_completion_tokens = Some(max_tokens);
        }

        self.request.temperature = None;
        self.request.parallel_tool_calls = None;

        if capabilities.is_legacy_o1_model {
            for message in &mut self.request.messages {
                if matches!(message.role, ChatCompletionRequestMessageRole::System) {
                    message.role = ChatCompletionRequestMessageRole::User;
                }
            }
        }

        Ok(())
    }

    fn normalize_user_messages_sync(&mut self) -> TransformResult<()> {
        // NOTE: The proxy removes `reasoning` fields from messages:
        //   if ("reasoning" in message) { delete message.reasoning; }
        // In Rust, this is handled automatically by the type system - since
        // `ChatCompletionRequestMessage` doesn't have a `reasoning` field,
        // any such field in input JSON is dropped during deserialization
        // and won't be serialized back out.

        for message in &mut self.request.messages {
            if matches!(message.role, ChatCompletionRequestMessageRole::User) {
                if let Some(ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) =
                    message.content.as_mut()
                {
                    for part in parts.iter_mut() {
                        normalize_content_part_sync(part)?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn normalize_user_messages_async(&mut self) -> TransformResult<()> {
        // NOTE: The proxy removes `reasoning` fields from messages:
        //   if ("reasoning" in message) { delete message.reasoning; }
        // In Rust, this is handled automatically by the type system - since
        // `ChatCompletionRequestMessage` doesn't have a `reasoning` field,
        // any such field in input JSON is dropped during deserialization
        // and won't be serialized back out.

        for message in &mut self.request.messages {
            if matches!(message.role, ChatCompletionRequestMessageRole::User) {
                if let Some(ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) =
                    message.content.as_mut()
                {
                    for part in parts.iter_mut() {
                        normalize_content_part_async(part).await?;
                    }
                }
            }
        }

        Ok(())
    }

    fn apply_response_format(&mut self, capabilities: &OpenAICapabilities) -> TransformResult<()> {
        let Some(response_format) = self.request.response_format.take() else {
            return Ok(());
        };

        match response_format.text_type {
            ResponseFormatType::Text => Ok(()),
            ResponseFormatType::JsonSchema => {
                if capabilities.supports_native_structured_output {
                    self.request.response_format = Some(response_format);
                    return Ok(());
                }

                if self
                    .request
                    .tools
                    .as_ref()
                    .is_some_and(|tools| !tools.is_empty())
                    || self.request.function_call.is_some()
                    || self.request.tool_choice.is_some()
                {
                    return Err(TransformError::Unsupported {
                        feature: "tools_with_structured_output".to_string(),
                    });
                }

                match response_format.json_schema {
                    Some(schema) => {
                        self.request.tools = Some(vec![ToolElement {
                            function: Some(FunctionObject {
                                description: Some("Output the result in JSON format".to_string()),
                                name: "json".to_string(),
                                parameters: schema.schema.clone(),
                                strict: schema.strict,
                            }),
                            tool_type: ToolType::Function,
                            custom: None,
                        }]);

                        self.request.tool_choice =
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

                        self.state.mark_managed_structured_output();
                        Ok(())
                    }
                    None => Err(TransformError::InvalidValue {
                        message: "json_schema response_format is missing schema".to_string(),
                    }),
                }
            }
            ResponseFormatType::JsonObject => {
                self.request.response_format = Some(response_format);
                Ok(())
            }
        }
    }

    fn apply_provider_sanitization(
        &mut self,
        capabilities: &OpenAICapabilities,
    ) -> TransformResult<()> {
        // Remove stream_options for providers that don't support it
        if !capabilities.supports_stream_options {
            self.request.stream_options = None;
        }

        // Remove parallel_tool_calls for providers that don't support it
        if !capabilities.supports_parallel_tools {
            self.request.parallel_tool_calls = None;
        }

        // Remove seed field for Azure with API version
        let has_api_version = self
            .config
            .provider_metadata
            .as_ref()
            .and_then(|meta| meta.get("api_version"))
            .is_some();

        if capabilities.should_remove_seed_for_azure(self.config.target_provider, has_api_version) {
            self.request.seed = None;
        }

        Ok(())
    }

    fn apply_model_normalization(&mut self) -> TransformResult<()> {
        // Normalize Vertex model names
        if self.config.target_provider == TargetProvider::Vertex {
            if self.request.model.starts_with("publishers/meta/models/") {
                // Strip to "meta/..." format
                self.request.model = self
                    .request
                    .model
                    .strip_prefix("publishers/")
                    .and_then(|s| s.strip_prefix("meta/models/"))
                    .map(|s| format!("meta/{}", s))
                    .unwrap_or_else(|| self.request.model.clone());
            } else if let Some(stripped) = self.request.model.strip_prefix("publishers/") {
                // Strip "publishers/X/models/Y" to "Y"
                if let Some(model_part) = stripped.split("/models/").nth(1) {
                    self.request.model = model_part.to_string();
                }
            }
        }

        Ok(())
    }

    fn check_responses_api_routing(&mut self, capabilities: &OpenAICapabilities) {
        if capabilities.requires_responses_api {
            self.state.use_responses_api = true;
        }
    }
}

// =============================================================================
// Synchronous content normalization (no URL fetching)
// =============================================================================

fn normalize_content_part_sync(
    part: &mut ChatCompletionRequestMessageContentPart,
) -> TransformResult<()> {
    match part.chat_completion_request_message_content_part_type {
        PurpleType::ImageUrl => normalize_image_part_sync(part),
        _ => Ok(()),
    }
}

fn normalize_image_part_sync(
    part: &mut ChatCompletionRequestMessageContentPart,
) -> TransformResult<()> {
    let Some(image_url_value) = part
        .image_url
        .as_ref()
        .map(|image_url| image_url.url.clone())
    else {
        return Ok(());
    };

    // Handle base64 data URLs - convert non-images to file type
    if let Some(data_url) = parse_base64_data_url(&image_url_value) {
        if !data_url.media_type.starts_with("image/") {
            convert_image_part_to_file(part, image_url_value.clone(), &data_url.media_type);
        }
    }

    Ok(())
}

// =============================================================================
// Asynchronous content normalization (with URL fetching)
// =============================================================================

async fn normalize_content_part_async(
    part: &mut ChatCompletionRequestMessageContentPart,
) -> TransformResult<()> {
    match part.chat_completion_request_message_content_part_type {
        PurpleType::ImageUrl => normalize_image_part_async(part).await,
        _ => Ok(()),
    }
}

/// Maximum media size for URL fetching (20 MB, matching proxy)
const MAX_MEDIA_BYTES: usize = 20 * 1024 * 1024;

async fn normalize_image_part_async(
    part: &mut ChatCompletionRequestMessageContentPart,
) -> TransformResult<()> {
    let Some(image_url_value) = part
        .image_url
        .as_ref()
        .map(|image_url| image_url.url.clone())
    else {
        return Ok(());
    };

    // Handle base64 data URLs - convert non-images to file type
    if let Some(data_url) = parse_base64_data_url(&image_url_value) {
        if !data_url.media_type.starts_with("image/") {
            convert_image_part_to_file(part, image_url_value.clone(), &data_url.media_type);
        }
        return Ok(());
    }

    // Check if this is a PDF URL that should be fetched and converted to file
    // Mirrors proxy: parseFileMetadataFromUrl + .pdf extension or content-type check
    if let Some(metadata) = parse_file_metadata_from_url(&image_url_value) {
        let is_pdf = metadata.filename.ends_with(".pdf")
            || metadata.content_type.as_deref() == Some("application/pdf");

        if is_pdf {
            // Fetch PDF and convert to file type
            match fetch_url_to_base64(
                &image_url_value,
                Some(&["application/pdf"]),
                Some(MAX_MEDIA_BYTES),
            )
            .await
            {
                Ok(media_block) => {
                    let data_url = media_block_to_url(&media_block);
                    part.chat_completion_request_message_content_part_type = PurpleType::File;
                    part.image_url = None;
                    part.file = Some(File {
                        file_data: Some(data_url),
                        file_id: None,
                        filename: Some(metadata.filename),
                    });
                    return Ok(());
                }
                Err(e) => {
                    // Log but don't fail - let OpenAI handle the URL
                    #[cfg(debug_assertions)]
                    eprintln!("Warning: Failed to fetch PDF URL: {}", e);
                    let _ = e; // silence unused warning in release
                }
            }
        }
    }

    // Check if this is a localhost URL that should be fetched
    // Mirrors proxy: http://127.0.0.1 or http://localhost
    if is_localhost_url(&image_url_value) {
        match fetch_url_to_base64(&image_url_value, None, Some(MAX_MEDIA_BYTES)).await {
            Ok(media_block) => {
                let data_url = media_block_to_url(&media_block);
                part.image_url = Some(ImageUrl {
                    url: data_url,
                    detail: part.image_url.as_ref().and_then(|u| u.detail.clone()),
                });
                return Ok(());
            }
            Err(e) => {
                // Log but don't fail - localhost URLs might not be accessible
                #[cfg(debug_assertions)]
                eprintln!("Warning: Failed to fetch localhost URL: {}", e);
                let _ = e; // silence unused warning in release
            }
        }
    }

    Ok(())
}

// =============================================================================
// Shared utilities
// =============================================================================

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
