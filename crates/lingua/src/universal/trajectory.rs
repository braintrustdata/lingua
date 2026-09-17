use crate::{
    universal::{AssistantContent, ToolContent, UserContent},
    Message, UniversalParams, UniversalUsage,
};
use serde_json::{Map, Value};
use std::vec::Vec;

pub struct Trajectory {
    pub scope: Vec<Scope>,
    pub agent: Agent,
    pub sections: Option<Vec<Section>>,
    pub turns: Vec<Turn>,
    pub metadata: Map<String, Value>,
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
    pub instructions: Option<UserContent>, // System prompt goes here
}

pub struct Section {
    pub name: String,
    pub description: String,
    pub step_ids: Vec<String>,
}

pub enum Step {
    Turn(Turn),
    Activity(Activity),
}

pub struct Turn {
    pub request_id: String,
    pub response_id: Option<String>,

    // Step definition
    pub request: Vec<Message>,
    pub response: Option<AgentResponse>,
    pub work: Vec<WorkStep>,
    pub model: Option<String>,
    pub params: Option<UniversalParams>,
}

pub struct AgentResponse {
    pub id: String,
    pub response: Option<AssistantContent>,

    pub usage: Option<UniversalUsage>,
    pub start_time: std::time::Instant,
    pub end_time: Option<std::time::Instant>,
}

pub struct ToolResult {
    pub id: String,
    pub content: ToolContent,

    pub start_time: std::time::Instant,
    pub end_time: Option<std::time::Instant>,
}

pub struct SupportingWork {
    pub id: String,
    pub work: Vec<Message>,
    pub usage: Option<UniversalUsage>,
    pub start_time: std::time::Instant,
    pub end_time: Option<std::time::Instant>,
}

pub enum WorkStep {
    AgentResponse(Box<AgentResponse>),
    ToolResult(Box<ToolResult>),
    SupportingWork(Box<SupportingWork>),
    SubAgent(Box<Trajectory>),
}
