/*!
TypeSafe System One API request types.

These types mirror `SystemOneRequest` in `specs/typesafe/openapi.json`. Named maps
are kept as ordered `(name, value)` pairs so requests serialize in caller order.
*/

use crate::serde_json;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, Map};

/// Text, a JSON object, or a JSON array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeSafeEntry {
    Text(String),
    Object(serde_json::Map<String, serde_json::Value>),
    Array(Vec<serde_json::Value>),
    /// A JSON object or array serialized exactly as written, keeping its key order.
    #[serde(skip_deserializing)]
    Json(TypeSafeJson),
}

/// JSON text that serializes verbatim.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct TypeSafeJson(Box<serde_json::value::RawValue>);

impl TypeSafeJson {
    /// Serialize `value`, keeping the field order of its `Serialize` implementation.
    pub fn from_serialize<T: Serialize + ?Sized>(value: &T) -> Result<Self, serde_json::Error> {
        serde_json::value::to_raw_value(value).map(Self)
    }

    pub fn get(&self) -> &str {
        self.0.get()
    }
}

impl PartialEq for TypeSafeJson {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

/// Request body for `POST /v1/systemone`.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemOneRequest {
    /// Name or alias of the model to use.
    pub model: String,
    /// The content all questions in this request refer to.
    pub state: TypeSafeEntry,
    /// Questions keyed by the names used to identify their answers.
    #[serde_as(as = "Map<_, _>")]
    pub questions: Vec<(String, TypeSafeQuestion)>,
    /// Media assets keyed by the placeholder that each replaces.
    #[serde_as(as = "Option<Map<_, _>>")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<(String, TypeSafeAsset)>>,
}

/// A question about the request's state, identified by its `type` field.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TypeSafeQuestion {
    /// A yes/no question, answered with the probability of yes.
    Noul {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<TypeSafeEntry>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        criteria: Option<NoulCriteria>,
    },
    /// Selects one of the named choices in `criteria`.
    Choice {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<TypeSafeEntry>,
        /// Choice names mapped to descriptions; `None` leaves a choice undescribed.
        #[serde_as(as = "Map<_, _>")]
        criteria: Vec<(String, Option<TypeSafeEntry>)>,
    },
    /// Rates the state against ordered levels, scored from zero.
    Score {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<TypeSafeEntry>,
        criteria: Vec<TypeSafeEntry>,
    },
    /// Locates what the instructions describe in the request's image.
    #[serde(rename = "box")]
    BoundingBox {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<TypeSafeEntry>,
    },
}

/// Descriptions of what counts as a yes or a no answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoulCriteria {
    #[serde(rename = "true", default, skip_serializing_if = "Option::is_none")]
    pub yes: Option<TypeSafeEntry>,
    #[serde(rename = "false", default, skip_serializing_if = "Option::is_none")]
    pub no: Option<TypeSafeEntry>,
}

/// An image that replaces its placeholder wherever it appears in a string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeSafeAsset {
    #[serde(rename = "type")]
    pub asset_type: TypeSafeAssetType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<TypeSafeImageFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeSafeAssetType {
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeSafeImageFormat {
    Png,
    Jpg,
    Jpeg,
}
