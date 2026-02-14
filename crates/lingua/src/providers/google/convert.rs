/*!
Google format conversions.

This module provides TryFromLLM trait implementations for converting between
Google's GenerateContent API format and Lingua's universal message format.
*/

use crate::capabilities::format::ProviderFormat;
use crate::error::ConvertError;
use crate::providers::google::generated::{
    Blob as GoogleBlob, Content as GoogleContent, FunctionCall as GoogleFunctionCall,
    FunctionCallingConfig, FunctionCallingConfigMode, FunctionDeclaration,
    FunctionResponse as GoogleFunctionResponse, GenerateContentRequest, GenerationConfig,
    Part as GooglePart, Tool as GoogleTool, ToolConfig,
};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::defaults::DEFAULT_MIME_TYPE;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use crate::universal::request::{
    JsonSchemaConfig, ResponseFormatConfig, ResponseFormatType, ToolChoiceConfig, ToolChoiceMode,
};
use crate::universal::tools::{UniversalTool, UniversalToolType};
use crate::util::media::parse_base64_data_url;

// ============================================================================
// Google Content -> Universal Message
// ============================================================================

fn text_part(text: String) -> GooglePart {
    GooglePart {
        text: Some(text),
        ..Default::default()
    }
}

fn value_to_map(value: &Value) -> Option<Map<String, Value>> {
    match value {
        Value::Object(map) => Some(map.clone()),
        Value::Null => None,
        _ => {
            let mut wrapped = Map::new();
            wrapped.insert("output".to_string(), value.clone());
            Some(wrapped)
        }
    }
}

impl TryFromLLM<GoogleContent> for Message {
    type Error = ConvertError;

    fn try_from(content: GoogleContent) -> Result<Self, Self::Error> {
        let role = content
            .role
            .as_deref()
            .ok_or(ConvertError::MissingRequiredField {
                field: "role".to_string(),
            })?;
        let parts = content.parts.ok_or(ConvertError::MissingRequiredField {
            field: "parts".to_string(),
        })?;

        match role {
            "model" => {
                let mut assistant_parts: Vec<AssistantContentPart> = Vec::new();

                for part in &parts {
                    if let Some(t) = &part.text {
                        if part.thought_signature.is_some() {
                            assistant_parts.push(AssistantContentPart::Reasoning {
                                text: t.clone(),
                                encrypted_content: part.thought_signature.clone(),
                            });
                        } else {
                            assistant_parts.push(AssistantContentPart::Text(TextContentPart {
                                text: t.clone(),
                                provider_options: None,
                            }));
                        }
                    } else if let Some(fc) = &part.function_call {
                        if let Some(tool_name) = &fc.name {
                            let args_value = match fc.args.as_ref() {
                                Some(map) => Value::Object(map.clone()),
                                None => Value::Null,
                            };
                            let encrypted_content = part.thought_signature.clone();
                            let args_string = serde_json::to_string(&args_value).map_err(|e| {
                                ConvertError::ContentConversionFailed {
                                    reason: format!("Failed to serialize function call args: {e}"),
                                }
                            })?;
                            assistant_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: fc.id.clone().unwrap_or_default(),
                                tool_name: tool_name.clone(),
                                arguments: ToolCallArguments::from(args_string),
                                encrypted_content,
                                provider_options: None,
                                provider_executed: None,
                            });
                        }
                    }
                }

                Ok(Message::Assistant {
                    content: AssistantContent::Array(assistant_parts),
                    id: None,
                })
            }

