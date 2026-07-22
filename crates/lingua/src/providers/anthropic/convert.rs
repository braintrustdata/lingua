use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::import_parse::{
    try_convert_non_empty, try_parse, try_parse_vec_or_single, try_parsers_in_order, MessageParser,
};
use crate::providers::anthropic::generated;
use crate::providers::anthropic::generated::{
    CreateMessageParams, CustomTool, JsonOutputFormat, JsonOutputFormatType, Tool, ToolChoice,
    ToolChoiceType,
};
use crate::providers::anthropic::tool_discovery;
use crate::providers::google::generated::GoogleSearch;
use crate::serde_json;
use crate::serde_json::{json, Value};
use crate::universal::request::{
    JsonSchemaConfig, ResponseFormatConfig, ResponseFormatType, ToolChoiceConfig, ToolChoiceMode,
};
use crate::universal::response_format::normalize_response_schema_for_strict_target;
use crate::universal::tools::{
    BuiltinToolProvider, ToolAvailability, UniversalTool, UniversalToolCaller, UniversalToolType,
};
use crate::universal::{
    convert::TryFromLLM, message::ProviderOptions, AssistantContent, AssistantContentPart,
    CacheControl, Message, TextContentPart, ToolCallArguments, ToolContentPart,
    ToolResultContentPart, UserContent, UserContentPart,
};
use crate::util::media::parse_base64_data_url;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct AnthropicFileProviderOptionsView {
    anthropic_type: Option<String>,
    title: Option<String>,
    context: Option<String>,
}

fn anthropic_file_provider_options_view(
    provider_options: &Option<ProviderOptions>,
) -> Option<AnthropicFileProviderOptionsView> {
    provider_options.as_ref().and_then(|opts| {
        serde_json::from_value::<AnthropicFileProviderOptionsView>(Value::Object(
            opts.options.clone(),
        ))
        .ok()
    })
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct AnthropicToolUseProviderOptionsView {
    caller: Option<generated::Caller>,
}

fn anthropic_tool_use_provider_options_from_caller(
    caller: Option<generated::Caller>,
) -> Option<ProviderOptions> {
    let caller = caller?;
    let value = serde_json::to_value(&caller).ok()?;
    let mut options = serde_json::Map::new();
    options.insert("caller".into(), value);
    Some(ProviderOptions { options })
}

fn anthropic_allowed_callers_from_universal(
    callers: &Option<Vec<UniversalToolCaller>>,
) -> Result<Option<Vec<generated::AllowedCaller>>, ConvertError> {
    let Some(callers) = callers.as_ref() else {
        return Ok(None);
    };

    if callers.contains(&UniversalToolCaller::Programmatic) {
        return Err(ConvertError::UnsupportedToolType {
            tool_name: "allowed_callers".to_string(),
            tool_type: "programmatic caller restriction".to_string(),
            target_provider: ProviderFormat::Anthropic,
        });
    }

    let mapped_callers = callers
        .iter()
        .map(|caller| match caller {
            UniversalToolCaller::Direct => generated::AllowedCaller::Direct,
            UniversalToolCaller::CodeExecution20250825 => {
                generated::AllowedCaller::CodeExecution20250825
            }
            UniversalToolCaller::CodeExecution20260120 => {
                generated::AllowedCaller::CodeExecution20260120
            }
            UniversalToolCaller::CodeExecution20260521 => {
                generated::AllowedCaller::CodeExecution20260521
            }
            UniversalToolCaller::Programmatic => {
                unreachable!("programmatic callers are rejected above")
            }
        })
        .collect::<Vec<_>>();

    Ok((!mapped_callers.is_empty()).then_some(mapped_callers))
}

fn universal_allowed_callers_from_anthropic(
    callers: Option<Vec<generated::AllowedCaller>>,
) -> Option<Vec<UniversalToolCaller>> {
    callers
        .and_then(|callers| serde_json::to_value(callers).ok())
        .and_then(|value| serde_json::from_value(value).ok())
}

fn anthropic_tool_use_caller_from_provider_options(
    provider_options: &Option<ProviderOptions>,
) -> Option<generated::Caller> {
    provider_options
        .as_ref()
        .and_then(|opts| {
            serde_json::from_value::<AnthropicToolUseProviderOptionsView>(Value::Object(
                opts.options.clone(),
            ))
            .ok()
        })
        .and_then(|view| view.caller)
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct AnthropicTextProviderOptionsView {
    #[serde(default)]
    citations: Option<Value>,
}

fn anthropic_text_citations_from_provider_options<T>(
    provider_options: &Option<ProviderOptions>,
) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    provider_options
        .as_ref()
        .and_then(|opts| serde_json::to_value(&opts.options).ok())
        .and_then(|value| serde_json::from_value::<AnthropicTextProviderOptionsView>(value).ok())
        .and_then(|view| view.citations)
        .and_then(|citations| serde_json::from_value::<T>(citations).ok())
}

fn universal_cache_control_from_anthropic(
    cache_control: Option<generated::CacheControlEphemeral>,
) -> Option<CacheControl> {
    universal_cache_control_from_serde(cache_control)
}

fn universal_cache_control_from_serde<C>(cache_control: Option<C>) -> Option<CacheControl>
where
    C: serde::Serialize,
{
    cache_control
        .and_then(|cache_control| serde_json::to_value(cache_control).ok())
        .and_then(|value| serde_json::from_value(value).ok())
}

pub(crate) fn anthropic_cache_control_from_universal(
    cache_control: Option<CacheControl>,
) -> Option<generated::CacheControlEphemeral> {
    cache_control
        .and_then(|cache_control| serde_json::to_value(cache_control).ok())
        .and_then(|value| serde_json::from_value(value).ok())
}

fn infer_media_type_from_reference(reference: &str) -> Option<String> {
    let extension = reference
        .rsplit('/')
        .next()
        .and_then(|segment| segment.split('?').next())
        .and_then(|name| name.rsplit('.').next());

    match extension {
        Some("txt") => Some("text/plain".to_string()),
        Some("pdf") => Some("application/pdf".to_string()),
        Some("png") => Some("image/png".to_string()),
        Some("jpg") | Some("jpeg") => Some("image/jpeg".to_string()),
        Some("gif") => Some("image/gif".to_string()),
        Some("webp") => Some("image/webp".to_string()),
        _ => None,
    }
}

fn anthropic_text_provider_options<C, T>(
    _cache_control: Option<C>,
    citations: Option<T>,
) -> Result<Option<ProviderOptions>, ConvertError>
where
    C: serde::Serialize,
    T: serde::Serialize,
{
    let mut options = serde_json::Map::new();
    if let Some(citations) = citations {
        options.insert(
            "citations".into(),
            serde_json::to_value(citations).map_err(|e| ConvertError::JsonSerializationFailed {
                field: "citations".to_string(),
                error: e.to_string(),
            })?,
        );
    }

    Ok((!options.is_empty()).then_some(ProviderOptions { options }))
}

fn anthropic_user_text_part<C, T>(
    text: String,
    cache_control: Option<C>,
    citations: Option<T>,
) -> Result<UserContentPart, ConvertError>
where
    C: serde::Serialize,
    T: serde::Serialize,
{
    Ok(UserContentPart::Text(TextContentPart {
        text,
        encrypted_content: None,
        cache_control: universal_cache_control_from_serde(cache_control),
        provider_options: anthropic_text_provider_options(None::<C>, citations)?,
    }))
}

fn combine_anthropic_text_provider_options(
    inherited: Option<ProviderOptions>,
    own: Option<ProviderOptions>,
) -> Option<ProviderOptions> {
    match (inherited, own) {
        (None, None) => None,
        (Some(options), None) | (None, Some(options)) => Some(options),
        (Some(inherited), Some(own)) => {
            let mut options = inherited.options;
            options.extend(own.options);
            Some(ProviderOptions { options })
        }
    }
}

fn anthropic_mid_conv_system_parts(
    content: generated::InputContentBlockContent,
    parent_cache_control: Option<CacheControl>,
    parent_provider_options: Option<ProviderOptions>,
) -> Result<Vec<UserContentPart>, ConvertError> {
    match content {
        generated::InputContentBlockContent::PurpleString(text) => {
            Ok(vec![UserContentPart::Text(TextContentPart {
                text,
                encrypted_content: None,
                cache_control: parent_cache_control,
                provider_options: parent_provider_options,
            })])
        }
        generated::InputContentBlockContent::BlockArray(blocks) => {
            let mut parts = Vec::new();
            let mut inherited_parent_cache_control = parent_cache_control;
            let mut inherited_parent_options = parent_provider_options;
            for block in blocks {
                if let Some(text) = block.text {
                    let cache_control =
                        universal_cache_control_from_serde(block.cache_control.clone())
                            .or_else(|| inherited_parent_cache_control.take());
                    let provider_options = combine_anthropic_text_provider_options(
                        inherited_parent_options.take(),
                        anthropic_text_provider_options(
                            None::<generated::CacheControlEphemeral>,
                            block.citations,
                        )?,
                    );
                    parts.push(UserContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        cache_control,
                        provider_options,
                    }));
                }
                if let Some(content) = block.content {
                    for text_block in content {
                        let cache_control =
                            universal_cache_control_from_serde(text_block.cache_control.clone())
                                .or_else(|| inherited_parent_cache_control.take());
                        let provider_options = combine_anthropic_text_provider_options(
                            inherited_parent_options.take(),
                            anthropic_text_provider_options(
                                None::<generated::CacheControlEphemeral>,
                                text_block.citations,
                            )?,
                        );
                        parts.push(UserContentPart::Text(TextContentPart {
                            text: text_block.text,
                            encrypted_content: None,
                            cache_control,
                            provider_options,
                        }));
                    }
                }
            }
            Ok(parts)
        }
        generated::InputContentBlockContent::Request(_) => {
            Err(ConvertError::ContentConversionFailed {
                reason: "web search tool result errors cannot be used as Anthropic system content"
                    .to_string(),
            })
        }
    }
}

fn anthropic_system_message_content(
    content: generated::MessageContent,
) -> Result<UserContent, ConvertError> {
    match content {
        generated::MessageContent::PurpleString(text) => Ok(UserContent::String(text)),
        generated::MessageContent::InputContentBlockArray(blocks) => {
            let mut parts = Vec::new();
            for block in blocks {
                match block.input_content_block_type {
                    generated::InputContentBlockType::Text => {
                        if let Some(text) = block.text {
                            parts.push(anthropic_user_text_part(
                                text,
                                block.cache_control,
                                block.citations,
                            )?);
                        }
                    }
                    generated::InputContentBlockType::MidConvSystem => {
                        let parent_cache_control =
                            universal_cache_control_from_anthropic(block.cache_control);
                        let parent_provider_options = anthropic_text_provider_options(
                            None::<generated::CacheControlEphemeral>,
                            block.citations,
                        )?;
                        if let Some(content) = block.content {
                            parts.extend(anthropic_mid_conv_system_parts(
                                content,
                                parent_cache_control,
                                parent_provider_options,
                            )?);
                        }
                    }
                    other => {
                        return Err(ConvertError::ContentConversionFailed {
                            reason: format!(
                                "unsupported Anthropic system content block type: {other:?}"
                            ),
                        });
                    }
                }
            }

            if parts.is_empty() {
                Ok(UserContent::String(String::new()))
            } else {
                Ok(UserContent::Array(parts))
            }
        }
    }
}

fn normalize_anthropic_tool_schema_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(schema_type)) = map.get_mut("type") {
                *schema_type = schema_type.to_lowercase();
            }
            map.retain(|_, nested| !nested.is_null());
            for nested in map.values_mut() {
                normalize_anthropic_tool_schema_value(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_anthropic_tool_schema_value(item);
            }
        }
        _ => {}
    }
}

fn normalize_anthropic_tool_schema(
    tool_name: &str,
    schema: Option<Value>,
) -> Result<Value, ConvertError> {
    #[derive(serde::Deserialize)]
    struct ToolSchemaRootTypeView {
        #[serde(rename = "type")]
        schema_type: Option<String>,
    }

    let mut schema = schema.unwrap_or_else(|| json!({ "type": "object" }));
    normalize_anthropic_tool_schema_value(&mut schema);

    if !schema.is_object() {
        return Err(ConvertError::InvalidToolSchema {
            tool_name: tool_name.to_string(),
            reason: "tool schema must be a JSON object".to_string(),
        });
    }

    let schema_type = serde_json::from_value::<ToolSchemaRootTypeView>(schema.clone())
        .map_err(|e| ConvertError::JsonSerializationFailed {
            field: format!("tool schema '{}'", tool_name),
            error: e.to_string(),
        })?
        .schema_type
        .ok_or_else(|| ConvertError::InvalidToolSchema {
            tool_name: tool_name.to_string(),
            reason: "tool schema root type is required".to_string(),
        })?;

    if schema_type != "object" {
        return Err(ConvertError::InvalidToolSchema {
            tool_name: tool_name.to_string(),
            reason: format!("tool schema root type must be 'object', got '{schema_type}'"),
        });
    }

    Ok(schema)
}

