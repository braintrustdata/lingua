/*!
Catalog lookup for format detection.

This module provides a simple interface for model-to-format lookup.
The actual catalog is injected by the host application (e.g., the gateway)
via `set_catalog_lookup`.

The catalog uses `ArcSwap` for lock-free atomic updates, allowing the catalog
to be refreshed at runtime without blocking readers.
*/

use crate::capabilities::ProviderFormat;
use arc_swap::ArcSwap;
use std::sync::{Arc, LazyLock};

/// Type alias for the catalog lookup function.
type CatalogLookupFn = Arc<dyn Fn(&str) -> Option<ProviderFormat> + Send + Sync>;

/// Global catalog lookup function, injected at startup and refreshable at runtime.
static CATALOG_LOOKUP: LazyLock<ArcSwap<Option<CatalogLookupFn>>> =
    LazyLock::new(|| ArcSwap::from_pointee(None));

/// Set the catalog lookup function.
///
/// This can be called at startup by the host application and again later
/// to refresh the catalog. Each call atomically replaces the previous lookup function.
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
    CATALOG_LOOKUP.store(Arc::new(Some(Arc::new(f))));
}

/// Look up a model's format using the injected catalog.
///
/// Returns `None` if:
/// - No catalog has been set via `set_catalog_lookup`
/// - The model is not found in the catalog
pub fn catalog_lookup(model: &str) -> Option<ProviderFormat> {
    let guard = CATALOG_LOOKUP.load();
    guard.as_ref().as_ref().and_then(|f| f(model))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_set_and_lookup() {
        set_catalog_lookup(|model| match model {
            "test-model-a1" => Some(ProviderFormat::OpenAI),
            _ => None,
        });
        assert_eq!(
            catalog_lookup("test-model-a1"),
            Some(ProviderFormat::OpenAI)
        );
        assert_eq!(catalog_lookup("unknown-a1"), None);
    }

    #[test]
    #[serial]
    fn test_catalog_can_be_refreshed() {
        // First catalog
        set_catalog_lookup(|model| match model {
            "test-model-b1" => Some(ProviderFormat::OpenAI),
            _ => None,
        });
        assert_eq!(
            catalog_lookup("test-model-b1"),
            Some(ProviderFormat::OpenAI)
        );

        // Refresh with new catalog
        set_catalog_lookup(|model| match model {
            "test-model-b1" => Some(ProviderFormat::Anthropic),
            _ => None,
        });
        assert_eq!(
            catalog_lookup("test-model-b1"),
            Some(ProviderFormat::Anthropic)
        );
    }

    #[test]
    #[serial]
    fn test_lookup_without_catalog_returns_none() {
        // Uses unique model name that no other test sets
        assert_eq!(catalog_lookup("never-set-model-c1"), None);
    }
}
