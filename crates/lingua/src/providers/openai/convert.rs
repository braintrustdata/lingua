use crate::error::ConvertError;
use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::defaults::{EMPTY_OBJECT_STR, REFUSAL_TEXT};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, ProviderOptions, TextContentPart,
    ToolCallArguments, ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use crate::util::media::parse_base64_data_url;
use serde::{Deserialize, Serialize};

/// Extended ChatCompletionRequest/ResponseMessage with reasoning support.
///
/// The official OpenAI Chat Completions API doesn't include a `reasoning` field on messages.`                                         
/// With the release of gpt-oss, OpenAI's guidance is to handle reasoning content with                                                 
/// a top-level `reasoning` field. https://cookbook.openai.com/articles/gpt-oss/handle-raw-cot#chat-completions-api                    
///
/// These extension type uses `#[serde(flatten)]` to wrap the generated type while adding
/// the `reasoning` field, keeping generated types faithful to the official spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponseMessageExt {
    #[serde(flatten)]
    pub base: openai::ChatCompletionResponseMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Encrypted reasoning signature for cross-provider roundtrips (e.g., Anthropic's signature)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageExt {
    #[serde(flatten)]
    pub base: openai::ChatCompletionRequestMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Encrypted reasoning signature for cross-provider roundtrips (e.g., Anthropic's signature)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_signature: Option<String>,
}

/// Helper function to build ToolCallArguments from a JSON value
fn build_tool_arguments(value: &serde_json::Value) -> ToolCallArguments {
    match value.as_object() {
        Some(map) => ToolCallArguments::Valid(map.clone()),
        None => ToolCallArguments::Invalid(value.to_string()),
    }
}

/// Helper to parse an optional field from JSON with proper error handling.
///
/// Returns `Ok(None)` if the field is missing or null, `Ok(Some(value))` if parsing succeeds,
/// or `Err` with a descriptive error if parsing fails.
fn parse_builtin_field<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
    field: &str,
    tool_name: &str,
) -> Result<Option<T>, ConvertError> {
    match value.get(field) {
        Some(v) if v.is_null() => Ok(None),
        Some(v) => serde_json::from_value(v.clone()).map(Some).map_err(|e| {
            ConvertError::JsonSerializationFailed {
                field: format!("{}.{}", tool_name, field),
                error: e.to_string(),
            }
        }),
        None => Ok(None),
    }
}

const OPENAI_CHAT_ROLE_MARKER: &str = "__lingua_openai_chat_role";
const OPENAI_CHAT_ROLE_WAS_STRING_MARKER: &str = "__lingua_openai_chat_role_was_string";

fn user_part_provider_options_mut(part: &mut UserContentPart) -> &mut Option<ProviderOptions> {
    match part {
        UserContentPart::Text(text_part) => &mut text_part.provider_options,
        UserContentPart::Image {
            provider_options, ..
        } => provider_options,
        UserContentPart::File {
            provider_options, ..
        } => provider_options,
    }
}

fn add_openai_chat_role_marker(part: &mut UserContentPart, role: &str, was_string: bool) {
    let opts = user_part_provider_options_mut(part).get_or_insert_with(|| ProviderOptions {
        options: serde_json::Map::new(),
    });
    opts.options.insert(
        OPENAI_CHAT_ROLE_MARKER.to_string(),
        serde_json::Value::String(role.to_string()),
    );
    if was_string {
        opts.options.insert(
            OPENAI_CHAT_ROLE_WAS_STRING_MARKER.to_string(),
            serde_json::Value::Bool(true),
        );
    }
}

fn mark_user_content_openai_role(content: UserContent, role: &str) -> UserContent {
    match content {
        UserContent::String(text) => {
            let mut part = UserContentPart::Text(TextContentPart {
                text,
                provider_options: None,
            });
            add_openai_chat_role_marker(&mut part, role, true);
            UserContent::Array(vec![part])
        }
        UserContent::Array(mut parts) => {
            if let Some(first) = parts.first_mut() {
                add_openai_chat_role_marker(first, role, false);
            }
            UserContent::Array(parts)
        }
    }
}

fn extract_openai_chat_role(
    content: UserContent,
) -> (
    UserContent,
    Option<openai::ChatCompletionRequestMessageRole>,
) {
    let UserContent::Array(mut parts) = content else {
        return (content, None);
    };

    let mut role_override = None;
    let mut was_string = false;

    if let Some(first) = parts.first_mut() {
        if let Some(opts) = user_part_provider_options_mut(first) {
            if let Some(role) = opts
                .options
                .remove(OPENAI_CHAT_ROLE_MARKER)
                .and_then(|v| v.as_str().map(str::to_string))
            {
                if role == "developer" {
                    role_override = Some(openai::ChatCompletionRequestMessageRole::Developer);
                }
            }
            was_string = opts
                .options
                .remove(OPENAI_CHAT_ROLE_WAS_STRING_MARKER)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if opts.options.is_empty() {
                *user_part_provider_options_mut(first) = None;
            }
        }
    }

    if was_string && parts.len() == 1 {
        let is_plain_text = matches!(
            &parts[0],
            UserContentPart::Text(TextContentPart {
                provider_options: None,
                ..
            })
        );
        if is_plain_text {
            let UserContentPart::Text(TextContentPart { text, .. }) = parts.remove(0) else {
                unreachable!("validated plain text part");
            };
            return (UserContent::String(text), role_override);
        }
    }

    (UserContent::Array(parts), role_override)
}

/// Convert OpenAI InputItem collection to universal Message collection
/// This handles OpenAI-specific logic for combining or transforming multiple items
impl TryFromLLM<Vec<openai::InputItem>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(inputs: Vec<openai::InputItem>) -> Result<Self, Self::Error> {
        let mut result = Vec::new();
        for mut input in inputs {
            match input.input_item_type {
                // Built-in tool calls - convert to ToolCall with provider_executed: true
                Some(openai::InputItemType::WebSearchCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "web_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": input.action,
                            "queries": input.queries,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::CodeInterpreterCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "code_interpreter".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "code": input.code,
                            "container_id": input.container_id,
                            "outputs": input.outputs,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::FileSearchCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "file_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "queries": input.queries,
                            "results": input.results,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::ComputerCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "computer".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": input.action,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::ImageGenerationCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "image_generation".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "result": input.result,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::LocalShellCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "local_shell".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": input.action,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::McpCall) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "mcp_call".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": input.server_label,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::McpListTools) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "mcp_list_tools".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": input.server_label,
                            "tools": input.tools,
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::McpApprovalRequest) => {
                    let tool_call = AssistantContentPart::ToolCall {
                        tool_call_id: input.id.clone().unwrap_or_default(),
                        tool_name: "mcp_approval_request".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "status": input.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    };
                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call]),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::Reasoning) => {
                    let mut summaries = vec![];
                    let mut first = true;
                    for summary in input.summary.unwrap_or_default() {
                        summaries.push(AssistantContentPart::Reasoning {
                            text: summary.text,
                            // OpenAI returns encrypted content on the message level, but may
                            // return multiple summary parts. To keep it simple, we just match this
                            // convention by putting the encrypted content on the first part.
                            encrypted_content: if first {
                                first = false;
                                input.encrypted_content.take()
                            } else {
                                None
                            },
                        });
                    }

                    if summaries.is_empty() {
                        // Handle case where there are no summary parts (empty reasoning). This way
                        // we stil get the encrypted content and make it clear that there was a
                        // reasoning step.
                        summaries.push(AssistantContentPart::Reasoning {
                            text: "".to_string(),
                            encrypted_content: input.encrypted_content.take(),
                        });
                    }

                    result.push(Message::Assistant {
                        content: AssistantContent::Array(summaries),
                        id: input.id,
                    });
                }
                Some(openai::InputItemType::FunctionCall)
                | Some(openai::InputItemType::CustomToolCall) => {
                    // Function calls are converted to tool calls in assistant messages
                    let tool_call_id =
                        input
                            .call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call call_id".to_string(),
                            })?;
                    let tool_name =
                        input
                            .name
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call name".to_string(),
                            })?;
                    let arguments_str = input
                        .arguments
                        .unwrap_or_else(|| EMPTY_OBJECT_STR.to_string());

                    let tool_call_part = AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments: arguments_str.into(),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: None,
                    };

                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call_part]),
                        id: input.id.clone(),
                    });
                }
                Some(openai::InputItemType::FunctionCallOutput)
                | Some(openai::InputItemType::CustomToolCallOutput) => {
                    // Function call outputs are converted to tool messages
                    let tool_call_id =
                        input
                            .call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call output call_id".to_string(),
                            })?;

                    let output = input.output.unwrap_or_else(|| "".to_string());

                    let output_value = serde_json::Value::String(output);

                    let tool_result = ToolResultContentPart {
                        tool_call_id,
                        tool_name: String::new(), // OpenAI doesn't provide tool name in output
                        output: output_value,
                        provider_options: None,
                    };

                    result.push(Message::Tool {
                        content: vec![ToolContentPart::ToolResult(tool_result)],
                    });
                }
                _ => {
                    let role = input
                        .role
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "role".to_string(),
                        })?;

                    let content =
                        input
                            .content
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "content".to_string(),
                            })?;

                    result.push(match role {
                        openai::InputItemRole::System | openai::InputItemRole::Developer => {
                            Message::System {
                                content: TryFromLLM::try_from(content)?,
                            }
                        }
                        openai::InputItemRole::User => Message::User {
                            content: TryFromLLM::try_from(content)?,
                        },
                        openai::InputItemRole::Assistant => Message::Assistant {
                            id: input.id,
                            content: TryFromLLM::try_from(content)?,
                        },
                    });
                }
            };
        }

        Ok(result)
    }
}

impl TryFromLLM<openai::InputItemContent> for UserContent {
    type Error = ConvertError;