/// Convert Anthropic's standalone `system` field into universal `UserContent`.
///
/// This is message-shape conversion logic, so it lives in `convert.rs` rather
/// than adapter orchestration code.
pub(crate) fn system_to_user_content(system: generated::System) -> UserContent {
    match system {
        generated::System::PurpleString(text) => UserContent::String(text),
        generated::System::RequestTextBlockArray(blocks) => UserContent::Array(
            blocks
                .into_iter()
                .map(|block| {
                    let mut options = serde_json::Map::new();
                    if let Some(citations) = block.citations {
                        if let Ok(v) = serde_json::to_value(citations) {
                            options.insert("citations".into(), v);
                        }
                    }

                    let provider_options = if options.is_empty() {
                        None
                    } else {
                        Some(ProviderOptions { options })
                    };

                    UserContentPart::Text(TextContentPart {
                        text: block.text,
                        encrypted_content: None,
                        cache_control: universal_cache_control_from_anthropic(block.cache_control),
                        provider_options,
                    })
                })
                .collect(),
        ),
    }
}

impl TryFromLLM<generated::InputMessage> for Message {
    type Error = ConvertError;

    fn try_from(input_msg: generated::InputMessage) -> Result<Self, Self::Error> {
        // Check if this is a user message that contains only tool results
        // If so, convert it to a Tool message instead
        if let generated::MessageRole::User = input_msg.role {
            if let generated::MessageContent::InputContentBlockArray(blocks) = &input_msg.content {
                // Check if all blocks are tool results
                let all_tool_results = blocks.iter().all(input_content_block_is_tool_result);

                let has_tool_results = blocks.iter().any(input_content_block_is_tool_result);

                // If we have tool results and no other content, convert to Tool message
                if has_tool_results && all_tool_results {
                    // Take ownership of the content for conversion
                    if let generated::MessageContent::InputContentBlockArray(owned_blocks) =
                        input_msg.content
                    {
                        let mut tool_content_parts = Vec::new();

                        for block in owned_blocks {
                            match block.input_content_block_type {
                                generated::InputContentBlockType::ToolResult => {
                                    if let (Some(tool_use_id), Some(content)) =
                                        (block.tool_use_id, block.content)
                                    {
                                        let output = match content {
                                            generated::InputContentBlockContent::PurpleString(
                                                s,
                                            ) => serde_json::from_str(&s)
                                                .unwrap_or(serde_json::Value::String(s)),
                                            generated::InputContentBlockContent::BlockArray(
                                                blocks,
                                            ) => serde_json::to_value(blocks).map_err(|e| {
                                                ConvertError::JsonSerializationFailed {
                                                    field: "BlockArray".to_string(),
                                                    error: e.to_string(),
                                                }
                                            })?,
                                            generated::InputContentBlockContent::Request(err) => {
                                                serde_json::to_value(err).map_err(|e| {
                                                    ConvertError::JsonSerializationFailed {
                                                        field: "RequestWebSearchToolResultError"
                                                            .to_string(),
                                                        error: e.to_string(),
                                                    }
                                                })?
                                            }
                                        };

                                        tool_content_parts.push(ToolContentPart::ToolResult(
                                            ToolResultContentPart {
                                                tool_call_id: tool_use_id,
                                                tool_name: String::new(), // Anthropic doesn't provide tool name in results
                                                output,
                                                custom_tool_call: None,
                                                caller: None,
                                                provider_options: None,
                                            },
                                        ));
                                    }
                                }
                                generated::InputContentBlockType::ToolSearchToolResult => {
                                    if let Some(tool_use_id) = block.tool_use_id {
                                        tool_content_parts.push(
                                            ToolContentPart::ToolDiscoveryResult(
                                                tool_discovery::result_from_input_content(
                                                    tool_use_id,
                                                    "tool_search".to_string(),
                                                    block.content,
                                                )?,
                                            ),
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }

                        return Ok(Message::Tool {
                            content: tool_content_parts,
                        });
                    }
                }
            }
        }

        match input_msg.role {
            generated::MessageRole::System => Ok(Message::System {
                content: anthropic_system_message_content(input_msg.content)?,
            }),
            generated::MessageRole::User => {
                let content = match input_msg.content {
                    generated::MessageContent::PurpleString(text) => UserContent::String(text),
                    generated::MessageContent::InputContentBlockArray(blocks) => {
                        let mut content_parts = Vec::new();

                        for block in blocks {
                            match block.input_content_block_type {
                                generated::InputContentBlockType::Text => {
                                    if let Some(text) = block.text {
                                        let provider_options = anthropic_text_provider_options(
                                            block.cache_control.clone(),
                                            block.citations,
                                        )?;
                                        content_parts.push(UserContentPart::Text(
                                            TextContentPart {
                                                text,
                                                encrypted_content: None,
                                                cache_control:
                                                    universal_cache_control_from_anthropic(
                                                        block.cache_control,
                                                    ),
                                                provider_options,
                                            },
                                        ));
                                    }
                                }
                                generated::InputContentBlockType::Image => {
                                    if let Some(source) = block.source {
                                        // Convert Anthropic image source to universal format
                                        match source {
                                            generated::SourceUnion::Source(purple_source) => {
                                                if let Some(data) = purple_source.data {
                                                    let media_type = purple_source.media_type.map(|mt| match mt {
                                                        generated::Base64ImageSourceMediaType::ImageJpeg => "image/jpeg".to_string(),
                                                        generated::Base64ImageSourceMediaType::ImagePng => "image/png".to_string(),
                                                        generated::Base64ImageSourceMediaType::ImageGif => "image/gif".to_string(),
                                                        generated::Base64ImageSourceMediaType::ImageWebp => "image/webp".to_string(),
                                                        generated::Base64ImageSourceMediaType::ApplicationPdf => "application/pdf".to_string(),
                                                        generated::Base64ImageSourceMediaType::TextPlain => "text/plain".to_string(),
                                                    });
                                                    content_parts.push(UserContentPart::Image {
                                                        image: serde_json::Value::String(data),
                                                        media_type,
                                                        provider_options: None,
                                                    });
                                                }
                                            }
                                            _ => {
                                                // Skip other source types for now
                                                continue;
                                            }
                                        }
                                    }
                                }
                                generated::InputContentBlockType::ToolResult => {
                                    // Skip tool results for now - should be handled properly
                                    continue;
                                }
                                generated::InputContentBlockType::Document => {
                                    // Map document to File with provider_options for title/context
                                    if let Some(source) = &block.source {
                                        let mut opts = serde_json::Map::new();
                                        // Store document-specific fields in provider_options
                                        opts.insert(
                                            "anthropic_type".into(),
                                            serde_json::Value::String("document".to_string()),
                                        );
                                        if let Some(title) = &block.title {
                                            opts.insert(
                                                "title".into(),
                                                serde_json::Value::String(title.clone()),
                                            );
                                        }
                                        if let Some(context) = &block.context {
                                            opts.insert(
                                                "context".into(),
                                                serde_json::Value::String(context.clone()),
                                            );
                                        }

                                        // Extract data and media_type from source
                                        match source {
                                            generated::SourceUnion::Source(s) => {
                                                let data = s.data.clone().or_else(|| s.url.clone());
                                                let media_type = s
                                                    .media_type
                                                    .as_ref()
                                                    .map(|mt| match mt {
                                                        generated::Base64ImageSourceMediaType::ImageJpeg => {
                                                            "image/jpeg".to_string()
                                                        }
                                                        generated::Base64ImageSourceMediaType::ImagePng => {
                                                            "image/png".to_string()
                                                        }
                                                        generated::Base64ImageSourceMediaType::ImageGif => {
                                                            "image/gif".to_string()
                                                        }
                                                        generated::Base64ImageSourceMediaType::ImageWebp => {
                                                            "image/webp".to_string()
                                                        }
                                                        generated::Base64ImageSourceMediaType::ApplicationPdf => {
                                                            "application/pdf".to_string()
                                                        }
                                                        generated::Base64ImageSourceMediaType::TextPlain => {
                                                            "text/plain".to_string()
                                                        }
                                                    })
                                                    .or_else(|| {
                                                        data.as_deref()
                                                            .and_then(infer_media_type_from_reference)
                                                    });
                                                content_parts.push(UserContentPart::File {
                                                    data: data
                                                        .map(serde_json::Value::String)
                                                        .unwrap_or(serde_json::Value::Null),
                                                    filename: None,
                                                    media_type: media_type.unwrap_or_else(|| {
                                                        "text/plain".to_string()
                                                    }),
                                                    provider_options: Some(ProviderOptions {
                                                        options: opts,
                                                    }),
                                                });
                                            }
                                            _ => {
                                                // Skip other source types
                                                continue;
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // Skip other types for now
                                    continue;
                                }
                            }
                        }

                        if content_parts.is_empty() {
                            UserContent::String(String::new())
                        } else {
                            UserContent::Array(content_parts)
                        }
                    }
                };

                Ok(Message::User { content })
            }
            generated::MessageRole::Assistant => {
                let content = match input_msg.content {
                    generated::MessageContent::PurpleString(text) => AssistantContent::String(text),
                    generated::MessageContent::InputContentBlockArray(blocks) => {
                        let mut content_parts = Vec::new();

                        for block in blocks {
                            match block.input_content_block_type {
                                generated::InputContentBlockType::Text => {
                                    if let Some(text) = block.text {
                                        let provider_options = anthropic_text_provider_options(
                                            block.cache_control.clone(),
                                            block.citations,
                                        )?;

                                        content_parts.push(AssistantContentPart::Text(
                                            TextContentPart {
                                                text,
                                                encrypted_content: None,
                                                cache_control:
                                                    universal_cache_control_from_anthropic(
                                                        block.cache_control,
                                                    ),
                                                provider_options,
                                            },
                                        ));
                                    }
                                }
                                generated::InputContentBlockType::Thinking => {
                                    if let Some(thinking) = block.thinking {
                                        content_parts.push(AssistantContentPart::Reasoning {
                                            text: thinking,
                                            // Preserve the signature in encrypted_content for roundtrip
                                            encrypted_content: block.signature.clone(),
                                        });
                                    }
                                }
                                generated::InputContentBlockType::ToolUse => {
                                    if let (Some(id), Some(name)) = (&block.id, &block.name) {
                                        // The input field type is wrong in generated code, use serde_json for now
                                        let input = if let Some(input_map) = &block.input {
                                            // Convert HashMap to JSON value
                                            serde_json::to_value(input_map)
                                                .unwrap_or(serde_json::Value::Null)
                                        } else {
                                            serde_json::Value::Null
                                        };

                                        let provider_options =
                                            anthropic_tool_use_provider_options_from_caller(
                                                block.caller.clone(),
                                            );

                                        content_parts.push(AssistantContentPart::ToolCall {
                                            tool_call_id: id.clone(),
                                            tool_name: name.clone(),
                                            arguments: serde_json::to_string(&input)
                                                .unwrap_or_else(|_| "{}".to_string())
                                                .into(),
                                            encrypted_content: None,
                                            provider_options,
                                            status: None,
                                            caller: None,
                                            provider_executed: None,
                                        });
                                    }
                                }
                                generated::InputContentBlockType::ServerToolUse => {
                                    // Server-executed tool use (web search, etc.)
                                    if let (Some(id), Some(name)) = (&block.id, &block.name) {
                                        if tool_discovery::is_tool_search_name(name) {
                                            content_parts.push(
                                                AssistantContentPart::ToolDiscoveryCall {
                                                    tool_call_id: id.clone(),
                                                    discovery_tool_name: name.clone(),
                                                    query: tool_discovery::query(&block.input)?,
                                                    arguments: tool_discovery::arguments(
                                                        block.input.clone(),
                                                    ),
                                                    status: None,
                                                    execution: None,
                                                    provider_options: None,
                                                },
                                            );
                                        } else {
                                            let input = if let Some(input_map) = &block.input {
                                                serde_json::to_value(input_map)
                                                    .unwrap_or(serde_json::Value::Null)
                                            } else {
                                                serde_json::Value::Null
                                            };

                                            let provider_options =
                                                anthropic_tool_use_provider_options_from_caller(
                                                    block.caller.clone(),
                                                );

                                            content_parts.push(AssistantContentPart::ToolCall {
                                                tool_call_id: id.clone(),
                                                tool_name: name.clone(),
                                                arguments: serde_json::to_string(&input)
                                                    .unwrap_or_else(|_| "{}".to_string())
                                                    .into(),
                                                encrypted_content: None,
                                                provider_options,
                                                status: None,
                                                caller: None,
                                                provider_executed: Some(true), // Mark as server-executed
                                            });
                                        }
                                    }
                                }
                                generated::InputContentBlockType::WebSearchToolResult => {
                                    // Web search tool result - convert to ToolResult with marker
                                    if let Some(id) = &block.tool_use_id {
                                        let mut output = serde_json::Map::new();
                                        output.insert(
                                            "anthropic_type".into(),
                                            serde_json::Value::String(
                                                "web_search_tool_result".to_string(),
                                            ),
                                        );
                                        if let Some(content) = &block.content {
                                            if let Ok(v) = serde_json::to_value(content) {
                                                output.insert("content".into(), v);
                                            }
                                        }

                                        content_parts.push(AssistantContentPart::ToolResult {
                                            tool_call_id: id.clone(),
                                            tool_name: "web_search".to_string(), // Server-executed web search tool
                                            output: serde_json::Value::Object(output),
                                            caller: None,
                                            provider_options: None,
                                        });
                                    }
                                }
                                _ => {
                                    // Skip other types for now
                                    continue;
                                }
                            }
                        }

                        if content_parts.is_empty() {
                            AssistantContent::Array(vec![AssistantContentPart::Text(
                                TextContentPart {
                                    text: String::new(),
                                    encrypted_content: None,
                                    cache_control: None,
                                    provider_options: None,
                                },
                            )])
                        } else {
                            AssistantContent::Array(content_parts)
                        }
                    }
                };

                Ok(Message::Assistant { content, id: None })
            }
        }
    }
}

// Vec conversion is handled by the blanket implementation in universal/convert.rs

impl TryFromLLM<Message> for generated::InputMessage {
    type Error = ConvertError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            Message::User { content } => {
                let anthropic_content = match content {
                    UserContent::String(text) => generated::MessageContent::PurpleString(text),
                    UserContent::Array(parts) => {
                        let blocks = parts
                            .into_iter()
                            .filter_map(|part| match part {
                                UserContentPart::Text(text_part) => {
                                    let cache_control = anthropic_cache_control_from_universal(
                                        text_part.cache_control,
                                    );
                                    Some(generated::InputContentBlock {
                                        cache_control,
                                        citations: None,
                                        text: Some(text_part.text),
                                        input_content_block_type:
                                            generated::InputContentBlockType::Text,
                                        source: None,
                                        context: None,
                                        title: None,
                                        content: None,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        caller: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        is_error: None,
                                        tool_use_id: None,
                                        file_id: None,
                                    })
                                },
                                UserContentPart::Image {
                                    image, media_type, ..
                                } => {
                                    // Convert universal image back to Anthropic format
                                    let data = match image {
                                        serde_json::Value::String(s) => Some(s),
                                        _ => None,
                                    };

                                    if let Some(image_data) = data {
                                        // Check if this is a URL - use URL source type (no media_type required)
                                        let is_url = image_data.starts_with("http://")
                                            || image_data.starts_with("https://");

                                        // Handle text content types - decode to text block instead of document
                                        // Anthropic's Document block only accepts PDFs, not text files
                                        if !is_url {
                                            if let Some(mt) = &media_type {
                                                if mt.starts_with("text/") {
                                                    // Parse data URL and decode base64 to text
                                                    if let Some(media_block) = parse_base64_data_url(&image_data) {
                                                        use base64::Engine;
                                                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&media_block.data) {
                                                            if let Ok(text) = String::from_utf8(bytes) {
                                                                return Some(generated::InputContentBlock {
                                                                    cache_control: None,
                                                                    citations: None,
                                                                    text: Some(text),
                                                                    input_content_block_type: generated::InputContentBlockType::Text,
                                                                    source: None,
                                                                    context: None,
                                                                    title: None,
                                                                    content: None,
                                                                    signature: None,
                                                                    thinking: None,
                                                                    data: None,
                                                                    caller: None,
                                                                    id: None,
                                                                    input: None,
                                                                    name: None,
                                                                    is_error: None,
                                                                    tool_use_id: None,
                                                                    file_id: None,
                                                                });
                                                            }
                                                        }
                                                    }
                                                    // Skip if can't decode text
                                                    return None;
                                                }
                                            }
                                        }

                                        let (source_type, source_url, source_data, anthropic_media_type) = if is_url {
                                            (
                                                generated::Base64ImageSourceType::Url,
                                                Some(image_data),
                                                None,
                                                None,
                                            )
                                        } else {
                                            // Base64 data - parse media_type (images and PDFs only)
                                            let anthropic_media_type =
                                                media_type.as_ref().and_then(|mt| match mt.as_str() {
                                                    "image/jpeg" => {
                                                        Some(generated::Base64ImageSourceMediaType::ImageJpeg)
                                                    }
                                                    "image/png" => {
                                                        Some(generated::Base64ImageSourceMediaType::ImagePng)
                                                    }
                                                    "image/gif" => {
                                                        Some(generated::Base64ImageSourceMediaType::ImageGif)
                                                    }
                                                    "image/webp" => {
                                                        Some(generated::Base64ImageSourceMediaType::ImageWebp)
                                                    }
                                                    "application/pdf" => {
                                                        Some(generated::Base64ImageSourceMediaType::ApplicationPdf)
                                                    }
                                                    // Text types are handled above, shouldn't reach here
                                                    _ => None,
                                                });
                                            (
                                                generated::Base64ImageSourceType::Base64,
                                                None,
                                                Some(image_data),
                                                anthropic_media_type,
                                            )
                                        };

                                        // Block type: only PDF uses Document, everything else is Image
                                        let block_type = match anthropic_media_type {
                                            Some(generated::Base64ImageSourceMediaType::ApplicationPdf) => {
                                                generated::InputContentBlockType::Document
                                            }
                                            _ => generated::InputContentBlockType::Image,
                                        };

                                        Some(generated::InputContentBlock {
                                            cache_control: None,
                                            citations: None,
                                            text: None,
                                            input_content_block_type: block_type,
                                            source: Some(generated::SourceUnion::Source(
                                                generated::Source {
                                                    data: source_data,
                                                    media_type: anthropic_media_type,
                                                    source_type,
                                                    url: source_url,
                                                    content: None,
                                                },
                                            )),
                                            context: None,
                                            title: None,
                                            content: None,
                                            signature: None,
                                            thinking: None,
                                            data: None,
                                            caller: None,
                                            id: None,
                                            input: None,
                                            name: None,
                                            is_error: None,
                                            tool_use_id: None,
                                            file_id: None,
                                        })
                                    } else {
                                        None
                                    }
                                }
                                UserContentPart::File {
                                    data,
                                    media_type,
                                    provider_options,
                                    ..
                                } => {
                                    let file_provider_options =
                                        anthropic_file_provider_options_view(&provider_options);
                                    let title = file_provider_options
                                        .as_ref()
                                        .and_then(|opts| opts.title.clone());

                                    let context = file_provider_options
                                        .as_ref()
                                        .and_then(|opts| opts.context.clone());

                                    let anthropic_media_type = match media_type.as_ref() {
                                        "image/jpeg" => {
                                            Some(generated::Base64ImageSourceMediaType::ImageJpeg)
                                        }
                                        "image/png" => Some(generated::Base64ImageSourceMediaType::ImagePng),
                                        "image/gif" => Some(generated::Base64ImageSourceMediaType::ImageGif),
                                        "image/webp" => {
                                            Some(generated::Base64ImageSourceMediaType::ImageWebp)
                                        }
                                        "application/pdf" => {
                                            Some(generated::Base64ImageSourceMediaType::ApplicationPdf)
                                        }
                                        "text/plain" => Some(generated::Base64ImageSourceMediaType::TextPlain),
                                        _ => Some(generated::Base64ImageSourceMediaType::TextPlain),
                                    };

                                    let data_str = match data {
                                        serde_json::Value::String(s) => Some(s),
                                        _ => None,
                                    }?;

                                    let is_url = data_str.starts_with("http://")
                                        || data_str.starts_with("https://");

                                    Some(generated::InputContentBlock {
                                        cache_control: None,
                                        citations: None,
                                        text: None,
                                        input_content_block_type:
                                            generated::InputContentBlockType::Document,
                                        source: Some(generated::SourceUnion::Source(
                                            generated::Source {
                                                data: if is_url {
                                                    None
                                                } else {
                                                    Some(data_str.clone())
                                                },
                                                media_type: if is_url {
                                                    None
                                                } else {
                                                    anthropic_media_type
                                                },
                                                source_type: if is_url {
                                                    generated::Base64ImageSourceType::Url
                                                } else {
                                                    generated::Base64ImageSourceType::Text
                                                },
                                                url: if is_url { Some(data_str) } else { None },
                                                content: None,
                                            },
                                        )),
                                        context,
                                        title,
                                        content: None,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        caller: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        is_error: None,
                                        tool_use_id: None,
                                        file_id: None,
                                    })
                                },
                            })
                            .collect();
                        generated::MessageContent::InputContentBlockArray(blocks)
                    }
                };

                Ok(generated::InputMessage {
                    content: anthropic_content,
                    role: generated::MessageRole::User,
                })
            }
            Message::Assistant { content, .. } => {
                let content = match content {
                    AssistantContent::String(text) => generated::MessageContent::PurpleString(text),
                    AssistantContent::Array(parts) => {
                        let mut blocks = Vec::new();
                        for part in parts {
                            let block = match part {
                                AssistantContentPart::Text(text_part) => {
                                    let cache_control = anthropic_cache_control_from_universal(
                                        text_part.cache_control.clone(),
                                    );
                                    let citations = anthropic_text_citations_from_provider_options(
                                        &text_part.provider_options,
                                    );

                                    Some(generated::InputContentBlock {
                                        cache_control,
                                        citations,
                                        text: Some(text_part.text),
                                        input_content_block_type:
                                            generated::InputContentBlockType::Text,
                                        source: None,
                                        context: None,
                                        title: None,
                                        content: None,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        caller: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        is_error: None,
                                        tool_use_id: None,
                                        file_id: None,
                                    })
                                },
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content,
                                } => Some(generated::InputContentBlock {
                                    cache_control: None,
                                    citations: None,
                                    text: None,
                                    input_content_block_type:
                                        generated::InputContentBlockType::Thinking,
                                    source: None,
                                    context: None,
                                    title: None,
                                    content: None,
                                    // Restore signature from encrypted_content
                                    signature: encrypted_content,
                                    thinking: Some(text),
                                    data: None,
                                    caller: None,
                                    id: None,
                                    input: None,
                                    name: None,
                                    is_error: None,
                                    tool_use_id: None,
                                    file_id: None,
                                }),
                                AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                provider_executed,
                                provider_options,
                                ..
                            } => {
                                // Convert ToolCallArguments to serde_json::Map
                                let input_map = match &arguments {
                                    ToolCallArguments::Valid(map) => Some(map.clone()),
                                    ToolCallArguments::Invalid(_) | ToolCallArguments::Custom(_) => None,
                                };

                                // Use ServerToolUse for provider-executed tools
                                let block_type = if provider_executed == Some(true) {
                                    generated::InputContentBlockType::ServerToolUse
                                } else {
                                    generated::InputContentBlockType::ToolUse
                                };

                                let caller = anthropic_tool_use_caller_from_provider_options(
                                    &provider_options,
                                );

                                Some(generated::InputContentBlock {
                                    cache_control: None,
                                    citations: None,
                                    text: None,
                                    input_content_block_type: block_type,
                                    source: None,
                                    context: None,
                                    title: None,
                                    content: None,
                                    signature: None,
                                    thinking: None,
                                    data: None,
                                    caller,
                                    id: Some(tool_call_id.clone()),
                                    input: input_map,
                                    name: Some(tool_name.clone()),
                                    is_error: None,
                                    tool_use_id: None,
                                    file_id: None,
                                })
                                },
                                AssistantContentPart::ToolDiscoveryCall {
                                tool_call_id,
                                discovery_tool_name,
                                query,
                                arguments,
                                ..
                            } => Some(generated::InputContentBlock {
                                cache_control: None,
                                citations: None,
                                text: None,
                                input_content_block_type:
                                    generated::InputContentBlockType::ServerToolUse,
                                source: None,
                                context: None,
                                title: None,
                                content: None,
                                signature: None,
                                thinking: None,
                                data: None,
                                caller: None,
                                id: Some(tool_discovery::tool_search_call_id(&tool_call_id)),
                                input: tool_discovery::input_map(
                                    arguments.clone(),
                                    query.clone(),
                                )?,
                                name: Some(tool_discovery::tool_search_name(
                                    &discovery_tool_name,
                                )),
                                is_error: None,
                                tool_use_id: None,
                                file_id: None,
                                }),
                                AssistantContentPart::ToolResult {
                                tool_call_id,
                                output,
                                ..
                            } => {
                                // Check if this was a web_search_tool_result
                                let is_web_search_result = output.as_object()
                                    .and_then(|obj| obj.get("anthropic_type"))
                                    .and_then(|v| v.as_str())
                                    == Some("web_search_tool_result");

                                if is_web_search_result {
                                    // Restore WebSearchToolResult block
                                    let content = output.as_object()
                                        .and_then(|obj| obj.get("content"))
                                        .and_then(|v| serde_json::from_value::<generated::InputContentBlockContent>(v.clone()).ok());

                                    Some(generated::InputContentBlock {
                                        cache_control: None,
                                        citations: None,
                                        text: None,
                                        input_content_block_type:
                                            generated::InputContentBlockType::WebSearchToolResult,
                                        source: None,
                                        context: None,
                                        title: None,
                                        content,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        caller: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        is_error: None,
                                        tool_use_id: Some(tool_call_id.clone()),
                                        file_id: None,
                                    })
                                } else {
                                    None // Skip other tool results in assistant messages
                                }
                                },
                                _ => None, // Skip other types for now
                            };
                            if let Some(block) = block {
                                blocks.push(block);
                            }
                        }
                        generated::MessageContent::InputContentBlockArray(blocks)
                    }
                };

                Ok(generated::InputMessage {
                    content,
                    role: generated::MessageRole::Assistant,
                })
            }
            Message::Tool { content } => {
                // Convert tool results back to user message with tool_result blocks
                let mut blocks = Vec::new();
                for part in content {
                    match part {
                        ToolContentPart::ToolResult(tool_result) => {
                            let content = match &tool_result.output {
                                serde_json::Value::String(s) => {
                                    Some(generated::InputContentBlockContent::PurpleString(s.clone()))
                                }
                                other => Some(generated::InputContentBlockContent::PurpleString(
                                    serde_json::to_string(other)
                                        .map_err(|e| ConvertError::JsonSerializationFailed {
                                            field: "tool_result_output".to_string(),
                                            error: e.to_string(),
                                        })?,
                                )),
                            };

                            blocks.push(generated::InputContentBlock {
                                cache_control: None,
                                citations: None,
                                text: None,
                                input_content_block_type:
                                    generated::InputContentBlockType::ToolResult,
                                source: None,
                                context: None,
                                title: None,
                                content,
                                signature: None,
                                thinking: None,
                                data: None,
                                caller: None,
                                id: None,
                                input: None,
                                name: None,
                                is_error: None,
                                tool_use_id: Some(tool_result.tool_call_id),
                                file_id: None,
                            });
                        }
                        ToolContentPart::ToolDiscoveryResult(discovery_result) => {
                            blocks.push(generated::InputContentBlock {
                                cache_control: None,
                                citations: None,
                                text: None,
                                input_content_block_type:
                                    generated::InputContentBlockType::ToolSearchToolResult,
                                source: None,
                                context: None,
                                title: None,
                                content: Some(tool_discovery::input_result_content(
                                    discovery_result.tools,
                                )?),
                                signature: None,
                                thinking: None,
                                data: None,
                                caller: None,
                                id: None,
                                input: None,
                                name: None,
                                is_error: None,
                                tool_use_id: Some(tool_discovery::tool_search_call_id(
                                    &discovery_result.tool_call_id,
                                )),
                                file_id: None,
                            });
                        }
                    }
                }

                Ok(generated::InputMessage {
                    content: generated::MessageContent::InputContentBlockArray(blocks),
                    role: generated::MessageRole::User,
                })
            }
            Message::System { .. } | Message::Developer { .. } => {
                Err(ConvertError::UnsupportedInputType {
                    type_info: "Non-leading system/developer messages are not supported in Anthropic InputMessage; use the top-level system parameter for leading instructions".to_string(),
                })
            }
            Message::AdditionalTools { .. } => Err(ConvertError::UnsupportedMapping {
                from: "Message::AdditionalTools".to_string(),
                to: "Anthropic InputMessage",
            }),
        }
    }
}

fn input_content_block_is_tool_result(block: &generated::InputContentBlock) -> bool {
    matches!(
        block.input_content_block_type,
        generated::InputContentBlockType::ToolResult
            | generated::InputContentBlockType::ToolSearchToolResult
    )
}

fn mixed_user_tool_result_groups(
    input_msg: generated::InputMessage,
) -> Option<Vec<generated::InputMessage>> {
    if !matches!(input_msg.role, generated::MessageRole::User) {
        return None;
    }

    let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content else {
        return None;
    };

    let has_tool_results = blocks.iter().any(input_content_block_is_tool_result);
    if !has_tool_results || blocks.iter().all(input_content_block_is_tool_result) {
        return None;
    }

    let mut groups = Vec::new();
    let mut current_blocks = Vec::new();
    let mut current_is_tool_result = None;

    for block in blocks {
        let is_tool_result = input_content_block_is_tool_result(&block);
        if current_is_tool_result.is_some_and(|current| current != is_tool_result) {
            groups.push(generated::InputMessage {
                role: generated::MessageRole::User,
                content: generated::MessageContent::InputContentBlockArray(current_blocks),
            });
            current_blocks = Vec::new();
        }

        current_is_tool_result = Some(is_tool_result);
        current_blocks.push(block);
    }

    if !current_blocks.is_empty() {
        groups.push(generated::InputMessage {
            role: generated::MessageRole::User,
            content: generated::MessageContent::InputContentBlockArray(current_blocks),
        });
    }

    Some(groups)
}

fn text_input_content_block(text: String) -> generated::InputContentBlock {
    generated::InputContentBlock {
        cache_control: None,
        citations: None,
        text: Some(text),
        input_content_block_type: generated::InputContentBlockType::Text,
        source: None,
        context: None,
        title: None,
        content: None,
        signature: None,
        thinking: None,
        data: None,
        caller: None,
        id: None,
        input: None,
        name: None,
        is_error: None,
        tool_use_id: None,
        file_id: None,
    }
}

fn input_message_content_blocks(
    content: generated::MessageContent,
) -> Vec<generated::InputContentBlock> {
    match content {
        generated::MessageContent::PurpleString(text) => vec![text_input_content_block(text)],
        generated::MessageContent::InputContentBlockArray(blocks) => blocks,
    }
}

fn try_merge_adjacent_user_tool_result_message(
    previous: &mut generated::InputMessage,
    current: generated::InputMessage,
) -> Result<Option<generated::InputMessage>, ConvertError> {
    if previous.role != generated::MessageRole::User || current.role != generated::MessageRole::User
    {
        return Ok(Some(current));
    }

    let previous_is_pure_tool_result = match &previous.content {
        generated::MessageContent::PurpleString(_) => false,
        generated::MessageContent::InputContentBlockArray(blocks) => {
            !blocks.is_empty() && blocks.iter().all(input_content_block_is_tool_result)
        }
    };
    let current_is_pure_non_tool_result = match &current.content {
        generated::MessageContent::PurpleString(_) => true,
        generated::MessageContent::InputContentBlockArray(blocks) => {
            !blocks.is_empty()
                && blocks
                    .iter()
                    .all(|block| !input_content_block_is_tool_result(block))
        }
    };

    // Anthropic requires tool_result blocks to come before text in a mixed user
    // message, so only rejoin Tool -> User. User -> Tool must stay split.
    if !(previous_is_pure_tool_result && current_is_pure_non_tool_result) {
        return Ok(Some(current));
    }

    let previous_content = std::mem::replace(
        &mut previous.content,
        generated::MessageContent::PurpleString(String::new()),
    );
    let mut blocks = input_message_content_blocks(previous_content);
    blocks.extend(input_message_content_blocks(current.content));
    previous.content = generated::MessageContent::InputContentBlockArray(blocks);
    Ok(None)
}

fn input_message_has_tool_search_result(message: &generated::InputMessage) -> bool {
    match &message.content {
        generated::MessageContent::PurpleString(_) => false,
        generated::MessageContent::InputContentBlockArray(blocks) => blocks.iter().any(|block| {
            block.input_content_block_type == generated::InputContentBlockType::ToolSearchToolResult
        }),
    }
}

fn input_message_is_pure_tool_search_result(message: &generated::InputMessage) -> bool {
    match &message.content {
        generated::MessageContent::PurpleString(_) => false,
        generated::MessageContent::InputContentBlockArray(blocks) => {
            !blocks.is_empty()
                && blocks.iter().all(|block| {
                    block.input_content_block_type
                        == generated::InputContentBlockType::ToolSearchToolResult
                })
        }
    }
}

fn input_message_tool_search_result_ids(message: &generated::InputMessage) -> Option<Vec<String>> {
    if !input_message_is_pure_tool_search_result(message) {
        return None;
    }

    match &message.content {
        generated::MessageContent::PurpleString(_) => None,
        generated::MessageContent::InputContentBlockArray(blocks) => Some(
            blocks
                .iter()
                .filter_map(|block| block.tool_use_id.clone())
                .collect(),
        ),
    }
}

fn input_message_tool_search_call_ids(message: &generated::InputMessage) -> Vec<String> {
    match &message.content {
        generated::MessageContent::PurpleString(_) => Vec::new(),
        generated::MessageContent::InputContentBlockArray(blocks) => blocks
            .iter()
            .filter_map(|block| {
                let is_tool_search_call = block.input_content_block_type
                    == generated::InputContentBlockType::ServerToolUse
                    && block
                        .name
                        .as_deref()
                        .map(tool_discovery::is_tool_search_name)
                        .unwrap_or(false);
                if is_tool_search_call {
                    block.id.clone()
                } else {
                    None
                }
            })
            .collect(),
    }
}

fn unpaired_tool_discovery_result_error() -> ConvertError {
    ConvertError::UnsupportedMapping {
        from: "unpaired ToolDiscoveryResult".to_string(),
        to: "Anthropic InputMessage",
    }
}

fn try_merge_adjacent_assistant_tool_discovery_message(
    previous: &mut generated::InputMessage,
    current: generated::InputMessage,
) -> Result<Option<generated::InputMessage>, ConvertError> {
    let tool_search_result_ids = input_message_tool_search_result_ids(&current);

    if let Some(result_ids) = &tool_search_result_ids {
        if previous.role != generated::MessageRole::Assistant {
            return Err(unpaired_tool_discovery_result_error());
        }

        let call_ids = input_message_tool_search_call_ids(previous);
        if result_ids.is_empty()
            || !result_ids
                .iter()
                .all(|result_id| call_ids.iter().any(|call_id| call_id == result_id))
        {
            return Err(unpaired_tool_discovery_result_error());
        }
    }

    if previous.role != generated::MessageRole::Assistant {
        return Ok(Some(current));
    }

    let should_merge_tool_result =
        current.role == generated::MessageRole::User && tool_search_result_ids.is_some();
    let should_merge_assistant_continuation = current.role == generated::MessageRole::Assistant
        && input_message_has_tool_search_result(previous);

    if !(should_merge_tool_result || should_merge_assistant_continuation) {
        return Ok(Some(current));
    }

    let previous_content = std::mem::replace(
        &mut previous.content,
        generated::MessageContent::PurpleString(String::new()),
    );
    let mut blocks = input_message_content_blocks(previous_content);
    blocks.extend(input_message_content_blocks(current.content));
    previous.content = generated::MessageContent::InputContentBlockArray(blocks);
    Ok(None)
}

fn split_assistant_tool_discovery_result_message(
    input_message: generated::InputMessage,
) -> Result<Option<Vec<Message>>, ConvertError> {
    if input_message.role != generated::MessageRole::Assistant {
        return Ok(None);
    }

    let generated::MessageContent::InputContentBlockArray(blocks) = input_message.content else {
        return Ok(None);
    };

    if !blocks.iter().any(|block| {
        block.input_content_block_type == generated::InputContentBlockType::ToolSearchToolResult
    }) {
        return Ok(None);
    }

    let mut messages = Vec::new();
    let mut assistant_blocks = Vec::new();

    for block in blocks {
        if block.input_content_block_type == generated::InputContentBlockType::ToolSearchToolResult
        {
            if !assistant_blocks.is_empty() {
                messages.push(<Message as TryFromLLM<generated::InputMessage>>::try_from(
                    generated::InputMessage {
                        role: generated::MessageRole::Assistant,
                        content: generated::MessageContent::InputContentBlockArray(std::mem::take(
                            &mut assistant_blocks,
                        )),
                    },
                )?);
            }

            if let Some(tool_use_id) = block.tool_use_id {
                let discovery_result = tool_discovery::result_from_input_content(
                    tool_use_id,
                    "tool_search".to_string(),
                    block.content,
                )?;
                messages.push(Message::Tool {
                    content: vec![ToolContentPart::ToolDiscoveryResult(discovery_result)],
                });
            }
        } else {
            assistant_blocks.push(block);
        }
    }

    if !assistant_blocks.is_empty() {
        messages.push(<Message as TryFromLLM<generated::InputMessage>>::try_from(
            generated::InputMessage {
                role: generated::MessageRole::Assistant,
                content: generated::MessageContent::InputContentBlockArray(assistant_blocks),
            },
        )?);
    }

    Ok(Some(messages))
}

pub(crate) fn anthropic_input_messages_to_universal_messages(
    input_messages: Vec<generated::InputMessage>,
) -> Result<Vec<Message>, ConvertError> {
    // The blanket Vec TryFromLLM conversion is one input message to one universal
    // message. Anthropic user messages can mix text and tool_result blocks in a
    // single content array, which needs to become ordered user/tool messages so
    // OpenAI targets receive the required tool output after each tool call.
    let mut messages = Vec::new();

    for input_message in input_messages {
        if let Some(groups) = split_assistant_tool_discovery_result_message(input_message.clone())?
        {
            messages.extend(groups);
        } else if let Some(groups) = mixed_user_tool_result_groups(input_message.clone()) {
            for group in groups {
                messages.push(<Message as TryFromLLM<generated::InputMessage>>::try_from(
                    group,
                )?);
            }
        } else {
            messages.push(<Message as TryFromLLM<generated::InputMessage>>::try_from(
                input_message,
            )?);
        }
    }

    Ok(messages)
}

pub(crate) fn universal_messages_to_anthropic_input_messages(
    messages: Vec<Message>,
) -> Result<Vec<generated::InputMessage>, ConvertError> {
    let mut input_messages = Vec::new();

    for message in messages {
        for message in split_mixed_tool_discovery_message(message) {
            let mut input_message =
                <generated::InputMessage as TryFromLLM<Message>>::try_from(message)?;
            if let Some(previous) = input_messages.last_mut() {
                if let Some(unmerged) =
                    try_merge_adjacent_assistant_tool_discovery_message(previous, input_message)?
                {
                    input_message = unmerged;
                } else {
                    continue;
                }

                if let Some(unmerged) =
                    try_merge_adjacent_user_tool_result_message(previous, input_message)?
                {
                    input_messages.push(unmerged);
                }
            } else {
                if input_message_is_pure_tool_search_result(&input_message) {
                    return Err(unpaired_tool_discovery_result_error());
                }
                input_messages.push(input_message);
            }
        }
    }

    Ok(input_messages)
}

fn split_mixed_tool_discovery_message(message: Message) -> Vec<Message> {
    let Message::Tool { content } = message else {
        return vec![message];
    };

    let mut tool_results = Vec::new();
    let mut discovery_results = Vec::new();

    for part in content {
        match part {
            ToolContentPart::ToolResult(_) => tool_results.push(part),
            ToolContentPart::ToolDiscoveryResult(_) => discovery_results.push(part),
        }
    }

    if tool_results.is_empty() || discovery_results.is_empty() {
        return vec![Message::Tool {
            content: tool_results.into_iter().chain(discovery_results).collect(),
        }];
    }

    vec![
        Message::Tool {
            content: discovery_results,
        },
        Message::Tool {
            content: tool_results,
        },
    ]
}

pub(crate) fn try_parse_content_blocks_for_import(
    data: &serde_json::Value,
) -> Option<Vec<Message>> {
    let blocks = try_parse_vec_or_single::<generated::ContentBlock>(data)?;
    try_convert_non_empty(blocks)
}

fn try_messages_from_anthropic_request(request: CreateMessageParams) -> Option<Vec<Message>> {
    let mut messages = Vec::new();

    if let Some(system) = request.system {
        messages.push(Message::System {
            content: system_to_user_content(system),
        });
    }

    let mut request_messages =
        anthropic_input_messages_to_universal_messages(request.messages).ok()?;
    messages.append(&mut request_messages);

    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

fn try_messages_from_anthropic_response(response: generated::Message) -> Option<Vec<Message>> {
    try_convert_non_empty(response.content)
}

fn try_parse_input_messages_for_import(data: &serde_json::Value) -> Option<Vec<Message>> {
    let messages = try_parse::<Vec<generated::InputMessage>>(data)?;
    anthropic_input_messages_to_universal_messages(messages).ok()
}

fn try_parse_anthropic_request_for_import(data: &serde_json::Value) -> Option<Vec<Message>> {
    let request = try_parse::<CreateMessageParams>(data)?;
    try_messages_from_anthropic_request(request)
}

fn try_parse_anthropic_response_for_import(data: &serde_json::Value) -> Option<Vec<Message>> {
    let response = try_parse::<generated::Message>(data)?;
    try_messages_from_anthropic_response(response)
}

pub(crate) fn try_parse_anthropic_for_import(data: &serde_json::Value) -> Option<Vec<Message>> {
    const PARSERS: &[MessageParser] = &[
        try_parse_content_blocks_for_import,
        try_parse_input_messages_for_import,
        try_parse_anthropic_request_for_import,
        try_parse_anthropic_response_for_import,
    ];
    try_parsers_in_order(data, PARSERS)
}

// Convert from Anthropic response ContentBlock to Universal Message
impl TryFromLLM<Vec<generated::ContentBlock>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(content_blocks: Vec<generated::ContentBlock>) -> Result<Self, Self::Error> {
        let mut messages = Vec::new();
        let mut content_parts = Vec::new();

        for block in content_blocks {
            match block.content_block_type {
                generated::ContentBlockType::Text => {
                    if let Some(text) = block.text {
                        // Preserve citations in provider_options for roundtrip
                        let provider_options = block.citations.as_ref().map(|citations| {
                            let mut opts = serde_json::Map::new();
                            if let Ok(v) = serde_json::to_value(citations) {
                                opts.insert("citations".into(), v);
                            }
                            ProviderOptions { options: opts }
                        });
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text,
                            encrypted_content: None,
                            cache_control: None,
                            provider_options,
                        }));
                    }
                }
                generated::ContentBlockType::Thinking => {
                    if let Some(thinking) = block.thinking {
                        content_parts.push(AssistantContentPart::Reasoning {
                            text: thinking,
                            // Preserve signature in encrypted_content for roundtrip
                            encrypted_content: block.signature.clone(),
                        });
                    }
                }
                generated::ContentBlockType::ToolUse => {
                    if let (Some(id), Some(name)) = (block.id, block.name) {
                        // Convert HashMap to JSON value for response processing too
                        let input = if let Some(input_map) = block.input {
                            serde_json::to_value(input_map).unwrap_or(serde_json::Value::Null)
                        } else {
                            serde_json::Value::Null
                        };

                        let provider_options =
                            anthropic_tool_use_provider_options_from_caller(block.caller);

                        content_parts.push(AssistantContentPart::ToolCall {
                            tool_call_id: id,
                            tool_name: name,
                            arguments: serde_json::to_string(&input)
                                .unwrap_or_else(|_| "{}".to_string())
                                .into(),
                            encrypted_content: None,
                            provider_options,
                            status: None,
                            caller: None,
                            provider_executed: None,
                        });
                    }
                }
                generated::ContentBlockType::ServerToolUse => {
                    // Server-executed tool (similar to ToolUse but provider_executed=true)
                    if let (Some(id), Some(name)) = (block.id, block.name) {
                        if tool_discovery::is_tool_search_name(&name) {
                            let query = tool_discovery::query(&block.input)?;
                            let arguments = tool_discovery::arguments(block.input);
                            content_parts.push(AssistantContentPart::ToolDiscoveryCall {
                                tool_call_id: id,
                                discovery_tool_name: name,
                                query,
                                arguments,
                                status: None,
                                execution: None,
                                provider_options: None,
                            });
                        } else {
                            let input = if let Some(input_map) = block.input {
                                serde_json::to_value(input_map).unwrap_or(serde_json::Value::Null)
                            } else {
                                serde_json::Value::Null
                            };

                            let provider_options =
                                anthropic_tool_use_provider_options_from_caller(block.caller);

                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: id,
                                tool_name: name,
                                arguments: serde_json::to_string(&input)
                                    .unwrap_or_else(|_| "{}".to_string())
                                    .into(),
                                encrypted_content: None,
                                provider_options,
                                status: None,
                                caller: None,
                                provider_executed: Some(true), // Mark as server-executed
                            });
                        }
                    }
                }
                generated::ContentBlockType::WebSearchToolResult => {
                    // Web search tool result - convert to ToolResult with full data
                    if let Some(id) = block.tool_use_id {
                        // Store the entire block data for roundtrip
                        let mut output = serde_json::Map::new();
                        output.insert(
                            "anthropic_type".into(),
                            serde_json::Value::String("web_search_tool_result".to_string()),
                        );
                        if let Some(content) = &block.content {
                            if let Ok(v) = serde_json::to_value(content) {
                                output.insert("content".into(), v);
                            }
                        }

                        content_parts.push(AssistantContentPart::ToolResult {
                            tool_call_id: id,
                            tool_name: "web_search".to_string(),
                            output: serde_json::Value::Object(output),
                            caller: None,
                            provider_options: None,
                        });
                    }
                }
                generated::ContentBlockType::ToolSearchToolResult => {
                    if let Some(id) = block.tool_use_id {
                        if !content_parts.is_empty() {
                            messages.push(Message::Assistant {
                                content: AssistantContent::Array(std::mem::take(
                                    &mut content_parts,
                                )),
                                id: None,
                            });
                        }

                        let discovery_result = tool_discovery::result_from_response_content(
                            id,
                            "tool_search".to_string(),
                            block.content,
                        )?;
                        messages.push(Message::Tool {
                            content: vec![ToolContentPart::ToolDiscoveryResult(discovery_result)],
                        });
                    }
                }
                _ => {
                    // Skip other types (RedactedThinking, etc.)
                    continue;
                }
            }
        }

        if !content_parts.is_empty() {
            messages.push(Message::Assistant {
                content: AssistantContent::Array(std::mem::take(&mut content_parts)),
                id: None,
            });
        }

        if messages.is_empty() {
            content_parts.push(AssistantContentPart::Text(TextContentPart {
                text: String::new(),
                encrypted_content: None,
                cache_control: None,
                provider_options: None,
            }));
            messages.push(Message::Assistant {
                content: AssistantContent::Array(content_parts),
                id: None,
            });
        }

        Ok(messages)
    }
}

