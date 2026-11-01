/*!
TypeSafe System One API provider types and conversions.
*/

pub mod adapter;
pub mod convert;
pub mod request;
pub mod response;

pub use adapter::TypeSafeAdapter;
pub use convert::system_one_response_to_universal;
pub use request::{SystemOneRequest, TypeSafeEntry, TypeSafeJson, TypeSafeQuestion};
pub use response::{SystemOneResponse, TypeSafeAnswer, TypeSafeUsage};

#[cfg(all(test, feature = "openai"))]
mod tests;