    fn try_from(contents: openai::InputItemContent) -> Result<Self, Self::Error> {
        Ok(match contents {
            openai::InputItemContent::String(text) => UserContent::String(text),
            openai::InputItemContent::InputContentArray(parts) => {
                UserContent::Array(TryFromLLM::try_from(parts)?)
            }
        })
    }
}

impl TryFromLLM<openai::InputContent> for UserContentPart {
    type Error = ConvertError;

    fn try_from(value: openai::InputContent) -> Result<Self, Self::Error> {
        Ok(match value.input_content_type {
            openai::InputItemContentListType::InputText
            | openai::InputItemContentListType::OutputText => {
                UserContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    provider_options: None,
                })
            }
            // TODO: ToolCall and ToolResult content types - not yet implemented in generated types
            openai::InputItemContentListType::InputImage => {
                // Extract image URL from the InputContent
                let image_url =
                    value
                        .image_url
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "image_url".to_string(),
                        })?;

                // Preserve detail in provider_options
                let provider_options = if let Some(detail) = &value.detail {
                    let mut options = serde_json::Map::new();
                    options.insert(
                        "detail".to_string(),
                        serde_json::to_value(detail).map_err(|e| {
                            ConvertError::JsonSerializationFailed {
                                field: "detail".to_string(),
                                error: e.to_string(),
                            }
                        })?,
                    );
                    Some(crate::universal::message::ProviderOptions { options })
                } else {
                    None
                };

                // Parse data URLs to extract raw base64, keep HTTP URLs as-is
                let (image_data, media_type) =
                    if let Some(block) = parse_base64_data_url(&image_url) {
                        // Data URL: extract raw base64 and media type
                        (block.data, Some(block.media_type))
                    } else {
                        // HTTP URL or other: keep as-is with default media type
                        (image_url.clone(), Some("image/jpeg".to_string()))
                    };

                UserContentPart::Image {
                    image: serde_json::Value::String(image_data),
                    media_type,
                    provider_options,
                }
            }
            openai::InputItemContentListType::InputAudio => {
                // Handle audio input if needed in the future
                return Err(ConvertError::UnsupportedInputType {
                    type_info: "InputAudio content type".to_string(),
                });
            }
            openai::InputItemContentListType::InputFile => {
                // Handle file input if needed in the future
                return Err(ConvertError::UnsupportedInputType {
                    type_info: "InputFile content type".to_string(),
                });
            }
            openai::InputItemContentListType::ReasoningText => {
                // Handle reasoning text - treat as regular text for now
                UserContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    provider_options: None,
                })
            }
            openai::InputItemContentListType::Refusal => {
                // Handle refusal - treat as regular text for now
                UserContentPart::Text(TextContentPart {
                    text: value.text.unwrap_or_else(|| REFUSAL_TEXT.to_string()),
                    provider_options: None,
                })
            }
        })
    }
}

impl TryFromLLM<openai::InputItemContent> for AssistantContent {
    type Error = ConvertError;

    fn try_from(contents: openai::InputItemContent) -> Result<Self, Self::Error> {
        Ok(match contents {
            openai::InputItemContent::String(text) => AssistantContent::String(text),
            openai::InputItemContent::InputContentArray(parts) => {
                AssistantContent::Array(TryFromLLM::try_from(parts)?)
            }
        })
    }
}

// Add reverse conversions for the reciprocal pattern

impl TryFromLLM<UserContent> for openai::InputItemContent {
    type Error = ConvertError;

    fn try_from(content: UserContent) -> Result<Self, Self::Error> {
        Ok(match content {
            UserContent::String(text) => openai::InputItemContent::String(text),
            UserContent::Array(parts) => {
                let input_parts: Result<Vec<_>, _> =
                    parts.into_iter().map(TryFromLLM::try_from).collect();
                openai::InputItemContent::InputContentArray(input_parts?)
            }
        })
    }
}

impl TryFromLLM<UserContentPart> for openai::InputContent {
    type Error = ConvertError;

    fn try_from(part: UserContentPart) -> Result<Self, Self::Error> {
        Ok(match part {
            UserContentPart::Text(text_part) => openai::InputContent {
                input_content_type: openai::InputItemContentListType::InputText,
                text: Some(text_part.text),
                ..Default::default()
            },
            UserContentPart::Image {
                image,
                media_type,
                provider_options,
            } => {
                let image_str = match image {
                    serde_json::Value::String(url) => url,
                    _ => {
                        return Err(ConvertError::UnsupportedInputType {
                            type_info: format!("Image type must be string URL, got: {:?}", image),
                        })
                    }
                };

                // If we have raw base64 data (not a URL) and media_type, create a proper data URL
                let image_url = if !image_str.starts_with("data:")
                    && !image_str.starts_with("http://")
                    && !image_str.starts_with("https://")
                {
                    // Assume raw base64 data - create data URL with media_type
                    let mt = media_type.as_deref().unwrap_or("image/jpeg");
                    format!("data:{};base64,{}", mt, image_str)
                } else {
                    image_str
                };

                // Extract detail from provider_options if present
                let detail = provider_options
                    .as_ref()
                    .and_then(|opts| opts.options.get("detail"))
                    .and_then(|detail_val| serde_json::from_value(detail_val.clone()).ok());

                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::InputImage,
                    image_url: Some(image_url),
                    detail,
                    ..Default::default()
                }
            }
            _ => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!("UserContentPart variant: {:?}", part),
                })
            }
        })
    }
}

impl Default for openai::InputContent {
    fn default() -> Self {
        Self {
            text: None,
            input_content_type: openai::InputItemContentListType::InputText,
            detail: None,
            file_id: None,
            image_url: None,
            file_data: None,
            file_url: None,
            filename: None,
            input_audio: None,
            annotations: None,
            logprobs: None,
            refusal: None,
        }
    }
}

impl TryFromLLM<AssistantContent> for openai::InputItemContent {
    type Error = ConvertError;

    fn try_from(content: AssistantContent) -> Result<Self, Self::Error> {
        Ok(match content {
            AssistantContent::String(text) => openai::InputItemContent::String(text),
            AssistantContent::Array(parts) => {
                let input_parts: Result<Vec<_>, _> =
                    parts.into_iter().map(TryFromLLM::try_from).collect();
                openai::InputItemContent::InputContentArray(input_parts?)
            }
        })
    }
}

impl TryFromLLM<AssistantContentPart> for openai::InputContent {
    type Error = ConvertError;

    fn try_from(part: AssistantContentPart) -> Result<Self, Self::Error> {
        Ok(match part {
            AssistantContentPart::Text(text_part) => {
                // Extract annotations and logprobs from provider_options
                let annotations = text_part
                    .provider_options
                    .as_ref()
                    .and_then(|opts| opts.options.get("annotations"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let logprobs = text_part
                    .provider_options
                    .as_ref()
                    .and_then(|opts| opts.options.get("logprobs"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::OutputText,
                    text: Some(text_part.text),
                    annotations: Some(annotations),
                    logprobs: Some(logprobs),
                    ..Default::default()
                }
            }
            AssistantContentPart::ToolCall {
                tool_call_id: _,
                tool_name: _,
                arguments,
                ..
            } => openai::InputContent {
                input_content_type: openai::InputItemContentListType::OutputText,
                text: Some(format!("{}", arguments)),
                annotations: Some(vec![]),
                logprobs: Some(vec![]),
                ..Default::default()
            },
            AssistantContentPart::Reasoning {
                text,
                encrypted_content: _,
            } => {
                // Convert reasoning back to reasoning text content
                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::ReasoningText,
                    text: Some(text),
                    annotations: Some(vec![]),
                    logprobs: Some(vec![]),
                    ..Default::default()
                }
            }
            AssistantContentPart::ToolResult {
                tool_call_id: _,
                tool_name,
                output,
                ..
            } => {
                // Check for web search tool result marker from Anthropic
                let is_web_search = tool_name == "web_search"
                    || output.get("anthropic_type").and_then(|v| v.as_str())
                        == Some("web_search_tool_result");

                if is_web_search {
                    // Convert web search results to text representation for InputContent
                    // Extract search results content for display
                    let text = serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_string());
                    openai::InputContent {
                        input_content_type: openai::InputItemContentListType::OutputText,
                        text: Some(text),
                        annotations: Some(vec![]),
                        logprobs: Some(vec![]),
                        ..Default::default()
                    }
                } else {
                    return Err(ConvertError::UnsupportedInputType {
                        type_info: format!(
                            "AssistantContentPart::ToolResult for tool: {}",
                            tool_name
                        ),
                    });
                }
            }
            _ => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!("AssistantContentPart variant: {:?}", part),
                })
            }
        })
    }
}

impl TryFromLLM<openai::InputContent> for AssistantContentPart {
    type Error = ConvertError;

    fn try_from(value: openai::InputContent) -> Result<Self, Self::Error> {
        Ok(match value.input_content_type {
            openai::InputItemContentListType::InputText
            | openai::InputItemContentListType::OutputText => {
                // Build provider_options to preserve annotations and logprobs
                let provider_options = {
                    let mut options = serde_json::Map::new();
                    if let Some(annotations) = &value.annotations {
                        if !annotations.is_empty() {
                            options.insert(
                                "annotations".to_string(),
                                serde_json::to_value(annotations).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "annotations".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            );
                        }
                    }
                    if let Some(logprobs) = &value.logprobs {
                        if !logprobs.is_empty() {
                            options.insert(
                                "logprobs".to_string(),
                                serde_json::to_value(logprobs).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "logprobs".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            );
                        }
                    }
                    if options.is_empty() {
                        None
                    } else {
                        Some(crate::universal::message::ProviderOptions { options })
                    }
                };

                AssistantContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    provider_options,
                })
            }
            // TODO: ToolCall content type support - not yet implemented in generated types
            _ => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!("InputContent type: {:?}", value.input_content_type),
                });
            }
        })
    }
}

