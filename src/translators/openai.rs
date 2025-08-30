/*!
OpenAI translator implementation.
*/

use crate::providers::openai::request::{
    ChatCompletionAssistantMessageParam, ChatCompletionUserMessageParam,
};
use crate::providers::openai::{
    ChatCompletion, ChatCompletionCreateParams, ChatCompletionMessageParam,
    MessageContentWithParts, MessageContentWithRefusal,
};
use crate::translators::{TranslationResult, Translator};
use crate::universal::{SimpleMessage, SimpleRole};

/// OpenAI translator for simple messages
pub struct OpenAITranslator;

impl Translator<ChatCompletionCreateParams, ChatCompletion> for OpenAITranslator {
    fn to_provider_request(
        messages: Vec<SimpleMessage>,
    ) -> TranslationResult<ChatCompletionCreateParams> {
        let openai_messages = messages
            .into_iter()
            .map(|msg| match msg.role {
                SimpleRole::User => {
                    ChatCompletionMessageParam::User(ChatCompletionUserMessageParam {
                        content: MessageContentWithParts::String(msg.content),
                        name: None,
                    })
                }
                SimpleRole::Assistant => {
                    ChatCompletionMessageParam::Assistant(ChatCompletionAssistantMessageParam {
                        content: Some(MessageContentWithRefusal::String(msg.content)),
                        audio: None,
                        function_call: None,
                        name: None,
                        refusal: None,
                        tool_calls: None,
                    })
                }
            })
            .collect();

        let params = ChatCompletionCreateParams {
            model: "gpt-4o".to_string(),
            messages: openai_messages,
            audio: None,
            frequency_penalty: None,
            function_call: None,
            functions: None,
            logit_bias: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            metadata: None,
            modalities: None,
            n: None,
            parallel_tool_calls: None,
            prediction: None,
            presence_penalty: None,
            prompt_cache_key: None,
            reasoning_effort: None,
            response_format: None,
            safety_identifier: None,
            seed: None,
            service_tier: None,
            stop: None,
            store: None,
            stream: None,
            stream_options: None,
            temperature: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            user: None,
            verbosity: None,
            web_search_options: None,
        };

        Ok(params)
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
pub fn to_openai_format(
    messages: Vec<SimpleMessage>,
) -> TranslationResult<ChatCompletionCreateParams> {
    OpenAITranslator::to_provider_request(messages)
}

/// Convert OpenAI response to simple messages (convenience function)  
pub fn from_openai_response(response: ChatCompletion) -> TranslationResult<Vec<SimpleMessage>> {
    OpenAITranslator::from_provider_response(response)
}
