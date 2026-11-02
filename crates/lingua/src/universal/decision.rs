/*!
Universal decision types.

A decision asks a model questions about an input and returns one typed answer per
question instead of generated text. Providers convert their decision payloads to
and from these types.
*/

use crate::capabilities::ProviderFormat;
use crate::serde_json::{Map, Value};
use crate::universal::{Message, UniversalUsage, UserContent};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// A request to answer questions about an input.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UniversalDecisionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub model: Option<String>,
    /// The content every question refers to.
    pub input: DecisionInput,
    /// Questions to answer; answers come back in the same order.
    pub questions: Vec<DecisionQuestion>,
    #[serde(default, skip_serializing_if = "DecisionParams::is_empty")]
    #[ts(as = "Option<_>", optional)]
    pub params: DecisionParams,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prompt_cache_key: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_extras",
        skip_serializing_if = "HashMap::is_empty"
    )]
    #[ts(
        type = r#"Partial<Record<import("./ProviderFormat").ProviderFormat, Record<string, unknown>>>"#
    )]
    pub extras: HashMap<ProviderFormat, Map<String, Value>>,
}

fn deserialize_extras<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<HashMap<ProviderFormat, Map<String, Value>>, D::Error> {
    let extras = HashMap::<ProviderFormat, Map<String, Value>>::deserialize(deserializer)?;
    if extras.contains_key(&ProviderFormat::Unknown) {
        return Err(serde::de::Error::custom(
            "unknown decision provider format in params.extras",
        ));
    }
    Ok(extras)
}

fn deserialize_number<'de, D: Deserializer<'de>, T: serde::de::DeserializeOwned>(
    deserializer: D,
) -> Result<T, D::Error> {
    crate::serde_json::from_value(Value::deserialize(deserializer)?)
        .map_err(serde::de::Error::custom)
}

impl DecisionParams {
    fn is_empty(&self) -> bool {
        self.prompt_cache_key.is_none() && self.extras.is_empty()
    }
}

/// The content a decision is about: text, or a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum DecisionInput {
    Text(String),
    Messages(Vec<Message>),
}

/// A question about the decision input.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionQuestion {
    /// Identifies the answer to this question.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    pub instructions: UserContent,
    #[serde(flatten)]
    pub kind: DecisionQuestionKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecisionQuestionKind {
    /// How likely a statement about the input is to be true.
    Predicate,
    /// Which of the choices best fits the input.
    Choice { choices: Vec<DecisionChoice> },
    /// Where the input falls on ordered levels, scored from zero.
    Score { levels: Vec<DecisionScoreLevel> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionChoice {
    pub value: DecisionValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
}

/// A choice value. A string and a boolean with the same text are distinct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum DecisionValue {
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionScoreLevel {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
}

/// Answers to a decision request, in question order.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UniversalDecisionResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub model: Option<String>,
    pub answers: Vec<DecisionAnswer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub usage: Option<UniversalUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionAnswer {
    /// The name of the question this answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub result: DecisionResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecisionResult {
    /// The probability that the statement is true.
    Predicate {
        #[serde(deserialize_with = "deserialize_number")]
        probability: f64,
    },
    Choice {
        choice: DecisionValue,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(deserialize_with = "deserialize_number")]
        confidence: Option<f64>,
        /// Probability of each choice, in choice order.
        probabilities: Vec<DecisionChoiceProbability>,
    },
    Score {
        /// Probability-weighted average of the level indices.
        #[serde(deserialize_with = "deserialize_number")]
        score: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        #[serde(deserialize_with = "deserialize_number")]
        confidence: Option<f64>,
        /// Probability of each level, in level order.
        probabilities: Vec<DecisionLevelProbability>,
    },
    /// The model declined to answer this question.
    Refusal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionChoiceProbability {
    pub value: DecisionValue,
    #[serde(deserialize_with = "deserialize_number")]
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionLevelProbability {
    /// Index of the level, from zero.
    #[serde(deserialize_with = "deserialize_number")]
    pub level: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
    #[serde(deserialize_with = "deserialize_number")]
    pub probability: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_response_parses_directly_from_json() {
        let json = r#"{"answers":[{"type":"predicate","probability":0.75},{"type":"choice","choice":true,"confidence":0.9,"probabilities":[{"value":true,"probability":0.9},{"value":false,"probability":0.1}]},{"type":"score","score":0.5,"confidence":0.5,"probabilities":[{"level":0,"probability":0.5},{"level":1,"probability":0.5}]}]}"#;
        let response: UniversalDecisionResponse = crate::serde_json::from_str(json).unwrap();
        assert_eq!(
            crate::serde_json::to_value(response).unwrap(),
            crate::serde_json::from_str::<Value>(json).unwrap()
        );
    }

    #[test]
    fn rejects_unknown_extra_formats_but_allows_saved_chat_params() {
        assert!(crate::serde_json::from_str::<DecisionParams>(
            r#"{"extras":{"TypeSafe":{"long_context":true}}}"#
        )
        .is_err());
        let params: DecisionParams = crate::serde_json::from_str(
            r#"{"temperature":0,"max_tokens":100,"extras":{"typesafe":{"long_context":true}}}"#,
        )
        .unwrap();
        assert!(params.extras.contains_key(&ProviderFormat::TypeSafe));
    }
}
