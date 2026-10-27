use super::{ImportValue, InputKind};
use crate::serde_json::{Error, Value};
use js_sys::{Array, Map, Reflect};
use serde::de::{self, DeserializeOwned, DeserializeSeed, Visitor};
use serde::Deserializer;
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = Object, js_name = entries)]
    fn object_entries(value: &JsValue) -> Result<Array, JsValue>;
}

fn schema_error(error: impl std::fmt::Display) -> Error {
    de::Error::custom(error)
}

#[derive(Clone)]
pub(crate) struct JsInput {
    arena: Rc<RefCell<InputArena>>,
    index: usize,
}

type ObjectFields = Rc<[(FieldKey, usize)]>;

#[derive(Clone)]
enum FieldKey {
    String(String),
    Other(JsValue),
}

impl From<JsValue> for FieldKey {
    fn from(value: JsValue) -> Self {
        match value.as_string() {
            Some(name) => Self::String(name),
            None => Self::Other(value),
        }
    }
}

impl FieldKey {
    fn deserialize<'de, T: DeserializeSeed<'de>>(&self, seed: T) -> Result<T::Value, Error> {
        match self {
            Self::String(name) => seed.deserialize(de::value::StrDeserializer::<Error>::new(name)),
            Self::Other(value) => seed
                .deserialize(serde_wasm_bindgen::Deserializer::from(value.clone()))
                .map_err(schema_error),
        }
    }
}

struct InputArena {
    nodes: Vec<Node>,
    failure: Option<JsValue>,
}

struct Node {
    source: Source,
    value: Option<Result<JsValue, JsValue>>,
    array: Option<Rc<[usize]>>,
    fields: Option<Result<ObjectFields, JsValue>>,
}

impl Node {
    fn new(source: Source) -> Self {
        Self {
            source,
            value: None,
            array: None,
            fields: None,
        }
    }
}

#[derive(Clone)]
enum Source {
    Value(JsValue),
    Property { object: JsValue, key: JsValue },
}

