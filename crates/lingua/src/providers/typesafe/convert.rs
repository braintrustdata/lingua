/*!
Conversion between universal decision types and TypeSafe System One payloads.

Text input becomes a string `state`. A conversation becomes
`{"messages": [...]}`, with each message in Lingua's universal JSON form (minus
an unset assistant `id`), since decision models read the state as text. Questions are keyed by name, or by index
when unnamed, so answers can be returned in question order with their typed
choice values.
*/

use super::request::{SystemOneRequest, TypeSafeEntry, TypeSafeJson, TypeSafeQuestion};
use super::response::{SystemOneResponse, TypeSafeAnswer};
use crate::error::ConvertError;
use crate::serde_json;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContentPart, UserContent,
    UserContentPart,
};
use crate::universal::{
    DecisionAnswer, DecisionChoice, DecisionChoiceProbability, DecisionInput,
    DecisionLevelProbability, DecisionQuestionKind, DecisionResult, DecisionScoreLevel,
    DecisionValue, UniversalDecisionRequest, UniversalDecisionResponse, UniversalUsage,
};
use serde::Serialize;

const TARGET: &str = "TypeSafe System One";

/// What is needed to map a System One response back to the request it answers.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemOneDecisionContext {
    questions: Vec<QuestionContext>,
}

#[derive(Debug, Clone, PartialEq)]
struct QuestionContext {
    key: String,
    name: Option<String>,
    kind: QuestionKind,
}

#[derive(Debug, Clone, PartialEq)]
enum QuestionKind {
    Predicate,
    Choice(Vec<(String, DecisionValue)>),
    Score(Vec<String>),
}

#[derive(Serialize)]
struct MessagesState<'a> {
    messages: Vec<StateMessage<'a>>,
}

/// A universal message as written into System One state: the universal JSON form,
/// without an unset assistant `id`.
#[derive(Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
enum StateMessage<'a> {
    System {
        content: &'a UserContent,
    },
    Developer {
        content: &'a UserContent,
    },
    User {
        content: &'a UserContent,
    },
    Assistant {
        content: &'a AssistantContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<&'a str>,
    },
    Tool {
        content: &'a [ToolContentPart],
    },
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

