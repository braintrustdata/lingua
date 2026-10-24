use super::{ImportValue, InputKind};
use crate::serde_json::Value;
use js_sys::{Array, Map, Reflect};
use serde::de::{self, DeserializeOwned, DeserializeSeed, IntoDeserializer, Visitor};
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

#[derive(Debug)]
pub(crate) enum Error {
    Schema(String),
    JavaScript(JsValue),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(message) => formatter.write_str(message),
            Self::JavaScript(error) => write!(formatter, "{error:?}"),
        }
    }
}

impl std::error::Error for Error {}

impl de::Error for Error {
    fn custom<T: std::fmt::Display>(message: T) -> Self {
        Self::Schema(message.to_string())
    }
}

impl From<JsValue> for Error {
    fn from(error: JsValue) -> Self {
        Self::JavaScript(error)
    }
}

impl From<serde_wasm_bindgen::Error> for Error {
    fn from(error: serde_wasm_bindgen::Error) -> Self {
        Self::Schema(error.to_string())
    }
}

impl From<Error> for JsValue {
    fn from(error: Error) -> Self {
        match error {
            Error::Schema(message) => js_sys::Error::new(&message).into(),
            Error::JavaScript(error) => error,
        }
    }
}

fn or_throw<T>(result: Result<T, Error>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => wasm_bindgen::throw_val(error.into()),
    }
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
            Self::String(name) => seed.deserialize(de::value::StrDeserializer::new(name)),
            Self::Other(value) => seed
                .deserialize(serde_wasm_bindgen::Deserializer::from(value.clone()))
                .map_err(Error::from),
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
        Error::from(error)
    }

    fn resolve(&self) -> Result<JsValue, Error> {
        let cached = self.arena.borrow().nodes[self.index].value.clone();
        if let Some(value) = cached {
            return value.map_err(Error::from);
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
            return fields.map_err(Error::from);
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
                let descriptor = Reflect::get_own_property_descriptor(value.unchecked_ref(), &key)?;
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

    fn visit_object<'de, V: Visitor<'de>>(
        &self,
        visitor: V,
        eager: bool,
    ) -> Result<V::Value, Error> {
        if self.kind() != InputKind::Object {
            return Err(de::Error::custom("Expected an object"));
        }
        visitor.visit_map(Fields {
            input: self.clone(),
            fields: self.fields(eager)?,
            position: 0,
            value: None,
        })
    }

    fn parse_result<T: DeserializeOwned>(&self) -> Result<T, Error> {
        let result = T::deserialize(self);
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
        let value = or_throw(self.resolve());
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
        let value = or_throw(self.resolve());
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
        Cow::Owned(or_throw(self.parse_result()))
    }
}

struct Fields {
    input: JsInput,
    fields: ObjectFields,
    position: usize,
    value: Option<usize>,
}

impl<'de> de::MapAccess<'de> for Fields {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        let Some((key, value)) = self.fields.get(self.position) else {
            return Ok(None);
        };
        self.position += 1;
        self.value = Some(*value);
        key.deserialize(seed).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let value = self
            .input
            .child(self.value.take().expect("map value follows its key"));
        seed.deserialize(&value)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len() - self.position)
    }
}

struct Items(JsItems);

impl<'de> de::SeqAccess<'de> for Items {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        self.0
            .next()
            .map(|item| seed.deserialize(&item))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

macro_rules! delegate {
    ($($method:ident),* $(,)?) => {
        $(fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            serde_wasm_bindgen::Deserializer::from(self.resolve()?).$method(visitor).map_err(Error::from)
        })*
    };
}

impl<'de> Deserializer<'de> for &JsInput {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let value = self.resolve()?;
        if Array::is_array(&value) {
            return self.deserialize_seq(visitor);
        }
        if value.is_object()
            && !value.is_instance_of::<js_sys::Uint8Array>()
            && !value.is_instance_of::<js_sys::ArrayBuffer>()
            && (!js_sys::Symbol::iterator().js_in(&value) || value.is_instance_of::<Map>())
        {
            return self.visit_object(visitor, true);
        }
        serde_wasm_bindgen::Deserializer::from(value)
            .deserialize_any(visitor)
            .map_err(Error::from)
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
        if self.kind() == InputKind::Null {
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
        let Some(items) = self.array() else {
            return Err(de::Error::custom("Expected an array"));
        };
        visitor.visit_seq(Items(items))
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
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        if let Some(tag) = self.resolve()?.as_string() {
            return visitor.visit_enum(tag.into_deserializer());
        }
        if self.kind() != InputKind::Object {
            return Err(de::Error::custom("Expected an enum string or object"));
        }
        let fields = self.fields(true)?;
        if fields.len() != 1 {
            return Err(de::Error::custom("Expected a single enum variant"));
        }
        visitor.visit_enum(Variant {
            key: fields[0].0.clone(),
            value: self.child(fields[0].1),
        })
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.kind() == InputKind::Array {
            return self.deserialize_seq(visitor);
        }
        serde_wasm_bindgen::Deserializer::from(self.resolve()?)
            .deserialize_byte_buf(visitor)
            .map_err(Error::from)
    }
}

struct Variant {
    key: FieldKey,
    value: JsInput,
}

impl<'de> de::EnumAccess<'de> for Variant {
    type Error = Error;
    type Variant = JsInput;

    fn variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<(T::Value, Self::Variant), Error> {
        let tag = self.key.deserialize(seed)?;
        Ok((tag, self.value))
    }
}

impl<'de> de::VariantAccess<'de> for JsInput {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        serde::Deserialize::deserialize(&self)
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(&self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        (&self).deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        (&self).deserialize_struct("", fields, visitor)
    }
}