impl JsInput {
    fn or_throw<T>(&self, result: Result<T, Error>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                let error = match self.arena.borrow().failure.clone() {
                    Some(error) => error,
                    None => js_sys::Error::new(&error.to_string()).into(),
                };
                wasm_bindgen::throw_val(error)
            }
        }
    }

    pub(crate) fn new(value: JsValue) -> Self {
        let mut nodes = Vec::with_capacity(32);
        nodes.push(Node::new(Source::Value(value)));
        Self {
            arena: Rc::new(RefCell::new(InputArena {
                nodes,
                failure: None,
            })),
            index: 0,
        }
    }

    fn child(&self, index: usize) -> Self {
        Self {
            arena: self.arena.clone(),
            index,
        }
    }

    fn allocate(&self, source: Source) -> usize {
        let mut arena = self.arena.borrow_mut();
        let index = arena.nodes.len();
        arena.nodes.push(Node::new(source));
        index
    }

    fn record_error(&self, error: JsValue) -> Error {
        self.arena
            .borrow_mut()
            .failure
            .get_or_insert_with(|| error.clone());
        de::Error::custom("JavaScript input access failed")
    }

    fn resolve(&self) -> Result<JsValue, Error> {
        let cached = self.arena.borrow().nodes[self.index].value.clone();
        if let Some(value) = cached {
            return value.map_err(|error| self.record_error(error));
        }
        let source = self.arena.borrow().nodes[self.index].source.clone();
        let value = match source {
            Source::Value(value) => Ok(value),
            Source::Property { object, key } => Reflect::get(&object, &key),
        };
        self.arena.borrow_mut().nodes[self.index].value = Some(value.clone());
        value.map_err(|error| self.record_error(error))
    }

    fn fields(&self, eager: bool) -> Result<ObjectFields, Error> {
        let cached = self.arena.borrow().nodes[self.index].fields.clone();
        if let Some(fields) = cached {
            return fields.map_err(|error| self.record_error(error));
        }
        let value = self.resolve()?;
        let fields = (|| {
            if let Some(map) = value.dyn_ref::<Map>() {
                let mut fields = Vec::with_capacity(map.size() as usize);
                self.arena.borrow_mut().nodes.reserve(map.size() as usize);
                for entry in map.entries() {
                    let pair = Array::from(&entry?);
                    fields.push((
                        pair.get(0).into(),
                        self.allocate(Source::Value(pair.get(1))),
                    ));
                }
                return Ok(Rc::from(fields));
            }
            if eager {
                let entries = object_entries(&value)?;
                let mut fields = Vec::with_capacity(entries.length() as usize);
                self.arena
                    .borrow_mut()
                    .nodes
                    .reserve(entries.length() as usize);
                for entry in entries.iter() {
                    let pair = entry.unchecked_into::<Array>();
                    fields.push((
                        pair.get(0).into(),
                        self.allocate(Source::Value(pair.get(1))),
                    ));
                }
                return Ok(Rc::from(fields));
            }
            let keys = Reflect::own_keys(&value)?;
            let mut fields = Vec::with_capacity(keys.length() as usize);
            self.arena
                .borrow_mut()
                .nodes
                .reserve(keys.length() as usize);
            for key in keys.iter().filter(JsValue::is_string) {
                let descriptor = Reflect::get_own_property_descriptor(
                    value.unchecked_ref::<js_sys::Object>(),
                    &key,
                )?;
                if descriptor.is_undefined()
                    || Reflect::get(&descriptor, &"enumerable".into())? != JsValue::TRUE
                {
                    continue;
                }
                let index = self.allocate(Source::Property {
                    object: value.clone(),
                    key: key.clone(),
                });
                fields.push((key.into(), index));
            }
            Ok(Rc::from(fields))
        })();
        self.arena.borrow_mut().nodes[self.index].fields = Some(fields.clone());
        fields.map_err(|error| self.record_error(error))
    }

    fn parse_result<T: DeserializeOwned>(&self) -> Result<T, Error> {
        let result = T::deserialize(Input::JavaScript(self.clone()));
        let failure = self.arena.borrow().failure.clone();
        if let Some(error) = failure {
            wasm_bindgen::throw_val(error);
        }
        result
    }
}

pub(crate) struct JsItems {
    input: JsInput,
    indices: Rc<[usize]>,
    position: usize,
}

impl Iterator for JsItems {
    type Item = JsInput;

    fn next(&mut self) -> Option<Self::Item> {
        let index = *self.indices.get(self.position)?;
        self.position += 1;
        Some(self.input.child(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for JsItems {
    fn len(&self) -> usize {
        self.indices.len() - self.position
    }
}

impl ImportValue for JsInput {
    type Item<'a> = Self;
    type Items<'a> = JsItems;

    fn kind(&self) -> InputKind {
        let value = self.or_throw(self.resolve());
        if value.is_null() || value.is_undefined() {
            InputKind::Null
        } else if value.is_string() {
            InputKind::String
        } else if Array::is_array(&value) {
            InputKind::Array
        } else if value.is_object() {
            InputKind::Object
        } else {
            InputKind::Other
        }
    }

    fn parse<T: DeserializeOwned>(&self) -> Option<T> {
        self.parse_result().ok()
    }

    fn array(&self) -> Option<Self::Items<'_>> {
        let value = self.or_throw(self.resolve());
        if !Array::is_array(&value) {
            return None;
        }
        let cached = self.arena.borrow().nodes[self.index].array.clone();
        let indices = if let Some(indices) = cached {
            indices
        } else {
            let length = value.unchecked_ref::<Array>().length();
            self.arena.borrow_mut().nodes.reserve(length as usize);
            let indices: Rc<[usize]> = (0..length)
                .map(|index| {
                    self.allocate(Source::Property {
                        object: value.clone(),
                        key: JsValue::from_f64(index as f64),
                    })
                })
                .collect();
            self.arena.borrow_mut().nodes[self.index].array = Some(indices.clone());
            indices
        };
        Some(JsItems {
            input: self.clone(),
            indices,
            position: 0,
        })
    }

    fn to_value(&self) -> Cow<'_, Value> {
        Cow::Owned(self.or_throw(self.parse_result()))
    }
}

pub(super) fn parse_value<T: DeserializeOwned>(value: &Value) -> Option<T> {
    T::deserialize(Input::Native(value)).ok()
}

enum Input<'de> {
    Native(&'de Value),
    JavaScript(JsInput),
}

enum FieldSource<'de> {
    Native(crate::serde_json::map::Iter<'de>),
    JavaScript {
        input: JsInput,
        fields: ObjectFields,
        position: usize,
    },
}

struct Fields<'de> {
    source: FieldSource<'de>,
    value: Option<Input<'de>>,
}

impl Fields<'_> {
    fn remaining(&self) -> usize {
        match &self.source {
            FieldSource::Native(fields) => fields.len(),
            FieldSource::JavaScript {
                fields, position, ..
            } => fields.len() - position,
        }
    }
}