/// Convert a universal decision request to a TypeSafe System One request.
pub fn universal_to_system_one(
    request: UniversalDecisionRequest,
) -> Result<(SystemOneRequest, SystemOneDecisionContext), ConvertError> {
    if request.prompt_cache_key.is_some() {
        return Err(unsupported("Lingua decision prompt_cache_key"));
    }
    let model = request.model.ok_or_else(|| missing("model"))?;
    let state = match request.input {
        DecisionInput::Text(text) => TypeSafeEntry::Text(text),
        DecisionInput::Messages(messages) => {
            let messages = messages
                .iter()
                .enumerate()
                .map(|(index, message)| state_message(message, index))
                .collect::<Result<_, _>>()?;
            TypeSafeEntry::Json(
                TypeSafeJson::from_serialize(&MessagesState { messages }).map_err(|error| {
                    ConvertError::JsonSerializationFailed {
                        field: "state".into(),
                        error: error.to_string(),
                    }
                })?,
            )
        }
    };

    let mut questions = Vec::with_capacity(request.questions.len());
    let mut contexts: Vec<QuestionContext> = Vec::with_capacity(request.questions.len());
    for (index, question) in request.questions.into_iter().enumerate() {
        let key = question.name.clone().unwrap_or_else(|| index.to_string());
        if contexts.iter().any(|context| context.key == key) {
            return Err(invalid(format!(
                "question name \"{key}\" is not unique; TypeSafe identifies answers by question name"
            )));
        }
        let instructions = Some(convert_instructions(question.instructions)?);
        let (typesafe_question, kind) = match question.kind {
            DecisionQuestionKind::Predicate => (
                TypeSafeQuestion::Noul {
                    instructions,
                    criteria: None,
                },
                QuestionKind::Predicate,
            ),
            DecisionQuestionKind::Choice { choices } => {
                let (criteria, values) = convert_choices(choices)?;
                (
                    TypeSafeQuestion::Choice {
                        instructions,
                        criteria,
                    },
                    QuestionKind::Choice(values),
                )
            }
            DecisionQuestionKind::Score { levels } => {
                let labels = levels.iter().map(|level| level.label.clone()).collect();
                (
                    TypeSafeQuestion::Score {
                        instructions,
                        criteria: levels.into_iter().map(score_criterion).collect(),
                    },
                    QuestionKind::Score(labels),
                )
            }
        };
        questions.push((key.clone(), typesafe_question));
        contexts.push(QuestionContext {
            key,
            name: question.name,
            kind,
        });
    }

    Ok((
        SystemOneRequest {
            model,
            state,
            questions,
            assets: None,
        },
        SystemOneDecisionContext {
            questions: contexts,
        },
    ))
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

fn state_message(message: &Message, index: usize) -> Result<StateMessage<'_>, ConvertError> {
    let location = format!("Lingua message {index}");
    match message {
        Message::System { content } => {
            validate_user_content(content, &location)?;
            Ok(StateMessage::System { content })
        }
        Message::Developer { content } => {
            validate_user_content(content, &location)?;
            Ok(StateMessage::Developer { content })
        }
        Message::User { content } => {
            validate_user_content(content, &location)?;
            Ok(StateMessage::User { content })
        }
        Message::Assistant { content, id } => {
            if let AssistantContent::Array(parts) = content {
                validate_assistant_parts(parts, &location)?;
            }
            Ok(StateMessage::Assistant {
                content,
                id: id.as_deref(),
            })
        }
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
            Ok(StateMessage::Tool { content })
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

type ChoiceCriteria = Vec<(String, Option<TypeSafeEntry>)>;

fn convert_choices(
    choices: Vec<DecisionChoice>,
) -> Result<(ChoiceCriteria, Vec<(String, DecisionValue)>), ConvertError> {
    let mut criteria = Vec::with_capacity(choices.len());
    let mut values: Vec<(String, DecisionValue)> = Vec::with_capacity(choices.len());
    for choice in choices {
        let key = match &choice.value {
            DecisionValue::String(value) => value.clone(),
            DecisionValue::Bool(value) => value.to_string(),
        };
        if values.iter().any(|(existing, _)| *existing == key) {
            return Err(invalid(format!(
                "choice \"{key}\" appears more than once; TypeSafe choices are named by text"
            )));
        }
        criteria.push((key.clone(), choice.description.map(TypeSafeEntry::Text)));
        values.push((key, choice.value));
    }
    Ok((criteria, values))
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

/// Convert a TypeSafe System One response to universal answers for the request
/// described by `context`.
pub fn system_one_response_to_universal(
    response: SystemOneResponse,
    context: &SystemOneDecisionContext,
) -> Result<UniversalDecisionResponse, ConvertError> {
    let mut answers = Vec::with_capacity(context.questions.len());
    for question in &context.questions {
        let answer = response
            .answers
            .iter()
            .find(|(key, _)| *key == question.key)
            .map(|(_, answer)| answer)
            .ok_or_else(|| missing(format!("answers.{}", question.key)))?;
        answers.push(DecisionAnswer {
            name: question.name.clone(),
            result: convert_answer(question, answer)?,
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
    question: &QuestionContext,
    answer: &TypeSafeAnswer,
) -> Result<DecisionResult, ConvertError> {
    match (&question.kind, answer) {
        (QuestionKind::Predicate, TypeSafeAnswer::Noul { noul }) => {
            Ok(DecisionResult::Predicate { probability: *noul })
        }
        (
            QuestionKind::Choice(values),
            TypeSafeAnswer::Choice {
                choice,
                confidence,
                probabilities,
            },
        ) => {
            let selected = values
                .iter()
                .find(|(key, _)| key == choice)
                .map(|(_, value)| value.clone())
                .ok_or_else(|| {
                    invalid(format!(
                        "TypeSafe returned unknown choice \"{choice}\" for question \"{}\"",
                        question.key
                    ))
                })?;
            Ok(DecisionResult::Choice {
                choice: selected,
                confidence: Some(*confidence),
                probabilities: values
                    .iter()
                    .map(|(key, value)| {
                        Ok(DecisionChoiceProbability {
                            value: value.clone(),
                            probability: probability_for(probabilities, key, &question.key)?,
                        })
                    })
                    .collect::<Result<_, ConvertError>>()?,
            })
        }
        (
            QuestionKind::Score(labels),
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
                .zip(labels)
                .map(|(level, label)| {
                    Ok(DecisionLevelProbability {
                        level,
                        label: Some(label.clone()),
                        probability: probability_for(
                            probabilities,
                            &level.to_string(),
                            &question.key,
                        )?,
                    })
                })
                .collect::<Result<_, ConvertError>>()?,
        }),
        (_, answer) => Err(invalid(format!(
            "TypeSafe returned a {} answer for {} question \"{}\"",
            answer_type_name(answer),
            question_kind_name(&question.kind),
            question.key
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

fn question_kind_name(kind: &QuestionKind) -> &'static str {
    match kind {
        QuestionKind::Predicate => "predicate",
        QuestionKind::Choice(_) => "choice",
        QuestionKind::Score(_) => "score",
    }
}