/// Default implementation for InputItem
impl Default for openai::InputItem {
    fn default() -> Self {
        Self {
            role: None,
            content: None,
            input_item_type: None,
            status: None,
            id: None,
            queries: None,
            results: None,
            action: None,
            call_id: None,
            pending_safety_checks: None,
            acknowledged_safety_checks: None,
            output: None,
            arguments: None,
            name: None,
            encrypted_content: None,
            summary: None,
            result: None,
            code: None,
            container_id: None,
            server_label: None,
            tools: None,
            approval_request_id: None,
            approve: None,
            reason: None,
            request_id: None,
            input: None,
            error: None,
            outputs: None,
        }
    }
}

/// Convert universal Message to OpenAI InputItem (for Responses API)
impl TryFromLLM<Message> for openai::InputItem {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::System { content } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::System),
                content: Some(TryFromLLM::try_from(content)?),
                ..Default::default()
            }),
            Message::User { content } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::User),
                content: Some(TryFromLLM::try_from(content)?),
                ..Default::default()
            }),
            Message::Assistant { content, id } => {
                match content {
                    AssistantContent::String(text) => Ok(openai::InputItem {
                        role: Some(openai::InputItemRole::Assistant),
                        content: Some(openai::InputItemContent::String(text)),
                        id,
                        input_item_type: Some(openai::InputItemType::Message),
                        status: Some(openai::FunctionCallItemStatus::Completed),
                        ..Default::default()
                    }),
                    AssistantContent::Array(parts) => {
                        let mut has_reasoning = false;
                        let mut encrypted_content = None;
                        let mut reasoning_parts: Vec<openai::SummaryText> = vec![];
                        let mut normal_parts: Vec<openai::InputContent> = vec![];
                        let mut tool_call_info: Option<(
                            String,
                            String,
                            ToolCallArguments,
                            Option<bool>,
                        )> = None; // (tool_call_id, name, arguments, provider_executed)

                        for part in parts {
                            match part {
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content: ec,
                                } => {
                                    has_reasoning = true;
                                    encrypted_content = ec;
                                    if !text.is_empty() {
                                        reasoning_parts.push(openai::SummaryText {
                                            text,
                                            summary_text_type: openai::SummaryType::SummaryText,
                                        });
                                    }
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    encrypted_content: _,
                                    provider_options: _,
                                    provider_executed,
                                } => {
                                    tool_call_info = Some((
                                        tool_call_id,
                                        tool_name,
                                        arguments.clone(),
                                        provider_executed,
                                    ));
                                }
                                _ => {
                                    normal_parts.push(TryFromLLM::try_from(part)?);
                                }
                            }
                        }

                        if has_reasoning {
                            if tool_call_info.is_some() || !normal_parts.is_empty() {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed reasoning and other content parts are not supported in OpenAI format".to_string(),
                                });
                            }

                            // Pure reasoning message - convert to reasoning InputItem
                            let reasoning_item = openai::InputItem {
                                role: None, // Don't set role for reasoning items - let the original data determine this
                                content: None,
                                input_item_type: Some(openai::InputItemType::Reasoning),
                                id: id.clone(),
                                summary: Some(reasoning_parts),
                                encrypted_content,
                                ..Default::default()
                            };
                            Ok(reasoning_item)
                        } else if let Some((call_id, name, arguments, provider_executed)) =
                            tool_call_info
                        {
                            if !normal_parts.is_empty() {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed tool call and normal content parts are not supported in OpenAI format".to_string(),
                                });
                            }

                            // Check if this is a provider-executed built-in tool
                            if provider_executed == Some(true) {
                                // Convert back to the appropriate built-in tool type based on tool_name
                                let args_value = match &arguments {
                                    ToolCallArguments::Valid(map) => {
                                        serde_json::Value::Object(map.clone())
                                    }
                                    ToolCallArguments::Invalid(s) => {
                                        serde_json::Value::String(s.clone())
                                    }
                                };

                                let (input_item_type, mut item) = match name.as_str() {
                                    "web_search" => (
                                        openai::InputItemType::WebSearchCall,
                                        openai::InputItem {
                                            action: parse_builtin_field(
                                                &args_value,
                                                "action",
                                                "web_search",
                                            )?,
                                            queries: parse_builtin_field(
                                                &args_value,
                                                "queries",
                                                "web_search",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "code_interpreter" => (
                                        openai::InputItemType::CodeInterpreterCall,
                                        openai::InputItem {
                                            code: args_value
                                                .get("code")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            container_id: args_value
                                                .get("container_id")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            outputs: parse_builtin_field(
                                                &args_value,
                                                "outputs",
                                                "code_interpreter",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "file_search" => (
                                        openai::InputItemType::FileSearchCall,
                                        openai::InputItem {
                                            queries: parse_builtin_field(
                                                &args_value,
                                                "queries",
                                                "file_search",
                                            )?,
                                            results: parse_builtin_field(
                                                &args_value,
                                                "results",
                                                "file_search",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "computer" => (
                                        openai::InputItemType::ComputerCall,
                                        openai::InputItem {
                                            action: parse_builtin_field(
                                                &args_value,
                                                "action",
                                                "computer",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "image_generation" => (
                                        openai::InputItemType::ImageGenerationCall,
                                        openai::InputItem {
                                            result: args_value
                                                .get("result")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            ..Default::default()
                                        },
                                    ),
                                    "local_shell" => (
                                        openai::InputItemType::LocalShellCall,
                                        openai::InputItem {
                                            action: parse_builtin_field(
                                                &args_value,
                                                "action",
                                                "local_shell",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "mcp_call" => (
                                        openai::InputItemType::McpCall,
                                        openai::InputItem {
                                            server_label: args_value
                                                .get("server_label")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            ..Default::default()
                                        },
                                    ),
                                    "mcp_list_tools" => (
                                        openai::InputItemType::McpListTools,
                                        openai::InputItem {
                                            server_label: args_value
                                                .get("server_label")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            tools: parse_builtin_field(
                                                &args_value,
                                                "tools",
                                                "mcp_list_tools",
                                            )?,
                                            ..Default::default()
                                        },
                                    ),
                                    "mcp_approval_request" => (
                                        openai::InputItemType::McpApprovalRequest,
                                        openai::InputItem {
                                            ..Default::default()
                                        },
                                    ),
                                    _ => {
                                        // Unknown provider-executed tool - fall back to FunctionCall
                                        return Ok(openai::InputItem {
                                            role: None,
                                            content: None,
                                            input_item_type: Some(
                                                openai::InputItemType::FunctionCall,
                                            ),
                                            id: id.clone(),
                                            call_id: Some(call_id),
                                            name: Some(name),
                                            arguments: Some(arguments.to_string()),
                                            status: Some(openai::FunctionCallItemStatus::Completed),
                                            ..Default::default()
                                        });
                                    }
                                };

                                // Set common fields
                                item.id = id.clone();
                                item.input_item_type = Some(input_item_type);
                                item.status = args_value
                                    .get("status")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                                Ok(item)
                            } else {
                                // Regular function call (not provider-executed)
                                let function_call_item = openai::InputItem {
                                    role: None, // Preserve original role state - request context function calls don't have roles
                                    content: None,
                                    input_item_type: Some(openai::InputItemType::FunctionCall),
                                    id: id.clone(),
                                    call_id: Some(call_id),
                                    name: Some(name),
                                    arguments: Some(arguments.to_string()),
                                    status: Some(openai::FunctionCallItemStatus::Completed),
                                    ..Default::default()
                                };
                                Ok(function_call_item)
                            }
                        } else {
                            // Regular message - use normal conversion
                            Ok(openai::InputItem {
                                role: Some(openai::InputItemRole::Assistant),
                                content: Some(openai::InputItemContent::InputContentArray(
                                    normal_parts,
                                )),
                                input_item_type: Some(openai::InputItemType::Message),
                                id,
                                status: Some(openai::FunctionCallItemStatus::Completed),
                                ..Default::default()
                            })
                        }
                    }
                }
            }
            Message::Tool { content } => {
                // Convert tool results to appropriate InputItems
                let mut result_items = Vec::new();

                for tool_part in content {
                    match tool_part {
                        ToolContentPart::ToolResult(tool_result) => {
                            // Convert tool result output to string for Refusal::String
                            let output_string = match &tool_result.output {
                                serde_json::Value::String(s) => s.clone(),
                                other => serde_json::to_string(other).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "tool_result_output".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            };

                            // Create a tool result InputItem using FunctionCallOutput type
                            result_items.push(openai::InputItem {
                                role: None,    // Function call outputs don't have roles
                                content: None, // Function call outputs use the output field, not content
                                input_item_type: Some(openai::InputItemType::FunctionCallOutput),
                                call_id: Some(tool_result.tool_call_id.clone()),
                                output: Some(output_string),
                                ..Default::default()
                            });
                        }
                    }
                }

                // For now, return the first tool result or a placeholder
                result_items.into_iter().next().ok_or_else(|| {
                    ConvertError::ContentConversionFailed {
                        reason: "Empty tool content".to_string(),
                    }
                })
            }
        }
    }
}

/// Create an InputItem for a function call (regular or built-in tool).
///
/// This helper extracts the logic for converting a universal tool call to an OpenAI InputItem,
/// handling both provider-executed built-in tools and regular function calls.
fn create_function_call_input_item(
    call_id: &str,
    name: &str,
    arguments: &ToolCallArguments,
    provider_executed: Option<bool>,
    id: Option<String>,
) -> Result<openai::InputItem, ConvertError> {
    // Check if this is a provider-executed built-in tool
    if provider_executed == Some(true) {
        // Convert back to the appropriate built-in tool type based on tool_name
        let args_value = match &arguments {
            ToolCallArguments::Valid(map) => serde_json::Value::Object(map.clone()),
            ToolCallArguments::Invalid(s) => serde_json::Value::String(s.clone()),
        };

        let (input_item_type, mut item) = match name {
            "web_search" => (
                openai::InputItemType::WebSearchCall,
                openai::InputItem {
                    action: args_value
                        .get("action")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    queries: args_value
                        .get("queries")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "code_interpreter" => (
                openai::InputItemType::CodeInterpreterCall,
                openai::InputItem {
                    code: args_value
                        .get("code")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    container_id: args_value
                        .get("container_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    outputs: args_value
                        .get("outputs")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "file_search" => (
                openai::InputItemType::FileSearchCall,
                openai::InputItem {
                    queries: args_value
                        .get("queries")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    results: args_value
                        .get("results")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "computer" => (
                openai::InputItemType::ComputerCall,
                openai::InputItem {
                    action: args_value
                        .get("action")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "image_generation" => (
                openai::InputItemType::ImageGenerationCall,
                openai::InputItem {
                    result: args_value
                        .get("result")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ..Default::default()
                },
            ),
            "local_shell" => (
                openai::InputItemType::LocalShellCall,
                openai::InputItem {
                    action: args_value
                        .get("action")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "mcp_call" => (
                openai::InputItemType::McpCall,
                openai::InputItem {
                    server_label: args_value
                        .get("server_label")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ..Default::default()
                },
            ),
            "mcp_list_tools" => (
                openai::InputItemType::McpListTools,
                openai::InputItem {
                    server_label: args_value
                        .get("server_label")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    tools: args_value
                        .get("tools")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    ..Default::default()
                },
            ),
            "mcp_approval_request" => (
                openai::InputItemType::McpApprovalRequest,
                openai::InputItem {
                    ..Default::default()
                },
            ),
            _ => {
                // Unknown provider-executed tool - fall back to FunctionCall
                return Ok(openai::InputItem {
                    role: None,
                    content: None,
                    input_item_type: Some(openai::InputItemType::FunctionCall),
                    id,
                    call_id: Some(call_id.to_string()),
                    name: Some(name.to_string()),
                    arguments: Some(arguments.to_string()),
                    status: Some(openai::FunctionCallItemStatus::Completed),
                    ..Default::default()
                });
            }
        };

        // Set common fields
        item.id = id;
        item.input_item_type = Some(input_item_type);
        item.status = args_value
            .get("status")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        Ok(item)
    } else {
        // Regular function call (not provider-executed)
        Ok(openai::InputItem {
            role: None, // Preserve original role state - request context function calls don't have roles
            content: None,
            input_item_type: Some(openai::InputItemType::FunctionCall),
            id,
            call_id: Some(call_id.to_string()),
            name: Some(name.to_string()),
            arguments: Some(arguments.to_string()),
            status: Some(openai::FunctionCallItemStatus::Completed),
            ..Default::default()
        })
    }
}

/// Convert universal messages to OpenAI Responses API InputItem format.
///
/// This function handles the 1:N expansion for Tool messages - a single Tool message
/// can contain multiple tool results, and each result becomes a separate InputItem
/// (which is required by the Responses API).
///
/// It also handles 1:N expansion for Assistant messages with mixed content (reasoning,
/// text, and tool calls). Each content type becomes a separate InputItem in order:
/// 1. Reasoning item (if reasoning parts exist)
/// 2. Message item (if text/normal parts exist)
/// 3. Function call items (one per tool call)
///
/// This is provided as a standalone function rather than a TryFromLLM impl because
/// Rust's coherence rules don't allow overriding the blanket Vec implementation.
pub fn universal_to_responses_input(
    messages: &[Message],
) -> Result<Vec<openai::InputItem>, ConvertError> {
    let mut result = Vec::with_capacity(messages.len());

    for msg in messages {
        match msg {
            Message::Tool { content } => {
                // Expand: one Tool message → multiple InputItems
                for tool_part in content {
                    match tool_part {
                        ToolContentPart::ToolResult(tool_result) => {
                            let output_string = match &tool_result.output {
                                serde_json::Value::String(s) => s.clone(),
                                other => serde_json::to_string(other).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "tool_result_output".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            };

                            result.push(openai::InputItem {
                                role: None,
                                content: None,
                                input_item_type: Some(openai::InputItemType::FunctionCallOutput),
                                call_id: Some(tool_result.tool_call_id.clone()),
                                output: Some(output_string),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            Message::Assistant { content, id } => {
                // Handle assistant messages with potential 1:N expansion for mixed content
                match content {
                    AssistantContent::String(text) => {
                        // Simple case: single message item
                        result.push(openai::InputItem {
                            role: Some(openai::InputItemRole::Assistant),
                            content: Some(openai::InputItemContent::String(text.clone())),
                            id: id.clone(),
                            input_item_type: Some(openai::InputItemType::Message),
                            status: Some(openai::FunctionCallItemStatus::Completed),
                            ..Default::default()
                        });
                    }
                    AssistantContent::Array(parts) => {
                        // Categorize all parts into separate collections
                        let mut reasoning_parts: Vec<openai::SummaryText> = vec![];
                        let mut has_reasoning = false;
                        let mut encrypted_content = None;
                        let mut normal_parts: Vec<openai::InputContent> = vec![];
                        let mut tool_calls: Vec<(String, String, ToolCallArguments, Option<bool>)> =
                            vec![];

                        for part in parts {
                            match part {
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content: ec,
                                } => {
                                    has_reasoning = true;
                                    encrypted_content = ec.clone();
                                    if !text.is_empty() {
                                        reasoning_parts.push(openai::SummaryText {
                                            text: text.clone(),
                                            summary_text_type: openai::SummaryType::SummaryText,
                                        });
                                    }
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    encrypted_content: _,
                                    provider_options: _,
                                    provider_executed,
                                } => {
                                    tool_calls.push((
                                        tool_call_id.clone(),
                                        tool_name.clone(),
                                        arguments.clone(),
                                        *provider_executed,
                                    ));
                                }
                                other_part => {
                                    normal_parts.push(TryFromLLM::try_from(other_part.clone())?);
                                }
                            }
                        }

                        // 1. Emit reasoning item if any reasoning part existed (even with empty text)
                        if has_reasoning {
                            result.push(openai::InputItem {
                                role: None,
                                content: None,
                                input_item_type: Some(openai::InputItemType::Reasoning),
                                id: id.clone(),
                                summary: Some(reasoning_parts),
                                encrypted_content: encrypted_content.clone(),
                                ..Default::default()
                            });
                        }

                        // 2. Emit message item if normal parts present
                        if !normal_parts.is_empty() {
                            result.push(openai::InputItem {
                                role: Some(openai::InputItemRole::Assistant),
                                content: Some(openai::InputItemContent::InputContentArray(
                                    normal_parts,
                                )),
                                input_item_type: Some(openai::InputItemType::Message),
                                // Only clear id if reasoning was emitted (it used the id)
                                id: if has_reasoning { None } else { id.clone() },
                                status: Some(openai::FunctionCallItemStatus::Completed),
                                ..Default::default()
                            });
                        }

                        // 3. Emit function call items (one per tool call)
                        for (call_id, name, arguments, provider_executed) in tool_calls {
                            result.push(create_function_call_input_item(
                                &call_id,
                                &name,
                                &arguments,
                                provider_executed,
                                id.clone(),
                            )?);
                        }
                    }
                }
            }
            other => {
                // For all other message types, use the standard conversion
                result.push(<openai::InputItem as TryFromLLM<Message>>::try_from(
                    other.clone(),
                )?);
            }
        }
    }

    Ok(result)
}

/// Convert OutputItem to InputItem for unified processing
/// OutputItem is used in responses while InputItem is used in requests,
/// but they have very similar structure for message content
impl TryFromLLM<openai::OutputItem> for openai::InputItem {
    type Error = ConvertError;

    fn try_from(output_item: openai::OutputItem) -> Result<Self, Self::Error> {
        // Convert OutputItem to InputItem by mapping the fields
        // The main differences are in content type and some field names

        let input_item_type = match output_item.output_item_type {
            Some(openai::OutputItemType::Message) => Some(openai::InputItemType::Message),
            Some(openai::OutputItemType::Reasoning) => Some(openai::InputItemType::Reasoning),
            Some(openai::OutputItemType::FunctionCall) => Some(openai::InputItemType::FunctionCall),
            Some(openai::OutputItemType::CustomToolCall) => {
                Some(openai::InputItemType::CustomToolCall)
            }
            // Map built-in tool types for proper handling during conversion
            Some(openai::OutputItemType::CodeInterpreterCall) => {
                Some(openai::InputItemType::CodeInterpreterCall)
            }
            Some(openai::OutputItemType::WebSearchCall) => {
                Some(openai::InputItemType::WebSearchCall)
            }
            Some(openai::OutputItemType::FileSearchCall) => {
                Some(openai::InputItemType::FileSearchCall)
            }
            Some(openai::OutputItemType::ComputerCall) => Some(openai::InputItemType::ComputerCall),
            Some(openai::OutputItemType::ImageGenerationCall) => {
                Some(openai::InputItemType::ImageGenerationCall)
            }
            Some(openai::OutputItemType::LocalShellCall) => {
                Some(openai::InputItemType::LocalShellCall)
            }
            Some(openai::OutputItemType::McpCall) => Some(openai::InputItemType::McpCall),
            Some(openai::OutputItemType::McpListTools) => Some(openai::InputItemType::McpListTools),
            Some(openai::OutputItemType::McpApprovalRequest) => {
                Some(openai::InputItemType::McpApprovalRequest)
            }
            // For other types, we might need to map them or handle specially
            _ => None, // Will be handled based on content
        };

        // Convert content from Vec<OutputMessageContent> to InputItemContent
        let content = if let Some(output_content) = output_item.content {
            if output_content.is_empty() {
                None
            } else if output_content.len() == 1 {
                // Single content item - check if we can convert to string
                // Only convert to string if there are no annotations or other metadata to preserve
                let first = &output_content[0];
                let has_annotations = first
                    .annotations
                    .as_ref()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                let has_logprobs = first
                    .logprobs
                    .as_ref()
                    .map(|l| !l.is_empty())
                    .unwrap_or(false);

                if first.output_message_content_type == openai::ContentType::OutputText
                    && !has_annotations
                    && !has_logprobs
                {
                    output_content
                        .into_iter()
                        .next()
                        .unwrap()
                        .text
                        .map(openai::InputItemContent::String)
                } else {
                    // Convert to InputContent array to preserve annotations/logprobs
                    let input_contents: Result<Vec<_>, _> = output_content
                        .into_iter()
                        .map(convert_output_message_content_to_input_content)
                        .collect();
                    Some(openai::InputItemContent::InputContentArray(input_contents?))
                }
            } else {
                // Multiple content items - convert to array
                let input_contents: Result<Vec<_>, _> = output_content
                    .into_iter()
                    .map(convert_output_message_content_to_input_content)
                    .collect();
                Some(openai::InputItemContent::InputContentArray(input_contents?))
            }
        } else {
            None
        };

        // Convert role from MessageRole to InputItemRole
        // If no role is provided, infer it from the output_item_type only for certain types
        let role = output_item
            .role
            .map(|mr| match mr {
                openai::MessageRole::Assistant => openai::InputItemRole::Assistant,
            })
            .or({
                // Only infer role for regular messages, not for function calls or other items
                // Function calls and other tool-related items should preserve their original role state
                match output_item.output_item_type {
                    Some(openai::OutputItemType::Message) => Some(openai::InputItemRole::Assistant),
                    _ => None, // Don't infer role for function calls, reasoning, and other types
                }
            });

        // Convert status
        let status = output_item.status;

        // Handle reasoning summary conversion - OutputItem has summary field
        let summary = output_item.summary;

        Ok(openai::InputItem {
            role,
            content,
            input_item_type,
            status,
            id: output_item.id,
            summary,
            // Preserve structured function call fields
            arguments: output_item.arguments,
            name: output_item.name,
            // Set other fields to None/default - many OutputItem fields don't have InputItem equivalents
            queries: output_item.queries,
            call_id: output_item.call_id,
            results: output_item.results,
            action: output_item.action,
            pending_safety_checks: output_item.pending_safety_checks,
            acknowledged_safety_checks: None,
            output: None,
            encrypted_content: output_item.encrypted_content,
            result: output_item.result,
            code: output_item.code,
            container_id: output_item.container_id,
            outputs: output_item.outputs,
            error: output_item.error,
            server_label: output_item.server_label,
            tools: output_item.tools,
            approval_request_id: None,
            approve: None,
            reason: None,
            request_id: None,
            input: output_item.input,
        })
    }
}

/// Convert InputItem to OutputItem (reverse of OutputItem -> InputItem conversion)
impl TryFromLLM<openai::InputItem> for openai::OutputItem {
    type Error = ConvertError;

    fn try_from(input_item: openai::InputItem) -> Result<Self, Self::Error> {
        // Convert InputItem to OutputItem by mapping the fields
        let output_item_type = match input_item.input_item_type {
            Some(openai::InputItemType::Message) => Some(openai::OutputItemType::Message),
            Some(openai::InputItemType::Reasoning) => Some(openai::OutputItemType::Reasoning),
            Some(openai::InputItemType::FunctionCall) => Some(openai::OutputItemType::FunctionCall),
            Some(openai::InputItemType::CustomToolCall) => {
                Some(openai::OutputItemType::CustomToolCall)
            }
            // Built-in tool types
            Some(openai::InputItemType::CodeInterpreterCall) => {
                Some(openai::OutputItemType::CodeInterpreterCall)
            }
            Some(openai::InputItemType::WebSearchCall) => {
                Some(openai::OutputItemType::WebSearchCall)
            }
            Some(openai::InputItemType::FileSearchCall) => {
                Some(openai::OutputItemType::FileSearchCall)
            }
            Some(openai::InputItemType::ComputerCall) => Some(openai::OutputItemType::ComputerCall),
            Some(openai::InputItemType::ImageGenerationCall) => {
                Some(openai::OutputItemType::ImageGenerationCall)
            }
            Some(openai::InputItemType::LocalShellCall) => {
                Some(openai::OutputItemType::LocalShellCall)
            }
            Some(openai::InputItemType::McpCall) => Some(openai::OutputItemType::McpCall),
            Some(openai::InputItemType::McpListTools) => Some(openai::OutputItemType::McpListTools),
            Some(openai::InputItemType::McpApprovalRequest) => {
                Some(openai::OutputItemType::McpApprovalRequest)
            }
            _ => None,
        };

        // Convert content from InputItemContent to Vec<OutputMessageContent>
        let content = if let Some(input_content) = input_item.content {
            match input_content {
                openai::InputItemContent::String(text) => {
                    // Single string content becomes single OutputMessageContent
                    Some(vec![openai::OutputMessageContent {
                        output_message_content_type: openai::ContentType::OutputText,
                        text: Some(text),
                        annotations: Some(vec![]),
                        logprobs: Some(vec![]),
                        refusal: None,
                    }])
                }
                openai::InputItemContent::InputContentArray(input_contents) => {
                    // Convert InputContent array to OutputMessageContent array
                    let output_contents: Result<Vec<_>, _> = input_contents
                        .into_iter()
                        .map(convert_input_content_to_output_message_content)
                        .collect();
                    Some(output_contents?)
                }
            }
        } else {
            None
        };

        // Convert role from InputItemRole to MessageRole
        let role = input_item.role.and_then(|ir| match ir {
            openai::InputItemRole::Assistant => Some(openai::MessageRole::Assistant),
            _ => None, // OutputItem only supports Assistant role
        });

        Ok(openai::OutputItem {
            role,
            content,
            output_item_type,
            status: input_item.status,
            id: input_item.id,
            summary: input_item.summary,
            arguments: input_item.arguments,
            name: input_item.name,
            queries: input_item.queries,
            call_id: input_item.call_id,
            results: input_item.results,
            action: input_item.action,
            pending_safety_checks: input_item.pending_safety_checks,
            encrypted_content: input_item.encrypted_content,
            result: input_item.result,
            code: input_item.code,
            container_id: input_item.container_id,
            outputs: input_item.outputs,
            error: input_item.error,
            output: input_item.output,
            server_label: input_item.server_label,
            tools: input_item.tools,
            input: input_item.input,
        })
    }
}

/// Convert OpenAI OutputItem collection to universal Message collection.
/// Each OutputItem becomes a separate Message to preserve the structure.
impl TryFromLLM<Vec<openai::OutputItem>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(items: Vec<openai::OutputItem>) -> Result<Vec<Message>, Self::Error> {
        let mut messages: Vec<Message> = Vec::new();

        for mut item in items {
            let item_id = item.id.clone();

            let parts: Vec<AssistantContentPart> = match item.output_item_type {
                Some(openai::OutputItemType::Message) => {
                    // Extract text content from message output items
                    let mut text_parts = Vec::new();
                    if let Some(content) = item.content {
                        for c in content {
                            if let Some(text) = c.text {
                                // Preserve annotations and logprobs in provider_options
                                let provider_options =
                                    if c.annotations.is_some() || c.logprobs.is_some() {
                                        let mut options = serde_json::Map::new();
                                        if let Some(annotations) = c.annotations {
                                            if let Ok(value) = serde_json::to_value(&annotations) {
                                                options.insert("annotations".to_string(), value);
                                            }
                                        }
                                        if let Some(logprobs) = c.logprobs {
                                            if let Ok(value) = serde_json::to_value(&logprobs) {
                                                options.insert("logprobs".to_string(), value);
                                            }
                                        }
                                        if options.is_empty() {
                                            None
                                        } else {
                                            Some(ProviderOptions { options })
                                        }
                                    } else {
                                        None
                                    };
                                text_parts.push(AssistantContentPart::Text(TextContentPart {
                                    text,
                                    provider_options,
                                }));
                            }
                        }
                    }
                    text_parts
                }
                Some(openai::OutputItemType::Reasoning) => {
                    // Convert reasoning output to reasoning content parts
                    let mut reasoning_parts = Vec::new();
                    let mut first = true;
                    for summary in item.summary.unwrap_or_default() {
                        reasoning_parts.push(AssistantContentPart::Reasoning {
                            text: summary.text,
                            encrypted_content: if first {
                                first = false;
                                item.encrypted_content.take()
                            } else {
                                None
                            },
                        });
                    }
                    // Handle empty reasoning (still preserve encrypted content)
                    if first {
                        reasoning_parts.push(AssistantContentPart::Reasoning {
                            text: String::new(),
                            encrypted_content: item.encrypted_content.take(),
                        });
                    }
                    reasoning_parts
                }
                Some(openai::OutputItemType::FunctionCall) => {
                    let tool_call_id =
                        item.call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call call_id".to_string(),
                            })?;
                    let tool_name =
                        item.name
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call name".to_string(),
                            })?;
                    let arguments_str = item
                        .arguments
                        .unwrap_or_else(|| EMPTY_OBJECT_STR.to_string());

                    vec![AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments: arguments_str.into(),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: None,
                    }]
                }
                Some(openai::OutputItemType::CodeInterpreterCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "code_interpreter".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "code": item.code,
                            "container_id": item.container_id,
                            "outputs": item.outputs,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::WebSearchCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "web_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": item.action,
                            "queries": item.queries,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::FileSearchCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "file_search".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "queries": item.queries,
                            "results": item.results,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::ComputerCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "computer".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": item.action,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::ImageGenerationCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "image_generation".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "result": item.result,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::LocalShellCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "local_shell".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "action": item.action,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::McpCall) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "mcp_call".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": item.server_label,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::McpListTools) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "mcp_list_tools".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "server_label": item.server_label,
                            "tools": item.tools,
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                Some(openai::OutputItemType::McpApprovalRequest) => {
                    vec![AssistantContentPart::ToolCall {
                        tool_call_id: item.id.clone().unwrap_or_default(),
                        tool_name: "mcp_approval_request".to_string(),
                        arguments: build_tool_arguments(&serde_json::json!({
                            "status": item.status,
                        })),
                        encrypted_content: None,
                        provider_options: None,
                        provider_executed: Some(true),
                    }]
                }
                _ => {
                    // Skip unknown output item types
                    continue;
                }
            };

            // Only create a message if there are parts
            if !parts.is_empty() {
                messages.push(Message::Assistant {
                    content: AssistantContent::Array(parts),
                    id: item_id,
                });
            }
        }

        Ok(messages)
    }
}

/// Convert universal Message collection to OpenAI OutputItem collection
/// This leverages the Message -> InputItem -> OutputItem conversion chain
/// Convert universal Message collection to OpenAI OutputItem collection.
/// This directly converts content parts to OutputItems, preserving order.
impl TryFromLLM<Vec<Message>> for Vec<openai::OutputItem> {
    type Error = ConvertError;

    fn try_from(messages: Vec<Message>) -> Result<Self, Self::Error> {
        let mut result = Vec::new();

        for msg in messages {
            if let Message::Assistant { content, id } = msg {
                match content {
                    AssistantContent::String(text) => {
                        result.push(openai::OutputItem {
                            output_item_type: Some(openai::OutputItemType::Message),
                            role: Some(openai::MessageRole::Assistant),
                            content: Some(vec![openai::OutputMessageContent {
                                output_message_content_type: openai::ContentType::OutputText,
                                text: Some(text),
                                annotations: None,
                                logprobs: None,
                                refusal: None,
                            }]),
                            id,
                            status: Some(openai::FunctionCallItemStatus::Completed),
                            ..Default::default()
                        });
                    }
                    AssistantContent::Array(parts) => {
                        // Track whether we've assigned the id to prevent duplicate IDs
                        let mut id_used = false;
                        let use_id = |used: &mut bool, id: &Option<String>| -> Option<String> {
                            if *used {
                                None
                            } else {
                                *used = true;
                                id.clone()
                            }
                        };

                        // Collect consecutive reasoning parts into a single OutputItem
                        let mut pending_reasoning_summaries: Vec<openai::SummaryText> = vec![];
                        let mut pending_encrypted_content: Option<String> = None;
                        let mut has_pending_reasoning = false;

                        let flush_reasoning =
                            |result: &mut Vec<openai::OutputItem>,
                             summaries: &mut Vec<openai::SummaryText>,
                             encrypted: &mut Option<String>,
                             has_reasoning: &mut bool,
                             id_used: &mut bool,
                             id: &Option<String>| {
                                if *has_reasoning {
                                    let use_id_inner =
                                        |used: &mut bool, id: &Option<String>| -> Option<String> {
                                            if *used {
                                                None
                                            } else {
                                                *used = true;
                                                id.clone()
                                            }
                                        };
                                    result.push(openai::OutputItem {
                                        output_item_type: Some(openai::OutputItemType::Reasoning),
                                        summary: Some(std::mem::take(summaries)),
                                        encrypted_content: encrypted.take(),
                                        id: use_id_inner(id_used, id),
                                        ..Default::default()
                                    });
                                    *has_reasoning = false;
                                }
                            };

                        for part in parts {
                            match part {
                                AssistantContentPart::Text(text_part) => {
                                    // Flush any pending reasoning before text
                                    flush_reasoning(
                                        &mut result,
                                        &mut pending_reasoning_summaries,
                                        &mut pending_encrypted_content,
                                        &mut has_pending_reasoning,
                                        &mut id_used,
                                        &id,
                                    );
                                    // Extract annotations and logprobs from provider_options
                                    let (annotations, logprobs) = if let Some(ref opts) =
                                        text_part.provider_options
                                    {
                                        let annotations =
                                            opts.options.get("annotations").and_then(|v| {
                                                serde_json::from_value::<Vec<openai::Annotation>>(
                                                    v.clone(),
                                                )
                                                .ok()
                                            });
                                        let logprobs = opts.options.get("logprobs").and_then(|v| {
                                            serde_json::from_value::<Vec<openai::LogProbability>>(
                                                v.clone(),
                                            )
                                            .ok()
                                        });
                                        (annotations, logprobs)
                                    } else {
                                        (None, None)
                                    };
                                    result.push(openai::OutputItem {
                                        output_item_type: Some(openai::OutputItemType::Message),
                                        role: Some(openai::MessageRole::Assistant),
                                        content: Some(vec![openai::OutputMessageContent {
                                            output_message_content_type:
                                                openai::ContentType::OutputText,
                                            text: Some(text_part.text),
                                            annotations,
                                            logprobs,
                                            refusal: None,
                                        }]),
                                        id: use_id(&mut id_used, &id),
                                        status: Some(openai::FunctionCallItemStatus::Completed),
                                        ..Default::default()
                                    });
                                }
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content,
                                } => {
                                    // Accumulate reasoning summaries
                                    has_pending_reasoning = true;
                                    if !text.is_empty() {
                                        pending_reasoning_summaries.push(openai::SummaryText {
                                            text,
                                            summary_text_type: openai::SummaryType::SummaryText,
                                        });
                                    }
                                    if encrypted_content.is_some() {
                                        pending_encrypted_content = encrypted_content;
                                    }
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    provider_executed,
                                    ..
                                } => {
                                    // Flush any pending reasoning before tool call
                                    flush_reasoning(
                                        &mut result,
                                        &mut pending_reasoning_summaries,
                                        &mut pending_encrypted_content,
                                        &mut has_pending_reasoning,
                                        &mut id_used,
                                        &id,
                                    );
                                    if provider_executed == Some(true) {
                                        // Built-in tool: convert to appropriate OutputItem type
                                        let args_value = match &arguments {
                                            ToolCallArguments::Valid(map) => {
                                                serde_json::Value::Object(map.clone())
                                            }
                                            ToolCallArguments::Invalid(s) => {
                                                serde_json::Value::String(s.clone())
                                            }
                                        };

                                        let item = match tool_name.as_str() {
                                            "code_interpreter" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::CodeInterpreterCall,
                                                ),
                                                id: Some(tool_call_id),
                                                code: args_value
                                                    .get("code")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string()),
                                                container_id: args_value
                                                    .get("container_id")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string()),
                                                outputs: args_value.get("outputs").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "web_search" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::WebSearchCall,
                                                ),
                                                id: Some(tool_call_id),
                                                action: args_value.get("action").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                queries: args_value.get("queries").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "file_search" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::FileSearchCall,
                                                ),
                                                id: Some(tool_call_id),
                                                queries: args_value.get("queries").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                results: args_value.get("results").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "computer" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::ComputerCall,
                                                ),
                                                id: Some(tool_call_id),
                                                action: args_value.get("action").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "image_generation" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::ImageGenerationCall,
                                                ),
                                                id: Some(tool_call_id),
                                                result: args_value
                                                    .get("result")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string()),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "local_shell" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::LocalShellCall,
                                                ),
                                                id: Some(tool_call_id),
                                                action: args_value.get("action").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "mcp_call" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::McpCall,
                                                ),
                                                id: Some(tool_call_id),
                                                server_label: args_value
                                                    .get("server_label")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string()),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "mcp_list_tools" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::McpListTools,
                                                ),
                                                id: Some(tool_call_id),
                                                server_label: args_value
                                                    .get("server_label")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string()),
                                                tools: args_value.get("tools").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            "mcp_approval_request" => openai::OutputItem {
                                                output_item_type: Some(
                                                    openai::OutputItemType::McpApprovalRequest,
                                                ),
                                                id: Some(tool_call_id),
                                                status: args_value.get("status").and_then(|v| {
                                                    serde_json::from_value(v.clone()).ok()
                                                }),
                                                ..Default::default()
                                            },
                                            _ => {
                                                // Unknown provider-executed tool - fall back to FunctionCall
                                                openai::OutputItem {
                                                    output_item_type: Some(
                                                        openai::OutputItemType::FunctionCall,
                                                    ),
                                                    call_id: Some(tool_call_id),
                                                    name: Some(tool_name),
                                                    arguments: Some(arguments.to_string()),
                                                    status: Some(
                                                        openai::FunctionCallItemStatus::Completed,
                                                    ),
                                                    ..Default::default()
                                                }
                                            }
                                        };
                                        result.push(item);
                                    } else {
                                        // Regular function call
                                        result.push(openai::OutputItem {
                                            output_item_type: Some(
                                                openai::OutputItemType::FunctionCall,
                                            ),
                                            id: use_id(&mut id_used, &id),
                                            call_id: Some(tool_call_id),
                                            name: Some(tool_name),
                                            arguments: Some(arguments.to_string()),
                                            status: Some(openai::FunctionCallItemStatus::Completed),
                                            ..Default::default()
                                        });
                                    }
                                }
                                // Skip File and ToolResult variants as they don't map to OutputItems
                                _ => {}
                            }
                        }
                        // Flush any remaining pending reasoning at the end
                        flush_reasoning(
                            &mut result,
                            &mut pending_reasoning_summaries,
                            &mut pending_encrypted_content,
                            &mut has_pending_reasoning,
                            &mut id_used,
                            &id,
                        );
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Helper function to convert InputContent to OutputMessageContent
fn convert_input_content_to_output_message_content(
    input_content: openai::InputContent,
) -> Result<openai::OutputMessageContent, ConvertError> {
    match input_content.input_content_type {
        openai::InputItemContentListType::OutputText
        | openai::InputItemContentListType::InputText => Ok(openai::OutputMessageContent {
            output_message_content_type: openai::ContentType::OutputText,
            text: input_content.text,
            annotations: input_content.annotations,
            logprobs: input_content.logprobs,
            refusal: input_content.refusal,
        }),
        openai::InputItemContentListType::ReasoningText => Ok(openai::OutputMessageContent {
            output_message_content_type: openai::ContentType::OutputText,
            text: input_content.text,
            annotations: input_content.annotations,
            logprobs: input_content.logprobs,
            refusal: input_content.refusal,
        }),
        _ => {
            // For other content types, try to preserve as much information as possible
            Ok(openai::OutputMessageContent {
                output_message_content_type: openai::ContentType::OutputText,
                text: input_content.text,
                annotations: input_content.annotations,
                logprobs: input_content.logprobs,
                refusal: input_content.refusal,
            })
        }
    }
}

/// Default implementation for OutputItem
impl Default for openai::OutputItem {
    fn default() -> Self {
        Self {
            content: None,
            id: None,
            role: None,
            status: None,
            output_item_type: None, // Don't add type field if original didn't have it
            queries: None,
            results: None,
            arguments: None,
            call_id: None,
            name: None,
            action: None,
            pending_safety_checks: None,
            encrypted_content: None,
            summary: None,
            result: None,
            code: None,
            container_id: None,
            outputs: None,
            error: None,
            output: None,
            server_label: None,
            tools: None,
            input: None,
        }
    }
}

/// Helper function to convert OutputMessageContent to InputContent
fn convert_output_message_content_to_input_content(
    output_content: openai::OutputMessageContent,
) -> Result<openai::InputContent, ConvertError> {
    match output_content.output_message_content_type {
        openai::ContentType::OutputText => Ok(openai::InputContent {
            input_content_type: openai::InputItemContentListType::OutputText,
            text: output_content.text,
            annotations: output_content.annotations,
            logprobs: output_content.logprobs,
            refusal: output_content.refusal,
            ..Default::default()
        }),
        // TODO: Handle other content types like tool calls when they're properly supported
        _ => {
            // For other content types, try to preserve as much information as possible
            Ok(openai::InputContent {
                input_content_type: openai::InputItemContentListType::OutputText, // Default fallback
                text: output_content.text,
                annotations: output_content.annotations,
                logprobs: output_content.logprobs,
                refusal: output_content.refusal,
                ..Default::default()
            })
        }
    }
}

// ============================================================================
// Chat Completion Conversions
// ============================================================================

/// Convert ChatCompletionRequestMessageExt to universal Message
impl TryFromLLM<ChatCompletionRequestMessageExt> for Message {
    type Error = ConvertError;

    fn try_from(msg: ChatCompletionRequestMessageExt) -> Result<Self, Self::Error> {
        match msg.base.role {
            openai::ChatCompletionRequestMessageRole::System => {
                let content = match msg.base.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        UserContent::String(text)
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let user_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(TryFromLLM::try_from)
                            .collect();
                        UserContent::Array(user_parts?)
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };
                Ok(Message::System { content })
            }
            openai::ChatCompletionRequestMessageRole::User => {
                let content = match msg.base.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        UserContent::String(text)
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let user_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(TryFromLLM::try_from)
                            .collect();
                        UserContent::Array(user_parts?)
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };
                Ok(Message::User { content })
            }
            openai::ChatCompletionRequestMessageRole::Assistant => {
                let mut content_parts: Vec<AssistantContentPart> = Vec::new();

                // Add reasoning FIRST if present (natural model output order)
                // Note: We preserve empty reasoning strings because the presence of the
                // reasoning field indicates reasoning occurred (content may be hidden/summarized)
                if let Some(reasoning) = msg.reasoning {
                    content_parts.push(AssistantContentPart::Reasoning {
                        text: reasoning,
                        encrypted_content: msg.reasoning_signature.clone(),
                    });
                }

                // Add text content if present
                match msg.base.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        if !text.is_empty() {
                            content_parts.push(AssistantContentPart::Text(TextContentPart {
                                text,
                                provider_options: None,
                            }));
                        }
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let assistant_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(|part| {
                                // Convert ChatCompletionRequestMessageContentPart to AssistantContentPart
                                match part.chat_completion_request_message_content_part_type {
                                    openai::PurpleType::Text => {
                                        if let Some(text) = part.text {
                                            Ok(AssistantContentPart::Text(TextContentPart {
                                                text,
                                                provider_options: None,
                                            }))
                                        } else {
                                            Err(ConvertError::MissingRequiredField { field: "text".to_string() })
                                        }
                                    }
                                    _ => Err(ConvertError::UnsupportedInputType {
                                        type_info: format!("ChatCompletionRequestMessageContentPart type: {:?}", part.chat_completion_request_message_content_part_type),
                                    }),
                                }
                            })
                            .collect();
                        content_parts.extend(assistant_parts?);
                    }
                    None => {} // Handle empty assistant messages
                }

                // Add tool calls if present
                if let Some(tool_calls) = msg.base.tool_calls {
                    for tool_call in tool_calls {
                        if let Some(function) = tool_call.function {
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id,
                                tool_name: function.name,
                                arguments: function.arguments.into(),
                                encrypted_content: None,
                                provider_options: None,
                                provider_executed: None,
                            });
                        }
                    }
                }

                let content = if content_parts.is_empty() {
                    AssistantContent::String(String::new())
                } else if content_parts.len() == 1 {
                    // If there's only one text part, use string format
                    match &content_parts[0] {
                        AssistantContentPart::Text(text_part) => {
                            AssistantContent::String(text_part.text.clone())
                        }
                        _ => AssistantContent::Array(content_parts),
                    }
                } else {
                    AssistantContent::Array(content_parts)
                };

                Ok(Message::Assistant { content, id: None })
            }
            openai::ChatCompletionRequestMessageRole::Developer => {
                let content = match msg.base.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        UserContent::String(text)
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let user_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(TryFromLLM::try_from)
                            .collect();
                        UserContent::Array(user_parts?)
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };
                Ok(Message::System {
                    content: mark_user_content_openai_role(content, "developer"),
                })
            }
            openai::ChatCompletionRequestMessageRole::Tool => {
                // Tool messages should extract tool_call_id and content
                let content_text = match msg.base.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => text,
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(mut arr)) => {
                        if arr.len() != 1 {
                            return Err(ConvertError::UnsupportedInputType {
                                type_info: format!("Tool messages must have a single array element (found {})", arr.len()),
                            });
                        }
                        let part = arr.remove(0);
                        if let Some(text) = part.text {
                            text
                        } else {
                            return Err(ConvertError::UnsupportedInputType {
                                type_info: "Tool content part must have text".to_string(),
                            });
                        }
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };

                let tool_call_id =
                    msg.base
                        .tool_call_id
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "tool_call_id".to_string(),
                        })?;

                // Convert to universal Tool message format
                let tool_result = ToolResultContentPart {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: String::new(), // OpenAI doesn't provide tool name in tool messages
                    output: serde_json::Value::String(content_text),
                    provider_options: None,
                };

                Ok(Message::Tool {
                    content: vec![ToolContentPart::ToolResult(tool_result)],
                })
            }
            _ => Err(ConvertError::InvalidEnumValue {
                type_name: "role",
                value: format!("{:?}", msg.base.role),
            }),
        }
    }
}

