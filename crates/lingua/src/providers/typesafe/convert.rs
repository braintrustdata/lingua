/*!
Conversion between universal decision types and TypeSafe System One payloads.

Text input becomes a string `state`. A conversation becomes
`{"messages": [...]}`, using a stable transcript shape with plain tool arguments.
Questions are keyed by name, or by index when unnamed. System One answers carry
only those keys and choice names, so converting a response takes the request's
questions to restore question order and typed choice values.
*/

use super::entry::{TypeSafeEntry, TypeSafeJson};
use super::generated::{Answer, Question, SystemOneRequest, SystemOneResponse};
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
            .get(&crate::ProviderFormat::TypeSafe)
            .is_some_and(|extras| !extras.is_empty())
        {
            return Err(unsupported(
                "TypeSafe options require HTTP headers; use the universal gateway endpoint",
            ));
        }
        let model = request.model.ok_or_else(|| missing("model"))?;
        let state = match request.input {
            DecisionInput::Text(text) => TypeSafeEntry::Text(text),
            DecisionInput::Messages(messages) => TypeSafeEntry::Json(
                TypeSafeJson::from_serialize(&MessagesState {
                    messages: messages
                        .iter()
                        .enumerate()
                        .map(|(index, message)| state_message(message, index))
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
            let instructions = Some(convert_instructions(question.instructions)?);
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
            assets: IndexMap::new(),
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
                UserContentPart::Image { .. } => {
                    Err(unsupported(format!("{location} image content")))
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

fn state_message(message: &Message, index: usize) -> Result<StateMessage<'_>, ConvertError> {
    let location = format!("Lingua message {index}");
    let (role, content) = match message {
        Message::System { content } => ("system", state_user_content(content, &location)?),
        Message::Developer { content } => ("developer", state_user_content(content, &location)?),
        Message::User { content } => ("user", state_user_content(content, &location)?),
        Message::Assistant { content, .. } => ("assistant", match content {
            AssistantContent::String(text) => StateContent::Text(text),
            AssistantContent::Array(parts) => StateContent::Parts(parts.iter().map(|part| match part {
                AssistantContentPart::Text(text) => {
                    validate_text(text, &location)?;
                    Ok(StatePart::Text { text: &text.text })
                }
                AssistantContentPart::ToolCall { tool_call_id, tool_name, arguments,
                    caller, encrypted_content, provider_options, provider_executed, .. } => {
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

fn convert_instructions(instructions: UserContent) -> Result<TypeSafeEntry, ConvertError> {
    match instructions {
        UserContent::String(text) => Ok(TypeSafeEntry::Text(text)),
        UserContent::Array(parts) => parts
            .into_iter()
            .map(|part| match part {
                UserContentPart::Text(text) => {
                    validate_text(&text, "Lingua decision instructions")?;
                    Ok(serde_json::Value::String(text.text))
                }
                UserContentPart::Image { .. } => {
                    Err(unsupported("Lingua decision instruction images"))
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
