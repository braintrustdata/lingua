/*!
Unified payload transformation API.

This module provides a single entry point for validating and transforming
payloads between different provider formats. The key principle is:

**If a payload can be deserialized into the target provider struct, use it as-is (pass-through).**
**Otherwise, detect the source format, convert to universal, and transform to target format.**

This replaces heuristic-based detection with struct-based validation.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::defaults::apply_provider_defaults;
use crate::serde_json::{self, Map, Value};
use crate::universal::message::Message;
use serde::Deserialize;
use thiserror::Error;

#[cfg(feature = "anthropic")]
use crate::providers::anthropic::try_parse_anthropic;
#[cfg(feature = "bedrock")]
use crate::providers::bedrock::try_parse_bedrock;
#[cfg(feature = "google")]
use crate::providers::google::try_parse_google;
#[cfg(feature = "mistral")]
use crate::providers::mistral::MistralDetector;
#[cfg(feature = "openai")]
use crate::providers::openai::try_parse_openai;

#[cfg(feature = "mistral")]
use crate::processing::FormatDetector;

// ============================================================================
// Response envelope types for detection
// ============================================================================

/// OpenAI chat completion response envelope (for detection)
#[derive(Debug, Clone, Deserialize)]
struct OpenAIResponseEnvelope {
    choices: Vec<OpenAIResponseChoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIResponseChoice {
    message: Value,
}

/// Google GenerateContent response envelope (for detection)
#[derive(Debug, Clone, Deserialize)]
struct GoogleResponseEnvelope {
    candidates: Vec<GoogleResponseCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleResponseCandidate {
    content: Value,
}

/// Anthropic Message response envelope (for detection)
#[derive(Debug, Clone, Deserialize)]
struct AnthropicResponseEnvelope {
    content: Vec<Value>,
    #[serde(rename = "type")]
    response_type: Option<String>,
}

/// Bedrock Converse response envelope (for detection)
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields are read via serde deserialization
struct BedrockResponseEnvelope {
    output: BedrockResponseOutput,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields are read via serde deserialization
struct BedrockResponseOutput {
    message: Value,
}

/// Error type for transformation operations
#[derive(Debug, Error)]
pub enum TransformError {
    #[error("Unable to detect source format")]
    UnableToDetectFormat,

    #[error("Validation failed for target format {target:?}: {reason}")]
    ValidationFailed {
        target: ProviderFormat,
        reason: String,
    },

    #[error("Conversion to universal format failed: {0}")]
    ToUniversalFailed(String),

    #[error("Conversion from universal format failed: {0}")]
    FromUniversalFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Unsupported target format: {0:?}")]
    UnsupportedTargetFormat(ProviderFormat),

    #[error("Unsupported source format: {0:?}")]
    UnsupportedSourceFormat(ProviderFormat),
}

/// Result of a transformation operation
#[derive(Debug, Clone)]
pub enum TransformResult {
    /// Payload was already valid for target format - use original
    PassThrough,

    /// Payload was transformed to target format
    Transformed {
        /// The transformed payload
        payload: Value,
        /// The detected source format
        source_format: ProviderFormat,
    },
}

impl TransformResult {
    /// Check if this is a pass-through result
    pub fn is_pass_through(&self) -> bool {
        matches!(self, TransformResult::PassThrough)
    }

    /// Get the transformed payload, or return the original if pass-through
    pub fn payload_or_original(self, original: Value) -> Value {
        match self {
            TransformResult::PassThrough => original,
            TransformResult::Transformed { payload, .. } => payload,
        }
    }
}

/// Try to validate payload as target format, or transform it.
///
/// This is the main entry point for payload transformation. It:
/// 1. Tries to parse the payload as the target format (if it succeeds, return PassThrough)
/// 2. If parsing fails, detects the source format by trying each format in priority order
/// 3. Converts from source format to universal format
/// 4. Converts from universal format to target format
///
/// # Arguments
///
/// * `payload` - The incoming JSON payload
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough)` - Payload is already valid for target format
/// * `Ok(TransformResult::Transformed { payload, source_format })` - Payload was transformed
/// * `Err(TransformError)` - Transformation failed
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::validate_or_transform;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// let openai_payload = json!({
///     "model": "gpt-4",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// // If target is OpenAI and payload is OpenAI format, returns PassThrough
/// let result = validate_or_transform(&openai_payload, ProviderFormat::OpenAI);
/// ```
pub fn validate_or_transform(
    payload: &Value,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    // Step 1: Try to parse as target format
    if is_valid_for_format(payload, target_format) {
        return Ok(TransformResult::PassThrough);
    }

    // Step 2: Detect source format by trying each in priority order
    let source_format = detect_source_format(payload)?;

    // Step 3: Convert to universal format
    let universal = to_universal(payload, source_format)?;

    // Step 4: Convert from universal to target format
    let transformed = from_universal(&universal, target_format)?;

    Ok(TransformResult::Transformed {
        payload: transformed,
        source_format,
    })
}

/// Transform a full request payload to the target format with provider defaults applied.
///
/// Unlike `validate_or_transform` which only transforms messages, this function
/// handles the complete request including model parameters. It applies provider-specific
/// defaults (e.g., `max_tokens` for Anthropic) only when transformation is needed.
///
/// Returns `TransformResult::PassThrough` when the payload is already valid for the
/// target format - in this case, use the original payload as-is with zero overhead.
///
/// # Arguments
///
/// * `payload` - The incoming JSON payload (full request)
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough)` - Payload is already valid, use original as-is
/// * `Ok(TransformResult::Transformed { payload, source_format })` - Transformed with defaults
/// * `Err(TransformError)` - If transformation fails
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::{transform_request, TransformResult};
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// // OpenAI request without max_tokens
/// let openai_payload = json!({
///     "model": "gpt-4",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// // Transform to Anthropic - max_tokens will be added with default value
/// let result = transform_request(&openai_payload, ProviderFormat::Anthropic).unwrap();
/// let final_payload = result.payload_or_original(openai_payload);
/// ```
pub fn transform_request(
    payload: &Value,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    // Step 1: Validate or transform the messages
    let transform_result = validate_or_transform(payload, target_format)?;

    match transform_result {
        TransformResult::PassThrough => {
            // Payload is already valid for target format - zero overhead, use original as-is
            Ok(TransformResult::PassThrough)
        }
        TransformResult::Transformed {
            payload: transformed_messages,
            source_format,
        } => {
            // Build a new request with transformed messages and copied parameters
            let mut full_payload =
                build_request_payload(payload, transformed_messages, target_format)?;

            // Apply provider-specific defaults (only for transformed payloads)
            apply_provider_defaults(&mut full_payload, target_format);

            Ok(TransformResult::Transformed {
                payload: full_payload,
                source_format,
            })
        }
    }
}

/// Build a full request payload from transformed messages and source parameters.
///
/// This copies over common parameters from the source payload and inserts
/// the transformed messages.
fn build_request_payload(
    source: &Value,
    transformed_messages: Value,
    target_format: ProviderFormat,
) -> Result<Value, TransformError> {
    let mut result = Map::new();

    // Copy model if present
    if let Some(model) = source.get("model") {
        result.insert("model".into(), model.clone());
    }

    // Handle Anthropic's special format: from_universal returns {messages, system}
    if target_format == ProviderFormat::Anthropic {
        if let Some(obj) = transformed_messages.as_object() {
            if let Some(messages) = obj.get("messages") {
                result.insert("messages".into(), messages.clone());
            }
            if let Some(system) = obj.get("system") {
                result.insert("system".into(), system.clone());
            }
        }
    } else {
        // Standard format: transformed_messages is just the messages array
        let messages_key = match target_format {
            ProviderFormat::Google => "contents",
            ProviderFormat::Converse => "messages",
            _ => "messages",
        };
        result.insert(messages_key.into(), transformed_messages);
    }

    // Copy common parameters that are format-agnostic
    let common_params = [
        "temperature",
        "top_p",
        "top_k",
        "max_tokens",
        "max_completion_tokens",
        "stop",
        "stream",
        "seed",
        "presence_penalty",
        "frequency_penalty",
        "tools",
        "tool_choice",
        "response_format",
        "system",
    ];

    for param in common_params {
        if let Some(value) = source.get(param) {
            // Skip stream parameters for Google (uses endpoint-based streaming)
            if target_format == ProviderFormat::Google
                && (param == "stream" || param == "stream_options")
            {
                continue;
            }
            // Handle parameter name mapping for different formats
            let target_key = map_param_name(param, target_format);
            result.insert(target_key.into(), value.clone());
        }
    }

    Ok(Value::Object(result))
}

/// Map parameter names between formats.
///
/// Some parameters have different names in different provider APIs.
fn map_param_name(param: &str, target_format: ProviderFormat) -> &str {
    match (param, target_format) {
        // Google uses maxOutputTokens instead of max_tokens
        ("max_tokens", ProviderFormat::Google) => "maxOutputTokens",
        // Default: keep the same name
        _ => param,
    }
}

/// Check if a payload is valid for a specific format by attempting deserialization.
pub fn is_valid_for_format(payload: &Value, format: ProviderFormat) -> bool {
    match format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI => try_parse_openai(payload).is_ok(),

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => try_parse_anthropic(payload).is_ok(),

        #[cfg(feature = "google")]
        ProviderFormat::Google => try_parse_google(payload).is_ok(),

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => try_parse_bedrock(payload).is_ok(),

        #[cfg(feature = "mistral")]
        ProviderFormat::Mistral => {
            // Mistral needs both valid structure AND Mistral indicators
            MistralDetector.detect(payload)
        }

        ProviderFormat::Unknown => false,

        // When features are disabled, return false
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Detect the source format by trying to parse as each format in priority order.
///
/// Priority order (most specific first):
/// 1. Bedrock Converse (priority 95) - `modelId` field is unique
/// 2. Google (priority 90) - `contents[].parts[]` structure
/// 3. Anthropic (priority 80) - `max_tokens` required, specific roles
/// 4. Mistral (priority 70) - OpenAI-compatible with extras
/// 5. OpenAI (priority 50) - Most permissive, fallback
fn detect_source_format(payload: &Value) -> Result<ProviderFormat, TransformError> {
    // Try most specific formats first

    #[cfg(feature = "bedrock")]
    if try_parse_bedrock(payload).is_ok() {
        return Ok(ProviderFormat::Converse);
    }

    #[cfg(feature = "google")]
    if try_parse_google(payload).is_ok() {
        return Ok(ProviderFormat::Google);
    }

    #[cfg(feature = "anthropic")]
    if try_parse_anthropic(payload).is_ok() {
        return Ok(ProviderFormat::Anthropic);
    }

    #[cfg(feature = "mistral")]
    if MistralDetector.detect(payload) {
        return Ok(ProviderFormat::Mistral);
    }

    #[cfg(feature = "openai")]
    if try_parse_openai(payload).is_ok() {
        return Ok(ProviderFormat::OpenAI);
    }

    Err(TransformError::UnableToDetectFormat)
}

/// Convert a payload from its source format to universal message format.
///
/// This function detects the provider format of the input payload and converts
/// its messages to lingua's universal Message format.
///
/// # Arguments
///
/// * `payload` - The JSON payload in the source format
/// * `source_format` - The provider format of the payload
///
/// # Returns
///
/// * `Ok(Vec<Message>)` - Universal messages extracted from the payload
/// * `Err(TransformError)` - If parsing or conversion fails
pub fn to_universal(
    payload: &Value,
    source_format: ProviderFormat,
) -> Result<Vec<Message>, TransformError> {
    match source_format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI | ProviderFormat::Mistral => {
            // Parse as OpenAI and convert messages
            use crate::providers::openai::generated::CreateChatCompletionRequestClass;
            use crate::universal::convert::TryFromLLM;

            let request: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => {
            use crate::providers::anthropic::generated::CreateMessageParams;
            use crate::universal::convert::TryFromLLM;

            let request: CreateMessageParams = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        #[cfg(feature = "google")]
        ProviderFormat::Google => {
            use crate::providers::google::detect::{GoogleContent, GoogleGenerateContentRequest};
            use crate::universal::convert::TryFromLLM;

            let request: GoogleGenerateContentRequest = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(request.contents)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => {
            use crate::providers::bedrock::request::{BedrockMessage, ConverseRequest};
            use crate::universal::convert::TryFromLLM;

            let request: ConverseRequest = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<BedrockMessage>>>::try_from(request.messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        _ => Err(TransformError::UnsupportedSourceFormat(source_format)),
    }
}

/// Convert universal messages to a specific target format.
///
/// This is the main entry point for converting lingua's universal Message format
/// to any supported provider format. The function dispatches to the appropriate
/// TryFromLLM implementation based on the target format.
///
/// # Arguments
///
/// * `messages` - Slice of universal Message objects to convert
/// * `target_format` - The target provider format (OpenAI, Anthropic, Google, etc.)
///
/// # Returns
///
/// * `Ok(Value)` - JSON value containing the converted messages in target format
/// * `Err(TransformError)` - If conversion or serialization fails
pub fn from_universal(
    messages: &[Message],
    target_format: ProviderFormat,
) -> Result<Value, TransformError> {
    match target_format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI | ProviderFormat::Mistral => {
            use crate::providers::openai::generated::ChatCompletionRequestMessage;
            use crate::universal::convert::TryFromLLM;

            let openai_messages: Vec<ChatCompletionRequestMessage> =
                <Vec<ChatCompletionRequestMessage> as TryFromLLM<Vec<Message>>>::try_from(
                    messages.to_vec(),
                )
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            serde_json::to_value(openai_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => {
            use crate::providers::anthropic::generated::InputMessage;
            use crate::universal::convert::TryFromLLM;
            use crate::universal::message::UserContent;
            use crate::universal::transform::extract_system_messages;

            // Clone and extract system messages (Anthropic uses separate `system` param)
            let mut msgs = messages.to_vec();
            let system_contents = extract_system_messages(&mut msgs);

            // Convert remaining messages (may be empty if only system messages were present)
            let anthropic_messages: Vec<InputMessage> =
                <Vec<InputMessage> as TryFromLLM<Vec<Message>>>::try_from(msgs)
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            // Build result with both messages and system
            let mut result = Map::new();
            result.insert(
                "messages".into(),
                serde_json::to_value(anthropic_messages)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            );

            if !system_contents.is_empty() {
                // Convert system contents to Anthropic format (concatenate text)
                let system_text: String = system_contents
                    .iter()
                    .filter_map(|c| match c {
                        UserContent::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                result.insert("system".into(), Value::String(system_text));
            }

            Ok(Value::Object(result))
        }

        #[cfg(feature = "google")]
        ProviderFormat::Google => {
            use crate::providers::google::detect::GoogleContent;
            use crate::universal::convert::TryFromLLM;

            let google_contents: Vec<GoogleContent> =
                <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            serde_json::to_value(google_contents)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => {
            use crate::providers::bedrock::request::BedrockMessage;
            use crate::universal::convert::TryFromLLM;

            let bedrock_messages: Vec<BedrockMessage> =
                <Vec<BedrockMessage> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            serde_json::to_value(bedrock_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }

        _ => Err(TransformError::UnsupportedTargetFormat(target_format)),
    }
}

// ============================================================================
// Response transformation
// ============================================================================

/// Transform a response payload from one format to another.
///
/// This extracts the message(s) from the source response envelope, converts
/// them via the universal Message format, and builds a new response envelope
/// in the target format.
///
/// # Arguments
///
/// * `response` - The source response JSON payload
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough)` - Response is already valid for target format
/// * `Ok(TransformResult::Transformed { payload, source_format })` - Transformed response
/// * `Err(TransformError)` - If transformation fails
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::transform_response;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// // Google response
/// let google_response = json!({
///     "candidates": [{
///         "content": {
///             "role": "model",
///             "parts": [{"text": "Hello!"}]
///         }
///     }]
/// });
///
/// // Transform to OpenAI format
/// let result = transform_response(&google_response, ProviderFormat::OpenAI).unwrap();
/// ```
pub fn transform_response(
    response: &Value,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    // Step 1: Detect source format
    let source_format = detect_response_format(response)?;

    // Step 2: If source matches target, pass through
    if source_format == target_format {
        return Ok(TransformResult::PassThrough);
    }

    // Step 3: Extract message(s) from source response envelope
    let universal_messages = response_to_universal(response, source_format)?;

    // Step 4: Build target response envelope
    let transformed = build_response_envelope(&universal_messages, target_format)?;

    Ok(TransformResult::Transformed {
        payload: transformed,
        source_format,
    })
}

/// Detect the format of a response payload.
fn detect_response_format(response: &Value) -> Result<ProviderFormat, TransformError> {
    // Try each format in priority order (most specific first)

    // Bedrock: has output.message structure
    if serde_json::from_value::<BedrockResponseEnvelope>(response.clone()).is_ok() {
        return Ok(ProviderFormat::Converse);
    }

    // Google: has candidates[].content structure
    if serde_json::from_value::<GoogleResponseEnvelope>(response.clone()).is_ok() {
        return Ok(ProviderFormat::Google);
    }

    // Anthropic: has content[] array and type="message"
    if let Ok(envelope) = serde_json::from_value::<AnthropicResponseEnvelope>(response.clone()) {
        if envelope.response_type.as_deref() == Some("message") {
            return Ok(ProviderFormat::Anthropic);
        }
    }

    // OpenAI: has choices[].message structure
    if serde_json::from_value::<OpenAIResponseEnvelope>(response.clone()).is_ok() {
        return Ok(ProviderFormat::OpenAI);
    }

    Err(TransformError::UnableToDetectFormat)
}

/// Extract universal messages from a response envelope.
fn response_to_universal(
    response: &Value,
    source_format: ProviderFormat,
) -> Result<Vec<Message>, TransformError> {
    match source_format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI => {
            use crate::providers::openai::generated::ChatCompletionResponseMessage;
            use crate::universal::convert::TryFromLLM;

            let envelope: OpenAIResponseEnvelope = serde_json::from_value(response.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            let mut messages = Vec::new();
            for choice in envelope.choices {
                let response_msg: ChatCompletionResponseMessage =
                    serde_json::from_value(choice.message)
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal = <Message as TryFromLLM<&ChatCompletionResponseMessage>>::try_from(
                    &response_msg,
                )
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }
            Ok(messages)
        }

        #[cfg(feature = "google")]
        ProviderFormat::Google => {
            use crate::providers::google::detect::GoogleContent;
            use crate::universal::convert::TryFromLLM;

            let envelope: GoogleResponseEnvelope = serde_json::from_value(response.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            let mut messages = Vec::new();
            for candidate in envelope.candidates {
                let content: GoogleContent = serde_json::from_value(candidate.content)
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal = <Message as TryFromLLM<GoogleContent>>::try_from(content)
                    .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }
            Ok(messages)
        }

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => {
            use crate::providers::anthropic::generated::ContentBlock;
            use crate::universal::convert::TryFromLLM;

            let envelope: AnthropicResponseEnvelope = serde_json::from_value(response.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            let content_blocks: Vec<ContentBlock> = envelope
                .content
                .into_iter()
                .map(|v| {
                    serde_json::from_value(v)
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;

            <Vec<Message> as TryFromLLM<Vec<ContentBlock>>>::try_from(content_blocks)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => {
            use crate::providers::bedrock::response::BedrockOutputMessage;
            use crate::universal::convert::TryFromLLM;

            let envelope: BedrockResponseEnvelope = serde_json::from_value(response.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            let output_msg: BedrockOutputMessage = serde_json::from_value(envelope.output.message)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            let universal = <Message as TryFromLLM<BedrockOutputMessage>>::try_from(output_msg)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            Ok(vec![universal])
        }

        _ => Err(TransformError::UnsupportedSourceFormat(source_format)),
    }
}

/// Build a response envelope in the target format from universal messages.
fn build_response_envelope(
    messages: &[Message],
    target_format: ProviderFormat,
) -> Result<Value, TransformError> {
    match target_format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI => {
            use crate::providers::openai::generated::ChatCompletionResponseMessage;
            use crate::universal::convert::TryFromLLM;

            let choices: Vec<Value> = messages
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
                        "finish_reason": "stop"
                    }))
                })
                .collect::<Result<Vec<_>, TransformError>>()?;

            Ok(serde_json::json!({
                "id": "transformed",
                "object": "chat.completion",
                "created": 0,
                "model": "transformed",
                "choices": choices
            }))
        }

        #[cfg(feature = "google")]
        ProviderFormat::Google => {
            use crate::providers::google::detect::GoogleContent;
            use crate::universal::convert::TryFromLLM;

            let candidates: Vec<Value> = messages
                .iter()
                .enumerate()
                .map(|(i, msg)| {
                    let content = <GoogleContent as TryFromLLM<Message>>::try_from(msg.clone())
                        .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

                    let content_value = serde_json::to_value(&content)
                        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

                    Ok(serde_json::json!({
                        "index": i,
                        "content": content_value,
                        "finishReason": "STOP"
                    }))
                })
                .collect::<Result<Vec<_>, TransformError>>()?;

            Ok(serde_json::json!({
                "candidates": candidates
            }))
        }

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => {
            use crate::providers::anthropic::generated::ContentBlock;
            use crate::universal::convert::TryFromLLM;

            let content_blocks =
                <Vec<ContentBlock> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            let content_value = serde_json::to_value(&content_blocks)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

            Ok(serde_json::json!({
                "id": "transformed",
                "type": "message",
                "role": "assistant",
                "content": content_value,
                "model": "transformed",
                "stop_reason": "end_turn"
            }))
        }

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => {
            use crate::providers::bedrock::response::BedrockOutputMessage;
            use crate::universal::convert::TryFromLLM;

            // Take the first message for the response
            let msg = messages.first().ok_or_else(|| {
                TransformError::FromUniversalFailed("No messages to transform".to_string())
            })?;

            let output_msg = <BedrockOutputMessage as TryFromLLM<Message>>::try_from(msg.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            let message_value = serde_json::to_value(&output_msg)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

            Ok(serde_json::json!({
                "output": {
                    "message": message_value
                },
                "stopReason": "end_turn",
                "usage": {
                    "inputTokens": 0,
                    "outputTokens": 0,
                    "totalTokens": 0
                },
                "metrics": {
                    "latencyMs": 0
                }
            }))
        }

        _ => Err(TransformError::UnsupportedTargetFormat(target_format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    #[cfg(feature = "openai")]
    fn test_validate_openai_passthrough() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = validate_or_transform(&payload, ProviderFormat::OpenAI).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_openai_to_anthropic() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Anthropic).unwrap();
        match result {
            TransformResult::Transformed {
                payload: _,
                source_format,
            } => {
                assert_eq!(source_format, ProviderFormat::OpenAI);
            }
            TransformResult::PassThrough => {
                panic!("Expected transformation, got pass-through");
            }
        }
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_validate_anthropic_passthrough() {
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Anthropic).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(feature = "google")]
    fn test_validate_google_passthrough() {
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Google).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(feature = "bedrock")]
    fn test_validate_bedrock_passthrough() {
        let payload = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Converse).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_detect_source_format_openai() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let format = detect_source_format(&payload).unwrap();
        assert_eq!(format, ProviderFormat::OpenAI);
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_detect_source_format_anthropic() {
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let format = detect_source_format(&payload).unwrap();
        assert_eq!(format, ProviderFormat::Anthropic);
    }

    #[test]
    fn test_detect_source_format_fails_for_invalid() {
        let payload = json!({
            "invalid": "payload"
        });

        let result = detect_source_format(&payload);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_adds_anthropic_max_tokens() {
        // OpenAI request without max_tokens
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic).unwrap();

        // Should be transformed (not pass-through) since source is OpenAI
        assert!(!result.is_pass_through());

        let final_payload = result.payload_or_original(payload);

        // Should have max_tokens added with default value (4096)
        assert_eq!(
            final_payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(4096)
        );
        // Should have messages transformed
        assert!(final_payload.get("messages").is_some());
        // Should have model preserved
        assert_eq!(
            final_payload.get("model").and_then(|v| v.as_str()),
            Some("gpt-4")
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_preserves_existing_max_tokens() {
        // OpenAI request with max_tokens
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 8192
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic).unwrap();
        let final_payload = result.payload_or_original(payload);

        // Should preserve the existing max_tokens value
        assert_eq!(
            final_payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(8192)
        );
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_transform_request_passthrough_returns_passthrough() {
        // Valid Anthropic request - should pass through with zero overhead
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic).unwrap();

        // Should be pass-through since payload is already valid Anthropic format
        assert!(result.is_pass_through());

        // Using payload_or_original returns the original payload as-is
        let final_payload = result.payload_or_original(payload.clone());
        assert_eq!(final_payload, payload);
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_copies_common_params() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "temperature": 0.7,
            "top_p": 0.9,
            "stream": true
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic).unwrap();
        let final_payload = result.payload_or_original(payload);

        assert_eq!(
            final_payload.get("temperature").and_then(|v| v.as_f64()),
            Some(0.7)
        );
        assert_eq!(
            final_payload.get("top_p").and_then(|v| v.as_f64()),
            Some(0.9)
        );
        assert_eq!(
            final_payload.get("stream").and_then(|v| v.as_bool()),
            Some(true)
        );
    }
}
