use crate::import_value::{ImportValue, InputKind};
use crate::universal::convert::TryFromLLM;
use crate::universal::Message;
use serde::de::DeserializeOwned;

pub(crate) type MessageParser<V> = fn(&V) -> Option<Vec<Message>>;

pub(crate) fn try_parsers_in_order<V: ImportValue>(
    data: &V,
    parsers: &[MessageParser<V>],
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

pub(crate) fn try_parse<T>(data: &impl ImportValue) -> Option<T>
where
    T: DeserializeOwned,
{
    data.parse()
}

pub(crate) fn try_convert_non_empty<T>(value: T) -> Option<Vec<Message>>
where
    Vec<Message>: TryFromLLM<T>,
{
    let messages = <Vec<Message> as TryFromLLM<T>>::try_from(value).ok()?;
    non_empty_messages(messages)
}

pub(crate) fn try_parse_vec_or_single<T>(data: &impl ImportValue) -> Option<Vec<T>>
where
    T: DeserializeOwned,
{
    match data.kind() {
        InputKind::Array => try_parse::<Vec<T>>(data),
        InputKind::Object => try_parse::<T>(data).map(|item| vec![item]),
        _ => None,
    }
}
