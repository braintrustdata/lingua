/*!
Conversion between OpenAI Decisions API payloads and universal decision types.

Decision input items map to universal messages: messages keep their role, function
and custom tool calls become assistant tool calls, and their outputs become tool
results. Features without a universal representation, such as JSON content parts,
refusals, and request-level prompt cache options, are rejected.
*/

use super::decisions_generated as decisions;
use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, CacheControl, CacheControlType, Message,
    ProviderOptions, TextContentPart, ToolCallArguments, ToolContentPart, ToolResultContentPart,
    UserContent, UserContentPart,
};
use crate::universal::{
    DecisionAnswer, DecisionChoice, DecisionChoiceProbability, DecisionInput,
    DecisionLevelProbability, DecisionQuestion, DecisionQuestionKind, DecisionResult,
    DecisionScoreLevel, DecisionValue, UniversalDecisionRequest, UniversalDecisionResponse,
    UniversalUsage,
};
use crate::util::media::parse_base64_data_url;
use serde::Deserialize;

const TO_UNIVERSAL: &str = "Lingua decision";
const TO_DECISIONS: &str = "OpenAI Decisions";

fn unsupported(from: impl Into<String>, to: &'static str) -> ConvertError {
    ConvertError::UnsupportedMapping {
        from: from.into(),
        to,
    }
}

fn missing(field: impl Into<String>) -> ConvertError {
    ConvertError::MissingRequiredField {
        field: field.into(),
    }
}

fn invalid(reason: impl Into<String>) -> ConvertError {
    ConvertError::ContentConversionFailed {
        reason: reason.into(),
    }
}

fn reject_present(location: &str, fields: &[(&str, bool)]) -> Result<(), ConvertError> {
    match fields.iter().find(|(_, present)| *present) {
        Some((field, _)) => Err(invalid(format!("{location} does not take `{field}`"))),
        None => Ok(()),
    }
}

fn ephemeral_cache_control() -> CacheControl {
    CacheControl {
        cache_control_type: CacheControlType::Ephemeral,
        ttl: None,
    }
}

fn cache_breakpoint(
    cache_control: Option<CacheControl>,
) -> Result<Option<decisions::PromptCacheBreakpoint>, ConvertError> {
    match cache_control {
        None => Ok(None),
        Some(CacheControl { ttl: Some(_), .. }) => Err(unsupported(
            "Lingua cache_control.ttl",
            "OpenAI Decisions prompt_cache_breakpoint (the TTL is set per request)",
        )),
        Some(_) => Ok(Some(decisions::PromptCacheBreakpoint {
            mode: decisions::PromptCacheBreakpointMode::Explicit,
        })),
    }
}

impl TryFromLLM<decisions::DecisionRequest> for UniversalDecisionRequest {
    type Error = ConvertError;

    fn try_from(request: decisions::DecisionRequest) -> Result<Self, Self::Error> {
        if request.prompt_cache_options.is_some() {
            return Err(unsupported(
                "OpenAI Decisions prompt_cache_options",
                TO_UNIVERSAL,
            ));
        }
        if request.safety_identifier.is_some() {
            return Err(unsupported(
                "OpenAI Decisions safety_identifier",
                TO_UNIVERSAL,
            ));
        }
        let input = match request.input {
            decisions::DecisionInput::PurpleString(text) => DecisionInput::Text(text),
            decisions::DecisionInput::DecisionInputMessageArray(items) => {
                let mut messages = Vec::with_capacity(items.len());
                for (index, item) in items.into_iter().enumerate() {
                    let message = message_from_item(item, index, &messages)?;
                    messages.push(message);
                }
                DecisionInput::Messages(messages)
            }
        };
        let questions = request
            .questions
            .into_iter()
            .enumerate()
            .map(|(index, question)| question_from_param(question, index))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            model: Some(request.model),
            input,
            questions,
            prompt_cache_key: request.prompt_cache_key,
        })
    }
}