/// Convert ChatCompletionRequestMessageContentPart to UserContentPart
impl TryFromLLM<openai::ChatCompletionRequestMessageContentPart> for UserContentPart {
    type Error = ConvertError;

    fn try_from(
        part: openai::ChatCompletionRequestMessageContentPart,
    ) -> Result<Self, Self::Error> {
        match part.chat_completion_request_message_content_part_type {
            openai::PurpleType::Text => {
                if let Some(text) = part.text {
                    Ok(UserContentPart::Text(TextContentPart {
                        text,
                        provider_options: None,
                    }))
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "text".to_string(),
                    })
                }
            }
            openai::PurpleType::ImageUrl => {
                if let Some(image_url) = part.image_url {
                    // Parse data URLs to extract raw base64, keep HTTP URLs as-is
                    let (image_data, media_type) =
                        if let Some(block) = parse_base64_data_url(&image_url.url) {
                            // Data URL: extract raw base64 and media type
                            (block.data, Some(block.media_type))
                        } else {
                            // HTTP URL or other: keep as-is, no media type
                            (image_url.url.clone(), None)
                        };

                    Ok(UserContentPart::Image {
                        image: serde_json::Value::String(image_data),
                        media_type,
                        provider_options: None,
                    })
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "image_url".to_string(),
                    })
                }
            }
            _ => Err(ConvertError::UnsupportedInputType {
                type_info: format!(
                    "ChatCompletionRequestMessageContentPart type: {:?}",
                    part.chat_completion_request_message_content_part_type
                ),
            }),
        }
    }
}

