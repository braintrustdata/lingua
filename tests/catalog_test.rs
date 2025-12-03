//! Integration tests for catalog lookup functionality.
//!
//! These tests run in a separate process, ensuring a fresh `OnceLock` state.

use lingua::capabilities::ProviderFormat;
use lingua::processing::{catalog_lookup, set_catalog_lookup};
use lingua::serde_json;
use std::collections::HashMap;
use std::sync::Arc;

/// A minimal model entry from the catalog JSON.
#[derive(serde::Deserialize)]
struct ModelEntry {
    format: String,
}

/// Load the test catalog and set up the lookup function.
fn setup_catalog() {
    // Load the test catalog at compile time
    let catalog_json = include_str!("fixtures/test_catalog.json");

    // Parse into a map of model name -> format string
    let raw_catalog: HashMap<String, ModelEntry> =
        serde_json::from_str(catalog_json).expect("Failed to parse test catalog JSON");

    // Convert to model name -> ProviderFormat
    let catalog: HashMap<String, ProviderFormat> = raw_catalog
        .into_iter()
        .map(|(model, entry)| {
            let format = entry.format.parse::<ProviderFormat>().unwrap_or_default();
            (model, format)
        })
        .collect();

    let catalog = Arc::new(catalog);

    // Set the global catalog lookup
    set_catalog_lookup(move |model| catalog.get(model).copied());
}

#[test]
fn test_catalog_lookup_openai() {
    setup_catalog();
    assert_eq!(catalog_lookup("gpt-4o"), Some(ProviderFormat::OpenAI));
}

#[test]
fn test_catalog_lookup_anthropic() {
    setup_catalog();
    assert_eq!(
        catalog_lookup("claude-3-5-sonnet-latest"),
        Some(ProviderFormat::Anthropic)
    );
}

#[test]
fn test_catalog_lookup_google() {
    setup_catalog();
    assert_eq!(
        catalog_lookup("gemini-2.0-flash"),
        Some(ProviderFormat::Google)
    );
}

#[test]
fn test_catalog_lookup_mistral() {
    setup_catalog();
    assert_eq!(
        catalog_lookup("mistral-large-latest"),
        Some(ProviderFormat::Mistral)
    );
}

#[test]
fn test_catalog_lookup_unknown_model() {
    setup_catalog();
    assert_eq!(catalog_lookup("unknown-model-xyz"), None);
}
