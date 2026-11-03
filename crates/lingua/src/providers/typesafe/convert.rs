/*!
Conversion between universal decision types and TypeSafe System One payloads.

Text input becomes a string `state`. A conversation becomes
`{"messages": [...]}`, using a stable transcript shape with plain tool arguments.
Questions are keyed by name, or by index when unnamed. System One answers carry
only those keys and choice names, so converting a response takes the request's
questions to restore question order and typed choice values.
*/

use super::entry::{TypeSafeEntry, TypeSafeJson};
use super::generated::{
    Answer, Asset, Format, Question, SystemOneRequest, SystemOneResponse, Type,
};
use crate::error::ConvertError;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, UserContent, UserContentPart,
};
use crate::universal::{
    DecisionAnswer, DecisionChoice, DecisionChoiceProbability, DecisionInput,
    DecisionLevelProbability, DecisionQuestion, DecisionQuestionKind, DecisionResult,
    DecisionScoreLevel, DecisionValue, UniversalDecisionRequest, UniversalDecisionResponse,
    UniversalUsage,
};
use crate::util::media::parse_base64_data_url;
use indexmap::IndexMap;
use serde::Serialize;
use std::borrow::Cow;

const TARGET: &str = "TypeSafe System One";

#[derive(Serialize)]
struct MessagesState<'a> {
    messages: Vec<StateMessage<'a>>,
}

