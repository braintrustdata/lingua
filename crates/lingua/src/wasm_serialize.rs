use js_sys::{Array, BigInt, JsString, Object};
use serde::ser::{self, Impossible, Serialize};
use std::fmt::{self, Display};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

const PRIVATE_NUMBER_TOKEN: &str = "$serde_json::private::Number";
const MAX_SAFE_INTEGER_LITERAL: &str = "9007199254740991";
const MAX_SAFE_INTEGER_I128: i128 = 9_007_199_254_740_991;
const MIN_SAFE_INTEGER_I128: i128 = -9_007_199_254_740_991;

#[wasm_bindgen]
extern "C" {
    type ObjectExt;

    #[wasm_bindgen(method, indexing_setter)]
    fn set(this: &ObjectExt, key: JsString, value: JsValue);
}

pub(crate) type Result<T = JsValue> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub(crate) struct Error(String);

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::new(msg.to_string())
    }
}

pub(crate) fn to_value<T: Serialize + ?Sized>(value: &T) -> Result {
    value.serialize(&Serializer)
}

#[derive(Default)]
pub(crate) struct Serializer;

pub(crate) struct ArraySerializer<'a> {
    serializer: &'a Serializer,
    target: Array,
    index: u32,
}

impl<'a> ArraySerializer<'a> {
    fn new(serializer: &'a Serializer) -> Self {
        Self {
            serializer,
            target: Array::new(),
            index: 0,
        }
    }
}

pub(crate) struct MapSerializer<'a> {
    serializer: &'a Serializer,
    target: Object,
    next_key: Option<JsValue>,
}

impl<'a> MapSerializer<'a> {
    fn new(serializer: &'a Serializer) -> Self {
        Self {
            serializer,
            target: Object::new(),
            next_key: None,
        }
    }
}

pub(crate) struct ObjectSerializer<'a> {
    serializer: &'a Serializer,
    target: Object,
}

impl<'a> ObjectSerializer<'a> {
    fn new(serializer: &'a Serializer) -> Self {
        Self {
            serializer,
            target: Object::new(),
        }
    }
}

pub(crate) struct VariantSerializer<S> {
    variant: &'static str,
    inner: S,
}

impl<S> VariantSerializer<S> {
    fn new(variant: &'static str, inner: S) -> Self {
        Self { variant, inner }
    }

    fn end(self, inner: impl FnOnce(S) -> Result) -> Result {
        let value = inner(self.inner)?;
        let object = Object::new();
        set_object_property(&object, self.variant, value);
        Ok(object.into())
    }
}

#[derive(Default)]
pub(crate) struct PrivateNumberSerializer {
    value: Option<JsValue>,
}

pub(crate) enum StructSerializer<'a> {
    Object(ObjectSerializer<'a>),
    PrivateNumber(PrivateNumberSerializer),
}

pub(crate) struct StringSerializer;

fn is_integer_literal(raw: &str) -> bool {
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_js_safe_integer(raw: &str) -> bool {
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    let magnitude = digits.trim_start_matches('0');
    let magnitude = if magnitude.is_empty() { "0" } else { magnitude };

    if magnitude.len() != MAX_SAFE_INTEGER_LITERAL.len() {
        return magnitude.len() < MAX_SAFE_INTEGER_LITERAL.len();
    }

    magnitude <= MAX_SAFE_INTEGER_LITERAL
}

fn bigint_from_string(raw: &str) -> Result {
    BigInt::new(&JsValue::from_str(raw))
        .map(JsValue::from)
        .map_err(|_| Error::new(format!("Failed to serialize bigint literal: {raw}")))
}

fn serialize_private_number(raw: &str) -> Result {
    if is_integer_literal(raw) {
        if is_js_safe_integer(raw) {
            let value = raw
                .parse::<f64>()
                .map_err(|_| Error::new(format!("Failed to serialize number literal: {raw}")))?;
            return Ok(JsValue::from_f64(value));
        }

        return bigint_from_string(raw);
    }

    let value = raw
        .parse::<f64>()
        .map_err(|_| Error::new(format!("Failed to serialize number literal: {raw}")))?;
    Ok(JsValue::from_f64(value))
}

fn set_object_property(object: &Object, key: &str, value: JsValue) {
    object
        .unchecked_ref::<ObjectExt>()
        .set(JsString::from(key), value);
}

impl ser::SerializeSeq for ArraySerializer<'_> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.target
            .set(self.index, value.serialize(self.serializer)?);
        self.index += 1;
        Ok(())
    }

    fn end(self) -> Result {
        Ok(self.target.into())
    }
}

