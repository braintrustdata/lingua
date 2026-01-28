// Google AI Generative Language API types
// Generated from official protobuf files

pub mod adapter;
pub mod convert;
pub mod detect;
pub mod generated;
pub mod params;

#[cfg(test)]
pub mod test_google;

// Re-export adapter
pub use adapter::GoogleAdapter;

// Re-export detection functions
pub use detect::{try_parse_google, DetectionError};

// Re-export conversion functions
pub use convert::{google_to_universal, universal_to_google};

// Re-export the most commonly used Google AI types for convenience
pub use generated::{
    safety_setting::HarmBlockThreshold, Candidate, Content, FunctionDeclaration,
    GenerateContentRequest, GenerateContentResponse, GenerationConfig, Part, SafetySetting, Tool,
};

// Type aliases for convenience
pub type SafetySettings = Vec<SafetySetting>;
