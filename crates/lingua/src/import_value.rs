use crate::serde_json::Value;
use serde::de::DeserializeOwned;
use std::borrow::Cow;

#[cfg(target_arch = "wasm32")]
mod js;
#[cfg(target_arch = "wasm32")]
pub(crate) use js::JsInput;

/// JSON shape used to select import parsers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Null,
    String,
    Array,
    Object,
    Other,
}

/// Read-only JSON input for typed message import without an intermediate owned tree.
///
/// This trait is intentionally open to implementations backed by external storage.
/// All methods must describe the same stable value throughout an import. Required
/// methods form a public compatibility contract; future methods must have defaults
/// unless introduced in a breaking release.
pub trait ImportValue {
    /// A view of one array element, borrowing storage when possible.
    type Item<'a>: ImportValue
    where
        Self: 'a;
    /// Array elements in source order, with an accurate remaining length.
    type Items<'a>: ExactSizeIterator<Item = Self::Item<'a>>
    where
        Self: 'a;

    /// Report the JSON shape without materializing an owned tree.
    fn kind(&self) -> InputKind;
    /// Deserialize directly into an owned type, returning `None` on a schema mismatch
    /// or deserialization failure. Returned data must not borrow from this input.
    fn parse<T: DeserializeOwned>(&self) -> Option<T>;
    /// Return all elements for arrays, or `None` for other shapes.
    fn array(&self) -> Option<Self::Items<'_>>;
    /// Preserve the complete JSON value when an import needs an opaque payload or
    /// a compatibility parser requires materialization. Prefer borrowing when possible.
    fn to_value(&self) -> Cow<'_, Value>;
}

impl ImportValue for Value {
    type Item<'a> = &'a Value;
    type Items<'a> = std::slice::Iter<'a, Value>;

    fn kind(&self) -> InputKind {
        match self {
            Value::Null => InputKind::Null,
            Value::String(_) => InputKind::String,
            Value::Array(_) => InputKind::Array,
            Value::Object(_) => InputKind::Object,
            _ => InputKind::Other,
        }
    }

    fn parse<T: DeserializeOwned>(&self) -> Option<T> {
        #[cfg(target_arch = "wasm32")]
        {
            js::parse_value(self)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            T::deserialize(self).ok()
        }
    }

    fn array(&self) -> Option<Self::Items<'_>> {
        self.as_array().map(|items| items.iter())
    }

    fn to_value(&self) -> Cow<'_, Value> {
        Cow::Borrowed(self)
    }
}

impl<V: ImportValue + ?Sized> ImportValue for &V {
    type Item<'a>
        = V::Item<'a>
    where
        Self: 'a;
    type Items<'a>
        = V::Items<'a>
    where
        Self: 'a;

    fn kind(&self) -> InputKind {
        V::kind(self)
    }

    fn parse<T: DeserializeOwned>(&self) -> Option<T> {
        V::parse(self)
    }

    fn array(&self) -> Option<Self::Items<'_>> {
        V::array(self)
    }

    fn to_value(&self) -> Cow<'_, Value> {
        V::to_value(self)
    }
}

impl<V: ImportValue> ImportValue for Option<V> {
    type Item<'a>
        = V::Item<'a>
    where
        Self: 'a;
    type Items<'a>
        = V::Items<'a>
    where
        Self: 'a;

    fn kind(&self) -> InputKind {
        self.as_ref().map_or(InputKind::Null, ImportValue::kind)
    }

    fn parse<T: DeserializeOwned>(&self) -> Option<T> {
        match self {
            Some(value) => value.parse(),
            None => T::deserialize(&Value::Null).ok(),
        }
    }

    fn array(&self) -> Option<Self::Items<'_>> {
        self.as_ref()?.array()
    }

    fn to_value(&self) -> Cow<'_, Value> {
        match self {
            Some(value) => value.to_value(),
            None => Cow::Owned(Value::Null),
        }
    }
}

#[derive(Default)]
pub(crate) enum Field<T> {
    #[default]
    Missing,
    Present(T),
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Field<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::Present)
    }
}

impl<T> Field<T> {
    pub(crate) fn is_present(&self) -> bool {
        matches!(self, Self::Present(_))
    }

    pub(crate) fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Missing => None,
            Self::Present(value) => Some(value),
        }
    }
}
