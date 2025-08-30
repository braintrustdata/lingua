// Google AI Generative Language API types
// Generated from official protobuf files

pub mod generated;

// Re-export the most commonly used Google AI types for convenience
pub use generated::{
    safety_setting::HarmBlockThreshold, Candidate, Content, GenerateContentRequest,
    GenerateContentResponse, GenerationConfig, HarmCategory, Part, SafetySetting,
};

// Type aliases for convenience
pub type SafetySettings = Vec<SafetySetting>;
