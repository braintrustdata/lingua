/*!
OpenAI Chat Completions API adapter.

This module provides the `OpenAIAdapter` for the standard Chat Completions API,
along with target-specific transformation utilities for providers like Azure,
Vertex, and Mistral.
*/

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::reject_params;
use std::collections::HashMap;

use crate::processing::adapters::{
    insert_opt_bool, insert_opt_f64, insert_opt_i64, insert_opt_value, ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::openai::capabilities::{OpenAICapabilities, TargetProvider};
use crate::providers::openai::convert::{
    ChatCompletionRequestMessageExt, ChatCompletionResponseMessageExt,
};
use crate::providers::openai::generated::{
    AllowedToolsFunction, ChatCompletionRequestMessageContent,
    ChatCompletionRequestMessageContentPart, ChatCompletionRequestMessageRole,
    ChatCompletionToolChoiceOption, CreateChatCompletionRequestClass, File, FunctionObject,
    FunctionToolChoiceClass, FunctionToolChoiceType, PurpleType, ResponseFormatType, ToolElement,
    ToolType,
};
use crate::providers::openai::params::OpenAIChatParams;
use crate::providers::openai::try_parse_openai;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::tools::{tools_to_openai_chat_value, UniversalTool};
use crate::universal::{
    parse_stop_sequences, UniversalParams, UniversalRequest, UniversalResponse,
    UniversalStreamChoice, UniversalStreamChunk, UniversalUsage, PLACEHOLDER_ID, PLACEHOLDER_MODEL,
};
use crate::util::media::parse_base64_data_url;
use std::convert::TryInto;

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
        // Parse params (messages will be parsed separately to preserve reasoning field)
        let typed_params: OpenAIChatParams = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract and parse messages as extended type to capture reasoning field
        let messages_val = payload
            .get("messages")
            .ok_or_else(|| {
                TransformError::ToUniversalFailed("OpenAI: missing 'messages' field".to_string())
            })?
            .as_array()
            .ok_or_else(|| {
                TransformError::ToUniversalFailed("OpenAI: 'messages' must be an array".to_string())
            })?;

        let provider_messages: Vec<ChatCompletionRequestMessageExt> = messages_val
            .iter()
            .map(|msg_val| {
                serde_json::from_value(msg_val.clone())
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(provider_messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract max_tokens first - needed for reasoning budget computation
        let max_tokens = typed_params
            .max_tokens
            .or(typed_params.max_completion_tokens);

        // Convert reasoning effort to ReasoningConfig, computing budget_tokens with max_tokens context
        let reasoning = typed_params
            .reasoning_effort
            .map(|effort| (effort, max_tokens).into());

        // Build canonical params from typed fields
        let mut params = UniversalParams {
            temperature: typed_params.temperature,
            top_p: typed_params.top_p,
            top_k: None, // OpenAI doesn't support top_k
            max_tokens,
            stop: typed_params.stop.as_ref().and_then(parse_stop_sequences),
            tools: typed_params
                .tools
                .as_ref()
                .map(UniversalTool::from_value_array),
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
            reasoning,
            metadata: typed_params.metadata,
            store: typed_params.store,
            service_tier: typed_params.service_tier,
            logprobs: typed_params.logprobs,
            top_logprobs: typed_params.top_logprobs,
        };

        // Sync parallel_tool_calls with tool_choice.disable_parallel for roundtrip fidelity
        // OpenAI uses parallel_tool_calls at params level, Anthropic uses tool_choice.disable_parallel
        if params.parallel_tool_calls == Some(false) {
            if let Some(ref mut tc) = params.tool_choice {
                if tc.disable_parallel.is_none() {
                    tc.disable_parallel = Some(true);
                }
            }
        }

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

        // Validate unsupported parameters
        reject_params!(req, ProviderFormat::OpenAI, top_k);

        let openai_messages: Vec<ChatCompletionRequestMessageExt> =
            <Vec<ChatCompletionRequestMessageExt> as TryFromLLM<Vec<Message>>>::try_from(
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
        // Output stop sequences as array (OpenAI accepts both string and array)
        if let Some(ref stop) = req.params.stop {
            if !stop.is_empty() {
                obj.insert(
                    "stop".into(),
                    Value::Array(stop.iter().map(|s| Value::String(s.clone())).collect()),
                );
            }
        }
        // Convert tools to OpenAI Chat format
        if let Some(tools) = &req.params.tools {
            if let Some(tools_value) = tools_to_openai_chat_value(tools)? {
                obj.insert("tools".into(), tools_value);
            }
        }
        // Use helper methods to reduce boilerplate
        insert_opt_value(
            &mut obj,
            "tool_choice",
            req.params.tool_choice_for(ProviderFormat::OpenAI),
        );
        insert_opt_value(
            &mut obj,
            "response_format",
            req.params.response_format_for(ProviderFormat::OpenAI),
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

        // Add reasoning_effort from canonical params
        if let Some(effort_value) = req.params.reasoning_for(ProviderFormat::OpenAI) {
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
                // Deserialize to extended type to capture reasoning field
                let response_msg: ChatCompletionResponseMessageExt =
                    serde_json::from_value(msg_val.clone())
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal =
                    <Message as TryFromLLM<ChatCompletionResponseMessageExt>>::try_from(
                        response_msg,
                    )
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finish_reason from first choice
            if finish_reason.is_none() {
                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                    finish_reason =
                        Some(reason.parse().map_err(|_| ConvertError::InvalidEnumValue {
                            type_name: "FinishReason",
                            value: reason.to_string(),
                        })?);
                }
            }
        }

        let usage = UniversalUsage::extract_from_response(&payload, self.format());

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
        let finish_reason = resp
            .finish_reason
            .as_ref()
            .map(|r| r.to_provider_string(self.format()).to_string())
            .unwrap_or_else(|| "stop".to_string());

        let choices: Vec<Value> = resp
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                // Use extended type to include reasoning field in output
                let response_msg =
                    <ChatCompletionResponseMessageExt as TryFromLLM<&Message>>::try_from(msg)
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

        let usage = resp
            .usage
            .as_ref()
            .map(|u| u.to_provider_value(self.format()));

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
        let usage = UniversalUsage::extract_from_response(&payload, self.format());

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
            obj_map.insert(
                "usage".into(),
                usage.to_provider_value(ProviderFormat::OpenAI),
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

    #[test]
    fn test_openai_reasoning_roundtrip() {
        use crate::universal::message::{AssistantContent, AssistantContentPart, TextContentPart};

        let adapter = OpenAIAdapter;

        // Create universal request with reasoning content
        let universal = UniversalRequest {
            model: Some("gpt-4".to_string()),
            messages: vec![
                Message::User {
                    content: crate::universal::message::UserContent::String("Hello".to_string()),
                },
                Message::Assistant {
                    content: AssistantContent::Array(vec![
                        AssistantContentPart::Reasoning {
                            text: "Let me think about this...".to_string(),
                            encrypted_content: None,
                        },
                        AssistantContentPart::Text(TextContentPart {
                            text: "OK".to_string(),
                            provider_options: None,
                        }),
                    ]),
                    id: None,
                },
                Message::User {
                    content: crate::universal::message::UserContent::String("Thanks".to_string()),
                },
            ],
            params: Default::default(),
            provider_extras: Default::default(),
        };

        // Convert universal to ChatCompletions format
        let openai_json = adapter.request_from_universal(&universal).unwrap();

        // Verify reasoning field is in the JSON output
        let messages = openai_json.get("messages").unwrap().as_array().unwrap();
        let assistant_msg = &messages[1];
        eprintln!(
            "Assistant message JSON: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        assert!(
            assistant_msg.get("reasoning").is_some(),
            "Assistant message should have reasoning field. Got: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );
        assert_eq!(
            assistant_msg.get("reasoning").unwrap().as_str().unwrap(),
            "Let me think about this..."
        );

        // Now convert back to universal and verify reasoning is preserved
        let universal2 = adapter.request_to_universal(openai_json.clone()).unwrap();

        // Check that reasoning is preserved in universal format
        let msg = &universal2.messages[1];
        match msg {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    let reasoning_part = parts
                        .iter()
                        .find(|p| matches!(p, AssistantContentPart::Reasoning { .. }));
                    assert!(
                        reasoning_part.is_some(),
                        "Should have reasoning part after roundtrip. Got: {:?}",
                        parts
                    );
                }
                _ => panic!("Expected Array content, got {:?}", content),
            },
            _ => panic!("Expected Assistant message, got {:?}", msg),
        }
    }

    #[test]
    fn test_openai_reasoning_only_roundtrip() {
        // Test case like Responses API where assistant message only has reasoning, no text
        use crate::universal::message::{AssistantContent, AssistantContentPart};

        let adapter = OpenAIAdapter;

        // Create universal request with reasoning-only content (like from Responses API)
        let universal = UniversalRequest {
            model: Some("gpt-4".to_string()),
            messages: vec![
                Message::User {
                    content: crate::universal::message::UserContent::String("Hello".to_string()),
                },
                Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                        text: "Let me think...".to_string(),
                        encrypted_content: None,
                    }]),
                    id: None,
                },
            ],
            params: Default::default(),
            provider_extras: Default::default(),
        };

        // Convert universal to ChatCompletions format
        let openai_json = adapter.request_from_universal(&universal).unwrap();
        eprintln!(
            "Full OpenAI JSON: {}",
            serde_json::to_string_pretty(&openai_json).unwrap()
        );

        // Verify reasoning field is in the JSON output
        let messages = openai_json.get("messages").unwrap().as_array().unwrap();
        let assistant_msg = &messages[1];
        eprintln!(
            "Assistant message JSON: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        assert!(
            assistant_msg.get("reasoning").is_some(),
            "Assistant message should have reasoning field. Got: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        // Now convert back to universal and verify reasoning is preserved
        let universal2 = adapter.request_to_universal(openai_json.clone()).unwrap();
        eprintln!("Universal2 messages: {:?}", universal2.messages);

        // Check that reasoning is preserved in universal format
        let msg = &universal2.messages[1];
        match msg {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    eprintln!("Parts: {:?}", parts);
                    let reasoning_part = parts
                        .iter()
                        .find(|p| matches!(p, AssistantContentPart::Reasoning { .. }));
                    assert!(
                        reasoning_part.is_some(),
                        "Should have reasoning part after roundtrip. Got: {:?}",
                        parts
                    );
                }
                AssistantContent::String(s) => {
                    panic!("Expected Array content, got String: {:?}", s)
                }
            },
            _ => panic!("Expected Assistant message, got {:?}", msg),
        }
    }

    #[test]
    fn test_openai_empty_reasoning_roundtrip() {
        // Test case like Responses API where assistant message has empty reasoning summary
        use crate::universal::message::{AssistantContent, AssistantContentPart};

        let adapter = OpenAIAdapter;

        // Create universal request with empty reasoning content (like from Responses API with empty summary)
        let universal = UniversalRequest {
            model: Some("gpt-4".to_string()),
            messages: vec![
                Message::User {
                    content: crate::universal::message::UserContent::String("Hello".to_string()),
                },
                Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                        text: "".to_string(), // Empty reasoning
                        encrypted_content: None,
                    }]),
                    id: None,
                },
            ],
            params: Default::default(),
            provider_extras: Default::default(),
        };

        // Convert universal to ChatCompletions format
        let openai_json = adapter.request_from_universal(&universal).unwrap();

        // Verify reasoning field is in the JSON output (even if empty)
        let messages = openai_json.get("messages").unwrap().as_array().unwrap();
        let assistant_msg = &messages[1];

        assert!(
            assistant_msg.get("reasoning").is_some(),
            "Assistant message should have reasoning field (even if empty). Got: {}",
            serde_json::to_string_pretty(assistant_msg).unwrap()
        );

        // Now convert back to universal and verify reasoning is preserved
        let universal2 = adapter.request_to_universal(openai_json.clone()).unwrap();

        // Check that empty reasoning is preserved in universal format
        let msg = &universal2.messages[1];
        match msg {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    let reasoning_part = parts
                        .iter()
                        .find(|p| matches!(p, AssistantContentPart::Reasoning { .. }));
                    assert!(
                        reasoning_part.is_some(),
                        "Should have reasoning part after roundtrip (even if empty). Got: {:?}",
                        parts
                    );
                }
                AssistantContent::String(s) => {
                    panic!("Expected Array content, got String: {:?}", s)
                }
            },
            _ => panic!("Expected Assistant message, got {:?}", msg),
        }
    }
}
