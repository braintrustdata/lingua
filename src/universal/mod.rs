/*!
Universal format definitions. This module is designed to be an ergonomic and durable way to create LLM
messages and specify parameters in a way that is compatible with multiple LLM providers.

It uses a few common conventions to provide this functionality while staying provider-agnostic:
* A curated set of common options that are compiled to various providers' specific formats
* Fallback to provider-specific options via a `provider_specific` field on many structs.
*/

pub mod citation;
pub mod message;
pub mod provider;
pub mod ts_export;

// #[cfg(test)]
// mod message_test;