// Convert from Universal Message to Anthropic response ContentBlock
impl TryFromLLM<Vec<Message>> for Vec<generated::ContentBlock> {
    type Error = ConvertError;

    fn try_from(messages: Vec<Message>) -> Result<Self, Self::Error> {
        let mut content_blocks = Vec::new();

        for message in messages {
            match message {
                Message::Assistant { content, .. } => match content {
                    AssistantContent::String(text) => {
                        content_blocks.push(generated::ContentBlock {
                            citations: None,
                            text: Some(text),
                            content_block_type: generated::ContentBlockType::Text,
                            signature: None,
                            thinking: None,
                            data: None,
                            caller: None,
                            id: None,
                            input: None,
                            name: None,
                            content: None,
                            tool_use_id: None,
                            file_id: None,
                        });
                    }
                    AssistantContent::Array(parts) => {
                        for part in parts {
                            match part {
                                AssistantContentPart::Text(text_part) => {
                                    let citations = anthropic_text_citations_from_provider_options(
                                        &text_part.provider_options,
                                    );
                                    content_blocks.push(generated::ContentBlock {
                                        citations,
                                        text: Some(text_part.text),
                                        content_block_type: generated::ContentBlockType::Text,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        caller: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        content: None,
                                        tool_use_id: None,
                                        file_id: None,
                                    });
                                }
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content,
                                } => {
                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
                                        text: None,
                                        content_block_type: generated::ContentBlockType::Thinking,
                                        // Restore signature from encrypted_content
                                        signature: encrypted_content,
                                        thinking: Some(text),
                                        data: None,
                                        caller: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        content: None,
                                        tool_use_id: None,
                                        file_id: None,
                                    });
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    provider_executed,
                                    provider_options,
                                    ..
                                } => {
                                    // Convert ToolCallArguments to serde_json::Map for response generation
                                    let input_map = match &arguments {
                                        ToolCallArguments::Valid(map) => Some(map.clone()),
                                        ToolCallArguments::Invalid(_)
                                        | ToolCallArguments::Custom(_) => None,
                                    };

                                    // Use ServerToolUse if provider_executed is true
                                    let block_type = if provider_executed == Some(true) {
                                        generated::ContentBlockType::ServerToolUse
                                    } else {
                                        generated::ContentBlockType::ToolUse
                                    };

                                    let caller = anthropic_tool_use_caller_from_provider_options(
                                        &provider_options,
                                    );

                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
                                        text: None,
                                        content_block_type: block_type,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        caller,
                                        id: Some(tool_call_id.clone()),
                                        input: input_map,
                                        name: Some(tool_name.clone()),
                                        content: None,
                                        tool_use_id: None,
                                        file_id: None,
                                    });
                                }
                                AssistantContentPart::ToolDiscoveryCall {
                                    tool_call_id,
                                    discovery_tool_name,
                                    query,
                                    arguments,
                                    ..
                                } => {
                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
                                        text: None,
                                        content_block_type:
                                            generated::ContentBlockType::ServerToolUse,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        caller: None,
                                        id: Some(tool_discovery::tool_search_call_id(
                                            &tool_call_id,
                                        )),
                                        input: tool_discovery::input_map(
                                            arguments.clone(),
                                            query.clone(),
                                        )?,
                                        name: Some(tool_discovery::tool_search_name(
                                            &discovery_tool_name,
                                        )),
                                        content: None,
                                        tool_use_id: None,
                                        file_id: None,
                                    });
                                }
                                AssistantContentPart::ToolResult {
                                    tool_call_id,
                                    output,
                                    ..
                                } => {
                                    // Check if this is a web_search_tool_result
                                    let is_web_search_result =
                                        output.get("anthropic_type").and_then(|v| v.as_str())
                                            == Some("web_search_tool_result");

                                    if is_web_search_result {
                                        // Restore as WebSearchToolResult
                                        let content = output.get("content").and_then(|v| {
                                            serde_json::from_value::<
                                                    generated::ContentBlockContent,
                                                >(
                                                    v.clone()
                                                )
                                                .ok()
                                        });

                                        content_blocks.push(generated::ContentBlock {
                                            citations: None,
                                            text: None,
                                            content_block_type:
                                                generated::ContentBlockType::WebSearchToolResult,
                                            signature: None,
                                            thinking: None,
                                            data: None,
                                            caller: None,
                                            id: None,
                                            input: None,
                                            name: None,
                                            content,
                                            tool_use_id: Some(tool_call_id.clone()),
                                            file_id: None,
                                        });
                                    }
                                    // Skip other tool results - they shouldn't appear in response content
                                }
                                _ => {
                                    // Skip other types for now
                                    continue;
                                }
                            }
                        }
                    }
                },
                Message::Tool { content } => {
                    for part in content {
                        if let ToolContentPart::ToolDiscoveryResult(discovery_result) = part {
                            content_blocks.push(generated::ContentBlock {
                                citations: None,
                                text: None,
                                content_block_type:
                                    generated::ContentBlockType::ToolSearchToolResult,
                                signature: None,
                                thinking: None,
                                data: None,
                                caller: None,
                                id: None,
                                input: None,
                                name: None,
                                content: Some(tool_discovery::response_result_content(
                                    discovery_result.tools,
                                )?),
                                tool_use_id: Some(tool_discovery::tool_search_call_id(
                                    &discovery_result.tool_call_id,
                                )),
                                file_id: None,
                            });
                        }
                    }
                }
                _ => {
                    // Skip non-assistant messages
                    continue;
                }
            }
        }

        Ok(content_blocks)
    }
}

