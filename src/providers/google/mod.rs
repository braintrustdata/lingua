// Google AI Generative Language API types
// Generated from official protobuf files

pub mod generated;

// Re-export the most commonly used Google AI types for convenience
pub use generated::{
    safety_setting::HarmBlockThreshold, Candidate, Content, FunctionDeclaration,
    GenerateContentRequest, GenerateContentResponse, GenerationConfig, Part, SafetySetting, Tool,
};

// Type aliases for convenience
pub type SafetySettings = Vec<SafetySetting>;