            // "user" or unknown roles
            _ => {
                let mut user_parts: Vec<UserContentPart> = Vec::new();
                let mut tool_parts: Vec<ToolContentPart> = Vec::new();

                for part in &parts {
                    if let Some(t) = &part.text {
                        user_parts.push(UserContentPart::Text(TextContentPart {
                            text: t.clone(),
                            provider_options: None,
                        }));
                    } else if let Some(blob) = &part.inline_data {
                        if let Some(data) = &blob.data {
                            user_parts.push(UserContentPart::Image {
                                image: Value::String(data.clone()),
                                media_type: blob.mime_type.clone(),
                                provider_options: None,
                            });
                        }
                    } else if let Some(fr) = &part.function_response {
                        if let Some(tool_name) = &fr.name {
                            let output = match fr.response.as_ref() {
                                Some(map) => Value::Object(map.clone()),
                                None => Value::Null,
                            };
                            tool_parts.push(ToolContentPart::ToolResult(ToolResultContentPart {
                                tool_call_id: fr.id.clone().unwrap_or_default(),
                                tool_name: tool_name.clone(),
                                output,
                                provider_options: None,
                            }));
                        }
                    }
                }

                if !tool_parts.is_empty() {
                    Ok(Message::Tool {
                        content: tool_parts,
                    })
                } else if user_parts.len() == 1
                    && matches!(&user_parts[0], UserContentPart::Text(_))
                {
                    let text = match user_parts.remove(0) {
                        UserContentPart::Text(t) => t.text,
                        _ => unreachable!(),
                    };
                    Ok(Message::User {
                        content: UserContent::String(text),
                    })
                } else {
                    Ok(Message::User {
                        content: UserContent::Array(user_parts),
                    })
                }
            }
        }
    }
}

// ============================================================================
// Universal Message -> Google Content
// ============================================================================

impl TryFromLLM<Message> for GoogleContent {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let (role, parts) = match message {
            Message::System { content } => {
                let text = match content {
                    UserContent::String(s) => format!("System: {}", s),
                    UserContent::Array(parts) => {
                        let texts: Vec<String> = parts
                            .into_iter()
                            .filter_map(|p| match p {
                                UserContentPart::Text(t) => Some(t.text),
                                _ => None,
                            })
                            .collect();
                        format!("System: {}", texts.join(""))
                    }
                };
                ("user".to_string(), vec![text_part(text)])
            }
            Message::User { content } => {
                let parts = match content {
                    UserContent::String(s) => vec![text_part(s)],
                    UserContent::Array(parts) => {
                        let mut converted = Vec::new();
                        for part in parts {
                            match part {
                                UserContentPart::Text(t) => {
                                    converted.push(text_part(t.text));
                                }
                                UserContentPart::Image {
                                    image: Value::String(data),
                                    media_type,
                                    ..
                                } => {
                                    let mut inferred_media_type = None;
                                    let base64_data =
                                        if let Some(block) = parse_base64_data_url(&data) {
                                            inferred_media_type = Some(block.media_type);
                                            block.data
                                        } else {
                                            data
                                        };

                                    let mime_type = media_type
                                        .or(inferred_media_type)
                                        .unwrap_or_else(|| DEFAULT_MIME_TYPE.to_string());

                                    converted.push(GooglePart {
                                        inline_data: Some(GoogleBlob {
                                            mime_type: Some(mime_type),
                                            data: Some(base64_data),
                                        }),
                                        ..Default::default()
                                    });
                                }
                                _ => {}
                            }
                        }
                        converted
                    }
                };
                ("user".to_string(), parts)
            }
            Message::Assistant { content, .. } => {
                let parts = match content {
                    AssistantContent::String(s) => vec![text_part(s)],
                    AssistantContent::Array(parts) => {
                        let mut converted = Vec::new();
                        for p in parts {
                            match p {
                                AssistantContentPart::Text(t) => {
                                    converted.push(text_part(t.text));
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    encrypted_content,
                                    ..
                                } => {
                                    let value = match arguments {
                                        ToolCallArguments::Valid(map) => Some(Value::Object(map)),
                                        ToolCallArguments::Invalid(s) => {
                                            serde_json::from_str(&s).ok()
                                        }
                                    };
                                    let args = match value {
                                        Some(Value::Object(map)) => Some(map),
                                        _ => None,
                                    };

                                    converted.push(GooglePart {
                                        function_call: Some(GoogleFunctionCall {
                                            id: Some(tool_call_id).filter(|s| !s.is_empty()),
                                            name: Some(tool_name),
                                            args,
                                        }),
                                        thought_signature: encrypted_content,
                                        ..Default::default()
                                    });
                                }
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content,
                                } => {
                                    converted.push(GooglePart {
                                        text: Some(text),
                                        thought_signature: encrypted_content,
                                        ..Default::default()
                                    });
                                }
                                _ => {}
                            }
                        }
                        converted
                    }
                };
                ("model".to_string(), parts)
            }
            Message::Tool { content } => {
                let parts: Vec<GooglePart> = content
                    .into_iter()
                    .map(|part| {
                        let ToolContentPart::ToolResult(result) = part;
                        let response = value_to_map(&result.output);

                        Ok(GooglePart {
                            function_response: Some(GoogleFunctionResponse {
                                id: Some(result.tool_call_id).filter(|s| !s.is_empty()),
                                name: Some(result.tool_name),
                                response,
                                ..Default::default()
                            }),
                            ..Default::default()
                        })
                    })
                    .collect::<Result<Vec<_>, ConvertError>>()?;
                ("user".to_string(), parts)
            }
        };

        Ok(GoogleContent {
            role: Some(role),
            parts: Some(parts),
        })
    }
}