#[derive(Serialize)]
struct StateMessage<'a> {
    role: &'static str,
    content: StateContent<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum StateContent<'a> {
    Text(&'a str),
    Parts(Vec<StatePart<'a>>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StatePart<'a> {
    Text {
        text: &'a str,
    },
    Image {
        image: String,
    },
    ToolCall {
        tool_call_id: &'a str,
        tool_name: &'a str,
        arguments: StateArguments<'a>,
    },
    ToolResult {
        tool_call_id: &'a str,
        tool_name: &'a str,
        output: &'a serde_json::Value,
    },
}

#[derive(Serialize)]
#[serde(untagged)]
enum StateArguments<'a> {
    Json(&'a serde_json::Map<String, serde_json::Value>),
    Text(&'a str),
}

struct ImageAssets {
    prefix: String,
    values: IndexMap<String, Asset>,
}

impl ImageAssets {
    fn new(request: &UniversalDecisionRequest) -> Result<Self, ConvertError> {
        let has_images = |content: &UserContent| {
            matches!(content,
                UserContent::Array(parts) if parts.iter().any(|part| matches!(part, UserContentPart::Image { .. }))
            )
        };
        let mut prefix = "__lingua_image_".to_string();
        let input_has_images = match &request.input {
            DecisionInput::Content(content) => has_images(content),
            DecisionInput::Messages(messages) => messages.iter().any(|message| match message {
                Message::System { content }
                | Message::Developer { content }
                | Message::User { content } => has_images(content),
                _ => false,
            }),
        };
        if input_has_images
            || request
                .questions
                .iter()
                .any(|question| has_images(&question.instructions))
        {
            let source = serde_json::to_string(request).map_err(|error| {
                ConvertError::JsonSerializationFailed {
                    field: "decision request".into(),
                    error: error.to_string(),
                }
            })?;
            while source.contains(&prefix) {
                prefix.push('_');
            }
        }
        Ok(Self {
            prefix,
            values: IndexMap::new(),
        })
    }

    fn insert(
        &mut self,
        image: &serde_json::Value,
        media_type: Option<&str>,
    ) -> Result<String, ConvertError> {
        let serde_json::Value::String(image) = image else {
            return Err(unsupported(
                "Lingua image content that is not a base64 string",
            ));
        };
        if image.starts_with("http://") || image.starts_with("https://") {
            return Err(unsupported(
                "remote image content; TypeSafe requires inline PNG/JPEG data",
            ));
        }
        let block = parse_base64_data_url(image);
        if image.starts_with("data:") && block.is_none() {
            return Err(invalid("invalid image data URL"));
        }
        let (data, media_type): (&str, &str) = match &block {
            Some(block) => {
                if media_type.is_some_and(|media_type| media_type != block.media_type) {
                    return Err(invalid("image media_type does not match its data URL"));
                }
                (block.data.as_ref(), block.media_type.as_ref())
            }
            None => (
                image.as_ref(),
                media_type.ok_or_else(|| missing("media_type for raw base64 image"))?,
            ),
        };
        let format = match media_type {
            "image/png" => Format::Png,
            "image/jpeg" => Format::Jpeg,
            _ => {
                return Err(unsupported(format!(
                    "image media type {media_type}; TypeSafe supports PNG/JPEG"
                )))
            }
        };
        let placeholder = format!("{}{}__", self.prefix, self.values.len());
        self.values.insert(
            placeholder.clone(),
            Asset {
                data_base64: Some(data.to_string()),
                format: Some(format),
                part: None,
                type_: Some(Type::Image),
            },
        );
        Ok(placeholder)
    }
}

fn unsupported(from: impl Into<String>) -> ConvertError {
    ConvertError::UnsupportedMapping {
        from: from.into(),
        to: TARGET,
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

impl TryFromLLM<UniversalDecisionRequest> for SystemOneRequest {
    type Error = ConvertError;

    fn try_from(request: UniversalDecisionRequest) -> Result<Self, ConvertError> {
        if request
            .params
            .extras
            .typesafe
            .as_ref()
            .is_some_and(|extras| extras.images.is_some() || extras.long_context.is_some())
        {
            return Err(unsupported(
                "TypeSafe options require HTTP headers; use the universal gateway endpoint",
            ));
        }
        let mut assets = ImageAssets::new(&request)?;
        let model = request.model.ok_or_else(|| missing("model"))?;
        let state = match request.input {
            DecisionInput::Content(UserContent::String(text)) => TypeSafeEntry::Text(text),
            DecisionInput::Content(content) => TypeSafeEntry::Json(
                TypeSafeJson::from_serialize(&state_user_content(
                    &content,
                    "Lingua decision input",
                    &mut assets,
                )?)
                .map_err(|error| ConvertError::JsonSerializationFailed {
                    field: "state".into(),
                    error: error.to_string(),
                })?,
            ),
            DecisionInput::Messages(messages) => TypeSafeEntry::Json(
                TypeSafeJson::from_serialize(&MessagesState {
                    messages: messages
                        .iter()
                        .enumerate()
                        .map(|(index, message)| state_message(message, index, &mut assets))
                        .collect::<Result<_, _>>()?,
                })
                .map_err(|error| ConvertError::JsonSerializationFailed {
                    field: "state".into(),
                    error: error.to_string(),
                })?,
            ),
        };

        let mut questions = IndexMap::with_capacity(request.questions.len());
        for (index, question) in request.questions.into_iter().enumerate() {
            let key = question.name.unwrap_or_else(|| index.to_string());
            if questions.contains_key(&key) {
                return Err(invalid(format!(
                    "question name \"{key}\" is not unique; TypeSafe identifies answers by question name"
                )));
            }
            let instructions = Some(convert_instructions(question.instructions, &mut assets)?);
            let question = match question.kind {
                DecisionQuestionKind::Predicate => Question::Noul {
                    instructions,
                    criteria: None,
                },
                DecisionQuestionKind::Choice { choices } => Question::Choice {
                    instructions,
                    criteria: convert_choices(choices)?,
                },
                DecisionQuestionKind::Score { levels } => Question::Score {
                    instructions,
                    criteria: levels.into_iter().map(score_criterion).collect(),
                },
            };
            questions.insert(key, question);
        }

        Ok(SystemOneRequest {
            assets: assets.values,
            model,
            questions,
            state,
        })
    }
}

/// The key System One uses for the question at `index`.
fn question_key(question: &DecisionQuestion, index: usize) -> Cow<'_, str> {
    match &question.name {
        Some(name) => Cow::Borrowed(name),
        None => Cow::Owned(index.to_string()),
    }
}

/// The System One choice name for `value`.
fn choice_key(value: &DecisionValue) -> &str {
    match value {
        DecisionValue::String(value) => value,
        DecisionValue::Bool(true) => "true",
        DecisionValue::Bool(false) => "false",
    }
}

fn validate_text(text: &TextContentPart, location: &str) -> Result<(), ConvertError> {
    if text.cache_control.is_some() {
        return Err(unsupported(format!("{location} cache_control")));
    }
    if text.encrypted_content.is_some() || text.provider_options.is_some() {
        return Err(unsupported(format!(
            "{location} encrypted_content or provider_options"
        )));
    }
    Ok(())
}

fn state_user_content<'a>(
    content: &'a UserContent,
    location: &str,
    assets: &mut ImageAssets,
) -> Result<StateContent<'a>, ConvertError> {
    match content {
        UserContent::String(text) => Ok(StateContent::Text(text)),
        UserContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                UserContentPart::Text(text) => {
                    validate_text(text, location)?;
                    Ok(StatePart::Text { text: &text.text })
                }
                UserContentPart::Image {
                    image,
                    media_type,
                    provider_options,
                } => {
                    if provider_options.is_some() {
                        return Err(unsupported(format!("{location} image provider_options")));
                    }
                    Ok(StatePart::Image {
                        image: assets.insert(image, media_type.as_deref())?,
                    })
                }
                UserContentPart::Audio { .. } => {
                    Err(unsupported(format!("{location} audio content")))
                }
                UserContentPart::File { .. } => {
                    Err(unsupported(format!("{location} file content")))
                }
            })
            .collect::<Result<_, _>>()
            .map(StateContent::Parts),
    }
}

