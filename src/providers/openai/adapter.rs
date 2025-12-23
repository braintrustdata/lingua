/*!
OpenAI provider adapter for Chat Completions API.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{
    collect_extras, insert_opt_bool, insert_opt_f64, insert_opt_i64, insert_opt_value,
    ProviderAdapter,
};
use crate::processing::transform::TransformError;
use crate::providers::openai::generated::{
    ChatCompletionRequestMessage, ChatCompletionResponseMessage, CreateChatCompletionRequestClass,
};
use crate::providers::openai::try_parse_openai;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalUsage,
};

/// Known request fields for OpenAI Chat Completions API.
/// These are fields extracted into UniversalRequest/UniversalParams.
/// Fields not in this list go into `extras` for passthrough.
const OPENAI_KNOWN_KEYS: &[&str] = &[
    "model",
    "messages",
    "temperature",
    "top_p",
    "max_tokens",
    "max_completion_tokens",
    "stop",
    "tools",
    "tool_choice",
    "response_format",
    "seed",
    "presence_penalty",
    "frequency_penalty",
    "stream",
    // OpenAI-specific fields (not in UniversalParams) go to extras:
    // stream_options, n, logprobs, top_logprobs, logit_bias,
    // user, store, metadata, parallel_tool_calls, service_tier
];

/// Adapter for OpenAI Chat Completions API.
pub struct OpenAIAdapter;

impl ProviderAdapter for OpenAIAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    fn directory_name(&self) -> &'static str {
        "chat-completions"
    }

    fn display_name(&self) -> &'static str {
        "ChatCompletions"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_openai(payload).is_ok()
    }

    fn request_to_universal(&self, payload: &Value) -> Result<UniversalRequest, TransformError> {
        let request: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let params = UniversalParams {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None, // OpenAI doesn't support top_k
            max_tokens: request.max_tokens.or(request.max_completion_tokens),
            stop: request.stop.and_then(|s| serde_json::to_value(s).ok()),
            tools: request.tools.and_then(|t| serde_json::to_value(t).ok()),
            tool_choice: request.tool_choice.and_then(|t| serde_json::to_value(t).ok()),
            response_format: request.response_format.and_then(|r| serde_json::to_value(r).ok()),
            seed: request.seed,
            presence_penalty: request.presence_penalty,
            frequency_penalty: request.frequency_penalty,
            stream: request.stream,
        };

        Ok(UniversalRequest {
            model: Some(request.model),
            messages,
            params,
            extras: collect_extras(payload, OPENAI_KNOWN_KEYS),
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::OpenAI,
            reason: "missing model".to_string(),
        })?;

        let openai_messages: Vec<ChatCompletionRequestMessage> =
            <Vec<ChatCompletionRequestMessage> as TryFromLLM<Vec<Message>>>::try_from(
                req.messages.clone(),
            )
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert(
            "messages".into(),
            serde_json::to_value(openai_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
        );

        // Insert params
        insert_opt_f64(&mut obj, "temperature", req.params.temperature);
        insert_opt_f64(&mut obj, "top_p", req.params.top_p);
        insert_opt_i64(&mut obj, "max_tokens", req.params.max_tokens);
        insert_opt_value(&mut obj, "stop", req.params.stop.clone());
        insert_opt_value(&mut obj, "tools", req.params.tools.clone());
        insert_opt_value(&mut obj, "tool_choice", req.params.tool_choice.clone());
        insert_opt_value(&mut obj, "response_format", req.params.response_format.clone());
        insert_opt_i64(&mut obj, "seed", req.params.seed);
        insert_opt_f64(&mut obj, "presence_penalty", req.params.presence_penalty);
        insert_opt_f64(&mut obj, "frequency_penalty", req.params.frequency_penalty);
        insert_opt_bool(&mut obj, "stream", req.params.stream);

        // Merge extras (provider-specific fields)
        for (k, v) in &req.extras {
            obj.insert(k.clone(), v.clone());
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // OpenAI doesn't require any specific defaults
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // OpenAI chat completion response has choices[].message and object="chat.completion"
        payload.get("choices").and_then(Value::as_array).is_some()
            && payload
                .get("object")
                .and_then(Value::as_str)
                .is_some_and(|o| o == "chat.completion")
    }

    fn response_to_universal(&self, payload: &Value) -> Result<UniversalResponse, TransformError> {
        let choices = payload
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| TransformError::ToUniversalFailed("missing choices".to_string()))?;

        let mut messages = Vec::new();
        let mut finish_reason = None;

        for choice in choices {
            if let Some(msg_val) = choice.get("message") {
                let response_msg: ChatCompletionResponseMessage =
                    serde_json::from_value(msg_val.clone())
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                let universal =
                    <Message as TryFromLLM<&ChatCompletionResponseMessage>>::try_from(&response_msg)
                        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
                messages.push(universal);
            }

            // Get finish_reason from first choice
            if finish_reason.is_none() {
                if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                    finish_reason = Some(FinishReason::from_str(reason));
                }
            }
        }

        let usage = payload.get("usage").map(|u| UniversalUsage {
            input_tokens: u.get("prompt_tokens").and_then(Value::as_i64),
            output_tokens: u.get("completion_tokens").and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload
                .get("model")
                .and_then(Value::as_str)
                .map(String::from),
            messages,
            usage,
            finish_reason,
            extras: Map::new(), // TODO: preserve extras if needed
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        let finish_reason = self
            .map_finish_reason(resp.finish_reason.as_ref())
            .unwrap_or_else(|| "stop".to_string());

        let choices: Vec<Value> = resp
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let response_msg =
                    <ChatCompletionResponseMessage as TryFromLLM<&Message>>::try_from(msg)
                        .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

                let message_value = serde_json::to_value(&response_msg)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "index": i,
                    "message": message_value,
                    "finish_reason": finish_reason
                }))
            })
            .collect::<Result<Vec<_>, TransformError>>()?;

        let usage = resp.usage.as_ref().map(|u| {
            let input = u.input_tokens.unwrap_or(0);
            let output = u.output_tokens.unwrap_or(0);
            serde_json::json!({
                "prompt_tokens": input,
                "completion_tokens": output,
                "total_tokens": input + output
            })
        });

        let mut obj = serde_json::json!({
            "id": resp.extras.get("id").and_then(Value::as_str).unwrap_or("transformed"),
            "object": "chat.completion",
            "created": resp.extras.get("created").and_then(Value::as_i64).unwrap_or(0),
            "model": resp.model.as_deref().unwrap_or("transformed"),
            "choices": choices
        });

        if let Some(usage_val) = usage {
            obj.as_object_mut().unwrap().insert("usage".into(), usage_val);
        }

        Ok(obj)
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
        reason.map(|r| match r {
            FinishReason::Stop => "stop".to_string(),
            FinishReason::Length => "length".to_string(),
            FinishReason::ToolCalls => "tool_calls".to_string(),
            FinishReason::ContentFilter => "content_filter".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_openai_detect_request() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_openai_passthrough() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        assert_eq!(universal.model, Some("gpt-4".to_string()));
        assert_eq!(universal.messages.len(), 1);

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("model").unwrap(), "gpt-4");
        assert!(reconstructed.get("messages").is_some());
    }

    #[test]
    fn test_openai_preserves_extras() {
        let adapter = OpenAIAdapter;
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "user": "test-user-123",
            "custom_field": "should_be_preserved"
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        assert!(universal.extras.contains_key("user"));
        assert!(universal.extras.contains_key("custom_field"));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("user").unwrap(), "test-user-123");
        assert_eq!(reconstructed.get("custom_field").unwrap(), "should_be_preserved");
    }
}
