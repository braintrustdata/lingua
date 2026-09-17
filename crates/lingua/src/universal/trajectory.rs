use crate::{
    universal::{AssistantContent, UserContent},
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

pub struct Turn {
    pub id: String, // Best practice is to use the id of the first span
    pub kind: StepKind,

    // Step definition
    pub request: Vec<Message>,
    pub work: Vec<WorkStep>,
    pub response: Option<AssistantContent>,
    pub model: Option<String>,
    pub params: Option<UniversalParams>,

    // Metrics
    pub usage: Option<UniversalUsage>,
    pub start_time: std::time::Instant,
    pub end_time: Option<std::time::Instant>,
}

pub enum StepKind {
    Agent,
    SupportingWork, // Can u come up with a better name plz
    Custom(String),
}

pub enum WorkStep {
    Step(Box<Step>),
    Trajectory(Box<Trajectory>),
}
