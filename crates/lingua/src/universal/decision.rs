/*!
Universal decision types.

A decision asks a model questions about an input and returns one typed answer per
question instead of generated text. Providers convert their decision payloads to
and from these types.
*/

use crate::universal::{Message, UniversalUsage, UserContent};
use serde::{Deserialize, Serialize};
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
    /// Stable cache-bucketing key for prompt caching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prompt_cache_key: Option<String>,
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
    Predicate { probability: f64 },
    Choice {
        choice: DecisionValue,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        confidence: Option<f64>,
        /// Probability of each choice, in choice order.
        probabilities: Vec<DecisionChoiceProbability>,
    },
    Score {
        /// Probability-weighted average of the level indices.
        score: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
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
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionLevelProbability {
    /// Index of the level, from zero.
    pub level: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
    pub probability: f64,
}
