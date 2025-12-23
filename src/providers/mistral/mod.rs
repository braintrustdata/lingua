/*!
Mistral AI provider module.

Mistral uses an OpenAI-compatible API format with some provider-specific
extensions like `safe_prompt`. This module provides format detection for
Mistral payloads.

Note: Mistral payloads are parsed using OpenAI types since they share the
same structure.
*/

pub mod detect;

// Re-export detection functions
pub use detect::{detect_mistral, try_parse_mistral, DetectionError, MistralChatRequest};
