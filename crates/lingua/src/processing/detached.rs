use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use thiserror::Error;

const TOKEN_START: char = '\u{E000}';
const TOKEN_END: char = '\u{E001}';
const MIN_DETACHED_LEN: usize = 32;
const TOKEN_PREFIX_LEN: usize = 16;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DetachedStringError {
    #[error("malformed detached string token")]
    MalformedToken,
    #[error("detached string {0} was already moved out")]
    AlreadyAttached(usize),
}

/// Long JSON strings moved out of values so message import only sees short
/// placeholder tokens.
///
/// [`DetachedStrings::detach`] replaces each string of at least 32 bytes with
/// a token made of the string's first 16 bytes followed by a private-use
/// delimited id. Equal strings share one id, so token equality matches content
/// equality. Strings that look like JSON documents and whitespace-only strings
/// stay inline because import parses or trims them. [`DetachedStrings::attach`]
/// moves the originals back into the output, cloning only when a token appears
/// more than once.
#[derive(Debug, Clone, Default)]
pub struct DetachedStrings {
    strings: Vec<DetachedString>,
    by_hash: HashMap<u64, Vec<usize>>,
}

#[derive(Debug, Clone)]
struct DetachedString {
    value: Option<String>,
    prefix_len: usize,
}

fn content_hash(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn should_detach(value: &str) -> bool {
    if value.contains(TOKEN_START) {
        return true;
    }
    if value.len() < MIN_DETACHED_LEN {
        return false;
    }
    let trimmed = value.trim_start();
    !trimmed.is_empty() && !trimmed.starts_with(['{', '[', '"'])
}

fn token_prefix_len(value: &str) -> usize {
    let mut len = TOKEN_PREFIX_LEN.min(value.len());
    if let Some(start) = value.find(TOKEN_START) {
        len = len.min(start);
    }
    while !value.is_char_boundary(len) {
        len -= 1;
    }
    len
}

fn write_token(out: &mut String, prefix: &str, id: usize) {
    use std::fmt::Write;
    out.push_str(prefix);
    out.push(TOKEN_START);
    write!(out, "{id:x}").expect("writing to a String cannot fail");
    out.push(TOKEN_END);
}

struct TokenRef {
    marker: usize,
    end: usize,
    id: usize,
}

impl DetachedStrings {
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Detaches every eligible string in `value`, replacing it with a token.
    pub fn detach(&mut self, value: &mut Value) {
        match value {
            Value::String(text) => {
                if should_detach(text) {
                    let original = std::mem::take(text);
                    let id = self.insert(original);
                    let entry = &self.strings[id];
                    let prefix =
                        &entry.value.as_deref().expect("just inserted")[..entry.prefix_len];
                    let mut token = String::with_capacity(prefix.len() + 12);
                    write_token(&mut token, prefix, id);
                    *text = token;
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|item| self.detach(item)),
            Value::Object(map) => map.values_mut().for_each(|item| self.detach(item)),
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    fn insert(&mut self, value: String) -> usize {
        let candidates = self.by_hash.entry(content_hash(&value)).or_default();
        if let Some(&id) = candidates
            .iter()
            .find(|&&id| self.strings[id].value.as_deref() == Some(value.as_str()))
        {
            return id;
        }
        let id = self.strings.len();
        candidates.push(id);
        let prefix_len = token_prefix_len(&value);
        self.strings.push(DetachedString {
            value: Some(value),
            prefix_len,
        });
        id
    }

    /// Moves `other`'s strings into `self` and rewrites `other`'s tokens in
    /// `values` to refer to `self`.
    pub fn merge<'a>(
        &mut self,
        other: DetachedStrings,
        values: impl IntoIterator<Item = &'a mut Value>,
    ) -> Result<(), DetachedStringError> {
        if other.is_empty() {
            return Ok(());
        }
        let remap = other
            .strings
            .into_iter()
            .map(|entry| {
                entry
                    .value
                    .map(|value| self.insert(value))
                    .ok_or(DetachedStringError::MalformedToken)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for value in values {
            self.remap(value, &remap)?;
        }
        Ok(())
    }

    fn remap(&self, value: &mut Value, remap: &[usize]) -> Result<(), DetachedStringError> {
        match value {
            Value::String(text) if text.contains(TOKEN_START) => {
                let mut out = String::with_capacity(text.len());
                let mut last = 0;
                for token in tokens(text)? {
                    let id = *remap
                        .get(token.id)
                        .ok_or(DetachedStringError::MalformedToken)?;
                    let (start, original) = self.resolve(id, text, token.marker)?;
                    out.push_str(&text[last..start]);
                    write_token(&mut out, &original[..self.strings[id].prefix_len], id);
                    last = token.end;
                }
                out.push_str(&text[last..]);
                *text = out;
            }
            Value::Array(items) => {
                for item in items {
                    self.remap(item, remap)?;
                }
            }
            Value::Object(map) => {
                for item in map.values_mut() {
                    self.remap(item, remap)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Restores detached strings in `value`, moving each original into its last
    /// use and cloning it for earlier uses.
    pub fn attach(&mut self, value: &mut Value) -> Result<(), DetachedStringError> {
        let mut uses = vec![0usize; self.strings.len()];
        self.count_uses(value, &mut uses)?;
        self.attach_value(value, &mut uses)
    }

    /// Restores detached strings in `value` without consuming `self`.
    pub fn attach_cloned(&self, value: &mut Value) -> Result<(), DetachedStringError> {
        match value {
            Value::String(text) if text.contains(TOKEN_START) => {
                *text = self.attach_text_cloned(text)?;
            }
            Value::Array(items) => {
                for item in items {
                    self.attach_cloned(item)?;
                }
            }
            Value::Object(map) => {
                for item in map.values_mut() {
                    self.attach_cloned(item)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Restores detached strings embedded in rendered text.
    pub fn attach_text_cloned(&self, text: &str) -> Result<String, DetachedStringError> {
        let mut out = String::new();
        let mut last = 0;
        for token in tokens(text)? {
            let (start, original) = self.resolve(token.id, text, token.marker)?;
            out.push_str(&text[last..start]);
            out.push_str(original);
            last = token.end;
        }
        if last == 0 {
            return Ok(text.to_string());
        }
        out.push_str(&text[last..]);
        Ok(out)
    }

    fn resolve<'s>(
        &'s self,
        id: usize,
        text: &str,
        marker: usize,
    ) -> Result<(usize, &'s str), DetachedStringError> {
        let entry = self
            .strings
            .get(id)
            .ok_or(DetachedStringError::MalformedToken)?;
        let original = entry
            .value
            .as_deref()
            .ok_or(DetachedStringError::AlreadyAttached(id))?;
        let start = marker
            .checked_sub(entry.prefix_len)
            .ok_or(DetachedStringError::MalformedToken)?;
        if text.get(start..marker) != Some(&original[..entry.prefix_len]) {
            return Err(DetachedStringError::MalformedToken);
        }
        Ok((start, original))
    }

    fn count_uses(&self, value: &Value, uses: &mut [usize]) -> Result<(), DetachedStringError> {
        match value {
            Value::String(text) if text.contains(TOKEN_START) => {
                for token in tokens(text)? {
                    *uses
                        .get_mut(token.id)
                        .ok_or(DetachedStringError::MalformedToken)? += 1;
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.count_uses(item, uses)?;
                }
            }
            Value::Object(map) => {
                for item in map.values() {
                    self.count_uses(item, uses)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn attach_value(
        &mut self,
        value: &mut Value,
        uses: &mut [usize],
    ) -> Result<(), DetachedStringError> {
        match value {
            Value::String(text) if text.contains(TOKEN_START) => {
                let found = tokens(text)?;
                if let [token] = found.as_slice() {
                    let (start, _) = self.resolve(token.id, text, token.marker)?;
                    if start == 0 && token.end == text.len() {
                        uses[token.id] -= 1;
                        *text = if uses[token.id] == 0 {
                            self.strings[token.id].value.take().expect("checked above")
                        } else {
                            self.strings[token.id].value.clone().expect("checked above")
                        };
                        return Ok(());
                    }
                }
                let attached = self.attach_text_cloned(text)?;
                for token in found {
                    uses[token.id] -= 1;
                    if uses[token.id] == 0 {
                        self.strings[token.id].value = None;
                    }
                }
                *text = attached;
            }
            Value::Array(items) => {
                for item in items {
                    self.attach_value(item, uses)?;
                }
            }
            Value::Object(map) => {
                for item in map.values_mut() {
                    self.attach_value(item, uses)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn tokens(text: &str) -> Result<Vec<TokenRef>, DetachedStringError> {
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(offset) = text[search..].find(TOKEN_START) {
        let marker = search + offset;
        let id_start = marker + TOKEN_START.len_utf8();
        let id_len = text[id_start..]
            .find(TOKEN_END)
            .ok_or(DetachedStringError::MalformedToken)?;
        let id = usize::from_str_radix(&text[id_start..id_start + id_len], 16)
            .map_err(|_| DetachedStringError::MalformedToken)?;
        let end = id_start + id_len + TOKEN_END.len_utf8();
        found.push(TokenRef { marker, end, id });
        search = end;
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn long(label: &str) -> String {
        format!("{label}-{}", "x".repeat(64))
    }

    #[test]
    fn detaches_long_strings_and_shares_equal_content() {
        let mut detached = DetachedStrings::default();
        let mut value = json!({
            "role": "user",
            "content": long("a"),
            "parts": [long("a"), long("b"), "short", "{\"json\": true, \"padding\": \"................\"}", "                                        "],
        });
        let original = value.clone();
        detached.detach(&mut value);
        assert_eq!(value["role"], "user");
        assert_eq!(value["parts"][2], "short");
        assert_eq!(value["parts"][3], original["parts"][3]);
        assert_eq!(value["parts"][4], original["parts"][4]);
        assert_eq!(value["content"], value["parts"][0]);
        assert_ne!(value["content"], value["parts"][1]);
        assert!(value["content"]
            .as_str()
            .unwrap()
            .starts_with("a-xxxxxxxxxxxxxx"));
        assert!(value["content"].as_str().unwrap().len() < 32);
        detached.attach(&mut value).unwrap();
        assert_eq!(value, original);
    }

    #[test]
    fn attach_moves_the_last_use_and_clones_earlier_uses() {
        let mut detached = DetachedStrings::default();
        let mut input = Value::String(long("moved"));
        let pointer = input.as_str().unwrap().as_ptr();
        detached.detach(&mut input);
        let token = input.clone();
        let mut output = json!([input.clone(), input]);
        detached.attach(&mut output).unwrap();
        assert_eq!(output[0], output[1]);
        assert_ne!(output[0].as_str().unwrap().as_ptr(), pointer);
        assert_eq!(output[1].as_str().unwrap().as_ptr(), pointer);
        assert_eq!(
            detached.attach(&mut token.clone()),
            Err(DetachedStringError::AlreadyAttached(0))
        );
    }

    #[test]
    fn attaches_tokens_embedded_in_larger_strings() {
        let mut detached = DetachedStrings::default();
        let mut value = Value::String(long("embedded"));
        detached.detach(&mut value);
        let token = value.as_str().unwrap();
        let text = format!("before {token} after {token}");
        assert_eq!(
            detached.attach_text_cloned(&text).unwrap(),
            format!("before {} after {}", long("embedded"), long("embedded"))
        );
        let mut output = Value::String(text);
        detached.attach(&mut output).unwrap();
        assert_eq!(
            output,
            Value::String(format!(
                "before {} after {}",
                long("embedded"),
                long("embedded")
            ))
        );
    }

    #[test]
    fn merge_rewrites_tokens_and_shares_content_across_arenas() {
        let mut left = DetachedStrings::default();
        let mut right = DetachedStrings::default();
        let mut left_value = json!([long("shared"), long("left")]);
        let mut right_value = json!([long("right"), long("shared")]);
        left.detach(&mut left_value);
        right.detach(&mut right_value);
        left.merge(right, [&mut right_value]).unwrap();
        assert_eq!(left_value[0], right_value[1]);
        let mut output = json!([left_value, right_value]);
        left.attach(&mut output).unwrap();
        assert_eq!(
            output,
            json!([
                [long("shared"), long("left")],
                [long("right"), long("shared")]
            ])
        );
    }

    #[test]
    fn strings_containing_the_token_marker_are_always_detached() {
        let mut detached = DetachedStrings::default();
        let original = format!("ab{TOKEN_START}cd");
        let mut value = Value::String(original.clone());
        detached.detach(&mut value);
        assert_eq!(value.as_str().unwrap().matches(TOKEN_START).count(), 1);
        detached.attach(&mut value).unwrap();
        assert_eq!(value, Value::String(original));
    }

    #[test]
    fn rejects_truncated_or_unknown_tokens() {
        let mut detached = DetachedStrings::default();
        let mut value = Value::String(long("truncated"));
        detached.detach(&mut value);
        let token = value.as_str().unwrap().to_string();
        let truncated = &token[..token.len() - TOKEN_END.len_utf8()];
        assert_eq!(
            detached.attach_text_cloned(truncated),
            Err(DetachedStringError::MalformedToken)
        );
        assert_eq!(
            detached.attach_text_cloned(&format!("{TOKEN_START}ff{TOKEN_END}")),
            Err(DetachedStringError::MalformedToken)
        );
        assert_eq!(
            detached.attach_text_cloned(&token[2..]),
            Err(DetachedStringError::MalformedToken)
        );
    }
}
