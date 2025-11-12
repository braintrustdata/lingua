// Re-export lingua_serde_json as serde_json so all code can use crate::serde_json:: transparently
// while actually using our forked version with arbitrary_precision
pub use lingua_serde_json as serde_json;

pub mod capabilities;
pub mod processing;
pub mod providers;
pub mod universal;
pub mod util;
pub mod validation;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(feature = "python")]
pub mod python;
