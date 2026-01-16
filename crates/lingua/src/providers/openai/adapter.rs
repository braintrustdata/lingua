/*!
OpenAI Chat Completions API adapter.

This module provides the `OpenAIAdapter` for the standard Chat Completions API,
along with target-specific transformation utilities for providers like Azure,
Vertex, and Mistral.
*/

use crate::capabilities::ProviderFormat;
use std::collections::HashMap;

use crate::processing::adapters::{
    insert_opt_bool, insert_opt_f64, insert_opt_i64, insert_opt_value, ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::openai::capabilities::{OpenAICapabilities, TargetProvider};
use crate::providers::openai::generated::{
    AllowedToolsFunction, ChatCompletionRequestMessage, ChatCompletionRequestMessageContent,
    ChatCompletionRequestMessageContentPart, ChatCompletionRequestMessageRole,
    ChatCompletionResponseMessage, ChatCompletionToolChoiceOption,
    CreateChatCompletionRequestClass, File, FunctionObject, FunctionToolChoiceClass,
    FunctionToolChoiceType, PurpleType, ResponseFormatType, ToolElement, ToolType,
};
use crate::providers::openai::params::OpenAIChatParams;
use crate::providers::openai::try_parse_openai;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use std::convert::TryInto;
use crate::universal::tools::{anthropic_to_openai_tools, find_builtin_tool, is_openai_format};
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalStreamChoice,
    UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};
use crate::util::media::parse_base64_data_url;

/// Adapter for OpenAI Chat Completions API.
pub struct OpenAIAdapter;

