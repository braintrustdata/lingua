/*!
Typed helper views for Anthropic request conversion.

The Anthropic request boundary itself is `generated::CreateMessageParams`.
These views cover only partial maps where the generated request type is not the
right shape: provider extras and cross-provider detection guards.
*/

use crate::serde_json::Value;
use serde::Deserialize;

use super::generated::{JsonOutputFormat, OutputConfig};

#[derive(Deserialize)]
pub(crate) struct AnthropicMessageWithDirectives {
    pub output_config: Option<OutputConfig>,
}

#[derive(Deserialize)]
pub(crate) struct AnthropicMessagesView {
    pub messages: Vec<AnthropicMessageWithDirectives>,
}

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

const OPENAI_ONLY_FIELDS: &[&str] = &[
    "stream_options",
    "n",
    "logprobs",
    "top_logprobs",
    "logit_bias",
    "response_format",
    "functions",
    "function_call",
    "seed",
    "presence_penalty",
    "frequency_penalty",
    "store",
    "parallel_tool_calls",
    "stop",
    "reasoning_effort",
    "reasoning_enabled",
    "suffix_messages",
    "chat_template_kwargs",
    "max_completion_tokens",
];

pub(crate) fn first_openai_only_field(payload: &Value) -> Option<&'static str> {
    let Value::Object(fields) = payload else {
        return None;
    };
    OPENAI_ONLY_FIELDS
        .iter()
        .copied()
        .find(|field| fields.contains_key(*field))
}
