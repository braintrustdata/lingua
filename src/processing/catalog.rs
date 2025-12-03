/*!
Catalog lookup for format detection.

This module provides a simple interface for model-to-format lookup.
The actual catalog is injected by the host application (e.g., the gateway)
via `set_catalog_lookup`.

# Testing

Unit tests are not included in this module because the global `OnceLock` state
makes reliable testing impossible (test execution order is non-deterministic,
and `OnceLock` only allows a single initialization). See `tests/catalog_test.rs`
for integration tests that use `tests/fixtures/test_catalog.json` as a mock catalog.
*/

use crate::capabilities::ProviderFormat;
use std::sync::OnceLock;

/// Type alias for the catalog lookup function.
type CatalogLookupFn = Box<dyn Fn(&str) -> Option<ProviderFormat> + Send + Sync>;

/// Global catalog lookup function, injected at startup.
static CATALOG_LOOKUP: OnceLock<CatalogLookupFn> = OnceLock::new();

/// Set the catalog lookup function.
///
/// This should be called once at startup by the host application.
/// Subsequent calls are ignored (first write wins).
///
/// # Example
///
/// ```no_run
/// use lingua::processing::set_catalog_lookup;
/// use lingua::capabilities::ProviderFormat;
/// use std::collections::HashMap;
/// use std::sync::Arc;
///
/// // Create a simple model-to-format catalog
/// let catalog: HashMap<&str, ProviderFormat> = HashMap::from([
///     ("gpt-4", ProviderFormat::OpenAI),
///     ("claude-3", ProviderFormat::Anthropic),
/// ]);
/// let catalog = Arc::new(catalog);
///
/// set_catalog_lookup(move |model| catalog.get(model).copied());
/// ```
pub fn set_catalog_lookup<F>(f: F)
where
    F: Fn(&str) -> Option<ProviderFormat> + Send + Sync + 'static,
{
    let _ = CATALOG_LOOKUP.set(Box::new(f));
}

/// Look up a model's format using the injected catalog.
///
/// Returns `None` if:
/// - No catalog has been set via `set_catalog_lookup`
/// - The model is not found in the catalog
pub fn catalog_lookup(model: &str) -> Option<ProviderFormat> {
    CATALOG_LOOKUP.get().and_then(|f| f(model))
}
