/*!
Conversion between universal decision types and TypeSafe System One payloads.

Text input becomes a string `state`. A conversation becomes
`{"messages": [...]}`, with each message in Lingua's universal JSON form, since
decision models read the state as text.
Questions are keyed by name, or by index when unnamed. System One answers carry
only those keys and choice names, so converting a response takes the request's
questions to restore question order and typed choice values.
*/

use super::request::{SystemOneRequest, TypeSafeEntry, TypeSafeJson, TypeSafeQuestion};
use super::response::{SystemOneResponse, TypeSafeAnswer};
use crate::error::ConvertError;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContentPart, UserContent,
    UserContentPart,
};
use crate::universal::{
    DecisionAnswer, DecisionChoice, DecisionChoiceProbability, DecisionInput,
    DecisionLevelProbability, DecisionQuestion, DecisionQuestionKind, DecisionResult,
    DecisionScoreLevel, DecisionValue, UniversalDecisionRequest, UniversalDecisionResponse,
    UniversalUsage,
};
use serde::Serialize;
use std::borrow::Cow;

const TARGET: &str = "TypeSafe System One";

#[derive(Serialize)]
struct MessagesState<'a> {
    messages: &'a [Message],
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
        if request.prompt_cache_key.is_some() {
            return Err(unsupported("Lingua decision prompt_cache_key"));
        }
        let model = request.model.ok_or_else(|| missing("model"))?;
        let state = match request.input {
            DecisionInput::Text(text) => TypeSafeEntry::Text(text),
            DecisionInput::Messages(messages) => {
                for (index, message) in messages.iter().enumerate() {
                    validate_message(message, index)?;
                }
                TypeSafeEntry::Json(
                    TypeSafeJson::from_serialize(&MessagesState {
                        messages: &messages,
                    })
                    .map_err(|error| {
                        ConvertError::JsonSerializationFailed {
                            field: "state".into(),
                            error: error.to_string(),
                        }
                    })?,
                )
            }
        };

        let mut questions: Vec<(String, TypeSafeQuestion)> =
            Vec::with_capacity(request.questions.len());
        for (index, question) in request.questions.into_iter().enumerate() {
            let key = question.name.unwrap_or_else(|| index.to_string());
            if questions.iter().any(|(existing, _)| *existing == key) {
                return Err(invalid(format!(
                    "question name \"{key}\" is not unique; TypeSafe identifies answers by question name"
                )));
            }
            let instructions = Some(convert_instructions(question.instructions)?);
            let question = match question.kind {
                DecisionQuestionKind::Predicate => TypeSafeQuestion::Noul {
                    instructions,
                    criteria: None,
                },
                DecisionQuestionKind::Choice { choices } => TypeSafeQuestion::Choice {
                    instructions,
                    criteria: convert_choices(choices)?,
                },
                DecisionQuestionKind::Score { levels } => TypeSafeQuestion::Score {
                    instructions,
                    criteria: levels.into_iter().map(score_criterion).collect(),
                },
            };
            questions.push((key, question));
        }

        Ok(SystemOneRequest {
            model,
            state,
            questions,
            assets: None,
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

fn validate_user_content(content: &UserContent, location: &str) -> Result<(), ConvertError> {
    let UserContent::Array(parts) = content else {
        return Ok(());
    };
    for part in parts {
        match part {
            UserContentPart::Text(text) => validate_text(text, location)?,
            UserContentPart::Image { .. } => {
                return Err(unsupported(format!("{location} image content")));
            }
            UserContentPart::Audio { .. } => {
                return Err(unsupported(format!("{location} audio content")));
            }
            UserContentPart::File { .. } => {
                return Err(unsupported(format!("{location} file content")));
            }
        }
    }
    Ok(())
}

fn validate_message(message: &Message, index: usize) -> Result<(), ConvertError> {
    let location = format!("Lingua message {index}");
    match message {
        Message::System { content }
        | Message::Developer { content }
        | Message::User { content } => validate_user_content(content, &location),
        Message::Assistant { content, .. } => match content {
            AssistantContent::Array(parts) => validate_assistant_parts(parts, &location),
            AssistantContent::String(_) => Ok(()),
        },
        Message::Tool { content } => {
            for part in content {
                match part {
                    ToolContentPart::ToolResult(result) => {
                        if result.caller.is_some() || result.provider_options.is_some() {
                            return Err(unsupported(format!(
                                "{location} tool result caller or provider_options"
                            )));
                        }
                    }
                    ToolContentPart::ToolDiscoveryResult(_) => {
                        return Err(unsupported(format!("{location} tool discovery result")));
                    }
                }
            }
            Ok(())
        }
        Message::AdditionalTools { .. } => Err(unsupported(format!("{location} additional_tools"))),
    }
}

fn validate_assistant_parts(
    parts: &[AssistantContentPart],
    location: &str,
) -> Result<(), ConvertError> {
    for part in parts {
        match part {
            AssistantContentPart::Text(text) => validate_text(text, location)?,
            AssistantContentPart::ToolCall {
                caller,
                encrypted_content,
                provider_options,
                provider_executed,
                ..
            } => {
                if caller.is_some()
                    || encrypted_content.is_some()
                    || provider_options.is_some()
                    || provider_executed.is_some()
                {
                    return Err(unsupported(format!(
                        "{location} tool call caller, encrypted_content, provider_options, or provider_executed"
                    )));
                }
            }
            AssistantContentPart::File { .. } => {
                return Err(unsupported(format!("{location} file content")));
            }
            AssistantContentPart::Reasoning { .. } => {
                return Err(unsupported(format!("{location} reasoning content")));
            }
            AssistantContentPart::Program { .. } | AssistantContentPart::ProgramOutput { .. } => {
                return Err(unsupported(format!("{location} program content")));
            }
            AssistantContentPart::ToolDiscoveryCall { .. } => {
                return Err(unsupported(format!("{location} tool discovery call")));
            }
            AssistantContentPart::ToolResult { .. } => {
                return Err(unsupported(format!(
                    "{location} tool result in an assistant message"
                )));
            }
        }
    }
    Ok(())
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
) -> Result<Vec<(String, Option<TypeSafeEntry>)>, ConvertError> {
    let mut criteria: Vec<(String, Option<TypeSafeEntry>)> = Vec::with_capacity(choices.len());
    for choice in choices {
        let key = match choice.value {
            DecisionValue::String(value) => value,
            DecisionValue::Bool(value) => value.to_string(),
        };
        if criteria.iter().any(|(existing, _)| *existing == key) {
            return Err(invalid(format!(
                "choice \"{key}\" appears more than once; TypeSafe choices are named by text"
            )));
        }
        criteria.push((key, choice.description.map(TypeSafeEntry::Text)));
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
    let mut answers = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let key = question_key(question, index);
        let answer = response
            .answers
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, answer)| answer)
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
    answer: &TypeSafeAnswer,
) -> Result<DecisionResult, ConvertError> {
    match (kind, answer) {
        (DecisionQuestionKind::Predicate, TypeSafeAnswer::Noul { noul }) => {
            Ok(DecisionResult::Predicate { probability: *noul })
        }
        (
            DecisionQuestionKind::Choice { choices },
            TypeSafeAnswer::Choice {
                choice,
                confidence,
                probabilities,
            },
        ) => {
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
            TypeSafeAnswer::Score {
                score,
                confidence,
                probabilities,
                ..
            },
        ) => Ok(DecisionResult::Score {
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
        }),
        (_, answer) => Err(invalid(format!(
            "TypeSafe returned a {} answer for {} question \"{key}\"",
            answer_type_name(answer),
            question_kind_name(kind)
        ))),
    }
}

fn probability_for(
    probabilities: &[(String, f64)],
    key: &str,
    question: &str,
) -> Result<f64, ConvertError> {
    probabilities
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, probability)| *probability)
        .ok_or_else(|| missing(format!("answers.{question}.probabilities.{key}")))
}

fn answer_type_name(answer: &TypeSafeAnswer) -> &'static str {
    match answer {
        TypeSafeAnswer::Noul { .. } => "noul",
        TypeSafeAnswer::Choice { .. } => "choice",
        TypeSafeAnswer::Score { .. } => "score",
        TypeSafeAnswer::BoundingBox { .. } => "box",
    }
}

fn question_kind_name(kind: &DecisionQuestionKind) -> &'static str {
    match kind {
        DecisionQuestionKind::Predicate => "predicate",
        DecisionQuestionKind::Choice { .. } => "choice",
        DecisionQuestionKind::Score { .. } => "score",
    }
}