/// Convert universal Message to ChatCompletionRequestMessage
impl TryFromLLM<Message> for ChatCompletionRequestMessageExt {
    type Error = ConvertError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            Message::System { content } => {
                let (content, role_override) = extract_openai_chat_role(content);
                Ok(ChatCompletionRequestMessageExt {
                    base: openai::ChatCompletionRequestMessage {
                        role: role_override
                            .unwrap_or(openai::ChatCompletionRequestMessageRole::System),
                        content: Some(convert_user_content_to_chat_completion_content(content)?),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                        audio: None,
                        function_call: None,
                        refusal: None,
                    },
                    reasoning: None,
                    reasoning_signature: None,
                })
            }
            Message::User { content } => Ok(ChatCompletionRequestMessageExt {
                base: openai::ChatCompletionRequestMessage {
                    role: openai::ChatCompletionRequestMessageRole::User,
                    content: Some(convert_user_content_to_chat_completion_content(content)?),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                },
                reasoning: None,
                reasoning_signature: None,
            }),
            Message::Assistant { content, id: _ } => {
                let (text_content, tool_calls, reasoning, reasoning_signature) =
                    extract_content_tool_calls_and_reasoning(content)?;

                Ok(ChatCompletionRequestMessageExt {
                    base: openai::ChatCompletionRequestMessage {
                        role: openai::ChatCompletionRequestMessageRole::Assistant,
                        content: text_content,
                        name: None,
                        tool_calls,
                        tool_call_id: None,
                        audio: None,
                        function_call: None,
                        refusal: None,
                    },
                    reasoning,
                    reasoning_signature,
                })
            }
            Message::Tool { content } => {
                // Extract the tool result content
                let tool_result = content
                    .iter()
                    .map(|part| {
                        let ToolContentPart::ToolResult(result) = part;
                        result
                    })
                    .next()
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: "tool_result".to_string(),
                    })?;

                // Convert output to string for OpenAI
                let content_string = match &tool_result.output {
                    serde_json::Value::String(s) => s.clone(),
                    other => serde_json::to_string(other).map_err(|e| {
                        ConvertError::JsonSerializationFailed {
                            field: "tool_result_content".to_string(),
                            error: e.to_string(),
                        }
                    })?,
                };

                Ok(ChatCompletionRequestMessageExt {
                    base: openai::ChatCompletionRequestMessage {
                        role: openai::ChatCompletionRequestMessageRole::Tool,
                        content: Some(openai::ChatCompletionRequestMessageContent::String(
                            content_string,
                        )),
                        name: None,
                        tool_calls: None,
                        tool_call_id: Some(tool_result.tool_call_id.clone()),
                        audio: None,
                        function_call: None,
                        refusal: None,
                    },
                    reasoning: None,
                    reasoning_signature: None,
                })
            }
        }
    }
}