fn message_from_item(
    item: decisions::DecisionInputMessage,
    index: usize,
    previous: &[Message],
) -> Result<Message, ConvertError> {
    let location = format!("input[{index}]");
    let field = |name: &str| format!("{location}.{name}");
    let item_type = item
        .decision_input_message_type
        .unwrap_or(decisions::DecisionInputType::Message);
    match item_type {
        decisions::DecisionInputType::Message => {
            reject_present(
                &location,
                &[
                    ("arguments", item.arguments.is_some()),
                    ("call_id", item.call_id.is_some()),
                    ("id", item.id.is_some()),
                    ("input", item.input.is_some()),
                    ("name", item.name.is_some()),
                    ("namespace", item.namespace.is_some()),
                    ("output", item.output.is_some()),
                    ("status", item.status.is_some()),
                ],
            )?;
            if item.phase.is_some() {
                return Err(unsupported(
                    "OpenAI Decisions assistant message phase",
                    TO_UNIVERSAL,
                ));
            }
            let role = item.role.ok_or_else(|| missing(field("role")))?;
            let content = item.content.ok_or_else(|| missing(field("content")))?;
            Ok(match role {
                decisions::Role::System => Message::System {
                    content: user_content_from_decision(content, &location)?,
                },
                decisions::Role::Developer => Message::Developer {
                    content: user_content_from_decision(content, &location)?,
                },
                decisions::Role::User => Message::User {
                    content: user_content_from_decision(content, &location)?,
                },
                decisions::Role::Assistant => Message::Assistant {
                    content: assistant_content_from_decision(content, &location)?,
                    id: None,
                },
            })
        }
        decisions::DecisionInputType::FunctionCall
        | decisions::DecisionInputType::CustomToolCall => {
            let custom = item_type == decisions::DecisionInputType::CustomToolCall;
            reject_present(
                &location,
                &[
                    ("content", item.content.is_some()),
                    ("role", item.role.is_some()),
                    ("phase", item.phase.is_some()),
                    ("output", item.output.is_some()),
                    (
                        if custom { "arguments" } else { "input" },
                        if custom {
                            item.arguments.is_some()
                        } else {
                            item.input.is_some()
                        },
                    ),
                ],
            )?;
            if let Some(namespace) = item.namespace {
                return Err(unsupported(
                    format!("OpenAI Decisions tool call namespace '{namespace}'"),
                    "Lingua tool call (tool namespaces are provider-scoped registry state)",
                ));
            }
            let arguments = if custom {
                ToolCallArguments::Custom(item.input.ok_or_else(|| missing(field("input")))?)
            } else {
                ToolCallArguments::from(item.arguments.ok_or_else(|| missing(field("arguments")))?)
            };
            Ok(Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                    tool_call_id: item.call_id.ok_or_else(|| missing(field("call_id")))?,
                    tool_name: item.name.ok_or_else(|| missing(field("name")))?,
                    arguments,
                    status: match item.status {
                        None | Some(decisions::FunctionCallStatusParam::Completed) => None,
                        Some(decisions::FunctionCallStatusParam::InProgress) => {
                            Some("in_progress".into())
                        }
                        Some(decisions::FunctionCallStatusParam::Incomplete) => {
                            Some("incomplete".into())
                        }
                    },
                    caller: None,
                    encrypted_content: None,
                    provider_options: None,
                    provider_executed: None,
                }]),
                id: item.id,
            })
        }
        decisions::DecisionInputType::FunctionCallOutput
        | decisions::DecisionInputType::CustomToolCallOutput => {
            reject_present(
                &location,
                &[
                    ("arguments", item.arguments.is_some()),
                    ("content", item.content.is_some()),
                    ("input", item.input.is_some()),
                    ("name", item.name.is_some()),
                    ("namespace", item.namespace.is_some()),
                    ("phase", item.phase.is_some()),
                    ("role", item.role.is_some()),
                ],
            )?;
            if item.id.is_some() || item.status.is_some() {
                return Err(unsupported(
                    "OpenAI Decisions tool output id or status",
                    "Lingua tool result (tool results carry no item id or status)",
                ));
            }
            let tool_call_id = item.call_id.ok_or_else(|| missing(field("call_id")))?;
            let output = match item.output.ok_or_else(|| missing(field("output")))? {
                decisions::DecisionInputToolOutputContent::PurpleString(output) => {
                    serde_json::Value::String(output)
                }
                decisions::DecisionInputToolOutputContent::DecisionInputToolOutputContentPartElementArray(_) => {
                    return Err(unsupported(
                        "OpenAI Decisions tool output content parts",
                        "Lingua tool result",
                    ));
                }
            };
            let tool_name = tool_name_for_call(previous, &tool_call_id).ok_or_else(|| {
                invalid(format!(
                    "{location} has no earlier tool call with call_id \"{tool_call_id}\""
                ))
            })?;
            Ok(Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id,
                    tool_name,
                    output,
                    custom_tool_call: (item_type
                        == decisions::DecisionInputType::CustomToolCallOutput)
                        .then_some(true),
                    caller: None,
                    provider_options: None,
                })],
            })
        }
    }
}

fn tool_name_for_call(messages: &[Message], call_id: &str) -> Option<String> {
    messages.iter().rev().find_map(|message| match message {
        Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } => parts.iter().find_map(|part| match part {
            AssistantContentPart::ToolCall {
                tool_call_id,
                tool_name,
                ..
            } if tool_call_id == call_id => Some(tool_name.clone()),
            _ => None,
        }),
        _ => None,
    })
}

fn user_content_from_decision(
    content: decisions::DecisionInputContent,
    location: &str,
) -> Result<UserContent, ConvertError> {
    match content {
        decisions::DecisionInputContent::PurpleString(text) => Ok(UserContent::String(text)),
        decisions::DecisionInputContent::DecisionInputContentPartElementArray(parts) => parts
            .into_iter()
            .map(|part| user_part_from_decision(part, location))
            .collect::<Result<_, _>>()
            .map(UserContent::Array),
    }
}

fn user_part_from_decision(
    part: decisions::DecisionInputContentPartElement,
    location: &str,
) -> Result<UserContentPart, ConvertError> {
    let field = |name: &str| format!("{location}.content[].{name}");
    match part.decision_input_type {
        decisions::DecisionInputContentPartType::InputText => {
            Ok(UserContentPart::Text(TextContentPart {
                text: part.text.ok_or_else(|| missing(field("text")))?,
                encrypted_content: None,
                cache_control: part
                    .prompt_cache_breakpoint
                    .map(|_| ephemeral_cache_control()),
                provider_options: None,
            }))
        }
        decisions::DecisionInputContentPartType::InputImage => {
            if part.prompt_cache_breakpoint.is_some() {
                return Err(unsupported(
                    "OpenAI Decisions input_image prompt_cache_breakpoint",
                    "Lingua image content (cache control is unsupported)",
                ));
            }
            image_from_decision(
                part.image_url.ok_or_else(|| missing(field("image_url")))?,
                part.detail,
            )
        }
        decisions::DecisionInputContentPartType::InputJson => Err(unsupported(
            "OpenAI Decisions input_json content",
            "Lingua user content (no JSON content part)",
        )),
        decisions::DecisionInputContentPartType::OutputText
        | decisions::DecisionInputContentPartType::Refusal => Err(invalid(format!(
            "{location} is not an assistant message, so its content cannot be output text or a refusal"
        ))),
    }
}