// ============================================================================
// Convenience functions using trait implementations
// ============================================================================

/// Convert Google GenerateContentRequest to universal messages.
pub fn google_to_universal(request: &GenerateContentRequest) -> Result<Vec<Message>, ConvertError> {
    let contents = request
        .contents
        .clone()
        .ok_or(ConvertError::MissingRequiredField {
            field: "contents".to_string(),
        })?;
    <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(contents)
}

/// Convert universal messages to Google contents.
pub fn universal_to_google_contents(
    messages: &[Message],
) -> Result<Vec<GoogleContent>, ConvertError> {
    <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
}

/// Convert universal messages to Google GenerateContent format as JSON Value.
///
/// This serializes the converted GoogleContent structs to JSON for use
/// in contexts where a Value is needed (e.g., when building full requests).
pub fn universal_to_google(messages: &[Message]) -> Result<Value, ConvertError> {
    let contents = universal_to_google_contents(messages)?;
    serde_json::to_value(contents).map_err(|e| ConvertError::JsonSerializationFailed {
        field: "contents".to_string(),
        error: e.to_string(),
    })
}

impl From<&FunctionDeclaration> for UniversalTool {
    fn from(decl: &FunctionDeclaration) -> Self {
        let parameters = decl
            .parameters
            .as_ref()
            .as_ref()
            .and_then(|schema| serde_json::to_value(schema).ok());

        UniversalTool::function(
            decl.name.as_deref().unwrap_or(""),
            decl.description.clone(),
            parameters,
            None,
        )
    }
}

impl TryFrom<&UniversalTool> for FunctionDeclaration {
    type Error = ConvertError;

    fn try_from(tool: &UniversalTool) -> Result<Self, Self::Error> {
        match &tool.tool_type {
            UniversalToolType::Function => {
                let parameters = tool
                    .parameters
                    .as_ref()
                    .map(|v| {
                        serde_json::from_value(v.clone()).map_err(|e| {
                            ConvertError::JsonSerializationFailed {
                                field: format!("tool '{}' parameters", tool.name),
                                error: e.to_string(),
                            }
                        })
                    })
                    .transpose()?;

                Ok(FunctionDeclaration {
                    name: Some(tool.name.clone()),
                    description: tool.description.clone(),
                    parameters: Box::new(parameters),
                    ..Default::default()
                })
            }
            UniversalToolType::Builtin { builtin_type, .. } => {
                Err(ConvertError::UnsupportedToolType {
                    tool_name: tool.name.clone(),
                    tool_type: builtin_type.clone(),
                    target_provider: "Google".to_string(),
                })
            }
        }
    }
}