fn state_message<'a>(
    message: &'a Message,
    index: usize,
    assets: &mut ImageAssets,
) -> Result<StateMessage<'a>, ConvertError> {
    let location = format!("Lingua message {index}");
    let (role, content) = match message {
        Message::System { content } => ("system", state_user_content(content, &location, assets)?),
        Message::Developer { content } => ("developer", state_user_content(content, &location, assets)?),
        Message::User { content } => ("user", state_user_content(content, &location, assets)?),
        Message::Assistant { content, .. } => ("assistant", match content {
            AssistantContent::String(text) => StateContent::Text(text),
            AssistantContent::Array(parts) => StateContent::Parts(parts.iter().map(|part| match part {
                AssistantContentPart::Text(text) => {
                    validate_text(text, &location)?;
                    Ok(StatePart::Text { text: &text.text })
                }
                AssistantContentPart::ToolCall { tool_call_id, tool_name, arguments, status,
                    caller, encrypted_content, provider_options, provider_executed, .. } => {
                    if let Some(status) = status.as_deref().filter(|status| *status != "completed") {
                        return Err(unsupported(format!("{location} tool call status '{status}'")));
                    }
                    if caller.is_some() || encrypted_content.is_some()
                        || provider_options.is_some() || provider_executed.is_some() {
                        return Err(unsupported(format!("{location} tool call caller, encrypted_content, provider_options, or provider_executed")));
                    }
                    Ok(StatePart::ToolCall {
                        tool_call_id, tool_name,
                        arguments: match arguments {
                            ToolCallArguments::Valid(value) => StateArguments::Json(value),
                            ToolCallArguments::Invalid(text) | ToolCallArguments::Custom(text) => StateArguments::Text(text),
                        },
                    })
                }
                AssistantContentPart::File { .. } => Err(unsupported(format!("{location} file content"))),
                AssistantContentPart::Reasoning { .. } => Err(unsupported(format!("{location} reasoning content"))),
                AssistantContentPart::Program { .. } | AssistantContentPart::ProgramOutput { .. } => Err(unsupported(format!("{location} program content"))),
                AssistantContentPart::ToolDiscoveryCall { .. } => Err(unsupported(format!("{location} tool discovery call"))),
                AssistantContentPart::ToolResult { .. } => Err(unsupported(format!("{location} tool result in an assistant message"))),
            }).collect::<Result<_, _>>()?),
        }),
        Message::Tool { content } => ("tool", StateContent::Parts(content.iter().map(|part| match part {
            ToolContentPart::ToolResult(result) => {
                if result.caller.is_some() || result.provider_options.is_some() {
                    return Err(unsupported(format!("{location} tool result caller or provider_options")));
                }
                Ok(StatePart::ToolResult {
                    tool_call_id: &result.tool_call_id,
                    tool_name: &result.tool_name,
                    output: &result.output,
                })
            }
            ToolContentPart::ToolDiscoveryResult(_) => Err(unsupported(format!("{location} tool discovery result"))),
        }).collect::<Result<_, _>>()?)),
        Message::AdditionalTools { .. } => return Err(unsupported(format!("{location} additional_tools"))),
    };
    Ok(StateMessage { role, content })
}