impl From<&ToolChoice> for ToolChoiceConfig {
    fn from(tc: &ToolChoice) -> Self {
        let mode = Some(match tc.tool_choice_type {
            ToolChoiceType::Auto => ToolChoiceMode::Auto,
            ToolChoiceType::TypeNone => ToolChoiceMode::None,
            ToolChoiceType::Any => ToolChoiceMode::Required,
            ToolChoiceType::Tool => ToolChoiceMode::Tool,
        });
        ToolChoiceConfig {
            mode,
            tool_name: tc.name.clone(),
        }
    }
}

impl TryFrom<&ToolChoiceConfig> for ToolChoice {
    type Error = ();

    fn try_from(config: &ToolChoiceConfig) -> Result<Self, Self::Error> {
        let mode = config.mode.ok_or(())?;
        Ok(ToolChoice {
            tool_choice_type: match mode {
                ToolChoiceMode::Auto => ToolChoiceType::Auto,
                ToolChoiceMode::None => ToolChoiceType::TypeNone,
                ToolChoiceMode::Required => ToolChoiceType::Any,
                ToolChoiceMode::Tool => ToolChoiceType::Tool,
            },
            name: if mode == ToolChoiceMode::Tool {
                config.tool_name.clone()
            } else {
                None
            },
            disable_parallel_tool_use: None,
        })
    }
}

