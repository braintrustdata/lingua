// Google AI Generative Language API types
// Generated from Discovery REST API spec

pub mod adapter;
pub mod capabilities;
pub mod convert;
pub mod detect;
pub mod generated;
pub mod params;

#[cfg(test)]
pub mod test_google;

// Re-export adapter
pub use adapter::GoogleAdapter;

// Re-export capabilities
pub use capabilities::{GoogleCapabilities, GoogleThinkingStyle};

// Re-export detection functions
pub use detect::{try_parse_google, DetectionError};

// Re-export conversion functions
pub use convert::{google_to_universal, universal_to_google};

// Re-export the most commonly used Google AI types for convenience
pub use generated::{
    Candidate, Content, FunctionDeclaration, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, Part, SafetySetting, Threshold, Tool,
};

// Type aliases for convenience
pub type SafetySettings = Vec<SafetySetting>;

/// Returns true if the model ID represents a Vertex AI model.
///
/// Vertex models use the `publishers/` prefix
/// (e.g. `publishers/google/models/gemini-2.5-flash-preview-04-17`).
pub fn is_vertex_model(model: &str) -> bool {
    model.starts_with("publishers/")
}

/// Returns true if the model is a Google model hosted on Vertex AI.
///
/// These models have IDs starting with `publishers/google/`
/// (e.g. `publishers/google/models/gemini-2.5-flash`).
pub fn is_vertex_google_model(model: &str) -> bool {
    model.starts_with("publishers/google/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_vertex_google_model_matches_publishers_google_prefix() {
        assert!(is_vertex_google_model(
            "publishers/google/models/gemini-2.5-flash"
        ));
        assert!(is_vertex_google_model(
            "publishers/google/models/gemini-pro"
        ));
        assert!(is_vertex_google_model("publishers/google/something-else"));
    }

    #[test]
    fn is_vertex_google_model_rejects_other_publishers() {
        assert!(!is_vertex_google_model(
            "publishers/anthropic/models/claude-haiku-4-5"
        ));
        assert!(!is_vertex_google_model("publishers/meta/models/llama3"));
        assert!(!is_vertex_google_model("gemini-2.5-flash"));
        assert!(!is_vertex_google_model("publishers/"));
        assert!(!is_vertex_google_model(""));
    }
}