/// Convert UserContent to ChatCompletionRequestMessageContent
fn convert_user_content_to_chat_completion_content(
    content: UserContent,
) -> Result<openai::ChatCompletionRequestMessageContent, ConvertError> {
    match content {
        UserContent::String(text) => Ok(openai::ChatCompletionRequestMessageContent::String(text)),
        UserContent::Array(parts) => {
            let chat_parts: Result<Vec<_>, _> = parts
                .into_iter()
                .map(convert_user_content_part_to_chat_completion_part)
                .collect();
            Ok(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(chat_parts?))
        }
    }
}

/// Convert UserContentPart to ChatCompletionRequestMessageContentPart
fn convert_user_content_part_to_chat_completion_part(
    part: UserContentPart,
) -> Result<openai::ChatCompletionRequestMessageContentPart, ConvertError> {
    match part {
        UserContentPart::Text(text_part) => Ok(openai::ChatCompletionRequestMessageContentPart {
            text: Some(text_part.text),
            chat_completion_request_message_content_part_type: openai::PurpleType::Text,
            image_url: None,
            input_audio: None,
            file: None,
            refusal: None,
        }),
        UserContentPart::Image {
            image,
            media_type,
            provider_options: _,
        } => {
            // Convert image to ImageUrl format
            let image_str = match image {
                serde_json::Value::String(url) => url,
                _ => {
                    return Err(ConvertError::UnsupportedInputType {
                        type_info: format!(
                            "Image must be string URL for ChatCompletion, got: {:?}",
                            image
                        ),
                    })
                }
            };

            // If we have raw base64 data (not a URL) and media_type, create a proper data URL
            let url = if !image_str.starts_with("data:")
                && !image_str.starts_with("http://")
                && !image_str.starts_with("https://")
            {
                // Assume raw base64 data - create data URL with media_type
                let mt = media_type.as_deref().unwrap_or("image/jpeg");
                format!("data:{};base64,{}", mt, image_str)
            } else {
                image_str
            };

            Ok(openai::ChatCompletionRequestMessageContentPart {
                text: None,
                chat_completion_request_message_content_part_type: openai::PurpleType::ImageUrl,
                image_url: Some(openai::ImageUrl { url, detail: None }),
                input_audio: None,
                file: None,
                refusal: None,
            })
        }
        _ => Err(ConvertError::UnsupportedInputType {
            type_info: format!(
                "UserContentPart variant in ChatCompletion conversion: {:?}",
                part
            ),
        }),
    }
}

