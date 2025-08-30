// Google AI Generative Language API types
// Generated from official protobuf files

pub mod generated;

// Re-export the most commonly used Google AI types for convenience
pub use generated::{
    Candidate, Content, GenerateContentRequest, GenerateContentResponse, GenerationConfig,
    HarmBlockThreshold, HarmCategory, Part, SafetySetting, SafetySettings,
};
