/*!
TypeSafe System One API provider types and conversions.
*/

pub mod adapter;
pub mod convert;
mod entry;
pub mod generated;

pub use adapter::TypeSafeAdapter;
pub use convert::system_one_response_to_universal;
pub use entry::{TypeSafeEntry, TypeSafeJson};
pub use generated::{SystemOneRequest, SystemOneResponse};

#[cfg(all(test, feature = "openai"))]
mod tests;
