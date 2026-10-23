use crate::serde_json::Value;
use std::convert::Infallible;

pub(crate) trait MessageValue: Sized {
    type Error;

    fn is_string(&self) -> bool;
    fn array_len(&self) -> Option<usize>;
    fn array_item(&self, index: usize) -> Result<Self, Self::Error>;
    fn has_any_field(&self, fields: &[&str]) -> Result<bool, Self::Error>;
}

impl MessageValue for &Value {
    type Error = Infallible;

    fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    fn array_len(&self) -> Option<usize> {
        match self {
            Value::Array(items) => Some(items.len()),
            _ => None,
        }
    }

    fn array_item(&self, index: usize) -> Result<Self, Self::Error> {
        let Value::Array(items) = self else {
            unreachable!()
        };
        Ok(&items[index])
    }

    fn has_any_field(&self, fields: &[&str]) -> Result<bool, Self::Error> {
        Ok(
            matches!(self, Value::Object(object) if fields.iter().any(|field| object.contains_key(*field))),
        )
    }
}

#[cfg(target_arch = "wasm32")]
impl MessageValue for wasm_bindgen::JsValue {
    type Error = wasm_bindgen::JsValue;

    fn is_string(&self) -> bool {
        wasm_bindgen::JsValue::is_string(self)
    }

    fn array_len(&self) -> Option<usize> {
        use wasm_bindgen::JsCast;

        self.dyn_ref::<js_sys::Array>()
            .map(|items| items.length() as usize)
    }

    fn array_item(&self, index: usize) -> Result<Self, Self::Error> {
        js_sys::Reflect::get(self, &wasm_bindgen::JsValue::from_f64(index as f64))
    }

    fn has_any_field(&self, fields: &[&str]) -> Result<bool, Self::Error> {
        use wasm_bindgen::JsCast;

        if !self.is_object() {
            return Ok(false);
        }
        if let Some(map) = self.dyn_ref::<js_sys::Map>() {
            return Ok(fields
                .iter()
                .any(|field| map.has(&wasm_bindgen::JsValue::from_str(field))));
        }
        for key in js_sys::Reflect::own_keys(self)?.iter() {
            if key
                .as_string()
                .is_some_and(|key| fields.contains(&key.as_str()))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub(crate) fn has_message_structure<V: MessageValue>(value: &V) -> Result<bool, V::Error> {
    fn check<V: MessageValue>(value: &V, depth: usize) -> Result<bool, V::Error> {
        if depth == 64 || value.is_string() {
            return Ok(true);
        }

        if let Some(len) = value.array_len() {
            for index in 0..len {
                if check(&value.array_item(index)?, depth + 1)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }

        value.has_any_field(&[
            "role",
            "messages",
            "message",
            "content",
            "type",
            "prompt",
            "system",
            "input",
            "output",
            "choices",
            "result",
            "response",
            "contents",
            "candidates",
            "parts",
            "user_prompt",
            "system_prompt",
            "message_history",
            "generations",
            "lc",
            "text",
            "object",
            "steps",
            "responseMessages",
            "toolCalls",
            "toolResults",
            "reasoning",
        ])
    }

    check(value, 0)
}

#[cfg(test)]
mod tests {
    use super::has_message_structure;
    use crate::serde_json::{json, Value};

    #[test]
    fn rejects_non_message_values() {
        for value in [
            Value::Null,
            json!(false),
            json!(42),
            json!([]),
            json!({ "unrelated": { "role": "user", "content": "Hello" } }),
            json!([1, false, { "unrelated": "Hello" }]),
        ] {
            assert!(!has_message_structure(&&value).unwrap(), "{value}");
        }
    }

    #[test]
    fn admits_provider_and_framework_shapes() {
        for value in [
            json!("Hello"),
            json!({ "role": "user", "content": "Hello" }),
            json!({ "contents": [{ "parts": [{ "text": "Hello" }] }] }),
            json!({ "candidates": [{ "content": {} }] }),
            json!({ "user_prompt": "Hello" }),
            json!([{ "kind": "request", "parts": [] }]),
            json!({ "generations": [] }),
            json!([[{ "type": "human", "content": "Hello" }]]),
            json!({ "steps": [] }),
            json!({ "type": "function_call", "name": "lookup" }),
        ] {
            assert!(has_message_structure(&&value).unwrap(), "{value}");
        }
    }

    #[test]
    fn admits_arrays_beyond_the_inspection_depth() {
        let value = (0..65).fold(Value::Null, |value, _| Value::Array(vec![value]));
        assert!(has_message_structure(&&value).unwrap());
    }
}