/// Convert Google tools to universal tools.
///
/// Each Google `Tool` can contain function declarations and/or builtin tool configs
/// (google_search, code_execution, etc.). This flattens them into individual `UniversalTool`s.
impl TryFromLLM<Vec<GoogleTool>> for Vec<UniversalTool> {
    type Error = ConvertError;

    fn try_from(tools: Vec<GoogleTool>) -> Result<Self, Self::Error> {
        let mut result = Vec::new();

        for tool in &tools {
            if let Some(decls) = &tool.function_declarations {
                for decl in decls {
                    result.push(UniversalTool::from(decl));
                }
            }

            if let Some(google_search) = &tool.google_search {
                let config = serde_json::to_value(google_search).map_err(|e| {
                    ConvertError::JsonSerializationFailed {
                        field: "google_search".to_string(),
                        error: e.to_string(),
                    }
                })?;
                result.push(UniversalTool::builtin(
                    "google_search",
                    ProviderFormat::Google,
                    "google_search",
                    Some(config),
                ));
            }

            if let Some(code_execution) = &tool.code_execution {
                let config = serde_json::to_value(code_execution).map_err(|e| {
                    ConvertError::JsonSerializationFailed {
                        field: "code_execution".to_string(),
                        error: e.to_string(),
                    }
                })?;
                result.push(UniversalTool::builtin(
                    "code_execution",
                    ProviderFormat::Google,
                    "code_execution",
                    Some(config),
                ));
            }

            if let Some(google_search_retrieval) = &tool.google_search_retrieval {
                let config = serde_json::to_value(google_search_retrieval).map_err(|e| {
                    ConvertError::JsonSerializationFailed {
                        field: "google_search_retrieval".to_string(),
                        error: e.to_string(),
                    }
                })?;
                result.push(UniversalTool::builtin(
                    "google_search_retrieval",
                    ProviderFormat::Google,
                    "google_search_retrieval",
                    Some(config),
                ));
            }
        }

        Ok(result)
    }
}

/// Convert universal tools back to Google Tool structs.
///
/// Groups function tools into a single `Tool { function_declarations }` and
/// reconstructs builtin tools (google_search, code_execution, etc.) from their configs.
impl TryFromLLM<Vec<UniversalTool>> for Vec<GoogleTool> {
    type Error = ConvertError;

    fn try_from(tools: Vec<UniversalTool>) -> Result<Self, Self::Error> {
        let mut function_decls = Vec::new();
        let mut builtin_tools = Vec::new();

        for tool in &tools {
            match &tool.tool_type {
                UniversalToolType::Function => {
                    function_decls.push(FunctionDeclaration::try_from(tool)?);
                }
                UniversalToolType::Builtin {
                    provider,
                    builtin_type,
                    config,
                } => {
                    if !matches!(provider, ProviderFormat::Google) {
                        continue;
                    }
                    let mut google_tool = GoogleTool::default();
                    match builtin_type.as_str() {
                        "google_search" => {
                            google_tool.google_search = config
                                .as_ref()
                                .and_then(|v| serde_json::from_value(v.clone()).ok());
                        }
                        "code_execution" => {
                            google_tool.code_execution = config
                                .as_ref()
                                .and_then(|v| serde_json::from_value(v.clone()).ok());
                        }
                        "google_search_retrieval" => {
                            google_tool.google_search_retrieval = config
                                .as_ref()
                                .and_then(|v| serde_json::from_value(v.clone()).ok());
                        }
                        _ => {
                            continue;
                        }
                    }
                    builtin_tools.push(google_tool);
                }
            }
        }

        let mut result = Vec::new();
        if !function_decls.is_empty() {
            result.push(GoogleTool {
                function_declarations: Some(function_decls),
                ..Default::default()
            });
        }
        result.extend(builtin_tools);

        Ok(result)
    }
}

