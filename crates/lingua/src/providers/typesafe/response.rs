/*!
TypeSafe System One API response types.

These types mirror `SystemOneResponse` in `specs/typesafe/openapi.json`.
*/

use super::request::TypeSafeEntry;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, Map};

/// Response body for `POST /v1/systemone`.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemOneResponse {
    /// The model that answered, which may differ from the requested alias.
    pub model: String,
    /// Answers keyed by the question names supplied in the request.
    #[serde_as(as = "Map<_, _>")]
    pub answers: Vec<(String, TypeSafeAnswer)>,
    pub usage: TypeSafeUsage,
}

/// An answer whose type matches its question.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TypeSafeAnswer {
    /// Probability of a yes answer, from 0 to 1.
    Noul { noul: f64 },
    Choice {
        /// The choice with the highest probability.
        choice: String,
        confidence: f64,
        /// Probability of each choice, keyed by choice name.
        #[serde_as(as = "Map<_, _>")]
        probabilities: Vec<(String, f64)>,
    },
    Score {
        /// Probability-weighted average of the level indices.
        score: f64,
        confidence: f64,
        /// The requested levels keyed by score.
        #[serde_as(as = "Map<_, _>")]
        legend: Vec<(String, TypeSafeEntry)>,
        /// Probability of each level, keyed by score.
        #[serde_as(as = "Map<_, _>")]
        probabilities: Vec<(String, f64)>,
    },
    #[serde(rename = "box")]
    BoundingBox {
        #[serde(rename = "box")]
        coordinates: TypeSafeBoxCoordinates,
        confidence: Option<f64>,
    },
}

/// Box edges as fractions of image height and width.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeSafeBoxCoordinates {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeSafeUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}
