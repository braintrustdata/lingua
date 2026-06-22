/*!
Typed helper views for Anthropic request conversion.

The Anthropic request boundary itself is `generated::CreateMessageParams`.
These views cover only partial maps where the generated request type is not the
right shape: provider extras and cross-provider detection guards.
*/

use crate::serde_json::{self, Value};
use serde::Deserialize;

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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct AnthropicOpenAiOnlyFieldsView {
    stream_options: Option<Value>,
    n: Option<Value>,
    logprobs: Option<Value>,
    top_logprobs: Option<Value>,
    logit_bias: Option<Value>,
    response_format: Option<Value>,
    functions: Option<Value>,
    function_call: Option<Value>,
    seed: Option<Value>,
    presence_penalty: Option<Value>,
    frequency_penalty: Option<Value>,
    store: Option<Value>,
    parallel_tool_calls: Option<Value>,
    stop: Option<Value>,
    reasoning_effort: Option<Value>,
    reasoning_enabled: Option<Value>,
    suffix_messages: Option<Value>,
    chat_template_kwargs: Option<Value>,
    max_completion_tokens: Option<Value>,
}

impl AnthropicOpenAiOnlyFieldsView {
    fn first_present(&self) -> Option<&'static str> {
        [
            ("stream_options", self.stream_options.is_some()),
            ("n", self.n.is_some()),
            ("logprobs", self.logprobs.is_some()),
            ("top_logprobs", self.top_logprobs.is_some()),
            ("logit_bias", self.logit_bias.is_some()),
            ("response_format", self.response_format.is_some()),
            ("functions", self.functions.is_some()),
            ("function_call", self.function_call.is_some()),
            ("seed", self.seed.is_some()),
            ("presence_penalty", self.presence_penalty.is_some()),
            ("frequency_penalty", self.frequency_penalty.is_some()),
            ("store", self.store.is_some()),
            ("parallel_tool_calls", self.parallel_tool_calls.is_some()),
            ("stop", self.stop.is_some()),
            ("reasoning_effort", self.reasoning_effort.is_some()),
            ("reasoning_enabled", self.reasoning_enabled.is_some()),
            ("suffix_messages", self.suffix_messages.is_some()),
            ("chat_template_kwargs", self.chat_template_kwargs.is_some()),
            (
                "max_completion_tokens",
                self.max_completion_tokens.is_some(),
            ),
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
