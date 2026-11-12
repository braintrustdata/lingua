/*!
TypeSafe's free-form content type.

TypeSafe accepts text, a JSON object, or a JSON array wherever it reads content:
request state, question instructions, and criteria. The generated System One
types use this hand-written type so Lingua can also embed JSON serialized in
its own key order, which decision models read as text.
*/

use crate::serde_json;
use serde::{Deserialize, Serialize};

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