impl From<&ToolConfig> for ToolChoiceConfig {
    fn from(config: &ToolConfig) -> Self {
        let fcc = config.function_calling_config.as_ref();

        let mode = fcc.and_then(|c| {
            c.mode.as_ref().map(|m| match m {
                FunctionCallingConfigMode::Auto | FunctionCallingConfigMode::Validated => {
                    ToolChoiceMode::Auto
                }
                FunctionCallingConfigMode::Any => ToolChoiceMode::Required,
                FunctionCallingConfigMode::None => ToolChoiceMode::None,
                FunctionCallingConfigMode::ModeUnspecified => ToolChoiceMode::Auto,
            })
        });

        // If mode is Any and there's exactly one allowed function name, treat as Tool mode
        let (mode, tool_name) = match (mode, fcc) {
            (Some(ToolChoiceMode::Required), Some(fcc_inner)) => {
                if let Some(names) = &fcc_inner.allowed_function_names {
                    if names.len() == 1 {
                        (Some(ToolChoiceMode::Tool), Some(names[0].clone()))
                    } else {
                        (Some(ToolChoiceMode::Required), None)
                    }
                } else {
                    (Some(ToolChoiceMode::Required), None)
                }
            }
            (mode, _) => (mode, None),
        };

        ToolChoiceConfig {
            mode,
            tool_name,
            disable_parallel: None,
        }
    }
}

impl TryFrom<&ToolChoiceConfig> for ToolConfig {
    type Error = ();

    fn try_from(config: &ToolChoiceConfig) -> Result<Self, Self::Error> {
        let mode = config.mode.ok_or(())?;

        let (google_mode, allowed_names) = match mode {
            ToolChoiceMode::Auto => (FunctionCallingConfigMode::Auto, None),
            ToolChoiceMode::Required => (FunctionCallingConfigMode::Any, None),
            ToolChoiceMode::None => (FunctionCallingConfigMode::None, None),
            ToolChoiceMode::Tool => {
                let name = config.tool_name.clone().ok_or(())?;
                (FunctionCallingConfigMode::Any, Some(vec![name]))
            }
        };

        Ok(ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: Some(google_mode),
                allowed_function_names: allowed_names,
            }),
            retrieval_config: None,
        })
    }
}

/// Extract response format from a Google GenerationConfig.
pub fn response_format_from_generation_config(
    config: &GenerationConfig,
) -> Option<ResponseFormatConfig> {
    let mime = config.response_mime_type.as_deref()?;

    match mime {
        "application/json" => {
            if let Some(schema) = config.response_schema.as_ref() {
                let schema_value = serde_json::to_value(schema).ok()?;
                Some(ResponseFormatConfig {
                    format_type: Some(ResponseFormatType::JsonSchema),
                    json_schema: Some(JsonSchemaConfig {
                        name: "response".to_string(),
                        schema: schema_value,
                        strict: None,
                        description: None,
                    }),
                })
            } else {
                Some(ResponseFormatConfig {
                    format_type: Some(ResponseFormatType::JsonObject),
                    json_schema: None,
                })
            }
        }
        "text/plain" => Some(ResponseFormatConfig {
            format_type: Some(ResponseFormatType::Text),
            json_schema: None,
        }),
        // text/x.enum or other types - store as Text for now
        _ => None,
    }
}

