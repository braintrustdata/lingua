/*!
OpenAI translator implementation.
*/

use crate::universal::{SimpleMessage, SimpleRole};
use crate::providers::openai::{ChatCompletionCreateParams, ChatCompletion, ChatCompletionMessageParam, MessageContent};
use crate::translators::{TranslationResult, Translator};

/// OpenAI translator for simple messages
pub struct OpenAITranslator;

impl Translator<ChatCompletionCreateParams, ChatCompletion> for OpenAITranslator {
    fn to_provider_request(messages: Vec<SimpleMessage>) -> TranslationResult<ChatCompletionCreateParams> {
        let openai_messages = messages
            .into_iter()
            .map(|msg| match msg.role {
                SimpleRole::User => ChatCompletionMessageParam::User {
                    content: MessageContent::Text(msg.content),
                    name: None,
                },
                SimpleRole::Assistant => ChatCompletionMessageParam::Assistant {
                    content: Some(MessageContent::Text(msg.content)),
                    audio: None,
                    function_call: None,
                    name: None,
                    refusal: None,
                    tool_calls: None,
                },
            })
            .collect();

        Ok(ChatCompletionCreateParams {
            model: "gpt-4o".to_string(),
            messages: openai_messages,
            audio: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            metadata: None,
            modalities: None,
            n: None,
            parallel_tool_calls: None,
            presence_penalty: None,
            reasoning_effort: None,
            response_format: None,
            seed: None,
            service_tier: None,
            stop: None,
            stream: None,
            temperature: None,
            tool_choice: None,
            tools: None,
            top_p: None,
            user: None,
        })
    }

    fn from_provider_response(response: ChatCompletion) -> TranslationResult<Vec<SimpleMessage>> {
        let messages = response
            .choices
            .into_iter()
            .map(|choice| SimpleMessage {
                role: SimpleRole::Assistant, // OpenAI responses are always assistant messages
                content: choice.message.content.unwrap_or_default(),
            })
            .collect();

        Ok(messages)
    }
}

/// Convert simple messages to OpenAI format (convenience function)
pub fn to_openai_format(messages: Vec<SimpleMessage>) -> TranslationResult<ChatCompletionCreateParams> {
    OpenAITranslator::to_provider_request(messages)
}

/// Convert OpenAI response to simple messages (convenience function)  
pub fn from_openai_response(response: ChatCompletion) -> TranslationResult<Vec<SimpleMessage>> {
    OpenAITranslator::from_provider_response(response)
}