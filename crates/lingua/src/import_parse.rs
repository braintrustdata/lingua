use crate::serde_json::{self, Value};
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
    serde_json::from_value::<T>(data.clone()).ok()
}

pub(crate) fn try_convert_non_empty<T>(value: T) -> Option<Vec<Message>>
where
    Vec<Message>: TryFromLLM<T>,
{
    let messages = <Vec<Message> as TryFromLLM<T>>::try_from(value).ok()?;
    non_empty_messages(messages)
}

pub(crate) fn try_parse_and_convert<T>(data: &Value) -> Option<Vec<Message>>
where
    T: DeserializeOwned,
    Vec<Message>: TryFromLLM<T>,
{
    let value = try_parse::<T>(data)?;
    try_convert_non_empty(value)
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