// =============================================================================
// Response Format Conversions (Anthropic ↔ Universal)
// =============================================================================

impl From<&JsonOutputFormat> for ResponseFormatConfig {
    fn from(format: &JsonOutputFormat) -> Self {
        let format_type = match format.json_output_format_type {
            JsonOutputFormatType::JsonSchema => Some(ResponseFormatType::JsonSchema),
        };
        let json_schema = format_type
            .filter(|ft| *ft == ResponseFormatType::JsonSchema)
            .map(|_| JsonSchemaConfig {
                name: "response".to_string(),
                schema: Value::Object(format.schema.clone()),
                strict: None,
                description: None,
            });
        ResponseFormatConfig {
            format_type,
            json_schema,
        }
    }
}

impl TryFrom<&ResponseFormatConfig> for JsonOutputFormat {
    type Error = ConvertError;

    fn try_from(config: &ResponseFormatConfig) -> Result<Self, Self::Error> {
        match config
            .format_type
            .ok_or_else(|| ConvertError::MissingRequiredField {
                field: "format_type".to_string(),
            })? {
            ResponseFormatType::Text => Err(ConvertError::InvalidResponseSchema {
                target_provider: ProviderFormat::Anthropic,
                reason: "text response format is not supported by Anthropic output_config.format"
                    .to_string(),
            }),
            // Anthropic json_object compatibility is handled in adapter.rs via synthetic json tool shim.
            // Do not emit output_config.format for json_object here.
            ResponseFormatType::JsonObject => Err(ConvertError::InvalidResponseSchema {
                target_provider: ProviderFormat::Anthropic,
                reason: "json_object response format uses the Anthropic JSON tool shim".to_string(),
            }),
            ResponseFormatType::JsonSchema => {
                let js = config.json_schema.as_ref().ok_or_else(|| {
                    ConvertError::MissingRequiredField {
                        field: "json_schema".to_string(),
                    }
                })?;
                // Anthropic's structured-output schema subset is narrower than
                // the cross-provider JSON-schema surface. When we emit a typed
                // `output_config.format` from canonical `response_format`, we
                // intentionally drop tuple-position hints plus array/numeric
                // bounds as a lossy compatibility fallback. Callers should not
                // expect strict schema fidelity for `prefixItems`, `minItems`,
                // `maxItems`, `minimum`, or `maximum`. Raw Anthropic
                // `output_config` passthrough remains verbatim in adapter.rs.
                match normalize_response_schema_for_strict_target(
                    &js.schema,
                    ProviderFormat::Anthropic,
                    false,
                )? {
                    Value::Object(m) => Ok(JsonOutputFormat {
                        schema: m,
                        json_output_format_type: JsonOutputFormatType::JsonSchema,
                    }),
                    _ => Err(ConvertError::InvalidResponseSchema {
                        target_provider: ProviderFormat::Anthropic,
                        reason: "response schema root must be a JSON object".to_string(),
                    }),
                }
            }
        }
    }
}