/// A typed view of the provider options Lingua keeps on OpenAI image parts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageOptionsView {
    detail: Option<decisions::ImageDetailParam>,
}

fn image_from_decision(
    image_url: String,
    detail: Option<decisions::ImageDetailParam>,
) -> Result<UserContentPart, ConvertError> {
    let provider_options = detail
        .map(|detail| {
            serde_json::to_value(detail)
                .map(|detail| {
                    let mut options = serde_json::Map::new();
                    options.insert("detail".into(), detail);
                    ProviderOptions { options }
                })
                .map_err(|error| ConvertError::JsonSerializationFailed {
                    field: "detail".into(),
                    error: error.to_string(),
                })
        })
        .transpose()?;
    let (image, media_type) = match parse_base64_data_url(&image_url) {
        Some(block) => (block.data, Some(block.media_type)),
        None => (image_url, None),
    };
    Ok(UserContentPart::Image {
        image: serde_json::Value::String(image),
        media_type,
        provider_options,
    })
}

fn image_to_decision(
    image: serde_json::Value,
    media_type: Option<String>,
    provider_options: Option<ProviderOptions>,
) -> Result<(String, Option<decisions::ImageDetailParam>), ConvertError> {
    let serde_json::Value::String(image) = image else {
        return Err(unsupported(
            "Lingua image content that is not a URL or base64 string",
            TO_DECISIONS,
        ));
    };
    let image_url = if image.starts_with("data:")
        || image.starts_with("http://")
        || image.starts_with("https://")
    {
        image
    } else {
        let media_type =
            media_type.ok_or_else(|| missing("media_type for base64 image content"))?;
        format!("data:{media_type};base64,{image}")
    };
    let detail = match provider_options {
        None => None,
        Some(options) => {
            serde_json::from_value::<ImageOptionsView>(serde_json::Value::Object(options.options))
                .map_err(|_| {
                    unsupported(
                        "Lingua image provider_options other than `detail`",
                        TO_DECISIONS,
                    )
                })?
                .detail
        }
    };
    Ok((image_url, detail))
}

fn assistant_content_from_decision(
    content: decisions::DecisionInputContent,
    location: &str,
) -> Result<AssistantContent, ConvertError> {
    match content {
        decisions::DecisionInputContent::PurpleString(text) => Ok(AssistantContent::String(text)),
        decisions::DecisionInputContent::DecisionInputContentPartElementArray(parts) => parts
            .into_iter()
            .map(|part| match part.decision_input_type {
                decisions::DecisionInputContentPartType::OutputText => {
                    if part.prompt_cache_breakpoint.is_some() {
                        return Err(invalid(format!(
                            "{location}.content[] output text does not take a prompt_cache_breakpoint"
                        )));
                    }
                    Ok(AssistantContentPart::Text(TextContentPart {
                        text: part
                            .text
                            .ok_or_else(|| missing(format!("{location}.content[].text")))?,
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }))
                }
                decisions::DecisionInputContentPartType::Refusal => Err(unsupported(
                    "OpenAI Decisions assistant refusal content",
                    "Lingua assistant content (no refusal part)",
                )),
                _ => Err(invalid(format!(
                    "{location} is an assistant message, so its content must be output text or a refusal"
                ))),
            })
            .collect::<Result<_, _>>()
            .map(AssistantContent::Array),
    }
}

fn question_from_param(
    question: decisions::QuestionParam,
    index: usize,
) -> Result<DecisionQuestion, ConvertError> {
    let location = format!("questions[{index}]");
    let kind = match question.question_param_type {
        decisions::QuestionParamType::Predicate => {
            reject_present(
                &location,
                &[
                    ("choices", question.choices.is_some()),
                    ("levels", question.levels.is_some()),
                ],
            )?;
            DecisionQuestionKind::Predicate
        }
        decisions::QuestionParamType::Choice => {
            reject_present(&location, &[("levels", question.levels.is_some())])?;
            DecisionQuestionKind::Choice {
                choices: question
                    .choices
                    .ok_or_else(|| missing(format!("{location}.choices")))?
                    .into_iter()
                    .map(|choice| DecisionChoice {
                        value: match choice.value {
                            decisions::ChoiceValueParam::Bool(value) => DecisionValue::Bool(value),
                            decisions::ChoiceValueParam::PurpleString(value) => {
                                DecisionValue::String(value)
                            }
                        },
                        description: choice.description,
                    })
                    .collect(),
            }
        }
        decisions::QuestionParamType::Score => {
            reject_present(&location, &[("choices", question.choices.is_some())])?;
            DecisionQuestionKind::Score {
                levels: question
                    .levels
                    .ok_or_else(|| missing(format!("{location}.levels")))?
                    .into_iter()
                    .map(|level| DecisionScoreLevel {
                        label: level.label,
                        description: level.description,
                    })
                    .collect(),
            }
        }
    };
    let instructions = match question.instructions {
        decisions::Instructions::PurpleString(text) => UserContent::String(text),
        decisions::Instructions::DecisionQuestionInputPartArray(parts) => parts
            .into_iter()
            .map(|part| match part.decision_question_input_part_type {
                decisions::DecisionQuestionInputPartType::InputText => {
                    Ok(UserContentPart::Text(TextContentPart {
                        text: part
                            .text
                            .ok_or_else(|| missing(format!("{location}.instructions[].text")))?,
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }))
                }
                decisions::DecisionQuestionInputPartType::InputImage => image_from_decision(
                    part.image_url
                        .ok_or_else(|| missing(format!("{location}.instructions[].image_url")))?,
                    part.detail,
                ),
                decisions::DecisionQuestionInputPartType::InputJson => Err(unsupported(
                    "OpenAI Decisions input_json instructions",
                    "Lingua user content (no JSON content part)",
                )),
            })
            .collect::<Result<_, _>>()
            .map(UserContent::Array)?,
    };
    Ok(DecisionQuestion {
        name: question.name,
        instructions,
        kind,
    })
}

