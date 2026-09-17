use crate::{
    universal::{AssistantContent, ToolContent, UserContent},
    Message, UniversalParams, UniversalUsage,
};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use std::vec::Vec;

pub struct Trajectory {
    pub scope: Vec<Scope>,
    pub agent: Agent,
    pub sections: Option<Vec<Section>>,
    pub turns: Vec<Turn>,
    pub metadata: Map<String, Value>,
    pub version: Option<String>,
}

pub enum Scope {
    Span { id: String },
    Trace { trace_id: String },
    Thread { key: String, value: String },
}

pub struct Agent {
    pub name: Option<String>,
    pub version: Option<String>,
    pub metadata: Map<String, Value>,
    pub instructions: Option<UserContent>,
}

pub struct Section {
    pub name: String,
    pub description: String,
    pub step_ids: Vec<String>,
}

pub struct Turn {
    // Each of these is the id of the corresponding span
    pub request_id: String,
    pub response_id: Option<String>,

    // Step definition
    pub request: Option<Vec<Message>>,
    pub response: Option<AgentResponse>,
    pub work: Vec<WorkStep>,

    pub model: Option<String>,
    pub params: Option<UniversalParams>,

    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub compaction: Option<Compaction>,
}

pub struct Compaction {
    pub id: String,
    pub replaced_message_count: Option<usize>,
}

pub struct AgentResponse {
    pub response: Option<AssistantContent>,

    pub usage: Option<UniversalUsage>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

pub struct ToolResult {
    pub content: Option<ToolContent>,
}

pub struct LLMAnalysis {
    pub work: Option<Vec<Message>>,
    pub model: Option<String>,
    pub params: Option<UniversalParams>,
    pub usage: Option<UniversalUsage>,
}

pub struct WorkStep {
    pub id: String,        // span's id
    pub span_type: String, // span type
    pub name: Option<String>,
    pub error: Option<Value>,
    pub tool_call_id: Option<String>,

    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,

    pub work: Work,
}

pub enum Work {
    AgentResponse(Box<AgentResponse>),
    ToolResult(Box<ToolResult>),
    LLMAnalysis(Box<LLMAnalysis>),
    SubAgent(Box<Trajectory>),
}
