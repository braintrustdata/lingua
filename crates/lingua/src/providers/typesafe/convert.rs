/*!
Conversion between OpenAI Decisions API payloads and TypeSafe System One payloads.

Decisions input becomes System One `state`: a string stays a string, and an item
list becomes `{"messages": [...]}` with each item's JSON copied as sent, since
decision models read the state as text. Questions keep their names as System One
keys, so answers can be returned in question order with their original typed
choice values.
*/

use super::request::{SystemOneRequest, TypeSafeEntry, TypeSafeJson, TypeSafeQuestion};
use super::response::{SystemOneResponse, TypeSafeAnswer};
use crate::error::ConvertError;
use crate::providers::openai::decisions_generated::{
    AnswerResource, AnswerResourceType, ChoiceOptionParam, ChoiceValueParam, ChoiceValueResource,
    Data, DecisionInput, DecisionInputContent, DecisionInputContentPartElement,
    DecisionInputContentPartType, DecisionInputMessage, DecisionInputToolOutputContent,
    DecisionInputToolOutputContentPartType, DecisionInputType, DecisionQuestionInputPartType,
    DecisionRequest, DecisionResponse, EProbabilityResource, InputTokensDetailsResource,
    Instructions, OutputTokensDetailsResource, QuestionParam, QuestionParamType,
    ResponseUsageResource, ScoreLevelParam, ValueUnion,
};
use crate::serde_json;
use crate::serde_json::value::RawValue;

const TARGET: &str = "TypeSafe System One";

/// What is needed to map a System One response back to the Decisions request it answers.
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
    Choice(Vec<(String, ChoiceValueParam)>),
    Score(Vec<String>),
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

/// Convert an OpenAI Decisions request to a TypeSafe System One request.
///
/// `raw_input` is the request's `input` JSON as sent, which item lists are
/// copied from.
pub fn decision_request_to_system_one(
    request: &DecisionRequest,
    raw_input: &RawValue,
) -> Result<(SystemOneRequest, SystemOneDecisionContext), ConvertError> {
    if request.prompt_cache_key.is_some() || request.prompt_cache_options.is_some() {
        return Err(unsupported("Decisions prompt caching options"));
    }
    if request.safety_identifier.is_some() {
        return Err(unsupported("Decisions safety_identifier"));
    }

    let state = match &request.input {
        DecisionInput::PurpleString(text) => TypeSafeEntry::Text(text.clone()),
        DecisionInput::DecisionInputMessageArray(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_input_item(item, index)?;
            }
            let raw_items: Vec<&RawValue> =
                serde_json::from_str(raw_input.get()).map_err(|error| {
                    ConvertError::JsonSerializationFailed {
                        field: "input".into(),
                        error: error.to_string(),
                    }
                })?;
            if raw_items.len() != items.len() {
                return Err(invalid("input does not match the parsed request"));
            }
            let mut state = String::with_capacity(raw_input.get().len() + 16);
            state.push_str(r#"{"messages":["#);
            for (index, item) in raw_items.iter().enumerate() {
                if index > 0 {
                    state.push(',');
                }
                state.push_str(item.get());
            }
            state.push_str("]}");
            TypeSafeEntry::Json(TypeSafeJson::new(state).map_err(|error| {
                ConvertError::JsonSerializationFailed {
                    field: "state".into(),
                    error: error.to_string(),
                }
            })?)
        }
    };

    let mut questions = Vec::with_capacity(request.questions.len());
    let mut contexts = Vec::with_capacity(request.questions.len());
    for (index, question) in request.questions.iter().enumerate() {
        let key = question.name.clone().unwrap_or_else(|| index.to_string());
        if contexts
            .iter()
            .any(|context: &QuestionContext| context.key == key)
        {
            return Err(invalid(format!(
                "question name \"{key}\" is not unique; TypeSafe identifies answers by question name"
            )));
        }
        let (typesafe_question, kind) = convert_question(question, index)?;
        questions.push((key.clone(), typesafe_question));
        contexts.push(QuestionContext {
            key,
            name: question.name.clone(),
            kind,
        });
    }

    Ok((
        SystemOneRequest {
            model: request.model.clone(),
            state,
            questions,
            assets: None,
        },
        SystemOneDecisionContext {
            questions: contexts,
        },
    ))
}

