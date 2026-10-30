/*!
TypeSafe System One API provider types and conversions.
*/

pub mod convert;
pub mod request;
pub mod response;

pub use convert::{
    system_one_response_to_universal, universal_to_system_one, SystemOneDecisionContext,
};
pub use request::{SystemOneRequest, TypeSafeEntry, TypeSafeJson, TypeSafeQuestion};
pub use response::{SystemOneResponse, TypeSafeAnswer, TypeSafeUsage};

#[cfg(test)]
mod tests;