impl<'de> de::MapAccess<'de> for Fields<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match &mut self.source {
            FieldSource::Native(fields) => {
                let Some((key, value)) = fields.next() else {
                    return Ok(None);
                };
                self.value = Some(Input::Native(value));
                seed.deserialize(de::value::StrDeserializer::<Error>::new(key))
                    .map(Some)
            }
            FieldSource::JavaScript {
                input,
                fields,
                position,
            } => {
                let Some((key, index)) = fields.get(*position) else {
                    return Ok(None);
                };
                *position += 1;
                self.value = Some(Input::JavaScript(input.child(*index)));
                key.deserialize(seed).map(Some)
            }
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        seed.deserialize(self.value.take().expect("map value follows its key"))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining())
    }
}

enum Items<'de> {
    Native(std::slice::Iter<'de, Value>),
    JavaScript(JsItems),
}

impl Items<'_> {
    fn remaining(&self) -> usize {
        match self {
            Self::Native(items) => items.len(),
            Self::JavaScript(items) => items.len(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for Items<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        let input = match self {
            Self::Native(items) => items.next().map(Input::Native),
            Self::JavaScript(items) => items.next().map(Input::JavaScript),
        };
        input.map(|input| seed.deserialize(input)).transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining())
    }
}

impl<'de> Input<'de> {
    fn visit_object<V: Visitor<'de>>(self, visitor: V, eager: bool) -> Result<V::Value, Error> {
        let (native, source) = match self {
            Self::Native(Value::Object(fields)) => (true, FieldSource::Native(fields.iter())),
            Self::Native(_) => return Err(de::Error::custom("Expected an object")),
            Self::JavaScript(input) => {
                if input.kind() != InputKind::Object {
                    return Err(de::Error::custom("Expected an object"));
                }
                let fields = input.fields(eager)?;
                (
                    false,
                    FieldSource::JavaScript {
                        input,
                        fields,
                        position: 0,
                    },
                )
            }
        };
        let mut fields = Fields {
            source,
            value: None,
        };
        let value = visitor.visit_map(&mut fields)?;
        if native && fields.remaining() != 0 {
            return Err(de::Error::custom(
                "Expected all object fields to be consumed",
            ));
        }
        Ok(value)
    }
}

macro_rules! delegate {
    ($($method:ident),* $(,)?) => {
        $(fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            match self {
                Self::Native(value) => value.$method(visitor),
                Self::JavaScript(input) => serde_wasm_bindgen::Deserializer::from(input.resolve()?).$method(visitor).map_err(schema_error),
            }
        })*
    };
}

