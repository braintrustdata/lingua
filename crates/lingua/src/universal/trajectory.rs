use crate::{
    universal::{AssistantContent, ToolContent, UserContent},
    Message, UniversalParams, UniversalUsage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;
use std::vec::Vec;
use ts_rs::TS;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Trajectory {
    pub scope: Vec<Scope>,
    pub agent: Agent,
    #[ts(optional)]
    pub sections: Option<Vec<Section>>,
    #[ts(optional)]
    pub findings: Option<Vec<Finding>>,
    pub turns: Vec<Turn>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Map<String, Value>,
    #[ts(optional)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scope {
    Span { id: String },
    Trace { trace_id: String },
    Thread { key: String, value: String },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Agent {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub version: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Map<String, Value>,
    #[ts(optional)]
    pub instructions: Option<UserContent>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Section {
    pub name: String,
    pub description: String,
    pub step_ids: Vec<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Finding {
    #[ts(optional)]
    pub title: Option<String>,
    #[ts(optional)]
    pub severity: Option<FindingSeverity>,
    #[ts(optional)]
    pub kind: Option<String>,
    #[ts(optional)]
    pub hypothesis: Option<String>,
    #[ts(optional)]
    pub root_cause: Option<String>,
    #[ts(optional)]
    pub next_steps: Option<Vec<String>>,
    #[ts(optional)]
    pub evidence: Option<Vec<FindingEvidence>>,
    #[ts(optional)]
    pub pattern_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FindingEvidence {
    pub step_id: String,
    #[ts(optional)]
    pub field: Option<EvidenceField>,
    #[ts(optional)]
    pub part: Option<EvidencePart>,
    #[ts(optional)]
    pub explanation: Option<String>,
    #[ts(optional)]
    pub quote: Option<String>,
    #[ts(optional)]
    pub highlight_terms: Option<Vec<String>>,
    #[ts(optional)]
    pub leading_text: Option<String>,
    #[ts(optional)]
    pub trailing_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceField {
    Input,
    Output,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePart {
    Reasoning,
    Arguments,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Turn {
    // Each of these is the id of the corresponding span
    pub request_id: String,
    #[ts(optional)]
    pub response_id: Option<String>,

    // Step definition
    #[ts(optional)]
    pub request: Option<Vec<Message>>,
    #[ts(optional)]
    pub response: Option<AgentResponse>,
    pub work: Vec<WorkStep>,

    #[ts(optional)]
    pub model: Option<String>,
    #[ts(optional)]
    pub params: Option<UniversalParams>,

    pub start_time: DateTime<Utc>,
    #[ts(optional)]
    pub end_time: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub compaction: Option<Compaction>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Compaction {
    pub id: String,
    #[ts(optional)]
    pub replaced_message_count: Option<usize>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentResponse {
    #[ts(optional)]
    pub response: Option<AssistantContent>,

    #[ts(optional)]
    pub usage: Option<UniversalUsage>,
    #[ts(optional)]
    pub start_time: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub end_time: Option<DateTime<Utc>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ToolResult {
    #[ts(optional, type = "unknown")]
    pub input: Option<Value>,
    #[ts(optional)]
    pub content: Option<ToolContent>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct LLMAnalysis {
    #[ts(optional)]
    pub work: Option<Vec<Message>>,
    #[ts(optional)]
    pub model: Option<String>,
    #[ts(optional)]
    pub params: Option<UniversalParams>,
    #[ts(optional)]
    pub usage: Option<UniversalUsage>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct WorkStep {
    pub id: String,        // span's id
    pub span_type: String, // span type
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional, type = "unknown")]
    pub error: Option<Value>,

    pub start_time: DateTime<Utc>,
    #[ts(optional)]
    pub end_time: Option<DateTime<Utc>>,

    pub work: Work,
    #[ts(optional)]
    pub sub_agent: Option<Box<Trajectory>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Work {
    AgentResponse(Box<AgentResponse>),
    ToolResult(Box<ToolResult>),
    #[serde(rename = "llm_analysis")]
    LLMAnalysis(Box<LLMAnalysis>),
}