impl ser::SerializeTuple for ArraySerializer<'_> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for ArraySerializer<'_> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for VariantSerializer<ArraySerializer<'_>> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeTupleStruct::serialize_field(&mut self.inner, value)
    }

    fn end(self) -> Result {
        self.end(ser::SerializeTupleStruct::end)
    }
}

impl ser::SerializeMap for MapSerializer<'_> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        self.next_key = Some(key.serialize(self.serializer)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| Error::new("Missing map key during serialization"))?;
        let key = key
            .dyn_into::<JsString>()
            .map_err(|_| Error::new("Map key is not a string and cannot be serialized"))?;
        self.target
            .unchecked_ref::<ObjectExt>()
            .set(key, value.serialize(self.serializer)?);
        Ok(())
    }

    fn end(self) -> Result {
        Ok(self.target.into())
    }
}

impl ser::SerializeStruct for ObjectSerializer<'_> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        set_object_property(&self.target, key, value.serialize(self.serializer)?);
        Ok(())
    }

    fn end(self) -> Result {
        Ok(self.target.into())
    }
}

impl ser::SerializeStruct for PrivateNumberSerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        if key != PRIVATE_NUMBER_TOKEN {
            return Err(Error::new(format!(
                "Unexpected field in serde_json number wrapper: {key}"
            )));
        }

        let raw = value.serialize(StringSerializer)?;
        self.value = Some(serialize_private_number(&raw)?);
        Ok(())
    }

    fn end(self) -> Result {
        self.value
            .ok_or_else(|| Error::new("Missing value in serde_json number wrapper"))
    }
}

impl ser::SerializeStruct for StructSerializer<'_> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        match self {
            Self::Object(serializer) => serializer.serialize_field(key, value),
            Self::PrivateNumber(serializer) => serializer.serialize_field(key, value),
        }
    }

    fn end(self) -> Result {
        match self {
            Self::Object(serializer) => serializer.end(),
            Self::PrivateNumber(serializer) => serializer.end(),
        }
    }
}

impl ser::SerializeStructVariant for VariantSerializer<ObjectSerializer<'_>> {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        ser::SerializeStruct::serialize_field(&mut self.inner, key, value)
    }

    fn end(self) -> Result {
        self.end(ser::SerializeStruct::end)
    }
}