impl<'de> Deserializer<'de> for Input<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let input = match self {
            Self::Native(Value::Array(_)) => return self.deserialize_seq(visitor),
            Self::Native(Value::Object(_)) => return self.deserialize_map(visitor),
            Self::Native(value) => return value.deserialize_any(visitor),
            Self::JavaScript(input) => input,
        };
        let value = input.resolve()?;
        if Array::is_array(&value) {
            return Self::JavaScript(input).deserialize_seq(visitor);
        }
        if value.is_object()
            && !value.is_instance_of::<js_sys::Uint8Array>()
            && !value.is_instance_of::<js_sys::ArrayBuffer>()
            && (!js_sys::Symbol::iterator().js_in(&value) || value.is_instance_of::<Map>())
        {
            return Self::JavaScript(input).visit_object(visitor, true);
        }
        serde_wasm_bindgen::Deserializer::from(value)
            .deserialize_any(visitor)
            .map_err(schema_error)
    }

    delegate!(
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_unit,
        deserialize_identifier,
    );

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let null = match &self {
            Self::Native(value) => value.is_null(),
            Self::JavaScript(input) => input.kind() == InputKind::Null,
        };
        if null {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let (native, mut items) = match self {
            Self::Native(Value::Array(items)) => (true, Items::Native(items.iter())),
            Self::Native(_) => return Err(de::Error::custom("Expected an array")),
            Self::JavaScript(input) => {
                let Some(items) = input.array() else {
                    return Err(de::Error::custom("Expected an array"));
                };
                (false, Items::JavaScript(items))
            }
        };
        let value = visitor.visit_seq(&mut items)?;
        if native && items.remaining() != 0 {
            return Err(de::Error::custom(
                "Expected all array elements to be consumed",
            ));
        }
        Ok(value)
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.visit_object(visitor, false)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        if matches!(self, Self::Native(Value::Array(_))) {
            return self.deserialize_seq(visitor);
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let variant = match self {
            Self::Native(Value::String(tag)) => {
                return visitor.visit_enum(de::value::StrDeserializer::<Error>::new(tag))
            }
            Self::Native(Value::Object(fields)) if fields.len() == 1 => {
                let (key, value) = fields.iter().next().expect("one enum variant");
                Variant {
                    key: VariantKey::Native(key),
                    value: Self::Native(value),
                }
            }
            Self::Native(_) => return Err(de::Error::custom("Expected an enum string or object")),
            Self::JavaScript(input) => {
                if let Some(tag) = input.resolve()?.as_string() {
                    return visitor.visit_enum(de::value::StrDeserializer::<Error>::new(&tag));
                }
                if input.kind() != InputKind::Object {
                    return Err(de::Error::custom("Expected an enum string or object"));
                }
                let fields = input.fields(true)?;
                if fields.len() != 1 {
                    return Err(de::Error::custom("Expected a single enum variant"));
                }
                Variant {
                    key: VariantKey::JavaScript(fields[0].0.clone()),
                    value: Self::JavaScript(input.child(fields[0].1)),
                }
            }
        };
        visitor.visit_enum(variant)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let input = match self {
            Self::Native(value) => return value.deserialize_byte_buf(visitor),
            Self::JavaScript(input) => input,
        };
        if input.kind() == InputKind::Array {
            return Self::JavaScript(input).deserialize_seq(visitor);
        }
        serde_wasm_bindgen::Deserializer::from(input.resolve()?)
            .deserialize_byte_buf(visitor)
            .map_err(schema_error)
    }
}

enum VariantKey<'de> {
    Native(&'de str),
    JavaScript(FieldKey),
}

struct Variant<'de> {
    key: VariantKey<'de>,
    value: Input<'de>,
}

impl<'de> de::EnumAccess<'de> for Variant<'de> {
    type Error = Error;
    type Variant = Input<'de>;

    fn variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<(T::Value, Self::Variant), Error> {
        let tag = match self.key {
            VariantKey::Native(key) => {
                seed.deserialize(de::value::StrDeserializer::<Error>::new(key))?
            }
            VariantKey::JavaScript(key) => key.deserialize(seed)?,
        };
        Ok((tag, self.value))
    }
}

impl<'de> de::VariantAccess<'de> for Input<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        serde::Deserialize::deserialize(self)
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_struct("", fields, visitor)
    }
}
