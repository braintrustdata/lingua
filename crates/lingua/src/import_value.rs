use crate::serde_json::Value;
use std::convert::Infallible;

pub(crate) trait MessageValue: Sized {
    type Error;

    fn is_string(&self) -> bool;
    fn array_len(&self) -> Option<usize>;
    fn array_item(&self, index: usize) -> Result<Self, Self::Error>;
    fn is_object(&self) -> bool;
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

    fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
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

    fn is_object(&self) -> bool {
        wasm_bindgen::JsValue::is_object(self)
    }
}

pub(crate) fn can_contain_messages<V: MessageValue>(value: &V) -> Result<bool, V::Error> {
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

        Ok(value.is_object())
    }

    check(value, 0)
}

#[cfg(test)]
mod tests {
    use super::can_contain_messages;
    use crate::serde_json::{json, Value};

    #[test]
    fn rejects_non_message_value_kinds() {
        for value in [
            Value::Null,
            json!(false),
            json!(42),
            json!([]),
            json!([1, false, [null, 42], []]),
        ] {
            assert!(!can_contain_messages(&&value).unwrap(), "{value}");
        }
    }

    #[test]
    fn admits_all_object_shapes() {
        for value in [
            json!("Hello"),
            json!({}),
            json!({ "unrelated": { "role": "user", "content": "Hello" } }),
            json!({ "future_discriminator": "Hello" }),
            json!({ "": null }),
            json!([{ "future_discriminator": "Hello" }]),
            json!([[{ "future_discriminator": "Hello" }]]),
        ] {
            assert!(can_contain_messages(&&value).unwrap(), "{value}");
        }
    }

    #[test]
    fn admits_arrays_beyond_the_inspection_depth() {
        let value = (0..65).fold(Value::Null, |value, _| Value::Array(vec![value]));
        assert!(can_contain_messages(&&value).unwrap());
    }
}
