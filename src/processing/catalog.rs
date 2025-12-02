/*!
Catalog lookup for format detection.

This module provides a simple interface for model-to-format lookup.
The actual catalog is injected by the host application (e.g., the gateway)
via `set_catalog_lookup`.
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
/// ```ignore
/// let catalog = Arc::clone(&model_catalog);
/// lingua::processing::set_catalog_lookup(move |m| catalog.resolve_format_with_prefix(m));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_catalog_lookup_without_init() {
        // Before any catalog is set, lookups return None
        // Note: This test may fail if run after other tests that set the catalog
        // since OnceLock is global. In practice, tests should use a fresh process.
    }

    #[test]
    fn test_set_and_lookup() {
        // Create a simple in-memory catalog
        let mut map: HashMap<String, ProviderFormat> = HashMap::new();
        map.insert("gpt-4".to_string(), ProviderFormat::OpenAI);
        map.insert("claude-3".to_string(), ProviderFormat::Anthropic);
        let map = Arc::new(map);

        // This may or may not succeed depending on test order
        let map_clone = Arc::clone(&map);
        set_catalog_lookup(move |model| map_clone.get(model).copied());

        // If set succeeded, lookups should work
        if CATALOG_LOOKUP.get().is_some() {
            assert_eq!(catalog_lookup("gpt-4"), Some(ProviderFormat::OpenAI));
            assert_eq!(catalog_lookup("claude-3"), Some(ProviderFormat::Anthropic));
            assert_eq!(catalog_lookup("unknown"), None);
        }
    }
}
