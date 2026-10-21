use crate::serde_json::Value;
use crate::universal::convert::TryFromLLM;
use crate::universal::Message;
use serde::de::DeserializeOwned;

pub(crate) type MessageParser = fn(&Value) -> Option<Vec<Message>>;

pub(crate) fn try_parsers_in_order(
    data: &Value,
    parsers: &[MessageParser],
) -> Option<Vec<Message>> {
    for parser in parsers {
        if let Some(messages) = parser(data) {
            if !messages.is_empty() {
                return Some(messages);
            }
        }
    }
    None
}

pub(crate) fn non_empty_messages(messages: Vec<Message>) -> Option<Vec<Message>> {
    if messages.is_empty() {
        None
    } else {
        Some(messages)
    }
}

pub(crate) fn try_parse<T>(data: &Value) -> Option<T>
where
    T: DeserializeOwned,
{
    T::deserialize(data).ok()
}

pub(crate) fn try_convert_non_empty<T>(value: T) -> Option<Vec<Message>>
where
    Vec<Message>: TryFromLLM<T>,
{
    let messages = <Vec<Message> as TryFromLLM<T>>::try_from(value).ok()?;
    non_empty_messages(messages)
}

pub(crate) fn try_parse_vec_or_single<T>(data: &Value) -> Option<Vec<T>>
where
    T: DeserializeOwned,
{
    match data {
        Value::Array(_) => try_parse::<Vec<T>>(data),
        Value::Object(_) => try_parse::<T>(data).map(|item| vec![item]),
        _ => None,
    }
}

#[cfg(feature = "openai")]
pub(crate) fn try_parse_items<T>(
    data: &Value,
) -> Option<impl Iterator<Item = Result<T, crate::error::ConvertError>> + '_>
where
    T: DeserializeOwned,
{
    let items = match data {
        Value::Array(items) => items.as_slice(),
        Value::Object(_) => std::slice::from_ref(data),
        _ => return None,
    };
    Some(items.iter().map(|item| {
        T::deserialize(item).map_err(
            |error| crate::error::ConvertError::ContentConversionFailed {
                reason: error.to_string(),
            },
        )
    }))
}