impl TryFromLLM<UniversalDecisionRequest> for decisions::DecisionRequest {
    type Error = ConvertError;

    fn try_from(request: UniversalDecisionRequest) -> Result<Self, Self::Error> {
        let input = match request.input {
            DecisionInput::Text(text) => decisions::DecisionInput::PurpleString(text),
            DecisionInput::Messages(messages) => {
                let mut items = Vec::with_capacity(messages.len());
                for message in messages {
                    push_items_from_message(message, &mut items)?;
                }
                decisions::DecisionInput::DecisionInputMessageArray(items)
            }
        };
        Ok(Self {
            input,
            model: request.model.ok_or_else(|| missing("model"))?,
            prompt_cache_key: request.prompt_cache_key,
            prompt_cache_options: None,
            questions: request
                .questions
                .into_iter()
                .map(question_to_param)
                .collect::<Result<_, _>>()?,
            safety_identifier: None,
        })
    }
}

fn empty_item(item_type: decisions::DecisionInputType) -> decisions::DecisionInputMessage {
    decisions::DecisionInputMessage {
        content: None,
        role: None,
        decision_input_message_type: Some(item_type),
        phase: None,
        arguments: None,
        call_id: None,
        id: None,
        name: None,
        namespace: None,
        status: None,
        output: None,
        input: None,
    }
}

fn message_item(
    role: decisions::Role,
    content: decisions::DecisionInputContent,
) -> decisions::DecisionInputMessage {
    decisions::DecisionInputMessage {
        role: Some(role),
        content: Some(content),
        ..empty_item(decisions::DecisionInputType::Message)
    }
}

fn text_part(
    part_type: decisions::DecisionInputContentPartType,
    text: TextContentPart,
) -> Result<decisions::DecisionInputContentPartElement, ConvertError> {
    if text.encrypted_content.is_some() || text.provider_options.is_some() {
        return Err(unsupported(
            "Lingua text encrypted_content or provider_options",
            TO_DECISIONS,
        ));
    }
    let prompt_cache_breakpoint = match part_type {
        decisions::DecisionInputContentPartType::OutputText if text.cache_control.is_some() => {
            return Err(unsupported(
                "Lingua assistant text cache_control",
                "OpenAI Decisions output_text (prompt_cache_breakpoint is unsupported)",
            ));
        }
        _ => cache_breakpoint(text.cache_control)?,
    };
    Ok(decisions::DecisionInputContentPartElement {
        prompt_cache_breakpoint,
        text: Some(text.text),
        decision_input_type: part_type,
        detail: None,
        image_url: None,
        data: None,
        refusal: None,
    })
}

fn user_content_to_decision(
    content: UserContent,
    allow_images: bool,
) -> Result<decisions::DecisionInputContent, ConvertError> {
    let parts = match content {
        UserContent::String(text) => {
            return Ok(decisions::DecisionInputContent::PurpleString(text))
        }
        UserContent::Array(parts) => parts,
    };
    parts
        .into_iter()
        .map(|part| match part {
            UserContentPart::Text(text) => {
                text_part(decisions::DecisionInputContentPartType::InputText, text)
            }
            UserContentPart::Image {
                image,
                media_type,
                provider_options,
            } if allow_images => {
                let (image_url, detail) = image_to_decision(image, media_type, provider_options)?;
                Ok(decisions::DecisionInputContentPartElement {
                    prompt_cache_breakpoint: None,
                    text: None,
                    decision_input_type: decisions::DecisionInputContentPartType::InputImage,
                    detail,
                    image_url: Some(image_url),
                    data: None,
                    refusal: None,
                })
            }
            UserContentPart::Image { .. } => Err(unsupported(
                "Lingua system or developer image content",
                "OpenAI Decisions system and developer messages (text only)",
            )),
            UserContentPart::Audio { .. } => Err(unsupported("Lingua audio content", TO_DECISIONS)),
            UserContentPart::File { .. } => Err(unsupported("Lingua file content", TO_DECISIONS)),
        })
        .collect::<Result<_, _>>()
        .map(decisions::DecisionInputContent::DecisionInputContentPartElementArray)
}

