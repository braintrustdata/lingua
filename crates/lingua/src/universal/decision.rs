/*!
Universal decision types.

A decision asks a model questions about an input and returns one typed answer per
question instead of generated text. Providers convert their decision payloads to
and from these types.
*/

use crate::providers::openai::decisions_generated::PromptCacheOptions;
use crate::serde_json::Value;
use crate::universal::{Message, UniversalUsage, UserContent};
use serde::{Deserialize, Deserializer, Serialize};
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
    #[serde(default, skip_serializing_if = "DecisionExtras::is_empty")]
    #[ts(as = "Option<_>", optional)]
    pub extras: DecisionExtras,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct DecisionExtras {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub typesafe: Option<TypeSafeDecisionOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub decisions: Option<OpenAIDecisionOptions>,
}

impl DecisionExtras {
    fn is_empty(&self) -> bool {
        self.typesafe.is_none() && self.decisions.is_none()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct TypeSafeDecisionOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub images: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub long_context: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct OpenAIDecisionOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prompt_cache_options: Option<PromptCacheOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub safety_identifier: Option<String>,
}

impl OpenAIDecisionOptions {
    pub(crate) fn is_empty(&self) -> bool {
        self.prompt_cache_options.is_none() && self.safety_identifier.is_none()
    }
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

/// The content a decision is about, or a conversation to evaluate.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum DecisionInput {
    Content(UserContent),
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
        assert_eq!(params.extras.typesafe.unwrap().long_context, Some(true));
    }

    #[test]
    fn rejects_invalid_typesafe_options_at_deserialization() {
        for json in [
            r#"{"extras":{"typesafe":{"danger_zone_imagez":true}}}"#,
            r#"{"extras":{"typesafe":{"long_context":"true"}}}"#,
        ] {
            assert!(crate::serde_json::from_str::<DecisionParams>(json).is_err());
        }
    }
}
