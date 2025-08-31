// Generated OpenAI types using quicktype
// Essential types for Elmir OpenAI integration

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::openai_schemas;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: openai_schemas = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};

pub type OpenaiSchemas = Option<serde_json::Value>;