fn push_items_from_message(
    message: Message,
    items: &mut Vec<decisions::DecisionInputMessage>,
) -> Result<(), ConvertError> {
    match message {
        Message::System { content } => items.push(message_item(
            decisions::Role::System,
            user_content_to_decision(content, false)?,
        )),
        Message::Developer { content } => items.push(message_item(
            decisions::Role::Developer,
            user_content_to_decision(content, false)?,
        )),
        Message::User { content } => items.push(message_item(
            decisions::Role::User,
            user_content_to_decision(content, true)?,
        )),
        Message::Assistant { content, id } => push_assistant_items(content, id, items)?,
        Message::Tool { content } => {
            for part in content {
                let ToolContentPart::ToolResult(result) = part else {
                    return Err(unsupported("Lingua tool discovery results", TO_DECISIONS));
                };
                if result.caller.is_some() || result.provider_options.is_some() {
                    return Err(unsupported(
                        "Lingua tool result caller or provider_options",
                        TO_DECISIONS,
                    ));
                }
                let output = match result.output {
                    serde_json::Value::String(output) => output,
                    output => serde_json::to_string(&output).map_err(|error| {
                        ConvertError::JsonSerializationFailed {
                            field: "tool result output".into(),
                            error: error.to_string(),
                        }
                    })?,
                };
                let item_type = if result.custom_tool_call == Some(true) {
                    decisions::DecisionInputType::CustomToolCallOutput
                } else {
                    decisions::DecisionInputType::FunctionCallOutput
                };
                items.push(decisions::DecisionInputMessage {
                    call_id: Some(result.tool_call_id),
                    output: Some(decisions::DecisionInputToolOutputContent::PurpleString(
                        output,
                    )),
                    ..empty_item(item_type)
                });
            }
        }
        Message::AdditionalTools { .. } => {
            return Err(unsupported(
                "Lingua additional_tools messages",
                TO_DECISIONS,
            ));
        }
    }
    Ok(())
}

fn push_assistant_items(
    content: AssistantContent,
    id: Option<String>,
    items: &mut Vec<decisions::DecisionInputMessage>,
) -> Result<(), ConvertError> {
    let parts = match content {
        AssistantContent::String(text) => {
            if id.is_some() {
                return Err(assistant_id_unsupported());
            }
            items.push(message_item(
                decisions::Role::Assistant,
                decisions::DecisionInputContent::PurpleString(text),
            ));
            return Ok(());
        }
        AssistantContent::Array(parts) => parts,
    };
    let first = items.len();
    let mut text = Vec::new();
    for part in parts {
        match part {
            AssistantContentPart::Text(part) => {
                text.push(text_part(
                    decisions::DecisionInputContentPartType::OutputText,
                    part,
                )?);
            }
            AssistantContentPart::ToolCall {
                tool_call_id,
                tool_name,
                arguments,
                status,
                caller,
                encrypted_content,
                provider_options,
                provider_executed,
            } => {
                if caller.is_some()
                    || encrypted_content.is_some()
                    || provider_options.is_some()
                    || provider_executed.is_some()
                {
                    return Err(unsupported(
                        "Lingua tool call caller, encrypted_content, provider_options, or provider_executed",
                        TO_DECISIONS,
                    ));
                }
                flush_assistant_text(&mut text, items);
                let status = status
                    .map(|status| {
                        serde_json::from_value(serde_json::Value::String(status.clone())).map_err(
                            |_| ConvertError::InvalidEnumValue {
                                type_name: "FunctionCallStatusParam",
                                value: status,
                            },
                        )
                    })
                    .transpose()?;
                let mut item = match arguments {
                    ToolCallArguments::Custom(input) => decisions::DecisionInputMessage {
                        input: Some(input),
                        ..empty_item(decisions::DecisionInputType::CustomToolCall)
                    },
                    arguments => decisions::DecisionInputMessage {
                        arguments: Some(arguments.to_string()),
                        ..empty_item(decisions::DecisionInputType::FunctionCall)
                    },
                };
                item.call_id = Some(tool_call_id);
                item.name = Some(tool_name);
                item.status = status;
                items.push(item);
            }
            AssistantContentPart::File { .. } => {
                return Err(unsupported("Lingua assistant file content", TO_DECISIONS));
            }
            AssistantContentPart::Reasoning { .. } => {
                return Err(unsupported("Lingua reasoning content", TO_DECISIONS));
            }
            AssistantContentPart::Program { .. } | AssistantContentPart::ProgramOutput { .. } => {
                return Err(unsupported("Lingua program content", TO_DECISIONS));
            }
            AssistantContentPart::ToolDiscoveryCall { .. } => {
                return Err(unsupported("Lingua tool discovery calls", TO_DECISIONS));
            }
            AssistantContentPart::ToolResult { .. } => {
                return Err(unsupported(
                    "Lingua tool results in assistant messages",
                    TO_DECISIONS,
                ));
            }
        }
    }
    flush_assistant_text(&mut text, items);
    if let Some(id) = id {
        match &mut items[first..] {
            [item]
                if item.decision_input_message_type
                    != Some(decisions::DecisionInputType::Message) =>
            {
                item.id = Some(id);
            }
            _ => return Err(assistant_id_unsupported()),
        }
    }
    Ok(())
}

fn assistant_id_unsupported() -> ConvertError {
    unsupported(
        "Lingua assistant message id",
        "OpenAI Decisions input (only a single tool call item carries an id)",
    )
}

fn flush_assistant_text(
    text: &mut Vec<decisions::DecisionInputContentPartElement>,
    items: &mut Vec<decisions::DecisionInputMessage>,
) {
    if text.is_empty() {
        return;
    }
    items.push(message_item(
        decisions::Role::Assistant,
        decisions::DecisionInputContent::DecisionInputContentPartElementArray(std::mem::take(text)),
    ));
}

