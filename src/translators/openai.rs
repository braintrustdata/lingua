/*!
OpenAI translator implementation.
*/

use crate::universal::{SimpleMessage, SimpleRole};
use crate::providers::openai::{OpenAIChatCompletionRequest, OpenAIChatCompletionResponse, OpenAIMessage, OpenAIRole};
use crate::translators::{TranslationResult, Translator};

/// OpenAI translator for simple messages
pub struct OpenAITranslator;

impl Translator<OpenAIChatCompletionRequest, OpenAIChatCompletionResponse> for OpenAITranslator {
    fn to_provider_request(messages: Vec<SimpleMessage>) -> TranslationResult<OpenAIChatCompletionRequest> {
        let openai_messages = messages
            .into_iter()
            .map(|msg| OpenAIMessage {
                role: match msg.role {
                    SimpleRole::User => OpenAIRole::User,
                    SimpleRole::Assistant => OpenAIRole::Assistant,
                },
                content: msg.content,
            })
            .collect();

        Ok(OpenAIChatCompletionRequest {
            model: "gpt-4".to_string(), // Default model
            messages: openai_messages,
        })
    }

    fn from_provider_response(response: OpenAIChatCompletionResponse) -> TranslationResult<Vec<SimpleMessage>> {
        let messages = response
            .choices
            .into_iter()
            .map(|choice| SimpleMessage {
                role: match choice.message.role {
                    OpenAIRole::User => SimpleRole::User,
                    OpenAIRole::Assistant => SimpleRole::Assistant,
                    OpenAIRole::System => SimpleRole::Assistant, // Map system to assistant
                    OpenAIRole::Tool => SimpleRole::Assistant,   // Map tool to assistant
                },
                content: choice.message.content,
            })
            .collect();

        Ok(messages)
    }
}

/// Convert simple messages to OpenAI format (convenience function)
pub fn to_openai_format(messages: Vec<SimpleMessage>) -> TranslationResult<OpenAIChatCompletionRequest> {
    OpenAITranslator::to_provider_request(messages)
}

/// Convert OpenAI response to simple messages (convenience function)  
pub fn from_openai_response(response: OpenAIChatCompletionResponse) -> TranslationResult<Vec<SimpleMessage>> {
    OpenAITranslator::from_provider_response(response)
}