#![allow(dead_code)]
use super::convert::{
    ChatCompletionRequestMessageContentPartExt, ChatCompletionRequestReasoning, ReasoningSignature,
};
use super::generated as openai;
use super::generated::*;
use crate::processing::borrowed::BorrowText;
use crate::serde_json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputItem<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(borrow)]
    pub content: Option<InputItemContent<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<MessagePhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<InputItemRole>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_item_type: Option<InputItemType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(borrow)]
    pub id: Option<BorrowText<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<ResultElement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<InputItemAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<ComputerAction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_safety_checks: Option<Vec<ComputerCallSafetyCheckParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_safety_checks: Option<Vec<ComputerCallSafetyCheckParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Output>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arguments>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<InputItemDirectToolCallCaller>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ToolSearchExecutionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<InputItemTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(borrow)]
    pub encrypted_content: Option<BorrowText<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(borrow)]
    pub summary: Option<Vec<SummaryText<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<CodeInterpreterOutput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<InputItemLocalEnvironmentParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<PurpleApplyPatchOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approve: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<BetaAgentTagParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputItemContent<'a> {
    #[serde(borrow)]
    InputContentArray(Vec<ContentInputItemContentList<'a>>),
    #[serde(borrow)]
    String(BorrowText<'a>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentInputItemContentList<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_breakpoint: Option<InputItemContentListPromptCacheBreakpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(borrow)]
    pub text: Option<BorrowText<'a>>,
    #[serde(rename = "type")]
    pub input_content_type: HilariousType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<DetailEnum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<LogProbability>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio: Option<InputAudio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryText<'a> {
    #[serde(borrow)]
    pub text: BorrowText<'a>,
    #[serde(rename = "type")]
    pub summary_text_type: SummaryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageExt<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(borrow)]
    pub content: Option<ChatCompletionRequestMessageContentExt<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub role: openai::ChatCompletionRequestMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<openai::ChatCompletionRequestMessageAudio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<openai::ChatCompletionRequestMessageFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<openai::ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
    #[serde(alias = "reasoning_content", skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ChatCompletionRequestReasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_signature: Option<ReasoningSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestMessageContentExt<'a> {
    Parts(Vec<ChatCompletionRequestMessageContentPartExt>),
    #[serde(borrow)]
    String(BorrowText<'a>),
}
