// Google AI Generative Language API types
// Generated from official protobuf files

pub mod detect;
pub mod generated;

use crate::serde_json::Value;

// Re-export the most commonly used Google AI types for convenience
pub use generated::{
    safety_setting::HarmBlockThreshold, Candidate, Content, FunctionDeclaration,
    GenerateContentRequest, GenerateContentResponse, GenerationConfig, Part, SafetySetting, Tool,
};

// Type aliases for convenience
pub type SafetySettings = Vec<SafetySetting>;

/// Wrapper for Google AI payloads used in format detection.
///
/// Since Google's `GenerateContentRequest` is protobuf-generated without serde support,
/// this wrapper stores the validated raw JSON for Google payloads, enabling a simpler
/// API for format detection and routing.
#[derive(Debug, Clone)]
pub struct GooglePayload {
    /// The raw JSON payload (validated to be Google format)
    pub raw: Value,
    /// The model name extracted from the payload
    pub model: Option<String>,
}

impl GooglePayload {
    /// Create a new GooglePayload from a validated JSON value.
    pub fn new(raw: Value) -> Self {
        let model = raw.get("model").and_then(|v| v.as_str()).map(String::from);
        Self { raw, model }
    }

    /// Get the raw JSON value.
    pub fn into_value(self) -> Value {
        self.raw
    }
}