impl ser::Serializer for StringSerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = Impossible<String, Error>;
    type SerializeTuple = Impossible<String, Error>;
    type SerializeTupleStruct = Impossible<String, Error>;
    type SerializeTupleVariant = Impossible<String, Error>;
    type SerializeMap = Impossible<String, Error>;
    type SerializeStruct = Impossible<String, Error>;
    type SerializeStructVariant = Impossible<String, Error>;

    fn serialize_str(self, value: &str) -> std::result::Result<String, Error> {
        Ok(value.to_owned())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::result::Result<String, Error> {
        value.serialize(self)
    }

    fn serialize_bool(self, _value: bool) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_i8(self, _value: i8) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_i16(self, _value: i16) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_i32(self, _value: i32) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_i64(self, _value: i64) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_i128(self, _value: i128) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_u8(self, _value: u8) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_u16(self, _value: u16) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_u32(self, _value: u32) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_u64(self, _value: u64) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_u128(self, _value: u128) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_f32(self, _value: f32) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_f64(self, _value: f64) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_char(self, _value: char) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_bytes(self, _value: &[u8]) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_none(self) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_some<T: ?Sized + Serialize>(
        self,
        value: &T,
    ) -> std::result::Result<String, Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> std::result::Result<String, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> std::result::Result<Self::SerializeSeq, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_tuple(self, _len: usize) -> std::result::Result<Self::SerializeTuple, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_map(self, _len: Option<usize>) -> std::result::Result<Self::SerializeMap, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Error> {
        Err(Error::new("Expected string"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Error> {
        Err(Error::new("Expected string"))
    }
}

impl<'a> ser::Serializer for &'a Serializer {
    type Ok = JsValue;
    type Error = Error;
    type SerializeSeq = ArraySerializer<'a>;
    type SerializeTuple = ArraySerializer<'a>;
    type SerializeTupleStruct = ArraySerializer<'a>;
    type SerializeTupleVariant = VariantSerializer<ArraySerializer<'a>>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = VariantSerializer<ObjectSerializer<'a>>;

    fn serialize_bool(self, value: bool) -> Result {
        Ok(JsValue::from_bool(value))
    }

    fn serialize_i8(self, value: i8) -> Result {
        Ok(JsValue::from_f64(value as f64))
    }

    fn serialize_i16(self, value: i16) -> Result {
        Ok(JsValue::from_f64(value as f64))
    }

    fn serialize_i32(self, value: i32) -> Result {
        Ok(JsValue::from_f64(value as f64))
    }

    fn serialize_i64(self, value: i64) -> Result {
        if (MIN_SAFE_INTEGER_I128..=MAX_SAFE_INTEGER_I128).contains(&(value as i128)) {
            return Ok(JsValue::from_f64(value as f64));
        }

        bigint_from_string(&value.to_string())
    }

    fn serialize_i128(self, value: i128) -> Result {
        if (MIN_SAFE_INTEGER_I128..=MAX_SAFE_INTEGER_I128).contains(&value) {
            return Ok(JsValue::from_f64(value as f64));
        }

        bigint_from_string(&value.to_string())
    }

    fn serialize_u8(self, value: u8) -> Result {
        Ok(JsValue::from_f64(value as f64))
    }

    fn serialize_u16(self, value: u16) -> Result {
        Ok(JsValue::from_f64(value as f64))
    }

    fn serialize_u32(self, value: u32) -> Result {
        Ok(JsValue::from_f64(value as f64))
    }

    fn serialize_u64(self, value: u64) -> Result {
        if value <= MAX_SAFE_INTEGER_I128 as u64 {
            return Ok(JsValue::from_f64(value as f64));
        }

        bigint_from_string(&value.to_string())
    }

    fn serialize_u128(self, value: u128) -> Result {
        if value <= MAX_SAFE_INTEGER_I128 as u128 {
            return Ok(JsValue::from_f64(value as f64));
        }

        bigint_from_string(&value.to_string())
    }

    fn serialize_f32(self, value: f32) -> Result {
        Ok(JsValue::from_f64(value as f64))
    }

    fn serialize_f64(self, value: f64) -> Result {
        Ok(JsValue::from_f64(value))
    }

    fn serialize_char(self, value: char) -> Result {
        Ok(JsValue::from_str(&value.to_string()))
    }

    fn serialize_str(self, value: &str) -> Result {
        Ok(JsValue::from_str(value))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result {
        let out = Array::new_with_length(value.len() as u32);
        for (index, byte) in value.iter().enumerate() {
            out.set(index as u32, JsValue::from_f64(*byte as f64));
        }
        Ok(out.into())
    }

    fn serialize_none(self) -> Result {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result {
        Ok(JsValue::NULL)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result {
        Ok(JsValue::from_str(variant))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result {
        VariantSerializer::new(variant, value.serialize(self)?).end(Ok)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(ArraySerializer::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(ArraySerializer::new(self))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(ArraySerializer::new(self))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(VariantSerializer::new(variant, ArraySerializer::new(self)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer::new(self))
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        if name == PRIVATE_NUMBER_TOKEN {
            return Ok(StructSerializer::PrivateNumber(
                PrivateNumberSerializer::default(),
            ));
        }

        Ok(StructSerializer::Object(ObjectSerializer::new(self)))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(VariantSerializer::new(variant, ObjectSerializer::new(self)))
    }

    fn collect_str<T: ?Sized + Display>(self, value: &T) -> Result {
        self.serialize_str(&value.to_string())
    }
}