type ExtractedContentResult = (
    Option<openai::ChatCompletionRequestMessageContent>,
    Option<Vec<openai::ToolCall>>,
    Option<String>,
    Option<String>, // reasoning_signature
);

/// Extract text content, tool calls, reasoning, and reasoning_signature from AssistantContent
fn extract_content_tool_calls_and_reasoning(
    content: AssistantContent,
) -> Result<ExtractedContentResult, ConvertError> {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();
    let mut reasoning_parts = Vec::new();
    let mut reasoning_signature: Option<String> = None;

    match content {
        AssistantContent::String(text) => {
            return Ok((
                Some(openai::ChatCompletionRequestMessageContent::String(text)),
                None,
                None,
                None,
            ));
        }
        AssistantContent::Array(parts) => {
            for part in parts {
                match part {
                    AssistantContentPart::Text(text_part) => {
                        text_parts.push(text_part.text);
                    }
                    AssistantContentPart::Reasoning {
                        text,
                        encrypted_content,
                    } => {
                        reasoning_parts.push(text);
                        // Take the first signature if multiple reasoning blocks exist
                        if reasoning_signature.is_none() {
                            reasoning_signature = encrypted_content;
                        }
                    }
                    AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments,
                        ..
                    } => {
                        tool_calls.push(openai::ToolCall {
                            id: tool_call_id,
                            tool_call_type: openai::ToolType::Function,
                            function: Some(openai::PurpleFunction {
                                name: tool_name,
                                arguments: arguments.to_string(),
                            }),
                            custom: None,
                        });
                    }
                    _ => {
                        // Handle other content types if needed
                    }
                }
            }
        }
    }

    let text_content = if text_parts.is_empty() && !tool_calls.is_empty() {
        None // When we have tool calls but no text, omit content entirely
    } else {
        Some(openai::ChatCompletionRequestMessageContent::String(
            text_parts.join(""),
        ))
    };

    let tool_calls_option = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join(""))
    };

    Ok((
        text_content,
        tool_calls_option,
        reasoning,
        reasoning_signature,
    ))
}

