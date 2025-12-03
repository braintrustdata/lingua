/*!
Unified payload detection API.

This module provides format detection for incoming request payloads by:
1. First checking the model catalog (if available via MODEL_CATALOG_PATH env var)
2. Falling back to content-based heuristics using registered detectors

Detectors are checked in priority order (highest first):
- **Converse (Bedrock)** - priority 95: `modelId` field, camelCase content types
- **Google** - priority 90: `contents[].parts[]` structure, role `"model"`
- **Anthropic** - priority 80: `max_tokens` required, roles only `user`/`assistant`
- **Mistral** - priority 70: Similar to OpenAI but with Mistral-specific fields
- **OpenAI** - priority 50: Default fallback

To add a new provider, implement `FormatDetector` in your provider module
and register it in the `detectors()` function below.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::catalog::catalog_lookup;
use crate::processing::FormatDetector;
use crate::serde_json::{self, Value};
use std::sync::OnceLock;
use thiserror::Error;

// Provider type imports for TypedPayload
#[cfg(feature = "anthropic")]
use crate::providers::anthropic::generated::CreateMessageParams;
#[cfg(feature = "openai")]
use crate::providers::openai::generated::CreateChatCompletionRequestClass;

// Import detectors from provider modules
#[cfg(feature = "anthropic")]
use crate::providers::anthropic::AnthropicDetector;
#[cfg(feature = "bedrock")]
use crate::providers::bedrock::ConverseDetector;
#[cfg(feature = "google")]
use crate::providers::google::GoogleDetector;
#[cfg(feature = "mistral")]
use crate::providers::mistral::MistralDetector;
#[cfg(feature = "openai")]
use crate::providers::openai::OpenAIDetector;

// Import payload wrappers from provider modules
#[cfg(feature = "bedrock")]
pub use crate::providers::bedrock::BedrockPayload;
#[cfg(feature = "google")]
pub use crate::providers::google::GooglePayload;

/// Returns all registered detectors, sorted by priority (highest first).
///
/// This is the central registry for format detectors. When adding a new provider:
/// 1. Implement `FormatDetector` in your provider module
/// 2. Add it to this list with appropriate feature gates
fn detectors() -> &'static [&'static dyn FormatDetector] {
    static DETECTORS: OnceLock<Vec<&'static dyn FormatDetector>> = OnceLock::new();
    // Allow vec_init_then_push: conditional compilation makes vec![] macro unusable here
    #[allow(clippy::vec_init_then_push)]
    DETECTORS.get_or_init(|| {
        let mut v: Vec<&dyn FormatDetector> = vec![];

        // Add detectors based on enabled features
        #[cfg(feature = "bedrock")]
        v.push(&ConverseDetector);

        #[cfg(feature = "google")]
        v.push(&GoogleDetector);

        #[cfg(feature = "anthropic")]
        v.push(&AnthropicDetector);

        #[cfg(feature = "mistral")]
        v.push(&MistralDetector);

        #[cfg(feature = "openai")]
        v.push(&OpenAIDetector);

        // Sort by priority (highest first)
        v.sort_by_key(|b| std::cmp::Reverse(b.priority()));
        v
    })
}

/// Errors that can occur during payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("JSON parsing failed: {0}")]
    JsonParseFailed(String),

    #[error("Invalid payload structure: {0}")]
    InvalidPayload(String),

    #[error("Unable to determine payload format")]
    UnableToDetermine,
}

/// Result of successful payload detection
#[derive(Debug, Clone)]
pub struct DetectedPayload {
    /// The detected provider format
    pub format: ProviderFormat,
    /// The model name extracted from the payload (if present)
    pub model: Option<String>,
    /// Whether the format was determined by catalog lookup (true) or heuristics (false)
    pub from_catalog: bool,
}

impl DetectedPayload {
    fn new(format: ProviderFormat, model: Option<String>, from_catalog: bool) -> Self {
        Self {
            format,
            model,
            from_catalog,
        }
    }
}

/// Type-safe payload that carries the parsed request for each provider format.
///
/// This enum ensures compile-time type safety by requiring exhaustive pattern matching.
/// Each variant contains the strongly-typed request structure for that provider.
///
/// # Example
///
/// ```ignore
/// use lingua::{parse, TypedPayload};
///
/// match parse(&payload)? {
///     TypedPayload::OpenAI(req) => {
///         // req is CreateChatCompletionRequestClass - fully typed
///         println!("OpenAI model: {}", req.model);
///     }
///     TypedPayload::Anthropic(req) => {
///         // req is CreateMessageParams - fully typed
///         println!("Anthropic max_tokens: {}", req.max_tokens);
///     }
///     // Compiler enforces handling all variants
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone)]
pub enum TypedPayload {
    /// OpenAI Chat Completions API request
    #[cfg(feature = "openai")]
    OpenAI(CreateChatCompletionRequestClass),
    /// Anthropic Messages API request
    #[cfg(feature = "anthropic")]
    Anthropic(CreateMessageParams),
    /// Google AI / Gemini GenerateContent API request (wrapped due to protobuf types)
    #[cfg(feature = "google")]
    Google(GooglePayload),
    /// AWS Bedrock Converse API request (wrapped for simpler API)
    #[cfg(feature = "bedrock")]
    Converse(BedrockPayload),
    /// Mistral AI request (uses OpenAI-compatible format)
    #[cfg(feature = "mistral")]
    Mistral(CreateChatCompletionRequestClass),
    /// Unknown format - raw JSON preserved for manual handling
    Unknown(Value),
}

impl TypedPayload {
    /// Returns the provider format for this payload.
    pub fn format(&self) -> ProviderFormat {
        match self {
            #[cfg(feature = "openai")]
            TypedPayload::OpenAI(_) => ProviderFormat::OpenAI,
            #[cfg(feature = "anthropic")]
            TypedPayload::Anthropic(_) => ProviderFormat::Anthropic,
            #[cfg(feature = "google")]
            TypedPayload::Google(_) => ProviderFormat::Google,
            #[cfg(feature = "bedrock")]
            TypedPayload::Converse(_) => ProviderFormat::Converse,
            #[cfg(feature = "mistral")]
            TypedPayload::Mistral(_) => ProviderFormat::Mistral,
            TypedPayload::Unknown(_) => ProviderFormat::Unknown,
        }
    }

    /// Extracts the model name from the payload, if present.
    pub fn model(&self) -> Option<&str> {
        match self {
            #[cfg(feature = "openai")]
            TypedPayload::OpenAI(req) => Some(&req.model),
            #[cfg(feature = "anthropic")]
            TypedPayload::Anthropic(req) => Some(&req.model),
            #[cfg(feature = "google")]
            TypedPayload::Google(payload) => payload.model.as_deref(),
            #[cfg(feature = "bedrock")]
            TypedPayload::Converse(payload) => payload.model_id.as_deref(),
            #[cfg(feature = "mistral")]
            TypedPayload::Mistral(req) => Some(&req.model),
            TypedPayload::Unknown(v) => v.get("model").and_then(|m| m.as_str()),
        }
    }

    /// Serializes the payload back to a JSON Value.
    pub fn into_value(self) -> Result<Value, DetectionError> {
        match self {
            #[cfg(feature = "openai")]
            TypedPayload::OpenAI(req) => {
                serde_json::to_value(req).map_err(|e| DetectionError::InvalidPayload(e.to_string()))
            }
            #[cfg(feature = "anthropic")]
            TypedPayload::Anthropic(req) => {
                serde_json::to_value(req).map_err(|e| DetectionError::InvalidPayload(e.to_string()))
            }
            #[cfg(feature = "google")]
            TypedPayload::Google(payload) => Ok(payload.into_value()),
            #[cfg(feature = "bedrock")]
            TypedPayload::Converse(payload) => Ok(payload.into_value()),
            #[cfg(feature = "mistral")]
            TypedPayload::Mistral(req) => {
                serde_json::to_value(req).map_err(|e| DetectionError::InvalidPayload(e.to_string()))
            }
            TypedPayload::Unknown(v) => Ok(v),
        }
    }

    /// Returns true if this is an unknown/unparsed payload.
    pub fn is_unknown(&self) -> bool {
        matches!(self, TypedPayload::Unknown(_))
    }
}

/// Parse a JSON payload into a strongly-typed provider request.
///
/// This is the main entry point for payload detection and parsing. It:
/// 1. Detects the provider format (via catalog lookup or heuristics)
/// 2. Parses the payload into the appropriate typed structure
///
/// # Arguments
///
/// * `payload` - The JSON payload as a `Value`
///
/// # Returns
///
/// * `Ok(TypedPayload)` - Successfully detected and parsed payload
/// * `Err(DetectionError)` - Failed to detect or parse the payload
///
/// # Example
///
/// ```ignore
/// let payload = serde_json::json!({
///     "model": "gpt-4o",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// match parse(&payload)? {
///     TypedPayload::OpenAI(req) => println!("Model: {}", req.model),
///     TypedPayload::Anthropic(req) => println!("Max tokens: {}", req.max_tokens),
///     _ => println!("Other format"),
/// }
/// ```
pub fn parse(payload: &Value) -> Result<TypedPayload, DetectionError> {
    // First detect the format
    let detected = detect_format(payload, false)?;

    // Then parse into the appropriate type
    match detected.format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI => {
            let req: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
                .map_err(|e| {
                    DetectionError::InvalidPayload(format!("OpenAI parse error: {}", e))
                })?;
            Ok(TypedPayload::OpenAI(req))
        }
        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => {
            let req: CreateMessageParams =
                serde_json::from_value(payload.clone()).map_err(|e| {
                    DetectionError::InvalidPayload(format!("Anthropic parse error: {}", e))
                })?;
            Ok(TypedPayload::Anthropic(req))
        }
        #[cfg(feature = "google")]
        ProviderFormat::Google => {
            // Google uses protobuf types without serde, so we wrap the validated JSON
            Ok(TypedPayload::Google(GooglePayload::new(payload.clone())))
        }
        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => {
            // Bedrock uses AWS SDK types, so we wrap the validated JSON
            Ok(TypedPayload::Converse(BedrockPayload::new(payload.clone())))
        }
        #[cfg(feature = "mistral")]
        ProviderFormat::Mistral => {
            // Mistral uses OpenAI-compatible format
            let req: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
                .map_err(|e| {
                    DetectionError::InvalidPayload(format!("Mistral parse error: {}", e))
                })?;
            Ok(TypedPayload::Mistral(req))
        }
        ProviderFormat::Unknown => {
            // Unknown format - preserve raw JSON
            Ok(TypedPayload::Unknown(payload.clone()))
        }
        // When features are disabled, fall back to Unknown
        #[allow(unreachable_patterns)]
        _ => Ok(TypedPayload::Unknown(payload.clone())),
    }
}

/// Parse a JSON string into a strongly-typed provider request.
///
/// Convenience wrapper that parses the JSON string first.
pub fn parse_from_str(payload: &str) -> Result<TypedPayload, DetectionError> {
    let value: Value = serde_json::from_str(payload)
        .map_err(|e| DetectionError::JsonParseFailed(e.to_string()))?;
    parse(&value)
}

/// Detect the format of an incoming payload.
///
/// Internal function that performs format detection:
/// 1. Extracts the model name from the payload (if present)
/// 2. Checks the model catalog for a known format
/// 3. Falls back to content-based heuristics if needed
///
/// # Arguments
///
/// * `payload` - The JSON payload as a `Value`
/// * `strict` - If true, return an error if format cannot be determined.
///   If false, default to OpenAI format.
///
/// # Returns
///
/// * `Ok(DetectedPayload)` - Successfully detected format with metadata
/// * `Err(DetectionError)` - Unable to detect format (only in strict mode)
fn detect_format(payload: &Value, strict: bool) -> Result<DetectedPayload, DetectionError> {
    // Extract model name from payload
    let model = extract_model_name(payload);

    // 1. Try model catalog first (if model field exists)
    if let Some(ref model_name) = model {
        if let Some(format) = catalog_lookup(model_name) {
            return Ok(DetectedPayload::new(format, model.clone(), true));
        }
    }

    // 2. Fall back to content-based heuristics
    detect_from_content(payload, model, strict)
}

/// Detect format from a JSON string.
///
/// Internal convenience wrapper that parses the JSON first.
#[cfg(test)]
fn detect_format_from_str(payload: &str, strict: bool) -> Result<DetectedPayload, DetectionError> {
    let value: Value = serde_json::from_str(payload)
        .map_err(|e| DetectionError::JsonParseFailed(e.to_string()))?;
    detect_format(&value, strict)
}

/// Extract the model name from a payload.
///
/// Different providers use different field names:
/// - OpenAI/Anthropic/Mistral: "model"
/// - Bedrock Converse: "modelId"
/// - Google: "model" (in the URL, not always in body)
fn extract_model_name(payload: &Value) -> Option<String> {
    // Try standard "model" field first
    if let Some(model) = payload.get("model").and_then(|v| v.as_str()) {
        return Some(model.to_string());
    }

    // Try Bedrock's "modelId" field
    if let Some(model_id) = payload.get("modelId").and_then(|v| v.as_str()) {
        return Some(model_id.to_string());
    }

    None
}

/// Detect format using content-based heuristics via registered detectors.
///
/// This function iterates through all registered detectors in priority order
/// and returns the first match. If no detector matches and strict mode is off,
/// defaults to OpenAI format.
fn detect_from_content(
    payload: &Value,
    model: Option<String>,
    strict: bool,
) -> Result<DetectedPayload, DetectionError> {
    // Iterate through detectors in priority order (already sorted)
    for detector in detectors() {
        if detector.detect(payload) {
            return Ok(DetectedPayload::new(detector.format(), model, false));
        }
    }

    // No format detected
    if strict {
        Err(DetectionError::UnableToDetermine)
    } else {
        // Default to OpenAI as the most common format
        Ok(DetectedPayload::new(ProviderFormat::OpenAI, model, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "openai")]
    fn test_detect_openai_format() {
        let payload = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Hello"}
            ]
        });

        let result = detect_format(&payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::OpenAI);
        assert_eq!(result.model, Some("gpt-4o".to_string()));
        assert!(!result.from_catalog);
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_detect_anthropic_format() {
        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let result = detect_format(&payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::Anthropic);
        assert_eq!(result.model, Some("claude-3-5-sonnet-20241022".to_string()));
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_detect_anthropic_with_tool_use() {
        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": "toolu_123",
                            "name": "get_weather",
                            "input": {"location": "SF"}
                        }
                    ]
                }
            ]
        });

        let result = detect_format(&payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::Anthropic);
    }

    #[test]
    #[cfg(feature = "google")]
    fn test_detect_google_format() {
        let payload = serde_json::json!({
            "contents": [
                {
                    "role": "user",
                    "parts": [{"text": "Hello"}]
                }
            ],
            "generationConfig": {
                "temperature": 0.7
            }
        });

        let result = detect_format(&payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::Google);
    }

    #[test]
    #[cfg(feature = "bedrock")]
    fn test_detect_bedrock_converse() {
        let payload = serde_json::json!({
            "modelId": "anthropic.claude-3-sonnet-20240229-v1:0",
            "messages": [
                {
                    "role": "user",
                    "content": [{"text": "Hello"}]
                }
            ],
            "inferenceConfig": {
                "maxTokens": 1024
            }
        });

        let result = detect_format(&payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::Converse);
        assert_eq!(
            result.model,
            Some("anthropic.claude-3-sonnet-20240229-v1:0".to_string())
        );
    }

    #[test]
    #[cfg(feature = "mistral")]
    fn test_detect_mistral_format() {
        let payload = serde_json::json!({
            "model": "mistral-large-latest",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "safe_prompt": true
        });

        let result = detect_format(&payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::Mistral);
    }

    #[test]
    fn test_strict_mode_unknown() {
        let payload = serde_json::json!({
            "unknown_field": "value"
        });

        let result = detect_format(&payload, true);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_non_strict_defaults_to_openai() {
        let payload = serde_json::json!({
            "unknown_field": "value"
        });

        let result = detect_format(&payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::OpenAI);
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_detect_format_from_str() {
        let payload = r#"{"model": "gpt-4", "messages": [{"role": "user", "content": "Hi"}]}"#;
        let result = detect_format_from_str(payload, false).unwrap();
        assert_eq!(result.format, ProviderFormat::OpenAI);
    }

    #[test]
    fn test_detect_format_from_str_invalid_json() {
        let payload = r#"{"invalid json"#;
        let result = detect_format_from_str(payload, false);
        assert!(matches!(result, Err(DetectionError::JsonParseFailed(_))));
    }

    // TypedPayload tests

    #[test]
    #[cfg(feature = "openai")]
    fn test_parse_openai() {
        let payload = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Hello"}
            ]
        });

        let result = parse(&payload).unwrap();
        assert!(matches!(result, TypedPayload::OpenAI(_)));
        assert_eq!(result.format(), ProviderFormat::OpenAI);
        assert_eq!(result.model(), Some("gpt-4o"));
        assert!(!result.is_unknown());
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_parse_anthropic() {
        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let result = parse(&payload).unwrap();
        assert!(matches!(result, TypedPayload::Anthropic(_)));
        assert_eq!(result.format(), ProviderFormat::Anthropic);
        assert_eq!(result.model(), Some("claude-3-5-sonnet-20241022"));
    }

    #[test]
    #[cfg(feature = "mistral")]
    fn test_parse_mistral() {
        let payload = serde_json::json!({
            "model": "mistral-large-latest",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "safe_prompt": true
        });

        let result = parse(&payload).unwrap();
        assert!(matches!(result, TypedPayload::Mistral(_)));
        assert_eq!(result.format(), ProviderFormat::Mistral);
        assert_eq!(result.model(), Some("mistral-large-latest"));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_parse_unknown() {
        let payload = serde_json::json!({
            "some_unknown_field": "value",
            "model": "unknown-model"
        });

        // Unknown payloads will be detected as OpenAI (default) but may fail to parse
        // Since the payload lacks "messages", OpenAI parsing will fail
        let result = parse(&payload);
        assert!(result.is_err());

        // A valid-looking but unknown payload should succeed
        let valid_payload = serde_json::json!({
            "model": "some-unknown-model",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let result = parse(&valid_payload).unwrap();
        // Detected as OpenAI since it has the right structure
        assert_eq!(result.format(), ProviderFormat::OpenAI);
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_parse_into_value() {
        let payload = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let typed = parse(&payload).unwrap();
        let roundtrip = typed.into_value().unwrap();

        // Verify key fields are preserved
        assert_eq!(
            roundtrip.get("model").and_then(|v| v.as_str()),
            Some("gpt-4o")
        );
        assert!(roundtrip.get("messages").is_some());
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_parse_from_str() {
        let payload = r#"{"model": "gpt-4", "messages": [{"role": "user", "content": "Hi"}]}"#;
        let result = parse_from_str(payload).unwrap();
        assert!(matches!(result, TypedPayload::OpenAI(_)));
        assert_eq!(result.format(), ProviderFormat::OpenAI);
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_parse_anthropic_with_typed_access() {
        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 2048,
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let result = parse(&payload).unwrap();

        // Demonstrate type-safe access
        match result {
            TypedPayload::Anthropic(req) => {
                // Compile-time typed access to Anthropic fields
                assert_eq!(req.model, "claude-3-5-sonnet-20241022");
                assert_eq!(req.max_tokens, 2048);
                assert_eq!(req.messages.len(), 1);
            }
            _ => panic!("Expected Anthropic payload"),
        }
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_parse_openai_with_typed_access() {
        let payload = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "Be helpful"},
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.7
        });

        let result = parse(&payload).unwrap();

        // Demonstrate type-safe access
        match result {
            TypedPayload::OpenAI(req) => {
                // Compile-time typed access to OpenAI fields
                assert_eq!(req.model, "gpt-4o");
                assert_eq!(req.messages.len(), 2);
                assert_eq!(req.temperature, Some(0.7));
            }
            _ => panic!("Expected OpenAI payload"),
        }
    }

    /// Integration test: full pipeline from detection through transformation to serialization.
    #[test]
    #[cfg(feature = "openai")]
    fn test_full_pipeline_detect_transform_serialize() {
        use crate::providers::openai::transformations::{OpenAIRequestTransformer, TargetProvider};

        // 1. Start with a raw OpenAI payload requiring transformation
        let input = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "stream_options": {"include_usage": true},
            "parallel_tool_calls": true
        });

        // 2. Detect and parse
        let typed = parse(&input).unwrap();
        assert_eq!(typed.format(), ProviderFormat::OpenAI);

        // 3. Extract typed request and transform for Mistral
        match typed {
            TypedPayload::OpenAI(mut req) => {
                // Mistral doesn't support stream_options or parallel_tool_calls
                OpenAIRequestTransformer::new(&mut req)
                    .with_target_provider(TargetProvider::Mistral)
                    .transform()
                    .expect("transform succeeds");

                // 4. Verify transformations applied
                assert!(req.stream_options.is_none());
                assert!(req.parallel_tool_calls.is_none());
                assert_eq!(req.model, "gpt-4o");
                assert_eq!(req.messages.len(), 1);

                // 5. Serialize back to JSON
                let output = serde_json::to_value(&req).expect("serialize succeeds");
                assert!(output.get("stream_options").is_none());
                assert!(output.get("parallel_tool_calls").is_none());
            }
            _ => panic!("Expected OpenAI payload"),
        }
    }
}