/// Apply a ResponseFormatConfig to a GenerationConfig.
pub fn apply_response_format_to_generation_config(
    config: &mut GenerationConfig,
    format: &ResponseFormatConfig,
) {
    match format.format_type {
        Some(ResponseFormatType::JsonSchema) => {
            config.response_mime_type = Some("application/json".to_string());
            if let Some(js) = &format.json_schema {
                let schema = serde_json::from_value(js.schema.clone()).ok();
                *config.response_schema = schema;
            }
        }
        Some(ResponseFormatType::JsonObject) => {
            config.response_mime_type = Some("application/json".to_string());
        }
        Some(ResponseFormatType::Text) => {
            config.response_mime_type = Some("text/plain".to_string());
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_google_content_to_message_user() {
        let content = GoogleContent {
            role: Some("user".to_string()),
            parts: Some(vec![text_part("Hello".to_string())]),
        };

        let message = <Message as TryFromLLM<GoogleContent>>::try_from(content).unwrap();
        match message {
            Message::User { content } => match content {
                UserContent::String(s) => assert_eq!(s, "Hello"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_google_content_to_message_model() {
        let content = GoogleContent {
            role: Some("model".to_string()),
            parts: Some(vec![text_part("Hi there!".to_string())]),
        };

        let message = <Message as TryFromLLM<GoogleContent>>::try_from(content).unwrap();
        match message {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::Text(t) => assert_eq!(t.text, "Hi there!"),
                        _ => panic!("Expected text part"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_google_content_to_message_function_call() {
        let args: Map<String, Value> = serde_json::from_value(json!({"location": "SF"})).unwrap();
        let content = GoogleContent {
            role: Some("model".to_string()),
            parts: Some(vec![GooglePart {
                function_call: Some(GoogleFunctionCall {
                    id: None,
                    name: Some("get_weather".to_string()),
                    args: Some(args),
                }),
                ..Default::default()
            }]),
        };

        let message = <Message as TryFromLLM<GoogleContent>>::try_from(content).unwrap();
        match message {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::ToolCall {
                            tool_name,
                            tool_call_id,
                            ..
                        } => {
                            assert_eq!(tool_name, "get_weather");
                            assert_eq!(tool_call_id, "");
                        }
                        _ => panic!("Expected tool call part"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_message_to_google_content_user() {
        let message = Message::User {
            content: UserContent::String("Hello".to_string()),
        };

        let content = <GoogleContent as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(content.role.as_deref(), Some("user"));
        let parts = content.parts.unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_message_to_google_content_assistant() {
        let message = Message::Assistant {
            content: AssistantContent::String("Hi there!".to_string()),
            id: None,
        };

        let content = <GoogleContent as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(content.role.as_deref(), Some("model"));
        let parts = content.parts.unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text.as_deref(), Some("Hi there!"));
    }

    #[test]
    fn test_message_to_google_content_tool_call() {
        let message = Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id: "call_123".to_string(),
                tool_name: "get_weather".to_string(),
                arguments: ToolCallArguments::from(r#"{"location":"SF"}"#.to_string()),
                encrypted_content: None,
                provider_options: None,
                provider_executed: None,
            }]),
            id: None,
        };

        let content = <GoogleContent as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(content.role.as_deref(), Some("model"));
        let parts = content.parts.unwrap();
        assert_eq!(parts.len(), 1);
        let fc = parts[0].function_call.as_ref().unwrap();
        assert_eq!(fc.name.as_deref(), Some("get_weather"));
    }

    #[test]
    fn test_google_to_universal_simple() {
        let request = GenerateContentRequest {
            contents: Some(vec![GoogleContent {
                role: Some("user".to_string()),
                parts: Some(vec![text_part("Hello".to_string())]),
            }]),
            ..Default::default()
        };

        let messages = google_to_universal(&request).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::User { content } => match content {
                UserContent::String(s) => assert_eq!(s, "Hello"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_universal_to_google_simple() {
        let messages = vec![Message::User {
            content: UserContent::String("Hello".to_string()),
        }];

        let result = universal_to_google(&messages).unwrap();
        let expected = json!([{
            "role": "user",
            "parts": [{"text": "Hello"}]
        }]);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_universal_to_google_with_assistant() {
        let messages = vec![
            Message::User {
                content: UserContent::String("Hello".to_string()),
            },
            Message::Assistant {
                content: AssistantContent::String("Hi there!".to_string()),
                id: None,
            },
        ];

        let result = universal_to_google(&messages).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["role"], "user");
        assert_eq!(arr[1]["role"], "model");
    }

    #[test]
    fn test_function_declaration_to_universal_tool() {
        let decl = FunctionDeclaration {
            name: Some("get_weather".to_string()),
            description: Some("Get weather info".to_string()),
            parameters: Box::new(Some(
                serde_json::from_value(json!({
                    "type": "OBJECT",
                    "properties": {
                        "location": {"type": "STRING"}
                    }
                }))
                .unwrap(),
            )),
            ..Default::default()
        };

        let tool = UniversalTool::from(&decl);
        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.description, Some("Get weather info".to_string()));
        assert!(tool.parameters.is_some());
        assert!(tool.is_function());
    }

    #[test]
    fn test_universal_tool_to_function_declaration() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather info".to_string()),
            Some(json!({"type": "OBJECT", "properties": {"location": {"type": "STRING"}}})),
            None,
        );

        let decl = FunctionDeclaration::try_from(&tool).unwrap();
        assert_eq!(decl.name, Some("get_weather".to_string()));
        assert_eq!(decl.description, Some("Get weather info".to_string()));
        assert!(decl.parameters.is_some());
    }

    #[test]
    fn test_google_tools_to_universal_roundtrip() {
        let google_tools = vec![GoogleTool {
            function_declarations: Some(vec![FunctionDeclaration {
                name: Some("get_weather".to_string()),
                description: Some("Get weather".to_string()),
                parameters: Box::new(None),
                ..Default::default()
            }]),
            ..Default::default()
        }];

        let universal =
            <Vec<UniversalTool> as TryFromLLM<Vec<GoogleTool>>>::try_from(google_tools).unwrap();
        assert_eq!(universal.len(), 1);
        assert_eq!(universal[0].name, "get_weather");

        let back =
            <Vec<GoogleTool> as TryFromLLM<Vec<UniversalTool>>>::try_from(universal).unwrap();
        assert_eq!(back.len(), 1);
        let decls = back[0].function_declarations.as_ref().unwrap();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, Some("get_weather".to_string()));
    }

    #[test]
    fn test_google_search_builtin_roundtrip() {
        let google_tools = vec![GoogleTool {
            google_search: Some(Default::default()),
            ..Default::default()
        }];

        let universal =
            <Vec<UniversalTool> as TryFromLLM<Vec<GoogleTool>>>::try_from(google_tools).unwrap();
        assert_eq!(universal.len(), 1);
        assert_eq!(universal[0].name, "google_search");
        assert!(!universal[0].is_function());

        let back =
            <Vec<GoogleTool> as TryFromLLM<Vec<UniversalTool>>>::try_from(universal).unwrap();
        assert_eq!(back.len(), 1);
        assert!(back[0].google_search.is_some());
    }

    #[test]
    fn test_tool_config_auto_to_tool_choice() {
        let config = ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: Some(FunctionCallingConfigMode::Auto),
                allowed_function_names: None,
            }),
            retrieval_config: None,
        };

        let choice = ToolChoiceConfig::from(&config);
        assert_eq!(choice.mode, Some(ToolChoiceMode::Auto));
        assert_eq!(choice.tool_name, None);
    }

    #[test]
    fn test_tool_config_any_to_tool_choice_required() {
        let config = ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: Some(FunctionCallingConfigMode::Any),
                allowed_function_names: None,
            }),
            retrieval_config: None,
        };

        let choice = ToolChoiceConfig::from(&config);
        assert_eq!(choice.mode, Some(ToolChoiceMode::Required));
    }

    #[test]
    fn test_tool_config_any_with_single_name_to_tool_mode() {
        let config = ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: Some(FunctionCallingConfigMode::Any),
                allowed_function_names: Some(vec!["get_weather".to_string()]),
            }),
            retrieval_config: None,
        };

        let choice = ToolChoiceConfig::from(&config);
        assert_eq!(choice.mode, Some(ToolChoiceMode::Tool));
        assert_eq!(choice.tool_name, Some("get_weather".to_string()));
    }

    #[test]
    fn test_tool_config_none_to_tool_choice() {
        let config = ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                mode: Some(FunctionCallingConfigMode::None),
                allowed_function_names: None,
            }),
            retrieval_config: None,
        };

        let choice = ToolChoiceConfig::from(&config);
        assert_eq!(choice.mode, Some(ToolChoiceMode::None));
    }

    #[test]
    fn test_tool_choice_auto_to_tool_config() {
        let choice = ToolChoiceConfig {
            mode: Some(ToolChoiceMode::Auto),
            tool_name: None,
            disable_parallel: None,
        };

        let config = ToolConfig::try_from(&choice).unwrap();
        let fcc = config.function_calling_config.unwrap();
        assert_eq!(fcc.mode, Some(FunctionCallingConfigMode::Auto));
    }

    #[test]
    fn test_tool_choice_required_to_tool_config_any() {
        let choice = ToolChoiceConfig {
            mode: Some(ToolChoiceMode::Required),
            tool_name: None,
            disable_parallel: None,
        };

        let config = ToolConfig::try_from(&choice).unwrap();
        let fcc = config.function_calling_config.unwrap();
        assert_eq!(fcc.mode, Some(FunctionCallingConfigMode::Any));
    }

    #[test]
    fn test_tool_choice_tool_to_tool_config_with_name() {
        let choice = ToolChoiceConfig {
            mode: Some(ToolChoiceMode::Tool),
            tool_name: Some("get_weather".to_string()),
            disable_parallel: None,
        };

        let config = ToolConfig::try_from(&choice).unwrap();
        let fcc = config.function_calling_config.unwrap();
        assert_eq!(fcc.mode, Some(FunctionCallingConfigMode::Any));
        assert_eq!(
            fcc.allowed_function_names,
            Some(vec!["get_weather".to_string()])
        );
    }

    #[test]
    fn test_response_format_json_schema_from_generation_config() {
        let config = GenerationConfig {
            response_mime_type: Some("application/json".to_string()),
            response_schema: Box::new(Some(
                serde_json::from_value(json!({
                    "type": "OBJECT",
                    "properties": {
                        "name": {"type": "STRING"}
                    }
                }))
                .unwrap(),
            )),
            ..Default::default()
        };

        let format = response_format_from_generation_config(&config).unwrap();
        assert_eq!(format.format_type, Some(ResponseFormatType::JsonSchema));
        assert!(format.json_schema.is_some());
    }

    #[test]
    fn test_response_format_json_object_from_generation_config() {
        let config = GenerationConfig {
            response_mime_type: Some("application/json".to_string()),
            response_schema: Box::new(None),
            ..Default::default()
        };

        let format = response_format_from_generation_config(&config).unwrap();
        assert_eq!(format.format_type, Some(ResponseFormatType::JsonObject));
        assert!(format.json_schema.is_none());
    }

    #[test]
    fn test_response_format_text_from_generation_config() {
        let config = GenerationConfig {
            response_mime_type: Some("text/plain".to_string()),
            ..Default::default()
        };

        let format = response_format_from_generation_config(&config).unwrap();
        assert_eq!(format.format_type, Some(ResponseFormatType::Text));
    }

    #[test]
    fn test_apply_json_schema_to_generation_config() {
        let format = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "response".to_string(),
                schema: json!({
                    "type": "OBJECT",
                    "properties": {
                        "name": {"type": "STRING"}
                    }
                }),
                strict: None,
                description: None,
            }),
        };

        let mut config = GenerationConfig::default();
        apply_response_format_to_generation_config(&mut config, &format);

        assert_eq!(
            config.response_mime_type,
            Some("application/json".to_string())
        );
        assert!(config.response_schema.is_some());
    }

    #[test]
    fn test_apply_json_object_to_generation_config() {
        let format = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonObject),
            json_schema: None,
        };

        let mut config = GenerationConfig::default();
        apply_response_format_to_generation_config(&mut config, &format);

        assert_eq!(
            config.response_mime_type,
            Some("application/json".to_string())
        );
        assert!(config.response_schema.is_none());
    }
}