/// Convert ChatCompletionResponseMessageExt to universal Message
impl TryFromLLM<ChatCompletionResponseMessageExt> for Message {
    type Error = ConvertError;

    fn try_from(msg: ChatCompletionResponseMessageExt) -> Result<Self, Self::Error> {
        match msg.base.role {
            openai::MessageRole::Assistant => {
                let mut content_parts: Vec<AssistantContentPart> = Vec::new();

                // Add reasoning FIRST if present (natural model output order: think first, respond after)
                // Note: We preserve empty reasoning strings because the presence of the
                // reasoning field indicates reasoning occurred (content may be hidden/summarized)
                if let Some(reasoning) = msg.reasoning {
                    content_parts.push(AssistantContentPart::Reasoning {
                        text: reasoning,
                        encrypted_content: msg.reasoning_signature.clone(),
                    });
                }

                // Add text content if present
                if let Some(text) = &msg.base.content {
                    if !text.is_empty() {
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text: text.clone(),
                            provider_options: None,
                        }));
                    }
                }

                // Add tool calls if present
                if let Some(tool_calls) = &msg.base.tool_calls {
                    for tool_call in tool_calls {
                        if let Some(function) = &tool_call.function {
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id.clone(),
                                tool_name: function.name.clone(),
                                arguments: function.arguments.clone().into(),
                                encrypted_content: None,
                                provider_options: None,
                                provider_executed: None,
                            });
                        }
                    }
                }

                let content = if content_parts.is_empty() {
                    AssistantContent::String(String::new())
                } else if content_parts.len() == 1 {
                    // If there's only one text part, use string format
                    match &content_parts[0] {
                        AssistantContentPart::Text(text_part) => {
                            AssistantContent::String(text_part.text.clone())
                        }
                        _ => AssistantContent::Array(content_parts),
                    }
                } else {
                    AssistantContent::Array(content_parts)
                };

                Ok(Message::Assistant { content, id: None })
            }
        }
    }
}

/// Convert universal Message to ChatCompletionResponseMessageExt
impl TryFromLLM<&Message> for ChatCompletionResponseMessageExt {
    type Error = ConvertError;

    fn try_from(msg: &Message) -> Result<Self, Self::Error> {
        match msg {
            Message::Assistant { content, id: _ } => {
                let (content_text, tool_calls, reasoning, reasoning_signature) = match content {
                    AssistantContent::String(text) => (Some(text.clone()), None, None, None),
                    AssistantContent::Array(parts) => {
                        // Extract text from parts and concatenate
                        let texts: Vec<String> = parts
                            .iter()
                            .filter_map(|part| match part {
                                AssistantContentPart::Text(text_part) => {
                                    Some(text_part.text.clone())
                                }
                                _ => None,
                            })
                            .collect();

                        // Extract reasoning from parts and concatenate, also capture signature
                        let mut reasonings: Vec<String> = Vec::new();
                        let mut signature: Option<String> = None;
                        for part in parts {
                            if let AssistantContentPart::Reasoning {
                                text,
                                encrypted_content,
                            } = part
                            {
                                reasonings.push(text.clone());
                                // Take the first signature if multiple reasoning blocks exist
                                if signature.is_none() {
                                    signature = encrypted_content.clone();
                                }
                            }
                        }

                        // Extract tool calls from parts
                        let tool_calls: Vec<openai::ToolCall> = parts
                            .iter()
                            .filter_map(|part| match part {
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    ..
                                } => Some(openai::ToolCall {
                                    id: tool_call_id.clone(),
                                    tool_call_type: openai::ToolType::Function,
                                    function: Some(openai::PurpleFunction {
                                        name: tool_name.clone(),
                                        arguments: arguments.to_string(),
                                    }),
                                    custom: None,
                                }),
                                _ => None,
                            })
                            .collect();

                        let content_text = if texts.is_empty() {
                            None
                        } else {
                            Some(texts.join(""))
                        };

                        let reasoning = if reasonings.is_empty() {
                            None
                        } else {
                            Some(reasonings.join(""))
                        };

                        let tool_calls_option = if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        };

                        (content_text, tool_calls_option, reasoning, signature)
                    }
                };

                Ok(ChatCompletionResponseMessageExt {
                    base: openai::ChatCompletionResponseMessage {
                        role: openai::MessageRole::Assistant,
                        content: content_text,
                        annotations: Some(vec![]), // Hardcode empty annotations for consistency
                        audio: None,
                        function_call: None,
                        refusal: None,
                        tool_calls,
                    },
                    reasoning,
                    reasoning_signature,
                })
            }
            _ => Err(ConvertError::InvalidEnumValue {
                type_name: "role",
                value: format!("{:?}", msg),
            }),
        }
    }
}