fn validate_input_item(item: &DecisionInputMessage, index: usize) -> Result<(), ConvertError> {
    let field = |name: &str| format!("input[{index}].{name}");
    match item
        .decision_input_message_type
        .as_ref()
        .unwrap_or(&DecisionInputType::Message)
    {
        DecisionInputType::Message => {
            if item.role.is_none() {
                return Err(missing(field("role")));
            }
            match &item.content {
                None => return Err(missing(field("content"))),
                Some(DecisionInputContent::PurpleString(_)) => {}
                Some(DecisionInputContent::DecisionInputContentPartElementArray(parts)) => {
                    for part in parts {
                        validate_content_part(part, index)?;
                    }
                }
            }
        }
        DecisionInputType::FunctionCall => {
            for (name, present) in [
                ("call_id", item.call_id.is_some()),
                ("name", item.name.is_some()),
                ("arguments", item.arguments.is_some()),
            ] {
                if !present {
                    return Err(missing(field(name)));
                }
            }
        }
        DecisionInputType::CustomToolCall => {
            for (name, present) in [
                ("call_id", item.call_id.is_some()),
                ("name", item.name.is_some()),
                ("input", item.input.is_some()),
            ] {
                if !present {
                    return Err(missing(field(name)));
                }
            }
        }
        DecisionInputType::FunctionCallOutput | DecisionInputType::CustomToolCallOutput => {
            if item.call_id.is_none() {
                return Err(missing(field("call_id")));
            }
            match &item.output {
                None => return Err(missing(field("output"))),
                Some(DecisionInputToolOutputContent::PurpleString(_)) => {}
                Some(
                    DecisionInputToolOutputContent::DecisionInputToolOutputContentPartElementArray(
                        parts,
                    ),
                ) => {
                    for part in parts {
                        if part.prompt_cache_breakpoint.is_some() {
                            return Err(unsupported("Decisions prompt_cache_breakpoint"));
                        }
                        match part.decision_input_type {
                            DecisionInputToolOutputContentPartType::InputImage => {
                                return Err(unsupported("Decisions input_image tool output"));
                            }
                            DecisionInputToolOutputContentPartType::InputText => {
                                if part.text.is_none() {
                                    return Err(missing(field("output[].text")));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_content_part(
    part: &DecisionInputContentPartElement,
    index: usize,
) -> Result<(), ConvertError> {
    if part.prompt_cache_breakpoint.is_some() {
        return Err(unsupported("Decisions prompt_cache_breakpoint"));
    }
    let required = match part.decision_input_type {
        DecisionInputContentPartType::InputImage => {
            return Err(unsupported("Decisions input_image content"));
        }
        DecisionInputContentPartType::InputText | DecisionInputContentPartType::OutputText => {
            ("text", part.text.is_some())
        }
        DecisionInputContentPartType::InputJson => ("data", part.data.is_some()),
        DecisionInputContentPartType::Refusal => ("refusal", part.refusal.is_some()),
    };
    if !required.1 {
        return Err(missing(format!("input[{index}].content[].{}", required.0)));
    }
    Ok(())
}

fn convert_question(
    question: &QuestionParam,
    index: usize,
) -> Result<(TypeSafeQuestion, QuestionKind), ConvertError> {
    let instructions = Some(convert_instructions(&question.instructions)?);
    match question.question_param_type {
        QuestionParamType::Predicate => Ok((
            TypeSafeQuestion::Noul {
                instructions,
                criteria: None,
            },
            QuestionKind::Predicate,
        )),
        QuestionParamType::Choice => {
            let choices = question
                .choices
                .as_ref()
                .ok_or_else(|| missing(format!("questions[{index}].choices")))?;
            let (criteria, values) = convert_choices(choices)?;
            Ok((
                TypeSafeQuestion::Choice {
                    instructions,
                    criteria,
                },
                QuestionKind::Choice(values),
            ))
        }
        QuestionParamType::Score => {
            let levels = question
                .levels
                .as_ref()
                .ok_or_else(|| missing(format!("questions[{index}].levels")))?;
            Ok((
                TypeSafeQuestion::Score {
                    instructions,
                    criteria: levels.iter().map(score_criterion).collect(),
                },
                QuestionKind::Score(levels.iter().map(|level| level.label.clone()).collect()),
            ))
        }
    }
}

fn convert_instructions(instructions: &Instructions) -> Result<TypeSafeEntry, ConvertError> {
    match instructions {
        Instructions::PurpleString(text) => Ok(TypeSafeEntry::Text(text.clone())),
        Instructions::DecisionQuestionInputPartArray(parts) => parts
            .iter()
            .map(|part| match part.decision_question_input_part_type {
                DecisionQuestionInputPartType::InputText => part
                    .text
                    .clone()
                    .map(serde_json::Value::String)
                    .ok_or_else(|| missing("questions[].instructions[].text")),
                DecisionQuestionInputPartType::InputJson => match &part.data {
                    Some(Data::AnythingMap(map)) => Ok(serde_json::Value::Object(map.clone())),
                    Some(Data::AnythingMapArray(items)) => Ok(serde_json::Value::Array(
                        items
                            .iter()
                            .cloned()
                            .map(serde_json::Value::Object)
                            .collect(),
                    )),
                    None => Err(missing("questions[].instructions[].data")),
                },
                DecisionQuestionInputPartType::InputImage => {
                    Err(unsupported("Decisions input_image instructions"))
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(TypeSafeEntry::Array),
    }
}

type ChoiceCriteria = Vec<(String, Option<TypeSafeEntry>)>;

fn convert_choices(
    choices: &[ChoiceOptionParam],
) -> Result<(ChoiceCriteria, Vec<(String, ChoiceValueParam)>), ConvertError> {
    let mut criteria = Vec::with_capacity(choices.len());
    let mut values: Vec<(String, ChoiceValueParam)> = Vec::with_capacity(choices.len());
    for option in choices {
        let key = match &option.value {
            ChoiceValueParam::PurpleString(value) => value.clone(),
            ChoiceValueParam::Bool(value) => value.to_string(),
        };
        if values.iter().any(|(existing, _)| *existing == key) {
            return Err(invalid(format!(
                "choice \"{key}\" appears more than once; TypeSafe choices are named by text"
            )));
        }
        criteria.push((
            key.clone(),
            option.description.clone().map(TypeSafeEntry::Text),
        ));
        values.push((key, option.value.clone()));
    }
    Ok((criteria, values))
}

fn score_criterion(level: &ScoreLevelParam) -> TypeSafeEntry {
    match &level.description {
        None => TypeSafeEntry::Text(level.label.clone()),
        Some(description) => {
            let mut criterion = serde_json::Map::new();
            criterion.insert(
                "label".into(),
                serde_json::Value::String(level.label.clone()),
            );
            criterion.insert(
                "description".into(),
                serde_json::Value::String(description.clone()),
            );
            TypeSafeEntry::Object(criterion)
        }
    }
}

/// Convert a TypeSafe System One response to the OpenAI Decisions response for the
/// request described by `context`.
pub fn system_one_response_to_decision(
    response: SystemOneResponse,
    context: &SystemOneDecisionContext,
) -> Result<DecisionResponse, ConvertError> {
    let mut answers = Vec::with_capacity(context.questions.len());
    for question in &context.questions {
        let answer = response
            .answers
            .iter()
            .find(|(key, _)| *key == question.key)
            .map(|(_, answer)| answer)
            .ok_or_else(|| missing(format!("answers.{}", question.key)))?;
        answers.push(convert_answer(question, answer)?);
    }
    let usage = response.usage;
    Ok(DecisionResponse {
        answers,
        model: response.model,
        usage: ResponseUsageResource {
            input_tokens: usage.input_tokens,
            input_tokens_details: InputTokensDetailsResource {
                cache_write_tokens: 0,
                cached_tokens: 0,
            },
            output_tokens: usage.output_tokens,
            output_tokens_details: OutputTokensDetailsResource {
                reasoning_tokens: 0,
            },
            total_tokens: usage.input_tokens + usage.output_tokens,
        },
    })
}

fn convert_answer(
    question: &QuestionContext,
    answer: &TypeSafeAnswer,
) -> Result<AnswerResource, ConvertError> {
    let base = AnswerResource {
        name: question.name.clone(),
        probability: None,
        answer_resource_type: AnswerResourceType::Refusal,
        choice: None,
        confidence: None,
        probabilities: None,
        score: None,
    };
    match (&question.kind, answer) {
        (QuestionKind::Predicate, TypeSafeAnswer::Noul { noul }) => Ok(AnswerResource {
            answer_resource_type: AnswerResourceType::Predicate,
            probability: Some(*noul),
            ..base
        }),
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
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    invalid(format!(
                        "TypeSafe returned unknown choice \"{choice}\" for question \"{}\"",
                        question.key
                    ))
                })?;
            let probabilities = values
                .iter()
                .map(|(key, value)| {
                    Ok(EProbabilityResource {
                        probability: probability_for(probabilities, key, &question.key)?,
                        value: match value {
                            ChoiceValueParam::PurpleString(value) => {
                                ValueUnion::PurpleString(value.clone())
                            }
                            ChoiceValueParam::Bool(value) => ValueUnion::Bool(*value),
                        },
                        label: None,
                    })
                })
                .collect::<Result<Vec<_>, ConvertError>>()?;
            Ok(AnswerResource {
                answer_resource_type: AnswerResourceType::Choice,
                choice: Some(match selected {
                    ChoiceValueParam::PurpleString(value) => {
                        ChoiceValueResource::PurpleString(value.clone())
                    }
                    ChoiceValueParam::Bool(value) => ChoiceValueResource::Bool(*value),
                }),
                confidence: Some(*confidence),
                probabilities: Some(probabilities),
                ..base
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
        ) => {
            let probabilities = labels
                .iter()
                .enumerate()
                .map(|(index, label)| {
                    Ok(EProbabilityResource {
                        probability: probability_for(
                            probabilities,
                            &index.to_string(),
                            &question.key,
                        )?,
                        value: ValueUnion::Integer(index as i64),
                        label: Some(label.clone()),
                    })
                })
                .collect::<Result<Vec<_>, ConvertError>>()?;
            Ok(AnswerResource {
                answer_resource_type: AnswerResourceType::Score,
                score: Some(*score),
                confidence: Some(*confidence),
                probabilities: Some(probabilities),
                ..base
            })
        }
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
