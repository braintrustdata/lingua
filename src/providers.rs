/*!
Provider-specific API type definitions.

This module contains types that exactly match each provider's API specifications.
*/

use serde::{Deserialize, Serialize};

//
// OpenAI API Types
//

/// OpenAI message role
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAIRole {
    System,
    User,
    Assistant,
    Tool,
}

/// OpenAI chat message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: OpenAIRole,
    pub content: String,
}

/// OpenAI chat completion request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
}

/// OpenAI chat completion response choice
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
}

/// OpenAI usage information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// OpenAI chat completion response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAIChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: Option<OpenAIUsage>,
}

//
// Anthropic API Types (TODO)
//

// TODO: Add Anthropic API types here

//
// Google Gemini API Types (TODO)
//

// TODO: Add Google Gemini API types here