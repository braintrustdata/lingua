/*!
Wrapper crate for serde_json with arbitrary_precision feature enabled.

This crate re-exports serde_json with the `arbitrary_precision` feature to avoid
forcing this feature on downstream crates due to Cargo's feature unification.

See: https://github.com/serde-rs/json/issues/1157
*/

pub use serde_json::*;
