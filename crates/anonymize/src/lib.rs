use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

const DEFAULT_PRESERVE_KEYS: &[&str] = &["role", "type"];
const DEFAULT_TOKEN_PREFIX: &str = "anon";

/// Options controlling JSON anonymization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymizeOptions {
    /// When true, anonymize every non-empty string except `metadata.model` paths.
    pub all_strings: bool,
    /// Keys whose string values are preserved when `all_strings` is false.
    pub preserve_keys: HashSet<String>,
    /// Prefix used when generating replacement tokens.
    pub token_prefix: String,
}

impl Default for AnonymizeOptions {
    fn default() -> Self {
        Self {
            all_strings: false,
            preserve_keys: DEFAULT_PRESERVE_KEYS
                .iter()
                .map(|key| (*key).to_string())
                .collect(),
            token_prefix: DEFAULT_TOKEN_PREFIX.to_string(),
        }
    }
}

impl AnonymizeOptions {
    /// Create default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether every string should be anonymized.
    pub fn with_all_strings(mut self, all_strings: bool) -> Self {
        self.all_strings = all_strings;
        self
    }

    /// Set keys whose string values should be preserved when `all_strings` is false.
    pub fn with_preserve_keys<I, S>(mut self, preserve_keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.preserve_keys = preserve_keys.into_iter().map(Into::into).collect();
        self
    }

    /// Set the token prefix.
    pub fn with_token_prefix<S: Into<String>>(mut self, token_prefix: S) -> Self {
        self.token_prefix = token_prefix.into();
        self
    }
}

/// Result returned by [`anonymize_json_value`].
#[derive(Debug, Clone, PartialEq)]
pub struct AnonymizeResult {
    pub value: Value,
    pub replaced_string_count: usize,
    pub unique_replacement_count: usize,
}

/// Identifies what is being passed to an anonymization filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnonymizeFilterKind {
    /// An object key. The filter receives the key as a JSON string value.
    Key,
    /// A JSON field value.
    Value,
}

/// Context passed to an anonymization filter.
#[derive(Debug, Clone, Copy)]
pub struct AnonymizeFilterContext<'a> {
    pub kind: AnonymizeFilterKind,
    /// Path to the key or value in the original JSON tree.
    pub path: &'a [String],
    /// Current object key for value filters. For key filters, this is the key being filtered.
    pub current_key: Option<&'a str>,
}

/// Optional extra anonymization filter.
///
/// The filter is called for keys and for field values that were not already anonymized by the
/// built-in token replacement logic. Return `Some(value)` to replace the key or field, or `None` to
/// keep it unchanged. Key replacements must be JSON strings; non-string key replacements are ignored.
pub type AnonymizeFilter<'filter> =
    dyn for<'a> FnMut(AnonymizeFilterContext<'a>, &'a Value) -> Option<Value> + 'filter;

struct Walker<'filter> {
    all_strings: bool,
    preserve_keys: HashSet<String>,
    token_prefix: String,
    replacements: HashMap<String, String>,
    next_token: usize,
    replaced_string_count: usize,
    filter: Option<&'filter mut AnonymizeFilter<'filter>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Scope {
    within_content: bool,
    within_metadata: bool,
    within_context: bool,
    within_output: bool,
}

/// Anonymize a JSON value using default options.
pub fn anonymize_json_value(input: Value) -> AnonymizeResult {
    anonymize_json_value_with_options(input, AnonymizeOptions::default())
}

/// Anonymize a JSON value using explicit options.
pub fn anonymize_json_value_with_options(
    input: Value,
    options: AnonymizeOptions,
) -> AnonymizeResult {
    anonymize_json_value_with_options_and_filter(input, options, None)
}

/// Anonymize a JSON value using explicit options and an optional extra filter.
pub fn anonymize_json_value_with_options_and_filter<'filter>(
    input: Value,
    options: AnonymizeOptions,
    filter: Option<&'filter mut AnonymizeFilter<'filter>>,
) -> AnonymizeResult {
    let mut walker = Walker::new(options, filter);
    let value = walker.walk(input, &mut Vec::new(), None, Scope::default());
    AnonymizeResult {
        value,
        replaced_string_count: walker.replaced_string_count,
        unique_replacement_count: walker.replacements.len(),
    }
}

impl<'filter> Walker<'filter> {
    fn new(
        options: AnonymizeOptions,
        filter: Option<&'filter mut AnonymizeFilter<'filter>>,
    ) -> Self {
        Self {
            all_strings: options.all_strings,
            preserve_keys: normalize_key_set(options.preserve_keys),
            token_prefix: options.token_prefix,
            replacements: HashMap::new(),
            next_token: 1,
            replaced_string_count: 0,
            filter,
        }
    }