impl TryFrom<&UniversalTool> for CustomTool {
    type Error = ConvertError;

    fn try_from(tool: &UniversalTool) -> Result<Self, Self::Error> {
        if tool.output_schema.is_some() {
            return Err(ConvertError::UnsupportedToolType {
                tool_name: tool.name.clone(),
                tool_type: "output_schema".to_string(),
                target_provider: ProviderFormat::Anthropic,
            });
        }

        match &tool.tool_type {
            UniversalToolType::Function => Ok(CustomTool {
                allowed_callers: anthropic_allowed_callers_from_universal(&tool.allowed_callers)?,
                name: tool.name.clone(),
                description: tool.description.clone(),
                defer_loading: (tool.availability == ToolAvailability::Deferred).then_some(true),
                input_examples: None,
                input_schema: normalize_anthropic_tool_schema(&tool.name, tool.parameters.clone())?,
                strict: tool.strict,
                cache_control: None,
                eager_input_streaming: None,
            }),
            UniversalToolType::Custom { .. } => Err(ConvertError::UnsupportedToolType {
                tool_name: tool.name.clone(),
                tool_type: "custom".to_string(),
                target_provider: ProviderFormat::Anthropic,
            }),
            UniversalToolType::Builtin { builtin_type, .. } => {
                Err(ConvertError::UnsupportedToolType {
                    tool_name: tool.name.clone(),
                    tool_type: builtin_type.clone(),
                    target_provider: ProviderFormat::Anthropic,
                })
            }
        }
    }
}

impl From<&Tool> for UniversalTool {
    fn from(tool: &Tool) -> Self {
        match tool {
            Tool::Custom(ct) => {
                let mut tool = Self::function(
                    &ct.name,
                    ct.description.clone(),
                    Some(ct.input_schema.clone()),
                    ct.strict,
                );
                if ct.defer_loading == Some(true) {
                    tool.availability = ToolAvailability::Deferred;
                }
                tool.allowed_callers =
                    universal_allowed_callers_from_anthropic(ct.allowed_callers.clone());
                tool
            }
            other => {
                let config = serde_json::to_value(other).ok();
                let type_str = config
                    .as_ref()
                    .and_then(|v| v.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let name = config
                    .as_ref()
                    .and_then(|v| v.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or(&type_str)
                    .to_string();
                Self::builtin(name, BuiltinToolProvider::Anthropic, type_str, config)
            }
        }
    }
}

impl TryFrom<&UniversalTool> for Tool {
    type Error = ConvertError;

    fn try_from(tool: &UniversalTool) -> Result<Self, Self::Error> {
        match &tool.tool_type {
            UniversalToolType::Function => Ok(Tool::Custom(CustomTool::try_from(tool)?)),
            UniversalToolType::Custom { .. } => Err(ConvertError::UnsupportedToolType {
                tool_name: tool.name.clone(),
                tool_type: "custom".to_string(),
                target_provider: ProviderFormat::Anthropic,
            }),
            UniversalToolType::Builtin {
                provider,
                builtin_type,
                config,
            } => {
                if matches!(provider, BuiltinToolProvider::Google)
                    && builtin_type == "google_search"
                {
                    let config_val =
                        config
                            .clone()
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: format!("config for Google builtin tool '{}'", tool.name),
                            })?;
                    let _google_search: GoogleSearch =
                        serde_json::from_value(config_val).map_err(|e| {
                            ConvertError::JsonSerializationFailed {
                                field: format!("Google search tool '{}'", tool.name),
                                error: e.to_string(),
                            }
                        })?;
                    return Ok(Tool::WebSearch20250305(generated::WebSearchTool20250305 {
                        allowed_callers: None,
                        allowed_domains: None,
                        blocked_domains: None,
                        cache_control: None,
                        defer_loading: None,
                        max_uses: None,
                        name: "web_search".to_string(),
                        strict: None,
                        user_location: None,
                    }));
                }
                if !matches!(provider, BuiltinToolProvider::Anthropic) {
                    return Err(ConvertError::UnsupportedToolType {
                        tool_name: tool.name.clone(),
                        tool_type: builtin_type.clone(),
                        target_provider: ProviderFormat::Anthropic,
                    });
                }
                let config_val =
                    config
                        .clone()
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: format!("config for Anthropic builtin tool '{}'", tool.name),
                        })?;
                serde_json::from_value::<Tool>(config_val).map_err(|e| {
                    ConvertError::JsonSerializationFailed {
                        field: format!("builtin tool '{}'", tool.name),
                        error: e.to_string(),
                    }
                })
            }
        }
    }
}

impl TryFromLLM<Vec<Tool>> for Vec<UniversalTool> {
    type Error = ConvertError;

    fn try_from(tools: Vec<Tool>) -> Result<Self, Self::Error> {
        Ok(tools.iter().map(UniversalTool::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;
    use crate::universal::convert::TryFromLLM;
    use std::collections::HashMap;

    #[derive(serde::Deserialize)]
    struct ToolSchemaPropertyView {
        #[serde(rename = "type")]
        schema_type: Option<String>,
        items: Option<Value>,
    }

    #[derive(serde::Deserialize)]
    struct ToolSchemaView {
        #[serde(rename = "type")]
        schema_type: String,
        #[serde(rename = "additionalProperties")]
        additional_properties: Option<Value>,
        properties: Option<HashMap<String, ToolSchemaPropertyView>>,
    }

    fn input_message(value: Value) -> generated::InputMessage {
        serde_json::from_value(value).expect("input message should deserialize")
    }

    fn preserved_unknown_tool_discovery_item() -> crate::universal::ToolDiscoveryResultItem {
        let mut options = serde_json::Map::new();
        options.insert(
            "content".to_string(),
            json!({
                "type": "tool_search_tool_unknown_result",
                "payload": {"opaque": true}
            }),
        );

        crate::universal::ToolDiscoveryResultItem {
            tool_name: "unknown".to_string(),
            tool: None,
            provider_options: Some(ProviderOptions { options }),
        }
    }

    fn text_from_user(message: &Message) -> &str {
        match message {
            Message::User {
                content: UserContent::String(text),
            } => text,
            Message::User {
                content: UserContent::Array(parts),
            } => match &parts[..] {
                [UserContentPart::Text(TextContentPart { text, .. })] => text,
                _ => panic!("expected single text user content part, got {message:?}"),
            },
            _ => panic!("expected user message, got {message:?}"),
        }
    }

    fn tool_call_id_from_tool(message: &Message) -> &str {
        match message {
            Message::Tool { content } => match &content[..] {
                [ToolContentPart::ToolResult(ToolResultContentPart { tool_call_id, .. })] => {
                    tool_call_id
                }
                _ => panic!("expected single tool result, got {message:?}"),
            },
            _ => panic!("expected tool message, got {message:?}"),
        }
    }

    #[test]
    fn test_json_object_response_format_is_not_converted_to_anthropic_format() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonObject),
            json_schema: None,
        };

        assert!(
            JsonOutputFormat::try_from(&config).is_err(),
            "json_object should not map to Anthropic output_config.format; adapter shim handles it"
        );
    }

    #[test]
    fn test_anthropic_system_role_imports_to_system_message() {
        let input: generated::InputMessage = serde_json::from_value(json!({
            "role": "system",
            "content": "Follow the project style guide."
        }))
        .unwrap();

        let message = <Message as TryFromLLM<generated::InputMessage>>::try_from(input).unwrap();

        match message {
            Message::System {
                content: UserContent::String(text),
            } => assert_eq!(text, "Follow the project style guide."),
            other => panic!("expected system message, got {other:?}"),
        }
    }