fn question_to_param(question: DecisionQuestion) -> Result<decisions::QuestionParam, ConvertError> {
    let instructions = match question.instructions {
        UserContent::String(text) => decisions::Instructions::PurpleString(text),
        UserContent::Array(parts) => parts
            .into_iter()
            .map(|part| match part {
                UserContentPart::Text(text) => {
                    if text.encrypted_content.is_some()
                        || text.cache_control.is_some()
                        || text.provider_options.is_some()
                    {
                        return Err(unsupported(
                            "Lingua instruction text encrypted_content, cache_control, or provider_options",
                            TO_DECISIONS,
                        ));
                    }
                    Ok(decisions::DecisionQuestionInputPart {
                        text: Some(text.text),
                        decision_question_input_part_type:
                            decisions::DecisionQuestionInputPartType::InputText,
                        detail: None,
                        image_url: None,
                        data: None,
                    })
                }
                UserContentPart::Image {
                    image,
                    media_type,
                    provider_options,
                } => {
                    let (image_url, detail) =
                        image_to_decision(image, media_type, provider_options)?;
                    Ok(decisions::DecisionQuestionInputPart {
                        text: None,
                        decision_question_input_part_type:
                            decisions::DecisionQuestionInputPartType::InputImage,
                        detail,
                        image_url: Some(image_url),
                        data: None,
                    })
                }
                UserContentPart::Audio { .. } | UserContentPart::File { .. } => Err(unsupported(
                    "Lingua audio or file instructions",
                    TO_DECISIONS,
                )),
            })
            .collect::<Result<_, _>>()
            .map(decisions::Instructions::DecisionQuestionInputPartArray)?,
    };
    let (question_param_type, choices, levels) = match question.kind {
        DecisionQuestionKind::Predicate => (decisions::QuestionParamType::Predicate, None, None),
        DecisionQuestionKind::Choice { choices } => (
            decisions::QuestionParamType::Choice,
            Some(
                choices
                    .into_iter()
                    .map(|choice| decisions::ChoiceOptionParam {
                        description: choice.description,
                        value: match choice.value {
                            DecisionValue::Bool(value) => decisions::ChoiceValueParam::Bool(value),
                            DecisionValue::String(value) => {
                                decisions::ChoiceValueParam::PurpleString(value)
                            }
                        },
                    })
                    .collect(),
            ),
            None,
        ),
        DecisionQuestionKind::Score { levels } => (
            decisions::QuestionParamType::Score,
            None,
            Some(
                levels
                    .into_iter()
                    .map(|level| decisions::ScoreLevelParam {
                        description: level.description,
                        label: level.label,
                    })
                    .collect(),
            ),
        ),
    };
    Ok(decisions::QuestionParam {
        instructions,
        name: question.name,
        question_param_type,
        choices,
        levels,
    })
}

impl TryFromLLM<decisions::DecisionResponse> for UniversalDecisionResponse {
    type Error = ConvertError;

    fn try_from(response: decisions::DecisionResponse) -> Result<Self, Self::Error> {
        let usage = serde_json::to_value(&response.usage).map_err(|error| {
            ConvertError::JsonSerializationFailed {
                field: "usage".into(),
                error: error.to_string(),
            }
        })?;
        Ok(Self {
            model: Some(response.model),
            answers: response
                .answers
                .into_iter()
                .enumerate()
                .map(|(index, answer)| answer_from_resource(answer, index))
                .collect::<Result<_, _>>()?,
            usage: Some(UniversalUsage::from_provider_value(
                &usage,
                ProviderFormat::Decisions,
            )),
        })
    }
}

fn answer_from_resource(
    answer: decisions::AnswerResource,
    index: usize,
) -> Result<DecisionAnswer, ConvertError> {
    let location = format!("answers[{index}]");
    let field = |name: &str| format!("{location}.{name}");
    let result = match answer.answer_resource_type {
        decisions::AnswerResourceType::Predicate => DecisionResult::Predicate {
            probability: answer
                .probability
                .ok_or_else(|| missing(field("probability")))?,
        },
        decisions::AnswerResourceType::Choice => DecisionResult::Choice {
            choice: match answer.choice.ok_or_else(|| missing(field("choice")))? {
                decisions::ChoiceValueResource::Bool(value) => DecisionValue::Bool(value),
                decisions::ChoiceValueResource::PurpleString(value) => DecisionValue::String(value),
            },
            confidence: answer.confidence,
            probabilities: answer
                .probabilities
                .ok_or_else(|| missing(field("probabilities")))?
                .into_iter()
                .map(|probability| {
                    let value = match probability.value {
                        decisions::ValueUnion::Bool(value) => DecisionValue::Bool(value),
                        decisions::ValueUnion::PurpleString(value) => DecisionValue::String(value),
                        decisions::ValueUnion::Integer(value) => {
                            return Err(invalid(format!(
                                "{location} is a choice answer, but has a probability for level {value}"
                            )));
                        }
                    };
                    Ok(DecisionChoiceProbability {
                        value,
                        probability: probability.probability,
                    })
                })
                .collect::<Result<_, _>>()?,
        },
        decisions::AnswerResourceType::Score => DecisionResult::Score {
            score: answer.score.ok_or_else(|| missing(field("score")))?,
            confidence: answer.confidence,
            probabilities: answer
                .probabilities
                .ok_or_else(|| missing(field("probabilities")))?
                .into_iter()
                .map(|probability| {
                    let level = match probability.value {
                        decisions::ValueUnion::Integer(level) => u32::try_from(level).ok(),
                        _ => None,
                    }
                    .ok_or_else(|| {
                        invalid(format!(
                            "{location} is a score answer, so its probabilities must be keyed by level index"
                        ))
                    })?;
                    Ok(DecisionLevelProbability {
                        level,
                        label: probability.label,
                        probability: probability.probability,
                    })
                })
                .collect::<Result<_, _>>()?,
        },
        decisions::AnswerResourceType::Refusal => DecisionResult::Refusal,
    };
    Ok(DecisionAnswer {
        name: answer.name,
        result,
    })
}