fn convert_instructions(
    instructions: UserContent,
    assets: &mut ImageAssets,
) -> Result<TypeSafeEntry, ConvertError> {
    match instructions {
        UserContent::String(text) => Ok(TypeSafeEntry::Text(text)),
        UserContent::Array(parts) => parts
            .into_iter()
            .map(|part| match part {
                UserContentPart::Text(text) => {
                    validate_text(&text, "Lingua decision instructions")?;
                    Ok(serde_json::Value::String(text.text))
                }
                UserContentPart::Image {
                    image,
                    media_type,
                    provider_options,
                } => {
                    if provider_options.is_some() {
                        return Err(unsupported(
                            "Lingua decision instruction image provider_options",
                        ));
                    }
                    Ok(serde_json::Value::String(
                        assets.insert(&image, media_type.as_deref())?,
                    ))
                }
                UserContentPart::Audio { .. } | UserContentPart::File { .. } => {
                    Err(unsupported("Lingua decision instruction audio or files"))
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(TypeSafeEntry::Array),
    }
}

fn convert_choices(
    choices: Vec<DecisionChoice>,
) -> Result<IndexMap<String, Option<TypeSafeEntry>>, ConvertError> {
    let mut criteria = IndexMap::with_capacity(choices.len());
    for choice in choices {
        let key = match choice.value {
            DecisionValue::String(value) => value,
            DecisionValue::Bool(value) => value.to_string(),
        };
        if criteria.contains_key(&key) {
            return Err(invalid(format!(
                "choice \"{key}\" appears more than once; TypeSafe choices are named by text"
            )));
        }
        criteria.insert(key, choice.description.map(TypeSafeEntry::Text));
    }
    Ok(criteria)
}

fn score_criterion(level: DecisionScoreLevel) -> TypeSafeEntry {
    match level.description {
        None => TypeSafeEntry::Text(level.label),
        Some(description) => {
            let mut criterion = serde_json::Map::new();
            criterion.insert("label".into(), serde_json::Value::String(level.label));
            criterion.insert("description".into(), serde_json::Value::String(description));
            TypeSafeEntry::Object(criterion)
        }
    }
}

/// Convert a TypeSafe System One response to universal answers, in the order of
/// `questions`, the questions of the request it answers.
pub fn system_one_response_to_universal(
    response: SystemOneResponse,
    questions: &[DecisionQuestion],
) -> Result<UniversalDecisionResponse, ConvertError> {
    if response.answers.len() > questions.len() {
        return Err(invalid("TypeSafe must return one answer per question"));
    }
    let mut answers = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let key = question_key(question, index);
        let answer = response
            .answers
            .get(key.as_ref())
            .ok_or_else(|| missing(format!("answers.{key}")))?;
        answers.push(DecisionAnswer {
            name: question.name.clone(),
            result: convert_answer(&question.kind, &key, answer)?,
        });
    }
    let usage = response.usage;
    Ok(UniversalDecisionResponse {
        model: Some(response.model),
        answers,
        usage: Some(UniversalUsage {
            prompt_tokens: Some(usage.input_tokens),
            completion_tokens: Some(usage.output_tokens),
            total_tokens: Some(usage.input_tokens + usage.output_tokens),
            ..UniversalUsage::default()
        }),
    })
}

fn convert_answer(
    kind: &DecisionQuestionKind,
    key: &str,
    answer: &Answer,
) -> Result<DecisionResult, ConvertError> {
    match (kind, answer) {
        (DecisionQuestionKind::Predicate, Answer::Noul { noul }) => {
            Ok(DecisionResult::Predicate { probability: *noul })
        }
        (
            DecisionQuestionKind::Choice { choices },
            Answer::Choice {
                choice,
                confidence,
                probabilities,
            },
        ) => {
            if probabilities.len() > choices.len() {
                return Err(invalid(
                    "TypeSafe must return exactly one probability per choice",
                ));
            }
            let selected = choices
                .iter()
                .find(|candidate| choice_key(&candidate.value) == choice)
                .map(|candidate| candidate.value.clone())
                .ok_or_else(|| {
                    invalid(format!(
                        "TypeSafe returned unknown choice \"{choice}\" for question \"{key}\""
                    ))
                })?;
            Ok(DecisionResult::Choice {
                choice: selected,
                confidence: Some(*confidence),
                probabilities: choices
                    .iter()
                    .map(|candidate| {
                        Ok(DecisionChoiceProbability {
                            value: candidate.value.clone(),
                            probability: probability_for(
                                probabilities,
                                choice_key(&candidate.value),
                                key,
                            )?,
                        })
                    })
                    .collect::<Result<_, ConvertError>>()?,
            })
        }
        (
            DecisionQuestionKind::Score { levels },
            Answer::Score {
                score,
                confidence,
                probabilities,
                ..
            },
        ) => {
            if probabilities.len() > levels.len() {
                return Err(invalid(
                    "TypeSafe must return exactly one probability per level",
                ));
            }
            Ok(DecisionResult::Score {
                score: *score,
                confidence: Some(*confidence),
                probabilities: (0u32..)
                    .zip(levels)
                    .map(|(level, score_level)| {
                        Ok(DecisionLevelProbability {
                            level,
                            label: Some(score_level.label.clone()),
                            probability: probability_for(probabilities, &level.to_string(), key)?,
                        })
                    })
                    .collect::<Result<_, ConvertError>>()?,
            })
        }
        (_, answer) => Err(invalid(format!(
            "TypeSafe returned a {} answer for {} question \"{key}\"",
            answer_type_name(answer),
            question_kind_name(kind)
        ))),
    }
}

fn probability_for(
    probabilities: &IndexMap<String, f64>,
    key: &str,
    question: &str,
) -> Result<f64, ConvertError> {
    probabilities
        .get(key)
        .copied()
        .ok_or_else(|| missing(format!("answers.{question}.probabilities.{key}")))
}

fn answer_type_name(answer: &Answer) -> &'static str {
    match answer {
        Answer::Noul { .. } => "noul",
        Answer::Choice { .. } => "choice",
        Answer::Score { .. } => "score",
        Answer::Box { .. } => "box",
    }
}

fn question_kind_name(kind: &DecisionQuestionKind) -> &'static str {
    match kind {
        DecisionQuestionKind::Predicate => "predicate",
        DecisionQuestionKind::Choice { .. } => "choice",
        DecisionQuestionKind::Score { .. } => "score",
    }
}
