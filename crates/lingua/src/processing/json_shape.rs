use crate::serde_json::{self, Value};
use serde::de::{self, DeserializeOwned, Deserializer, Visitor};
use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonValueKind {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl JsonValueKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

impl fmt::Display for JsonValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for JsonValueKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JsonValueKindVisitor;

        impl<'de> Visitor<'de> for JsonValueKindVisitor {
            type Value = JsonValueKind;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any JSON value")
            }

            fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::Bool)
            }

            fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::Number)
            }

            fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::Number)
            }

            fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::Number)
            }

            fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::String)
            }

            fn visit_string<E>(self, _v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::String)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonValueKind::Null)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut seq = seq;
                while seq.next_element::<de::IgnoredAny>()?.is_some() {}
                Ok(JsonValueKind::Array)
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut map = map;
                while map
                    .next_entry::<de::IgnoredAny, de::IgnoredAny>()?
                    .is_some()
                {}
                Ok(JsonValueKind::Object)
            }
        }

        deserializer.deserialize_any(JsonValueKindVisitor)
    }
}

pub fn json_value_kind(payload: &Value) -> JsonValueKind {
    serde_json::from_value::<JsonValueKind>(payload.clone()).unwrap_or(JsonValueKind::Object)
}

pub fn probe_shape<T: DeserializeOwned>(payload: &Value) -> Option<T> {
    serde_json::from_value::<T>(payload.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Probe {
        #[serde(default)]
        input: Option<JsonValueKind>,
    }

    #[test]
    fn json_value_kind_detects_top_level_shapes() {
        assert_eq!(json_value_kind(&json!(null)), JsonValueKind::Null);
        assert_eq!(json_value_kind(&json!(true)), JsonValueKind::Bool);
        assert_eq!(json_value_kind(&json!(1)), JsonValueKind::Number);
        assert_eq!(json_value_kind(&json!("x")), JsonValueKind::String);
        assert_eq!(json_value_kind(&json!([])), JsonValueKind::Array);
        assert_eq!(json_value_kind(&json!({})), JsonValueKind::Object);
    }

    #[test]
    fn probe_shape_extracts_structural_field_types() {
        let probe = probe_shape::<Probe>(&json!({"input": []})).unwrap();
        assert_eq!(
            probe,
            Probe {
                input: Some(JsonValueKind::Array),
            }
        );
    }
}