impl ProviderAdapter for OpenAIAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    fn directory_name(&self) -> &'static str {
        "chat-completions"
    }

    fn display_name(&self) -> &'static str {
        "ChatCompletions"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_openai(payload).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        // Parse into typed params - extras are automatically captured via #[serde(flatten)]
        let typed_params: OpenAIChatParams = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Parse again for strongly-typed message conversion
        let request: CreateChatCompletionRequestClass = serde_json::from_value(payload)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Build canonical params from typed fields
        let params = UniversalParams {
            temperature: typed_params.temperature,
            top_p: typed_params.top_p,
            top_k: None, // OpenAI doesn't support top_k
            max_tokens: typed_params.max_tokens.or(typed_params.max_completion_tokens),
            stop: typed_params
                .stop
                .as_ref()
                .and_then(|v| (ProviderFormat::OpenAI, v).try_into().ok()),
            tools: typed_params.tools,
            tool_choice: typed_params
                .tool_choice
                .as_ref()
                .and_then(|v| (ProviderFormat::OpenAI, v).try_into().ok()),
            response_format: typed_params
                .response_format
                .as_ref()
                .and_then(|v| (ProviderFormat::OpenAI, v).try_into().ok()),
            seed: typed_params.seed,
            presence_penalty: typed_params.presence_penalty,
            frequency_penalty: typed_params.frequency_penalty,
            stream: typed_params.stream,
            // New canonical fields
            parallel_tool_calls: typed_params.parallel_tool_calls,
            reasoning: typed_params.reasoning_effort.as_ref().and_then(|e| {
                let json_value = serde_json::json!(e);
                (ProviderFormat::OpenAI, &json_value).try_into().ok()
            }),
            metadata: typed_params.metadata,
            store: typed_params.store,
            service_tier: typed_params.service_tier,
            logprobs: typed_params.logprobs,
            top_logprobs: typed_params.top_logprobs,
        };

        // Collect provider-specific extras for round-trip preservation
        // This includes both unknown fields (from serde flatten) and known OpenAI fields
        // that aren't part of UniversalParams
        let mut extras_map: Map<String, Value> = typed_params.extras.into_iter().collect();

        // Add OpenAI-specific known fields that aren't in UniversalParams
        if let Some(user) = typed_params.user {
            extras_map.insert("user".into(), Value::String(user));
        }
        if let Some(n) = typed_params.n {
            extras_map.insert("n".into(), Value::Number(n.into()));
        }
        if let Some(logit_bias) = typed_params.logit_bias {
            extras_map.insert("logit_bias".into(), logit_bias);
        }
        if let Some(stream_options) = typed_params.stream_options {
            extras_map.insert("stream_options".into(), stream_options);
        }
        if let Some(prediction) = typed_params.prediction {
            extras_map.insert("prediction".into(), prediction);
        }
        if let Some(safety_identifier) = typed_params.safety_identifier {
            extras_map.insert("safety_identifier".into(), Value::String(safety_identifier));
        }
        if let Some(prompt_cache_key) = typed_params.prompt_cache_key {
            extras_map.insert("prompt_cache_key".into(), Value::String(prompt_cache_key));
        }

        let mut provider_extras = HashMap::new();
        if !extras_map.is_empty() {
            provider_extras.insert(ProviderFormat::OpenAI, extras_map);
        }

        Ok(UniversalRequest {
            model: typed_params.model,
            messages,
            params,
            provider_extras,
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::OpenAI,
            reason: "missing model".to_string(),
        })?;

        let openai_messages: Vec<ChatCompletionRequestMessage> =
            <Vec<ChatCompletionRequestMessage> as TryFromLLM<Vec<Message>>>::try_from(
                req.messages.clone(),
            )
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert(
            "messages".into(),
            serde_json::to_value(openai_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Insert params
        insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "max_completion_tokens", req.params.max_tokens);
        insert_opt_value(
            &mut obj,
            "stop",
            req.params
                .stop
                .as_ref()
                .and_then(|s| s.to_provider(ProviderFormat::OpenAI).ok())
                .flatten(),
        );
        // Convert tools from Anthropic format to OpenAI format if needed
        if let Some(tools) = &req.params.tools {
            // Check for Anthropic built-in tools that have no OpenAI equivalent
            if let Some(builtin_tool) = find_builtin_tool(tools) {
                return Err(TransformError::ProviderLimitation(format!(
                    "Anthropic built-in tool '{}' has no OpenAI equivalent",
                    builtin_tool
                )));
            }

            if is_openai_format(tools) {
                insert_opt_value(&mut obj, "tools", Some(tools.clone()));
            } else {
                // Convert from Anthropic format
                insert_opt_value(&mut obj, "tools", anthropic_to_openai_tools(tools));
            }
        }
        // OpenAI doesn't use parallel_tool_calls in tool_choice conversion, pass None
        insert_opt_value(
            &mut obj,
            "tool_choice",
            req.params
                .tool_choice
                .as_ref()
                .and_then(|tc| tc.to_provider(ProviderFormat::OpenAI, None).ok())
                .flatten(),
        );
        insert_opt_value(
            &mut obj,
            "response_format",
            req.params
                .response_format
                .as_ref()
                .and_then(|rf| rf.to_provider(ProviderFormat::OpenAI).ok())
                .flatten(),
        );
        insert_opt_i64(&mut obj, "seed", req.params.seed);
        insert_opt_f64(&mut obj, "presence_penalty", req.params.presence_penalty);
        insert_opt_f64(&mut obj, "frequency_penalty", req.params.frequency_penalty);
        insert_opt_bool(&mut obj, "logprobs", req.params.logprobs);
        insert_opt_i64(&mut obj, "top_logprobs", req.params.top_logprobs);
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Add parallel_tool_calls from canonical params
        if let Some(parallel) = req.params.parallel_tool_calls {
            obj.insert("parallel_tool_calls".into(), Value::Bool(parallel));
        }

        // Add reasoning_effort from canonical params (convert ReasoningConfig back to string)
        // to_provider returns Value::String for OpenAI format
        // max_tokens is passed explicitly for budget→effort conversion
        if let Some(effort_value) = req
            .params
            .reasoning
            .as_ref()
            .and_then(|r| r.to_provider(ProviderFormat::OpenAI, req.params.max_tokens).ok())
            .flatten()
        {
            obj.insert("reasoning_effort".into(), effort_value);
        }

        // Add metadata from canonical params
        if let Some(metadata) = req.params.metadata.as_ref() {
            obj.insert("metadata".into(), metadata.clone());
        }

        // Add store from canonical params
        if let Some(store) = req.params.store {
            obj.insert("store".into(), Value::Bool(store));
        }

        // Add service_tier from canonical params
        if let Some(ref service_tier) = req.params.service_tier {
            obj.insert("service_tier".into(), Value::String(service_tier.clone()));
        }

        // If streaming, ensure stream_options.include_usage is set for usage reporting
        if req.params.stream == Some(true) {
            let stream_options = obj
                .entry("stream_options")
                .or_insert_with(|| serde_json::json!({}));
            if let Value::Object(opts) = stream_options {
                opts.insert("include_usage".into(), Value::Bool(true));
            }
        }

        // Merge back provider-specific extras (only for OpenAI)
        if let Some(extras) = req.provider_extras.get(&ProviderFormat::OpenAI) {
            for (k, v) in extras {
                obj.insert(k.clone(), v.clone());
            }
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // OpenAI doesn't require any specific defaults
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // OpenAI chat completion response has choices[].message and object="chat.completion"
        payload.get("choices").and_then(Value::as_array).is_some()
            && payload
                .get("object")
                .and_then(Value::as_str)
                .is_some_and(|o| o == "chat.completion")
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing choices".to_string()))?;

        let mut messages = Vec::new();
        let mut finish_reason = None;

        for choice in choices {
            if let Some(msg_val) = choice.get("message") {
                let response_msg: ChatCompletionResponseMessage =
                    serde_json::from_value(msg_val.clone())
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal = <Message as TryFromLLM<&ChatCompletionResponseMessage>>::try_from(
                    &response_msg,
                )
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finish_reason from first choice
            if finish_reason.is_none() {
                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                    finish_reason = Some(reason.parse().unwrap());
                }
            }
        }

        let usage = payload.get("usage").map(|u| UniversalUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(Value::as_i64),
            completion_tokens: u.get("completion_tokens").and_then(Value::as_i64),
            prompt_cached_tokens: u
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(Value::as_i64),
            prompt_cache_creation_tokens: None, // OpenAI doesn't report cache creation tokens
            completion_reasoning_tokens: u
                .get("completion_tokens_details")
                .and_then(|d| d.get("reasoning_tokens"))
                .and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            messages,
            usage,
            finish_reason,
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let finish_reason = self
            .map_finish_reason(resp.finish_reason.as_ref())
            .unwrap_or_else(|| "stop".to_string());

        let choices: Vec<Value> = resp
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let response_msg =
                    <ChatCompletionResponseMessage as TryFromLLM<&Message>>::try_from(msg)
                        .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

                let message_value = serde_json::to_value(&response_msg)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "index": i,
                    "message": message_value,
                    "finish_reason": finish_reason
                }))
            })
            .collect::<Result<Vec<_>, TransformError>>()?;

        let usage = resp.usage.as_ref().map(|u| {
            let input = u.prompt_tokens.unwrap_or(0);
            let output = u.completion_tokens.unwrap_or(0);
            serde_json::json!({
                "prompt_tokens": input,
                "completion_tokens": output,
                "total_tokens": input + output
            })
        });

        let mut obj = serde_json::json!({
            "id": format!("chatcmpl-{}", PLACEHOLDER_ID),
            "object": "chat.completion",
            "created": 0,
            "model": resp.model.as_deref().unwrap_or(PLACEHOLDER_MODEL),
            "choices": choices
        });

        if let Some(usage_val) = usage {
            obj.as_object_mut()
                .unwrap()
                .insert("usage".into(), usage_val);
        }

        Ok(obj)
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
        reason.map(|r| match r {
            FinishReason::Stop => "stop".to_string(),
            FinishReason::Length => "length".to_string(),
            FinishReason::ToolCalls => "tool_calls".to_string(),
            FinishReason::ContentFilter => "content_filter".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
    }

    // =========================================================================
    // Streaming response handling
    // =========================================================================

    fn detect_stream_response(&self, payload: &Value) -> bool {
        // OpenAI streaming chunk has object="chat.completion.chunk"
        // or has choices array with delta field
        if let Some(obj) = payload.get("object").and_then(Value::as_str) {
            if obj == "chat.completion.chunk" {
                return true;
            }
        }

        // Fallback: check for choices with delta
        if let Some(choices) = payload.get("choices").and_then(Value::as_array) {
            if choices.iter().any(|c| c.get("delta").is_some()) {
                return true;
            }
        }

        false
    }

    fn stream_to_universal(
        &self,
        payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        // OpenAI is the canonical format, so this is mostly direct mapping
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .map(|c| {
                        let index = c.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let delta = c.get("delta").cloned();
                        let finish_reason = c
                            .get("finish_reason")
                            .and_then(Value::as_str)
                            .map(String::from);
                        UniversalStreamChoice {
                            index,
                            delta,
                            finish_reason,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Extract usage if present (usually only on final chunk)
        let usage = payload.get("usage").map(|u| UniversalUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(Value::as_i64),
            completion_tokens: u.get("completion_tokens").and_then(Value::as_i64),
            prompt_cached_tokens: u
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(Value::as_i64),
            prompt_cache_creation_tokens: None,
            completion_reasoning_tokens: u
                .get("completion_tokens_details")
                .and_then(|d| d.get("reasoning_tokens"))
                .and_then(Value::as_i64),
        });

        Ok(Some(UniversalStreamChunk::new(
            payload.get("id").and_then(Value::as_str).map(String::from),
            payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            choices,
            payload.get("created").and_then(Value::as_u64),
            usage,
        )))
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        // Convert back to OpenAI streaming format
        if chunk.is_keep_alive() {
            // Return empty chunk for keep-alive
            return Ok(serde_json::json!({
                "object": "chat.completion.chunk",
                "choices": []
            }));
        }

        let choices: Vec<Value> = chunk
            .choices
            .iter()
            .map(|c| {
                let mut choice = serde_json::json!({
                    "index": c.index,
                    "delta": c.delta.clone().unwrap_or(Value::Object(Map::new()))
                });
                if let Some(ref reason) = c.finish_reason {
                    choice
                        .as_object_mut()
                        .unwrap()
                        .insert("finish_reason".into(), Value::String(reason.clone()));
                } else {
                    choice
                        .as_object_mut()
                        .unwrap()
                        .insert("finish_reason".into(), Value::Null);
                }
                choice
            })
            .collect();

        let mut obj = serde_json::json!({
            "object": "chat.completion.chunk",
            "choices": choices
        });

        let obj_map = obj.as_object_mut().unwrap();
        if let Some(ref id) = chunk.id {
            obj_map.insert("id".into(), Value::String(id.clone()));
        }
        if let Some(ref model) = chunk.model {
            obj_map.insert("model".into(), Value::String(model.clone()));
        }
        if let Some(created) = chunk.created {
            obj_map.insert("created".into(), Value::Number(created.into()));
        }
        if let Some(ref usage) = chunk.usage {
            let prompt = usage.prompt_tokens.unwrap_or(0);
            let completion = usage.completion_tokens.unwrap_or(0);
            obj_map.insert(
                "usage".into(),
                serde_json::json!({
                    "prompt_tokens": prompt,
                    "completion_tokens": completion,
                    "total_tokens": prompt + completion
                }),
            );
        }

        Ok(obj)
    }
}

// =============================================================================
// OpenAI Target-Specific Transformations
// =============================================================================

/// Error type for transformation operations.
#[derive(Debug, thiserror::Error)]
pub enum OpenAITransformError {
    #[error("missing required field: {field}")]
    MissingField { field: &'static str },
    #[error("invalid value: {message}")]
    InvalidValue { message: String },
    #[error("unsupported feature: {feature}")]
    Unsupported { feature: String },
    #[error("serialization failed: {0}")]
    SerializationFailed(String),
}

/// Apply target-specific transformations to an OpenAI-format request payload.
///
/// This function applies transformations needed to make an OpenAI-format request
/// work with different target providers (Azure, Vertex, Mistral, etc.).
///
/// # Arguments
///
/// * `payload` - The OpenAI-format request payload (modified in place)
/// * `target_provider` - The target provider that will receive the request
/// * `provider_metadata` - Optional provider-specific metadata (e.g., api_version for Azure)
///
/// # Returns
///
/// The transformed payload, or an error if transformation fails.
pub fn apply_target_transforms(
    payload: &Value,
    target_provider: TargetProvider,
    provider_metadata: Option<&Map<String, Value>>,
) -> Result<Value, OpenAITransformError> {
    // Parse as OpenAI request
    let mut request: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
        .map_err(|e| OpenAITransformError::SerializationFailed(e.to_string()))?;

    // Detect capabilities based on request and target
    let capabilities = OpenAICapabilities::detect(&request, target_provider);

    // Apply reasoning model transformations
    if capabilities.requires_reasoning_transforms() {
        apply_reasoning_transforms(&mut request, &capabilities);
    }

    // Apply provider-specific field sanitization
    apply_provider_sanitization(
        &mut request,
        &capabilities,
        target_provider,
        provider_metadata,
    );

    // Apply model name normalization if needed
    if capabilities.requires_model_normalization {
        apply_model_normalization(&mut request, target_provider);
    }

    // Normalize user messages (handle non-image base64 content)
    normalize_user_messages(&mut request)?;

    // Apply response format transformations
    apply_response_format(&mut request, &capabilities)?;

    // Serialize back to Value
    serde_json::to_value(&request)
        .map_err(|e| OpenAITransformError::SerializationFailed(e.to_string()))
}

fn apply_reasoning_transforms(
    request: &mut CreateChatCompletionRequestClass,
    capabilities: &OpenAICapabilities,
) {
    // Remove unsupported fields for reasoning models
    request.temperature = None;
    request.parallel_tool_calls = None;

    // For legacy o1 models, convert system messages to user messages
    if capabilities.is_legacy_o1_model {
        for message in &mut request.messages {
            if matches!(message.role, ChatCompletionRequestMessageRole::System) {
                message.role = ChatCompletionRequestMessageRole::User;
            }
        }
    }
}

fn apply_provider_sanitization(
    request: &mut CreateChatCompletionRequestClass,
    capabilities: &OpenAICapabilities,
    target_provider: TargetProvider,
    provider_metadata: Option<&Map<String, Value>>,
) {
    // Remove stream_options for providers that don't support it
    if !capabilities.supports_stream_options {
        request.stream_options = None;
    }

    // Remove parallel_tool_calls for providers that don't support it
    if !capabilities.supports_parallel_tools {
        request.parallel_tool_calls = None;
    }

    // Remove seed field for Azure with API version
    let has_api_version = provider_metadata
        .and_then(|meta| meta.get("api_version"))
        .is_some();

    if capabilities.should_remove_seed_for_azure(target_provider, has_api_version) {
        request.seed = None;
    }
}

fn apply_model_normalization(
    request: &mut CreateChatCompletionRequestClass,
    target_provider: TargetProvider,
) {
    // Normalize Vertex model names
    if target_provider == TargetProvider::Vertex {
        if request.model.starts_with("publishers/meta/models/") {
            // Strip to "meta/..." format
            request.model = request
                .model
                .strip_prefix("publishers/")
                .and_then(|s| s.strip_prefix("meta/models/"))
                .map(|s| format!("meta/{}", s))
                .unwrap_or_else(|| request.model.clone());
        } else if let Some(stripped) = request.model.strip_prefix("publishers/") {
            // Strip "publishers/X/models/Y" to "Y"
            if let Some(model_part) = stripped.split("/models/").nth(1) {
                request.model = model_part.to_string();
            }
        }
    }
}

fn normalize_user_messages(
    request: &mut CreateChatCompletionRequestClass,
) -> Result<(), OpenAITransformError> {
    for message in &mut request.messages {
        if matches!(message.role, ChatCompletionRequestMessageRole::User) {
            if let Some(
                ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(
                    parts,
                ),
            ) = message.content.as_mut()
            {
                for part in parts.iter_mut() {
                    normalize_content_part(part)?;
                }
            }
        }
    }
    Ok(())
}

fn normalize_content_part(
    part: &mut ChatCompletionRequestMessageContentPart,
) -> Result<(), OpenAITransformError> {
    if !matches!(
        part.chat_completion_request_message_content_part_type,
        PurpleType::ImageUrl
    ) {
        return Ok(());
    }

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
            part.chat_completion_request_message_content_part_type = PurpleType::File;
            part.image_url = None;
            part.file = Some(File {
                file_data: Some(image_url_value),
                file_id: None,
                filename: Some(if data_url.media_type == "application/pdf" {
                    "file_from_base64.pdf".to_string()
                } else {
                    "file_from_base64".to_string()
                }),
            });
        }
    }

    Ok(())
}

fn apply_response_format(
    request: &mut CreateChatCompletionRequestClass,
    capabilities: &OpenAICapabilities,
) -> Result<(), OpenAITransformError> {
    let Some(response_format) = request.response_format.take() else {
        return Ok(());
    };

    match response_format.text_type {
        ResponseFormatType::Text => Ok(()),
        ResponseFormatType::JsonSchema => {
            if capabilities.supports_native_structured_output {
                request.response_format = Some(response_format);
                return Ok(());
            }

            // Check if tools are already being used
            if request
                .tools
                .as_ref()
                .is_some_and(|tools| !tools.is_empty())
                || request.function_call.is_some()
                || request.tool_choice.is_some()
            {
                return Err(OpenAITransformError::Unsupported {
                    feature: "tools_with_structured_output".to_string(),
                });
            }

            // Convert json_schema to a tool call
            match response_format.json_schema {
                Some(schema) => {
                    request.tools = Some(vec![ToolElement {
                        function: Some(FunctionObject {
                            description: Some("Output the result in JSON format".to_string()),
                            name: "json".to_string(),
                            parameters: schema.schema.clone(),
                            strict: schema.strict,
                        }),
                        tool_type: ToolType::Function,
                        custom: None,
                    }]);

                    request.tool_choice =
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

                    Ok(())
                }
                None => Err(OpenAITransformError::InvalidValue {
                    message: "json_schema response_format is missing schema".to_string(),
                }),
            }
        }
        ResponseFormatType::JsonObject => {
            request.response_format = Some(response_format);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_openai_detect_request() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_openai_passthrough() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        assert_eq!(universal.model, Some("gpt-4".to_string()));
        assert_eq!(universal.messages.len(), 1);

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("model").unwrap(), "gpt-4");
        assert!(reconstructed.get("messages").is_some());
    }

    #[test]
    fn test_openai_preserves_extras() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "user": "test-user-123",
            "custom_field": "should_be_preserved"
        });

        let universal = adapter.request_to_universal(payload).unwrap();
        let openai_extras = universal
            .provider_extras
            .get(&ProviderFormat::OpenAI)
            .expect("should have OpenAI extras");
        assert!(openai_extras.contains_key("user"));
        assert!(openai_extras.contains_key("custom_field"));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("user").unwrap(), "test-user-123");
        assert_eq!(
            reconstructed.get("custom_field").unwrap(),
            "should_be_preserved"
        );
    }
}