    fn walk(
        &mut self,
        value: Value,
        path: &mut Vec<String>,
        current_key: Option<&str>,
        scope: Scope,
    ) -> Value {
        match value {
            Value::String(value) => self.walk_string(value, path, current_key, scope),
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| {
                        path.push(index.to_string());
                        let anonymized = self.walk(item, path, current_key, scope);
                        path.pop();
                        anonymized
                    })
                    .collect(),
            ),
            Value::Object(object) => self.walk_object(object, path, scope),
            other => self.filter_value(other, path, current_key),
        }
    }

    fn walk_string(
        &mut self,
        value: String,
        path: &[String],
        current_key: Option<&str>,
        scope: Scope,
    ) -> Value {
        if is_metadata_model_path(path) {
            return self.filter_value(Value::String(value), path, current_key);
        }

        let is_tool_arguments = is_arguments_key(current_key);
        if !self.all_strings
            && !scope.within_content
            && !scope.within_metadata
            && !scope.within_context
            && !scope.within_output
            && !is_tool_arguments
        {
            return self.filter_value(Value::String(value), path, current_key);
        }

        if is_tool_arguments {
            if let Some(parsed_arguments) = try_parse_json_string(&value) {
                if parsed_arguments.is_array() || parsed_arguments.is_object() {
                    let mut parsed_path = path.to_vec();
                    parsed_path.push("<parsed_json>".to_string());
                    let mut parsed_scope = scope;
                    parsed_scope.within_content = true;
                    let anonymized_arguments =
                        self.walk(parsed_arguments, &mut parsed_path, None, parsed_scope);
                    return Value::String(
                        serde_json::to_string(&anonymized_arguments)
                            .expect("serializing serde_json::Value to a string should not fail"),
                    );
                }
            }
        }

        let (value, was_anonymized) = self.replace_string(value, current_key);
        if was_anonymized {
            Value::String(value)
        } else {
            self.filter_value(Value::String(value), path, current_key)
        }
    }

    fn walk_object(
        &mut self,
        object: Map<String, Value>,
        path: &mut Vec<String>,
        scope: Scope,
    ) -> Value {
        let mut out = Map::new();

        for (key, nested) in object {
            let lower_key = key.to_lowercase();
            let child_scope = Scope {
                within_content: scope.within_content || lower_key == "content",
                within_metadata: scope.within_metadata || lower_key.starts_with("metadata"),
                within_context: scope.within_context || lower_key == "context",
                within_output: scope.within_output || lower_key == "output",
            };

            // Remove metadata.prompt entirely: prompt key names/shape can leak sensitive context.
            if child_scope.within_metadata && lower_key == "prompt" {
                continue;
            }

            path.push(key.clone());
            let filtered_key = self.filter_key(key.clone(), path);
            let anonymized = self.walk(nested, path, Some(&key), child_scope);
            path.pop();
            out.insert(filtered_key, anonymized);
        }

        Value::Object(out)
    }

    fn replace_string(&mut self, value: String, current_key: Option<&str>) -> (String, bool) {
        if value.is_empty() {
            return (value, false);
        }

        if !self.all_strings {
            if let Some(current_key) = current_key {
                if self.preserve_keys.contains(&current_key.to_lowercase()) {
                    return (value, false);
                }
            }
        }

        if let Some(token) = self.replacements.get(&value) {
            self.replaced_string_count += 1;
            return (token.clone(), true);
        }

        let token = format!("{}_{}", self.token_prefix, self.next_token);
        self.next_token += 1;
        self.replacements.insert(value, token.clone());
        self.replaced_string_count += 1;
        (token, true)
    }

    fn filter_key(&mut self, key: String, path: &[String]) -> String {
        if is_generated_token(&key, &self.token_prefix) {
            return key;
        }

        let key_value = Value::String(key.clone());
        match self.apply_filter(
            AnonymizeFilterContext {
                kind: AnonymizeFilterKind::Key,
                path,
                current_key: Some(&key),
            },
            &key_value,
        ) {
            Some(Value::String(filtered_key)) => filtered_key,
            _ => key,
        }
    }

    fn filter_value(&mut self, value: Value, path: &[String], current_key: Option<&str>) -> Value {
        if value
            .as_str()
            .is_some_and(|value| is_generated_token(value, &self.token_prefix))
        {
            return value;
        }

        self.apply_filter(
            AnonymizeFilterContext {
                kind: AnonymizeFilterKind::Value,
                path,
                current_key,
            },
            &value,
        )
        .unwrap_or(value)
    }

    fn apply_filter(
        &mut self,
        context: AnonymizeFilterContext<'_>,
        value: &Value,
    ) -> Option<Value> {
        self.filter
            .as_mut()
            .and_then(|filter| filter(context, value))
    }
}

fn is_metadata_model_path(path: &[String]) -> bool {
    let n = path.len();
    n >= 2
        && path[n - 2].eq_ignore_ascii_case("metadata")
        && path[n - 1].eq_ignore_ascii_case("model")
}

fn normalize_key_set(keys: HashSet<String>) -> HashSet<String> {
    keys.into_iter().map(|key| key.to_lowercase()).collect()
}

fn is_arguments_key(current_key: Option<&str>) -> bool {
    current_key.is_some_and(|key| key.eq_ignore_ascii_case("arguments"))
}

fn is_generated_token(value: &str, token_prefix: &str) -> bool {
    let Some(suffix) = value.strip_prefix(token_prefix) else {
        return false;
    };
    let Some(number) = suffix.strip_prefix('_') else {
        return false;
    };
    !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit())
}

fn try_parse_json_string(value: &str) -> Option<Value> {
    serde_json::from_str(value).ok()
}
