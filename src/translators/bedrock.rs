/*!
Bedrock translator implementation.
*/

use crate::providers::bedrock::request::{
    BedrockContentBlock, BedrockConversationRole, BedrockMessage, BedrockSystemContentBlock,
    ConverseRequest,
};
use crate::providers::bedrock::response::{BedrockOutputContentBlock, ConverseResponse};
use crate::translators::{TranslationResult, Translator};
use crate::universal::{SimpleMessage, SimpleRole};

/// Bedrock translator for simple messages
pub struct BedrockTranslator;

impl Translator<ConverseRequest, ConverseResponse> for BedrockTranslator {
    fn to_provider_request(messages: Vec<SimpleMessage>) -> TranslationResult<ConverseRequest> {
        let bedrock_messages = messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    SimpleRole::User => BedrockConversationRole::User,
                    SimpleRole::Assistant => BedrockConversationRole::Assistant,
                };

                BedrockMessage {
                    role,
                    content: vec![BedrockContentBlock::Text { text: msg.content }],
                }
            })
            .collect();

        let request = ConverseRequest {
            model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(), // Default model
            messages: bedrock_messages,
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        Ok(request)
    }

    fn from_provider_response(response: ConverseResponse) -> TranslationResult<Vec<SimpleMessage>> {
        let content = response
            .output
            .message
            .content
            .into_iter()
            .filter_map(|block| match block {
                BedrockOutputContentBlock::Text { text } => Some(text),
                BedrockOutputContentBlock::ToolUse { .. } => None, // Skip tool use for simple messages
            })
            .collect::<Vec<_>>()
            .join("");

        let simple_message = SimpleMessage {
            role: SimpleRole::Assistant, // Bedrock responses are always assistant messages
            content,
        };

        Ok(vec![simple_message])
    }
}

/// Convert simple messages to Bedrock format (convenience function)
pub fn to_bedrock_format(messages: Vec<SimpleMessage>) -> TranslationResult<ConverseRequest> {
    BedrockTranslator::to_provider_request(messages)
}

/// Convert Bedrock response to simple messages (convenience function)
pub fn from_bedrock_response(response: ConverseResponse) -> TranslationResult<Vec<SimpleMessage>> {
    BedrockTranslator::from_provider_response(response)
}

/// Create a Bedrock request with specific model
pub fn to_bedrock_format_with_model(
    messages: Vec<SimpleMessage>,
    model_id: &str,
) -> TranslationResult<ConverseRequest> {
    let mut request = BedrockTranslator::to_provider_request(messages)?;
    request.model_id = model_id.to_string();
    Ok(request)
}

/// Create a Bedrock request with system message
pub fn to_bedrock_format_with_system(
    messages: Vec<SimpleMessage>,
    system_message: &str,
) -> TranslationResult<ConverseRequest> {
    let mut request = BedrockTranslator::to_provider_request(messages)?;
    request.system = Some(vec![BedrockSystemContentBlock {
        text: system_message.to_string(),
    }]);
    Ok(request)
}