impl TryFromLLM<UniversalDecisionResponse> for decisions::DecisionResponse {
    type Error = ConvertError;

    fn try_from(response: UniversalDecisionResponse) -> Result<Self, Self::Error> {
        let usage = response.usage.ok_or_else(|| missing("usage"))?;
        let usage = serde_json::from_value(usage.to_provider_value(ProviderFormat::Decisions))
            .map_err(|error| ConvertError::JsonSerializationFailed {
                field: "usage".into(),
                error: error.to_string(),
            })?;
        Ok(Self {
            answers: response
                .answers
                .into_iter()
                .map(answer_to_resource)
                .collect(),
            model: response.model.ok_or_else(|| missing("model"))?,
            usage,
        })
    }
}

fn answer_to_resource(answer: DecisionAnswer) -> decisions::AnswerResource {
    let base = decisions::AnswerResource {
        name: answer.name,
        probability: None,
        answer_resource_type: decisions::AnswerResourceType::Refusal,
        choice: None,
        confidence: None,
        probabilities: None,
        score: None,
    };
    let value_union = |value: DecisionValue| match value {
        DecisionValue::Bool(value) => decisions::ValueUnion::Bool(value),
        DecisionValue::String(value) => decisions::ValueUnion::PurpleString(value),
    };
    match answer.result {
        DecisionResult::Predicate { probability } => decisions::AnswerResource {
            answer_resource_type: decisions::AnswerResourceType::Predicate,
            probability: Some(probability),
            ..base
        },
        DecisionResult::Choice {
            choice,
            confidence,
            probabilities,
        } => decisions::AnswerResource {
            answer_resource_type: decisions::AnswerResourceType::Choice,
            choice: Some(match choice {
                DecisionValue::Bool(value) => decisions::ChoiceValueResource::Bool(value),
                DecisionValue::String(value) => decisions::ChoiceValueResource::PurpleString(value),
            }),
            confidence,
            probabilities: Some(
                probabilities
                    .into_iter()
                    .map(|probability| decisions::EProbabilityResource {
                        probability: probability.probability,
                        value: value_union(probability.value),
                        label: None,
                    })
                    .collect(),
            ),
            ..base
        },
        DecisionResult::Score {
            score,
            confidence,
            probabilities,
        } => decisions::AnswerResource {
            answer_resource_type: decisions::AnswerResourceType::Score,
            score: Some(score),
            confidence,
            probabilities: Some(
                probabilities
                    .into_iter()
                    .map(|probability| decisions::EProbabilityResource {
                        probability: probability.probability,
                        value: decisions::ValueUnion::Integer(i64::from(probability.level)),
                        label: probability.label,
                    })
                    .collect(),
            ),
            ..base
        },
        DecisionResult::Refusal => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::{json, Value};

    fn to_universal(request: Value) -> Result<UniversalDecisionRequest, ConvertError> {
        let request: decisions::DecisionRequest = serde_json::from_value(request).unwrap();
        TryFromLLM::try_from(request)
    }

    fn from_universal(request: UniversalDecisionRequest) -> Result<Value, ConvertError> {
        <decisions::DecisionRequest as TryFromLLM<_>>::try_from(request)
            .map(|request| serde_json::to_value(request).unwrap())
    }

    #[test]
    fn converts_requests_to_universal_decisions() {
        let universal = to_universal(json!({
            "model": "gpt-6-luna",
            "input": [
                {"role": "system", "content": "Route the ticket."},
                {"role": "user", "content": "I was charged twice."}
            ],
            "questions": [
                {"type": "predicate", "name": "billing", "instructions": "Is this about billing?"},
                {"type": "choice", "name": "department", "instructions": "Which department?",
                    "choices": [{"value": "billing"}, {"value": false, "description": "None"}]},
                {"type": "score", "instructions": "How urgent is it?",
                    "levels": [{"label": "low"}, {"label": "high", "description": "Blocks work"}]}
            ],
            "prompt_cache_key": "support"
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&universal).unwrap(),
            json!({
                "model": "gpt-6-luna",
                "input": [
                    {"role": "system", "content": "Route the ticket."},
                    {"role": "user", "content": "I was charged twice."}
                ],
                "questions": [
                    {"name": "billing", "instructions": "Is this about billing?", "type": "predicate"},
                    {"name": "department", "instructions": "Which department?", "type": "choice",
                        "choices": [{"value": "billing"}, {"value": false, "description": "None"}]},
                    {"instructions": "How urgent is it?", "type": "score",
                        "levels": [{"label": "low"}, {"label": "high", "description": "Blocks work"}]}
                ],
                "prompt_cache_key": "support"
            })
        );
    }

    #[test]
    fn requests_round_trip_through_universal_decisions() {
        let request = json!({
            "input": [
                {"type": "message", "role": "developer", "content": "Grade the transcript."},
                {"type": "message", "role": "user", "content": [
                    {"type": "input_text", "text": "Where is my order?",
                        "prompt_cache_breakpoint": {"mode": "explicit"}},
                    {"type": "input_image", "image_url": "data:image/png;base64,AAAA", "detail": "low"},
                    {"type": "input_image", "image_url": "https://example.com/receipt.png"}
                ]},
                {"type": "message", "role": "assistant", "content": [
                    {"type": "output_text", "text": "Let me look it up."}
                ]},
                {"type": "function_call", "id": "fc_1", "call_id": "c1", "name": "lookup_order",
                    "arguments": "{\"order\":\"A1\"}", "status": "in_progress"},
                {"type": "function_call_output", "call_id": "c1", "output": "shipped"},
                {"type": "custom_tool_call", "call_id": "c2", "name": "sql", "input": "select 1"},
                {"type": "custom_tool_call_output", "call_id": "c2", "output": "1"},
                {"type": "message", "role": "assistant", "content": "It shipped."}
            ],
            "model": "gpt-6-luna",
            "prompt_cache_key": "support",
            "questions": [
                {"instructions": [
                    {"type": "input_text", "text": "Did the agent resolve it?"},
                    {"type": "input_image", "image_url": "https://example.com/rubric.png"}
                ], "name": "resolved", "type": "predicate"},
                {"instructions": "Tone?", "type": "choice", "choices": [
                    {"value": "friendly"}, {"description": "Curt", "value": true}
                ]},
                {"instructions": "Quality?", "name": "quality", "type": "score", "levels": [
                    {"label": "poor"}, {"description": "Fully correct", "label": "great"}
                ]}
            ]
        });
        let universal = to_universal(request.clone()).unwrap();
        let DecisionInput::Messages(messages) = &universal.input else {
            panic!("expected messages");
        };
        assert_eq!(messages.len(), 8);
        assert!(matches!(
            &messages[4],
            Message::Tool { content } if matches!(
                &content[0],
                ToolContentPart::ToolResult(result) if result.tool_name == "lookup_order"
            )
        ));
        assert_eq!(from_universal(universal).unwrap(), request);
    }

    #[test]
    fn rejects_requests_universal_decisions_cannot_represent() {
        let request = |input: Value| {
            json!({
                "model": "gpt-6-luna",
                "input": input,
                "questions": [{"type": "predicate", "instructions": "?"}]
            })
        };
        let cases = [
            (
                "input_json",
                request(
                    json!([{"role": "user", "content": [{"type": "input_json", "data": {"a": 1}}]}]),
                ),
            ),
            (
                "refusal",
                request(
                    json!([{"role": "assistant", "content": [{"type": "refusal", "refusal": "no"}]}]),
                ),
            ),
            (
                "namespace",
                request(
                    json!([{"type": "function_call", "call_id": "c1", "name": "f",
                    "arguments": "{}", "namespace": "mcp"}]),
                ),
            ),
            (
                "tool output content parts",
                request(json!([
                    {"type": "function_call", "call_id": "c1", "name": "f", "arguments": "{}"},
                    {"type": "function_call_output", "call_id": "c1",
                        "output": [{"type": "input_text", "text": "x"}]}
                ])),
            ),
            (
                "does not take `call_id`",
                request(json!([{"role": "user", "content": "x", "call_id": "c1"}])),
            ),
            (
                "must be output text",
                request(
                    json!([{"role": "assistant", "content": [{"type": "input_text", "text": "x"}]}]),
                ),
            ),
        ];
        for (expected, request) in cases {
            let error = to_universal(request).expect_err(expected).to_string();
            assert!(error.contains(expected), "{expected}: {error}");
        }

        let assistant_with_id = UniversalDecisionRequest {
            model: Some("gpt-6-luna".into()),
            input: DecisionInput::Messages(vec![Message::Assistant {
                content: AssistantContent::String("x".into()),
                id: Some("msg_1".into()),
            }]),
            questions: vec![],
            prompt_cache_key: None,
        };
        let error = from_universal(assistant_with_id).unwrap_err().to_string();
        assert!(error.contains("assistant message id"), "{error}");
    }

    #[test]
    fn responses_round_trip_through_universal_decisions() {
        let response = json!({
            "answers": [
                {"name": "billing", "probability": 0.97, "type": "predicate"},
                {"name": null, "type": "choice", "choice": true, "confidence": 0.5,
                    "probabilities": [
                        {"probability": 0.75, "value": true},
                        {"probability": 0.25, "value": "unsure"}
                    ]},
                {"name": "quality", "type": "score", "confidence": 0.25,
                    "probabilities": [
                        {"probability": 0.25, "value": 0, "label": "poor"},
                        {"probability": 0.75, "value": 1, "label": "great"}
                    ], "score": 0.75},
                {"name": "unsafe", "type": "refusal"}
            ],
            "model": "gpt-6-luna",
            "usage": {
                "input_tokens": 1200,
                "input_tokens_details": {"cache_write_tokens": 0, "cached_tokens": 1024},
                "output_tokens": 0,
                "output_tokens_details": {"reasoning_tokens": 0},
                "total_tokens": 1200
            }
        });
        let decision: decisions::DecisionResponse =
            serde_json::from_value(response.clone()).unwrap();
        let universal: UniversalDecisionResponse = TryFromLLM::try_from(decision).unwrap();
        assert_eq!(
            serde_json::to_value(&universal.answers).unwrap(),
            json!([
                {"name": "billing", "type": "predicate", "probability": 0.97},
                {"type": "choice", "choice": true, "confidence": 0.5, "probabilities": [
                    {"value": true, "probability": 0.75},
                    {"value": "unsure", "probability": 0.25}
                ]},
                {"name": "quality", "type": "score", "score": 0.75, "confidence": 0.25,
                    "probabilities": [
                        {"level": 0, "label": "poor", "probability": 0.25},
                        {"level": 1, "label": "great", "probability": 0.75}
                    ]},
                {"name": "unsafe", "type": "refusal"}
            ])
        );
        let usage = universal.usage.as_ref().unwrap();
        assert_eq!(usage.prompt_tokens, Some(1200));
        assert_eq!(usage.prompt_cached_tokens, Some(1024));
        let back: decisions::DecisionResponse = TryFromLLM::try_from(universal).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), response);
    }
}
