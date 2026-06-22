/*!
Typed helper views for Anthropic request conversion.

The Anthropic request boundary itself is `generated::CreateMessageParams`.
These views cover only partial maps where the generated request type is not the
right shape: provider extras and cross-provider detection guards.
*/

use crate::serde_json::{self, Value};
use serde::{de::IgnoredAny, Deserialize};

use super::generated::JsonOutputFormat;

/// Typed view over `UniversalParams.extras[Anthropic]` used during universal
/// -> Anthropic reconstruction.
///
/// This is intentionally partial and preserves raw JSON values from extras.
/// It is not the full request type because extras may only contain a subset.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnthropicExtrasView {
    pub messages: Option<Value>,
    pub system: Option<Value>,
    pub temperature: Option<Value>,
    pub tools: Option<Value>,
    pub tool_choice: Option<Value>,
    pub output_config: Option<Value>,
    pub output_format: Option<JsonOutputFormat>,
    pub thinking: Option<Value>,
    pub metadata: Option<Value>,
}

fn deserialize_present<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    IgnoredAny::deserialize(deserializer)?;
    Ok(true)
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct AnthropicOpenAiOnlyFieldsView {
    #[serde(default, deserialize_with = "deserialize_present")]
    stream_options: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    n: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    logprobs: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    top_logprobs: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    logit_bias: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    response_format: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    functions: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    function_call: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    seed: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    presence_penalty: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    frequency_penalty: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    store: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    parallel_tool_calls: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    stop: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    reasoning_effort: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    reasoning_enabled: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    suffix_messages: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    chat_template_kwargs: bool,
    #[serde(default, deserialize_with = "deserialize_present")]
    max_completion_tokens: bool,
}

impl AnthropicOpenAiOnlyFieldsView {
    fn first_present(&self) -> Option<&'static str> {
        [
            ("stream_options", self.stream_options),
            ("n", self.n),
            ("logprobs", self.logprobs),
            ("top_logprobs", self.top_logprobs),
            ("logit_bias", self.logit_bias),
            ("response_format", self.response_format),
            ("functions", self.functions),
            ("function_call", self.function_call),
            ("seed", self.seed),
            ("presence_penalty", self.presence_penalty),
            ("frequency_penalty", self.frequency_penalty),
            ("store", self.store),
            ("parallel_tool_calls", self.parallel_tool_calls),
            ("stop", self.stop),
            ("reasoning_effort", self.reasoning_effort),
            ("reasoning_enabled", self.reasoning_enabled),
            ("suffix_messages", self.suffix_messages),
            ("chat_template_kwargs", self.chat_template_kwargs),
            ("max_completion_tokens", self.max_completion_tokens),
        ]
        .into_iter()
        .find_map(|(field, present)| present.then_some(field))
    }
}

pub(crate) fn first_openai_only_field(payload: &Value) -> Result<Option<&'static str>, String> {
    serde_json::from_value::<AnthropicOpenAiOnlyFieldsView>(payload.clone())
        .map(|view| view.first_present())
        .map_err(|e| e.to_string())
}
