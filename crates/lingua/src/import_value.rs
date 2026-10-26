use crate::serde_json::Value;
use serde::de::DeserializeOwned;
use std::borrow::Cow;

#[cfg(target_arch = "wasm32")]
mod js;
#[cfg(target_arch = "wasm32")]
pub(crate) use js::JsInput;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputKind {
    Null,
    String,
    Array,
    Object,
    Other,
}

pub(crate) trait ImportValue {
    type Item<'a>: ImportValue
    where
        Self: 'a;
    type Items<'a>: ExactSizeIterator<Item = Self::Item<'a>>
    where
        Self: 'a;

    fn kind(&self) -> InputKind;
    fn parse<T: DeserializeOwned>(&self) -> Option<T>;
    fn array(&self) -> Option<Self::Items<'_>>;
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