    #[test]
    fn anthropic_input_messages_to_universal_messages_preserves_pure_tool_results() {
        let input = input_message(json!({
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": "call_1",
                    "content": "result one"
                },
                {
                    "type": "tool_result",
                    "tool_use_id": "call_2",
                    "content": "result two"
                }
            ]
        }));

        let messages = anthropic_input_messages_to_universal_messages(vec![input])
            .expect("conversion should succeed");

        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::Tool { content } => {
                assert_eq!(content.len(), 2);
                let ToolContentPart::ToolResult(first) = &content[0] else {
                    panic!("expected first normal tool result");
                };
                assert_eq!(first.tool_call_id, "call_1");
                let ToolContentPart::ToolResult(second) = &content[1] else {
                    panic!("expected second normal tool result");
                };
                assert_eq!(second.tool_call_id, "call_2");
            }
            other => panic!("expected tool message, got {other:?}"),
        }
    }

    #[test]
    fn anthropic_input_messages_to_universal_messages_splits_tool_result_then_text() {
        let input = input_message(json!({
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": "call_repro_123",
                    "content": "{\"records\":[{\"id\":\"record_1\",\"status\":\"ok\"}]}"
                },
                {
                    "type": "text",
                    "text": "What details are available?"
                }
            ]
        }));

        let messages = anthropic_input_messages_to_universal_messages(vec![input])
            .expect("conversion should succeed");

        assert_eq!(messages.len(), 2);
        assert_eq!(tool_call_id_from_tool(&messages[0]), "call_repro_123");
        assert_eq!(text_from_user(&messages[1]), "What details are available?");
    }

    #[test]
    fn anthropic_input_messages_to_universal_messages_preserves_interleaved_order() {
        let input = input_message(json!({
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "Before"
                },
                {
                    "type": "tool_result",
                    "tool_use_id": "call_repro_123",
                    "content": "tool output"
                },
                {
                    "type": "text",
                    "text": "After"
                }
            ]
        }));

        let messages = anthropic_input_messages_to_universal_messages(vec![input])
            .expect("conversion should succeed");

        assert_eq!(messages.len(), 3);
        assert_eq!(text_from_user(&messages[0]), "Before");
        assert_eq!(tool_call_id_from_tool(&messages[1]), "call_repro_123");
        assert_eq!(text_from_user(&messages[2]), "After");
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_does_not_absorb_later_user_turns() {
        let messages = vec![
            Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id: "call_repro_123".to_string(),
                    tool_name: String::new(),
                    output: json!({"records": [{"id": "record_1", "status": "ok"}]}),
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                })],
            },
            Message::User {
                content: UserContent::String("first".to_string()),
            },
            Message::User {
                content: UserContent::String("second".to_string()),
            },
        ];

        let input_messages = universal_messages_to_anthropic_input_messages(messages)
            .expect("conversion should succeed");

        assert_eq!(input_messages.len(), 2);
        match &input_messages[0].content {
            generated::MessageContent::InputContentBlockArray(blocks) => {
                assert_eq!(blocks.len(), 2);
                assert_eq!(
                    blocks[0].input_content_block_type,
                    generated::InputContentBlockType::ToolResult
                );
                assert_eq!(
                    blocks[1].input_content_block_type,
                    generated::InputContentBlockType::Text
                );
                assert_eq!(blocks[1].text.as_deref(), Some("first"));
            }
            other => panic!("expected mixed content blocks, got {other:?}"),
        }
        assert_eq!(input_messages[1].role, generated::MessageRole::User);
        match &input_messages[1].content {
            generated::MessageContent::PurpleString(text) => assert_eq!(text, "second"),
            other => panic!("expected second user turn to remain separate, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_does_not_merge_user_before_tool_result() {
        let messages = vec![
            Message::User {
                content: UserContent::String("before".to_string()),
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id: "call_repro_123".to_string(),
                    tool_name: String::new(),
                    output: json!("result"),
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                })],
            },
        ];

        let input_messages = universal_messages_to_anthropic_input_messages(messages)
            .expect("conversion should succeed");

        assert_eq!(input_messages.len(), 2);
        match &input_messages[0].content {
            generated::MessageContent::PurpleString(text) => assert_eq!(text, "before"),
            other => panic!("expected first user turn to remain separate, got {other:?}"),
        }
        match &input_messages[1].content {
            generated::MessageContent::InputContentBlockArray(blocks) => {
                assert_eq!(blocks.len(), 1);
                assert_eq!(
                    blocks[0].input_content_block_type,
                    generated::InputContentBlockType::ToolResult
                );
            }
            other => panic!("expected pure tool result content, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_rejects_unpaired_tool_discovery_result() {
        let messages = vec![Message::Tool {
            content: vec![ToolContentPart::ToolDiscoveryResult(
                crate::universal::ToolDiscoveryResultContentPart {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    tools: vec![],
                    status: Some("completed".to_string()),
                    execution: Some("server".to_string()),
                    provider_options: None,
                },
            )],
        }];

        let err = universal_messages_to_anthropic_input_messages(messages).unwrap_err();
        match err {
            ConvertError::UnsupportedMapping { from, to } => {
                assert_eq!(from, "unpaired ToolDiscoveryResult");
                assert_eq!(to, "Anthropic InputMessage");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_rejects_query_only_tool_discovery_call() {
        let messages = vec![Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolDiscoveryCall {
                tool_call_id: "call_tool_search_123".to_string(),
                discovery_tool_name: "tool_search".to_string(),
                query: Some("search_code".to_string()),
                arguments: None,
                status: Some("completed".to_string()),
                execution: Some("server".to_string()),
                provider_options: None,
            }]),
            id: None,
        }];

        let err = universal_messages_to_anthropic_input_messages(messages).unwrap_err();
        match err {
            ConvertError::UnsupportedMapping { from, to } => {
                assert_eq!(from, "query-only ToolDiscoveryCall");
                assert_eq!(to, "Anthropic tool_search input");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_rejects_preserved_unknown_tool_search_result()
    {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolDiscoveryCall {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    query: None,
                    arguments: Some(json!({})),
                    status: Some("completed".to_string()),
                    execution: Some("server".to_string()),
                    provider_options: None,
                }]),
                id: None,
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolDiscoveryResult(
                    crate::universal::ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![preserved_unknown_tool_discovery_item()],
                        status: Some("completed".to_string()),
                        execution: Some("server".to_string()),
                        provider_options: None,
                    },
                )],
            },
        ];

        let err = universal_messages_to_anthropic_input_messages(messages).unwrap_err();
        match err {
            ConvertError::UnsupportedMapping { from, to } => {
                assert_eq!(
                    from,
                    "preserved unknown Anthropic tool_search_tool_result.content"
                );
                assert_eq!(to, "Anthropic InputContentBlock");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_response_blocks_rejects_preserved_unknown_tool_search_result(
    ) {
        let messages = vec![Message::Tool {
            content: vec![ToolContentPart::ToolDiscoveryResult(
                crate::universal::ToolDiscoveryResultContentPart {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    tools: vec![preserved_unknown_tool_discovery_item()],
                    status: Some("completed".to_string()),
                    execution: Some("server".to_string()),
                    provider_options: None,
                },
            )],
        }];

        let err = <Vec<generated::ContentBlock> as TryFromLLM<Vec<Message>>>::try_from(messages)
            .unwrap_err();
        match err {
            ConvertError::UnsupportedMapping { from, to } => {
                assert_eq!(
                    from,
                    "preserved unknown Anthropic tool_search_tool_result.content"
                );
                assert_eq!(to, "Anthropic ContentBlock");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn anthropic_input_tool_search_result_error_is_unsupported() {
        let input: generated::InputMessage = serde_json::from_value(json!({
            "role": "user",
            "content": [{
                "type": "tool_search_tool_result",
                "tool_use_id": "srvtoolu_call_tool_search_123",
                "content": {
                    "type": "tool_search_tool_result_error"
                }
            }]
        }))
        .unwrap();

        let err = <Message as TryFromLLM<generated::InputMessage>>::try_from(input).unwrap_err();
        match err {
            ConvertError::UnsupportedMapping { from, to } => {
                assert_eq!(from, "Anthropic tool_search_tool_result_error");
                assert_eq!(to, "ToolDiscoveryResult");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn anthropic_response_tool_search_result_error_is_unsupported() {
        let blocks: Vec<generated::ContentBlock> = serde_json::from_value(json!([{
            "type": "tool_search_tool_result",
            "tool_use_id": "srvtoolu_call_tool_search_123",
            "content": {
                "type": "tool_search_tool_result_error"
            }
        }]))
        .unwrap();

        let err = <Vec<Message> as TryFromLLM<Vec<generated::ContentBlock>>>::try_from(blocks)
            .unwrap_err();
        match err {
            ConvertError::UnsupportedMapping { from, to } => {
                assert_eq!(from, "Anthropic tool_search_tool_result_error");
                assert_eq!(to, "ToolDiscoveryResult");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_merges_matching_tool_discovery_result() {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolDiscoveryCall {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    query: None,
                    arguments: Some(json!({})),
                    status: Some("completed".to_string()),
                    execution: Some("server".to_string()),
                    provider_options: None,
                }]),
                id: None,
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolDiscoveryResult(
                    crate::universal::ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![],
                        status: Some("completed".to_string()),
                        execution: Some("server".to_string()),
                        provider_options: None,
                    },
                )],
            },
        ];

        let input_messages = universal_messages_to_anthropic_input_messages(messages)
            .expect("matching discovery result should merge");

        assert_eq!(input_messages.len(), 1);
        match &input_messages[0].content {
            generated::MessageContent::InputContentBlockArray(blocks) => {
                assert_eq!(blocks.len(), 2);
                assert_eq!(
                    blocks[0].input_content_block_type,
                    generated::InputContentBlockType::ServerToolUse
                );
                assert_eq!(
                    blocks[1].input_content_block_type,
                    generated::InputContentBlockType::ToolSearchToolResult
                );
                assert_eq!(blocks[0].id, blocks[1].tool_use_id);
            }
            other => panic!("expected merged content blocks, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_splits_mixed_tool_discovery_results() {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::Array(vec![
                    AssistantContentPart::ToolDiscoveryCall {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        query: None,
                        arguments: Some(json!({})),
                        status: Some("completed".to_string()),
                        execution: Some("server".to_string()),
                        provider_options: None,
                    },
                    AssistantContentPart::ToolCall {
                        tool_call_id: "call_normal_123".to_string(),
                        tool_name: "get_weather".to_string(),
                        arguments: ToolCallArguments::from("{}".to_string()),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: None,
                    },
                ]),
                id: None,
            },
            Message::Tool {
                content: vec![
                    ToolContentPart::ToolResult(ToolResultContentPart {
                        tool_call_id: "call_normal_123".to_string(),
                        tool_name: "get_weather".to_string(),
                        output: json!("sunny"),
                        custom_tool_call: None,
                        caller: None,
                        provider_options: None,
                    }),
                    ToolContentPart::ToolDiscoveryResult(
                        crate::universal::ToolDiscoveryResultContentPart {
                            tool_call_id: "call_tool_search_123".to_string(),
                            discovery_tool_name: "tool_search".to_string(),
                            tools: vec![],
                            status: Some("completed".to_string()),
                            execution: Some("server".to_string()),
                            provider_options: None,
                        },
                    ),
                ],
            },
        ];

        let input_messages = universal_messages_to_anthropic_input_messages(messages)
            .expect("mixed tool message should split and merge discovery result");

        assert_eq!(input_messages.len(), 2);
        match &input_messages[0].content {
            generated::MessageContent::InputContentBlockArray(blocks) => {
                assert_eq!(blocks.len(), 3);
                assert_eq!(
                    blocks[0].input_content_block_type,
                    generated::InputContentBlockType::ServerToolUse
                );
                assert_eq!(
                    blocks[1].input_content_block_type,
                    generated::InputContentBlockType::ToolUse
                );
                assert_eq!(
                    blocks[2].input_content_block_type,
                    generated::InputContentBlockType::ToolSearchToolResult
                );
                assert_eq!(blocks[0].id, blocks[2].tool_use_id);
            }
            other => panic!("expected merged assistant content blocks, got {other:?}"),
        }
        match &input_messages[1].content {
            generated::MessageContent::InputContentBlockArray(blocks) => {
                assert_eq!(blocks.len(), 1);
                assert_eq!(
                    blocks[0].input_content_block_type,
                    generated::InputContentBlockType::ToolResult
                );
                assert_eq!(blocks[0].tool_use_id.as_deref(), Some("call_normal_123"));
            }
            other => panic!("expected normal tool result user message, got {other:?}"),
        }
    }

    #[test]
    fn universal_messages_to_anthropic_input_messages_rejects_unpaired_mixed_tool_discovery_result()
    {
        let messages = vec![Message::Tool {
            content: vec![
                ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id: "call_normal_123".to_string(),
                    tool_name: "get_weather".to_string(),
                    output: json!("sunny"),
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                }),
                ToolContentPart::ToolDiscoveryResult(
                    crate::universal::ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![],
                        status: Some("completed".to_string()),
                        execution: Some("server".to_string()),
                        provider_options: None,
                    },
                ),
            ],
        }];

        let error = universal_messages_to_anthropic_input_messages(messages)
            .expect_err("unpaired discovery result should fail");

        match error {
            ConvertError::UnsupportedMapping { from, .. } => {
                assert_eq!(from, "unpaired ToolDiscoveryResult");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn test_anthropic_mid_conv_system_imports_to_system_message() {
        let input: generated::InputMessage = serde_json::from_value(json!({
            "role": "system",
            "content": [
                {
                    "type": "mid_conv_system",
                    "content": [
                        {
                            "type": "text",
                            "text": "Use the updated policy.",
                            "cache_control": { "type": "ephemeral" }
                        }
                    ]
                }
            ]
        }))
        .unwrap();

        let message = <Message as TryFromLLM<generated::InputMessage>>::try_from(input).unwrap();

        match message {
            Message::System {
                content: UserContent::Array(parts),
            } => {
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    UserContentPart::Text(text) => {
                        assert_eq!(text.text, "Use the updated policy.");
                        assert_eq!(
                            text.cache_control
                                .as_ref()
                                .expect("cache_control should be preserved")
                                .cache_control_type,
                            crate::universal::CacheControlType::Ephemeral
                        );
                    }
                    other => panic!("expected text part, got {other:?}"),
                }
            }
            other => panic!("expected system message with array content, got {other:?}"),
        }
    }

    #[test]
    fn test_anthropic_mid_conv_system_import_preserves_parent_cache_control() {
        let input: generated::InputMessage = serde_json::from_value(json!({
            "role": "system",
            "content": [
                {
                    "type": "mid_conv_system",
                    "cache_control": { "type": "ephemeral" },
                    "content": [
                        {
                            "type": "text",
                            "text": "Use the updated policy."
                        }
                    ]
                }
            ]
        }))
        .unwrap();

        let message = <Message as TryFromLLM<generated::InputMessage>>::try_from(input).unwrap();

        match message {
            Message::System {
                content: UserContent::Array(parts),
            } => {
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    UserContentPart::Text(text) => {
                        assert_eq!(text.text, "Use the updated policy.");
                        assert_eq!(
                            text.cache_control
                                .as_ref()
                                .expect("parent cache_control should be preserved")
                                .cache_control_type,
                            crate::universal::CacheControlType::Ephemeral
                        );
                    }
                    other => panic!("expected text part, got {other:?}"),
                }
            }
            other => panic!("expected system message with array content, got {other:?}"),
        }
    }

    #[test]
    fn test_json_schema_response_format_to_anthropic_is_lossy_for_unsupported_keywords() {
        let config = ResponseFormatConfig {
            format_type: Some(ResponseFormatType::JsonSchema),
            json_schema: Some(JsonSchemaConfig {
                name: "response".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "tuple": {
                            "type": "array",
                            "prefixItems": [
                                { "type": "string" },
                                { "type": "integer" }
                            ],
                            "minItems": 2,
                            "maxItems": 2
                        },
                        "score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 10
                        }
                    },
                    "required": ["tuple", "score"],
                    "additionalProperties": false
                }),
                strict: Some(true),
                description: None,
            }),
        };

        let format = JsonOutputFormat::try_from(&config).unwrap();
        let schema = Value::Object(format.schema);

        assert_eq!(schema.pointer("/properties/tuple/prefixItems"), None);
        assert_eq!(schema.pointer("/properties/tuple/minItems"), None);
        assert_eq!(schema.pointer("/properties/tuple/maxItems"), None);
        assert_eq!(schema.pointer("/properties/score/minimum"), None);
        assert_eq!(schema.pointer("/properties/score/maximum"), None);
    }

    #[test]
    fn test_google_search_builtin_maps_to_anthropic_web_search() {
        let tool = UniversalTool::builtin(
            "google_search",
            BuiltinToolProvider::Google,
            "google_search",
            Some(crate::serde_json::json!({
                "timeRangeFilter": {
                    "startTime": "2025-01-01T00:00:00Z",
                    "endTime": "2025-01-02T00:00:00Z"
                }
            })),
        );

        let anthropic_tool = Tool::try_from(&tool).unwrap();
        match anthropic_tool {
            Tool::WebSearch20250305(web_search) => {
                assert_eq!(web_search.name, "web_search");
                assert!(web_search.allowed_domains.is_none());
                assert!(web_search.blocked_domains.is_none());
                assert!(web_search.max_uses.is_none());
                assert!(web_search.user_location.is_none());
            }
            other => panic!("expected web_search_20250305 tool, got {:?}", other),
        }
    }

    #[test]
    fn test_dated_server_tool_variants_deserialize_and_roundtrip() {
        // Regression: the spec added web_search_20260318 / web_fetch_20260318 server
        // tools. Verify the newly generated Tool enum variants deserialize from provider
        // JSON and survive a builtin round trip (Tool -> UniversalTool -> Tool) without
        // being downgraded to Custom or losing their discriminator.
        let cases = [
            (
                crate::serde_json::json!({
                    "type": "web_search_20260318",
                    "name": "web_search",
                    "max_uses": 3,
                }),
                "web_search_20260318",
            ),
            (
                crate::serde_json::json!({
                    "type": "web_fetch_20260318",
                    "name": "web_fetch",
                    "max_content_tokens": 1000,
                }),
                "web_fetch_20260318",
            ),
        ];

        for (json, expected_type) in cases {
            let tool: Tool = crate::serde_json::from_value(json.clone())
                .unwrap_or_else(|e| panic!("{expected_type} must deserialize into Tool: {e}"));
            assert!(
                !matches!(tool, Tool::Custom(_)),
                "{expected_type} must not fall back to the untagged Custom variant"
            );

            // Provider tool -> universal builtin representation.
            let universal = UniversalTool::from(&tool);
            match &universal.tool_type {
                UniversalToolType::Builtin { builtin_type, .. } => {
                    assert_eq!(builtin_type, expected_type);
                }
                other => panic!("expected builtin tool type for {expected_type}, got {other:?}"),
            }

            // Universal builtin -> provider tool: the typed variant is preserved exactly
            // (Tool derives PartialEq, so this also guards the discriminator and fields).
            let roundtripped = Tool::try_from(&universal)
                .unwrap_or_else(|e| panic!("{expected_type} must convert back to Tool: {e}"));
            assert_eq!(
                tool, roundtripped,
                "roundtrip must preserve the {expected_type} tool variant"
            );
        }
    }

    #[test]
    fn test_file_to_anthropic_document_with_provider_options() {
        // Create a File content part marked as a document (via provider_options)
        let mut opts = serde_json::Map::new();
        opts.insert(
            "anthropic_type".into(),
            serde_json::Value::String("document".to_string()),
        );
        opts.insert(
            "title".into(),
            serde_json::Value::String("Test Document".to_string()),
        );

        let file_part = UserContentPart::File {
            data: serde_json::Value::String("base64encodeddata".to_string()),
            filename: Some("test.pdf".to_string()),
            media_type: "application/pdf".to_string(),
            provider_options: Some(ProviderOptions { options: opts }),
        };

        // Create a user message with the file part
        let message = Message::User {
            content: UserContent::Array(vec![file_part]),
        };

        // Convert to Anthropic InputMessage
        let result: Result<generated::InputMessage, _> =
            <generated::InputMessage as TryFromLLM<Message>>::try_from(message);

        assert!(result.is_ok(), "File conversion should succeed");
        let input_msg = result.unwrap();

        // Verify it's a user message with document block
        assert!(matches!(input_msg.role, generated::MessageRole::User));
        if let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content {
            assert_eq!(blocks.len(), 1, "Should have exactly one content block");
            let block = &blocks[0];
            assert!(
                matches!(
                    block.input_content_block_type,
                    generated::InputContentBlockType::Document
                ),
                "Should be a Document block"
            );
            assert_eq!(block.title, Some("Test Document".to_string()));
        } else {
            panic!("Expected InputContentBlockArray");
        }
    }

    #[test]
    fn test_regular_file_without_anthropic_marker_converts_to_document() {
        // Create a regular File content part (no anthropic_type marker)
        let file_part = UserContentPart::File {
            data: serde_json::Value::String("base64encodeddata".to_string()),
            filename: Some("test.pdf".to_string()),
            media_type: "application/pdf".to_string(),
            provider_options: None, // No anthropic_type marker
        };

        let message = Message::User {
            content: UserContent::Array(vec![file_part]),
        };

        let result: Result<generated::InputMessage, _> =
            <generated::InputMessage as TryFromLLM<Message>>::try_from(message);

        assert!(result.is_ok());
        let input_msg = result.unwrap();

        if let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content {
            assert_eq!(blocks.len(), 1, "regular file should be preserved");
            let block = &blocks[0];
            assert!(matches!(
                block.input_content_block_type,
                generated::InputContentBlockType::Document
            ));
            if let Some(generated::SourceUnion::Source(source)) = &block.source {
                assert!(matches!(
                    source.source_type,
                    generated::Base64ImageSourceType::Text
                ));
                assert_eq!(source.data.as_deref(), Some("base64encodeddata"));
                assert!(source.url.is_none());
            } else {
                panic!("Expected SourceSource");
            }
        }
    }

    #[test]
    fn test_regular_url_file_converts_to_anthropic_document() {
        let file_part = UserContentPart::File {
            data: serde_json::Value::String("https://example.com/report.pdf".to_string()),
            filename: None,
            media_type: "application/pdf".to_string(),
            provider_options: None,
        };

        let message = Message::User {
            content: UserContent::Array(vec![file_part]),
        };

        let result: Result<generated::InputMessage, _> =
            <generated::InputMessage as TryFromLLM<Message>>::try_from(message);

        assert!(result.is_ok());
        let input_msg = result.unwrap();

        if let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content {
            assert_eq!(blocks.len(), 1, "url-backed file should be preserved");
            let block = &blocks[0];
            assert!(matches!(
                block.input_content_block_type,
                generated::InputContentBlockType::Document
            ));
            if let Some(generated::SourceUnion::Source(source)) = &block.source {
                assert!(matches!(
                    source.source_type,
                    generated::Base64ImageSourceType::Url
                ));
                assert_eq!(
                    source.url.as_deref(),
                    Some("https://example.com/report.pdf")
                );
                assert!(source.data.is_none());
            } else {
                panic!("Expected SourceSource");
            }
        }
    }

    #[test]
    fn test_anthropic_document_url_imports_back_to_file_url() {
        let message = generated::InputMessage {
            role: generated::MessageRole::User,
            content: generated::MessageContent::InputContentBlockArray(vec![
                generated::InputContentBlock {
                    cache_control: None,
                    citations: None,
                    text: None,
                    input_content_block_type: generated::InputContentBlockType::Document,
                    source: Some(generated::SourceUnion::Source(generated::Source {
                        data: None,
                        media_type: None,
                        source_type: generated::Base64ImageSourceType::Url,
                        url: Some("https://example.com/report.pdf".to_string()),
                        content: None,
                    })),
                    context: None,
                    title: Some("Doc".to_string()),
                    content: None,
                    signature: None,
                    thinking: None,
                    data: None,
                    caller: None,
                    id: None,
                    input: None,
                    name: None,
                    is_error: None,
                    tool_use_id: None,
                    file_id: None,
                },
            ]),
        };

        let converted = <Message as TryFromLLM<generated::InputMessage>>::try_from(message)
            .expect("document message should import");

        match converted {
            Message::User {
                content: UserContent::Array(parts),
            } => match &parts[0] {
                UserContentPart::File {
                    data,
                    filename,
                    media_type,
                    provider_options,
                } => {
                    assert_eq!(
                        data,
                        &serde_json::Value::String("https://example.com/report.pdf".to_string())
                    );
                    assert!(filename.is_none());
                    assert_eq!(media_type, "application/pdf");
                    assert!(provider_options.is_some());
                }
                other => panic!("expected file content, got {:?}", other),
            },
            other => panic!("expected user message, got {:?}", other),
        }
    }

    #[test]
    fn test_image_url_to_anthropic() {
        let image_part = UserContentPart::Image {
            image: serde_json::Value::String("https://example.com/image.jpg".to_string()),
            media_type: Some("image/jpeg".to_string()),
            provider_options: None,
        };

        let message = Message::User {
            content: UserContent::Array(vec![image_part]),
        };

        let result: Result<generated::InputMessage, _> =
            <generated::InputMessage as TryFromLLM<Message>>::try_from(message);

        assert!(result.is_ok());
        let input_msg = result.unwrap();

        if let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content {
            assert_eq!(blocks.len(), 1);
            let block = &blocks[0];
            assert!(matches!(
                block.input_content_block_type,
                generated::InputContentBlockType::Image
            ));
            // Verify URL source type is used
            if let Some(generated::SourceUnion::Source(source)) = &block.source {
                assert!(matches!(
                    source.source_type,
                    generated::Base64ImageSourceType::Url
                ));
                assert_eq!(
                    source.url,
                    Some("https://example.com/image.jpg".to_string())
                );
            } else {
                panic!("Expected SourceSource");
            }
        } else {
            panic!("Expected InputContentBlockArray");
        }
    }

    #[test]
    fn test_custom_tool_normalizes_google_schema_types() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({
                "type": "OBJECT",
                "properties": {
                    "location": {
                        "type": "STRING",
                        "description": "The city",
                        "items": null
                    }
                },
                "required": ["location"],
                "additionalProperties": null
            })),
            None,
        );

        let custom_tool = CustomTool::try_from(&tool).expect("tool should convert");
        let schema: ToolSchemaView =
            serde_json::from_value(custom_tool.input_schema).expect("schema should deserialize");
        assert_eq!(schema.schema_type, "object");
        assert!(schema.additional_properties.is_none());
        let location = schema
            .properties
            .expect("properties should be present")
            .remove("location")
            .expect("location should be present");
        assert_eq!(location.schema_type.as_deref(), Some("string"));
        assert!(location.items.is_none());
    }

    #[test]
    fn test_custom_tool_rejects_programmatic_allowed_callers() {
        let mut tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                },
                "required": ["location"],
                "additionalProperties": false
            })),
            None,
        );
        tool.allowed_callers = Some(vec![
            UniversalToolCaller::Programmatic,
            UniversalToolCaller::Direct,
        ]);

        let err = CustomTool::try_from(&tool).expect_err("programmatic caller should fail");
        assert!(matches!(err, ConvertError::UnsupportedToolType { .. }));
        assert!(err.to_string().contains("programmatic caller restriction"));
    }

    #[test]
    fn test_custom_tool_preserves_supported_allowed_callers() {
        let mut tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                },
                "required": ["location"],
                "additionalProperties": false
            })),
            None,
        );
        tool.allowed_callers = Some(vec![
            UniversalToolCaller::Direct,
            UniversalToolCaller::CodeExecution20260521,
        ]);

        let custom_tool = CustomTool::try_from(&tool).expect("tool should convert");

        assert_eq!(
            custom_tool.allowed_callers,
            Some(vec![
                generated::AllowedCaller::Direct,
                generated::AllowedCaller::CodeExecution20260521,
            ])
        );
    }

    #[test]
    fn test_custom_tool_rejects_missing_root_schema_type() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({
                "properties": {
                    "location": { "type": "string" }
                }
            })),
            None,
        );

        let err = CustomTool::try_from(&tool).expect_err("missing type should fail");
        assert!(matches!(err, ConvertError::InvalidToolSchema { .. }));
        assert!(err.to_string().contains("root type is required"));
    }

    #[test]
    fn test_custom_tool_rejects_non_object_root_schema_type() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({
                "type": "string"
            })),
            None,
        );

        let err = CustomTool::try_from(&tool).expect_err("non-object type should fail");
        assert!(matches!(err, ConvertError::InvalidToolSchema { .. }));
        assert!(err.to_string().contains("must be 'object'"));
    }

    #[test]
    fn test_custom_tool_rejects_null_root_schema_type() {
        let tool = UniversalTool::function(
            "get_weather",
            Some("Get weather".to_string()),
            Some(json!({
                "type": null
            })),
            None,
        );

        let err = CustomTool::try_from(&tool).expect_err("null type should fail");
        assert!(matches!(err, ConvertError::InvalidToolSchema { .. }));
        assert!(err.to_string().contains("root type is required"));
    }
}
