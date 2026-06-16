mod fallback;
mod resolver;
pub mod spec;

pub use fallback::OverlayModelCatalog;
pub(crate) use resolver::is_gemini_api_model;
pub use resolver::ModelResolver;
pub use spec::{ModelFlavor, ModelSpec};

use lingua::ProviderFormat;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use crate::error::Result;

#[derive(Debug, Clone, Default)]
pub struct ModelCatalog {
    models: HashMap<String, Arc<ModelSpec>>,
    by_format: HashMap<ProviderFormat, Vec<String>>,
    by_parent: HashMap<String, Vec<String>>,
    fallback_models: HashMap<String, Vec<String>>,
    equivalence_index: HashMap<String, Vec<String>>,
}

/// Catalog view used by the router resolver.
///
/// `Base` preserves the existing router behavior. `Overlay` checks custom
/// models first and then falls back to the shared base catalog.
#[derive(Debug, Clone)]
pub enum CatalogResolver {
    Base(Arc<ModelCatalog>),
    Overlay(Box<OverlayModelCatalog>),
}

impl CatalogResolver {
    pub fn base_catalog(&self) -> Arc<ModelCatalog> {
        match self {
            Self::Base(catalog) => Arc::clone(catalog),
            Self::Overlay(overlay) => overlay.base_catalog(),
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<ModelSpec>> {
        match self {
            Self::Base(catalog) => catalog.get(name),
            Self::Overlay(overlay) => overlay.get(name),
        }
    }

    pub fn fallback_models(&self, name: &str) -> Vec<String> {
        match self {
            Self::Base(catalog) => catalog.fallback_models(name),
            Self::Overlay(overlay) => overlay.find_fallback_models(name),
        }
    }
}

impl From<Arc<ModelCatalog>> for CatalogResolver {
    fn from(catalog: Arc<ModelCatalog>) -> Self {
        Self::Base(catalog)
    }
}

impl ModelCatalog {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_json_str(content: &str) -> Result<Self> {
        let raw: HashMap<String, ModelSpec> = serde_json::from_str(content)?;
        let mut catalog = Self::empty();
        for (name, spec) in raw {
            catalog.insert(name, spec);
        }
        catalog.set_fallback_models_from_json(content, true)?;
        Ok(catalog)
    }

    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Self::from_json_str(&buf)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }

    pub fn get(&self, name: &str) -> Option<Arc<ModelSpec>> {
        self.models.get(name).cloned()
    }

    pub fn resolve_format(&self, model: &str) -> Option<ProviderFormat> {
        self.models.get(model).map(|spec| spec.format)
    }

    /// Resolve format with prefix matching fallback.
    ///
    /// Tries exact match first, then falls back to longest-prefix matching
    /// for versioned models (e.g., "gpt-4o-2024-08-06" matches "gpt-4o").
    pub fn resolve_format_with_prefix(&self, model: &str) -> Option<ProviderFormat> {
        // Exact match
        if let Some(spec) = self.models.get(model) {
            return Some(spec.format);
        }

        // Prefix match: find longest prefix followed by '-' or '/'
        let mut best_match: Option<(usize, ProviderFormat)> = None;
        for (name, spec) in &self.models {
            if model.starts_with(name.as_str()) && model.len() > name.len() {
                let next_char = model.chars().nth(name.len());
                // Only match if followed by a separator
                if next_char == Some('-') || next_char == Some('/') {
                    match &best_match {
                        Some((len, _)) if name.len() <= *len => {}
                        _ => best_match = Some((name.len(), spec.format)),
                    }
                }
            }
        }

        best_match.map(|(_, format)| format)
    }

    pub fn models_for_format(&self, format: ProviderFormat) -> Option<&[String]> {
        self.by_format.get(&format).map(Vec::as_slice)
    }

    pub fn child_models<'a>(&'a self, parent: &str) -> impl Iterator<Item = &'a String> + 'a {
        self.by_parent
            .get(parent)
            .map(|entries| entries.iter())
            .into_iter()
            .flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<ModelSpec>)> {
        self.models.iter()
    }

    pub fn map_specs<F>(&self, mut f: F) -> Self
    where
        F: FnMut(&str, &ModelSpec) -> ModelSpec,
    {
        let mut out = Self::empty();
        for (name, spec) in &self.models {
            out.insert(name.clone(), f(name, spec.as_ref()));
        }
        out.set_fallback_models_from_parsed(self.fallback_models.clone(), false)
            .expect("existing catalog fallback_models remain valid after mapping specs");
        out
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn insert(&mut self, name: String, mut spec: ModelSpec) {
        if spec.model.is_empty() {
            spec.model = name.clone();
        }
        let format = spec.format;
        let parent = spec.parent.clone();
        let spec = Arc::new(spec);
        self.models.insert(name.clone(), spec);
        self.by_format.entry(format).or_default().push(name.clone());
        if let Some(parent) = parent {
            self.by_parent.entry(parent).or_default().push(name);
        }
    }
}

pub fn load_catalog_from_disk<P: AsRef<Path>>(path: P) -> Result<Arc<ModelCatalog>> {
    Ok(Arc::new(ModelCatalog::from_file(path)?))
}

// The canonical model catalog lives in the braintrust-proxy repository:
// https://github.com/braintrustdata/braintrust-proxy/blob/main/packages/proxy/schema/model_list.json
//
// Consumers must load it explicitly via ModelCatalog::from_file() or
// ModelCatalog::from_json_str(). There is no bundled/default catalog.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn fallback_models_are_available_from_any_member() {
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat",
    "fallback_models": [
      "publishers/anthropic/models/claude-sonnet-4-6",
      "anthropic.claude-sonnet-4-6"
    ]
  },
  "publishers/anthropic/models/claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat"
  },
  "anthropic.claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");

        assert_eq!(
            catalog.fallback_models("claude-sonnet-4-6"),
            vec![
                "claude-sonnet-4-6".to_string(),
                "anthropic.claude-sonnet-4-6".to_string(),
                "publishers/anthropic/models/claude-sonnet-4-6".to_string(),
            ]
        );
        assert_eq!(
            catalog.fallback_models("publishers/anthropic/models/claude-sonnet-4-6"),
            vec![
                "publishers/anthropic/models/claude-sonnet-4-6".to_string(),
                "anthropic.claude-sonnet-4-6".to_string(),
                "claude-sonnet-4-6".to_string(),
            ]
        );
    }

    #[test]
    fn equivalent_model_groups_are_connected_components() {
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat",
    "fallback_models": ["model-b"]
  },
  "model-b": {
    "format": "openai",
    "flavor": "chat",
    "fallback_models": ["model-c"]
  },
  "model-c": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");

        assert_eq!(
            catalog.fallback_models("model-a"),
            vec![
                "model-a".to_string(),
                "model-b".to_string(),
                "model-c".to_string(),
            ]
        );
    }

    #[test]
    fn missing_fallback_model_reference_is_invalid() {
        let error = ModelCatalog::from_json_str(
            r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat",
    "fallback_models": ["missing-model"]
  }
}"#,
        )
        .expect_err("missing fallback model should fail");

        assert!(matches!(error, Error::InvalidRequest(_)));
    }

    #[test]
    fn add_fallback_models_rebuilds_index() {
        let mut catalog = ModelCatalog::from_json_str(
            r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat"
  },
  "model-b": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");

        catalog
            .add_fallback_models("model-a".to_string(), vec!["model-b".to_string()])
            .expect("equivalence is valid");

        assert_eq!(
            catalog.fallback_models("model-a"),
            vec!["model-a".to_string(), "model-b".to_string()]
        );
        assert_eq!(
            catalog.fallback_models("model-b"),
            vec!["model-b".to_string(), "model-a".to_string()]
        );
    }

    #[test]
    fn add_fallback_models_rejects_missing_reference() {
        let mut catalog = ModelCatalog::from_json_str(
            r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");

        let error = catalog
            .add_fallback_models("model-a".to_string(), vec!["missing".to_string()])
            .expect_err("missing fallback model should fail");

        assert!(matches!(error, Error::InvalidRequest(_)));
        assert_eq!(
            catalog.fallback_models("model-a"),
            vec!["model-a".to_string()]
        );
    }

    #[test]
    fn map_specs_preserves_equivalent_model_index() {
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat",
    "fallback_models": ["model-b"]
  },
  "model-b": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");

        let mapped = catalog.map_specs(|_, spec| {
            let mut spec = spec.clone();
            spec.available_providers = vec!["OPENAI_API_KEY".to_string()];
            spec
        });

        assert_eq!(
            mapped.fallback_models("model-a"),
            vec!["model-a".to_string(), "model-b".to_string()]
        );
    }

    #[test]
    fn overlay_equivalence_reaches_custom_and_touched_base_models() {
        let base = Arc::new(
            ModelCatalog::from_json_str(
                r#"{
  "base-a": {
    "format": "openai",
    "flavor": "chat",
    "fallback_models": ["base-b"]
  },
  "base-b": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
            )
            .expect("base catalog parses"),
        );
        let mut custom = ModelCatalog::empty();
        custom.insert(
            "custom-a".to_string(),
            ModelSpec {
                model: "custom-a".to_string(),
                format: ProviderFormat::Anthropic,
                flavor: ModelFlavor::Chat,
                display_name: None,
                parent: None,
                input_cost_per_mil_tokens: None,
                output_cost_per_mil_tokens: None,
                input_cache_read_cost_per_mil_tokens: None,
                multimodal: None,
                reasoning: None,
                max_input_tokens: None,
                max_output_tokens: None,
                supports_streaming: true,
                extra: Default::default(),
                available_providers: vec!["custom-provider".to_string()],
            },
        );
        custom
            .add_external_fallback_models("custom-a".to_string(), vec!["base-a".to_string()])
            .expect("fallback is valid");

        let overlay = OverlayModelCatalog::new(base, custom);

        assert_eq!(
            overlay.find_fallback_models("custom-a"),
            vec![
                "custom-a".to_string(),
                "base-a".to_string(),
                "base-b".to_string()
            ]
        );
        assert_eq!(
            overlay.find_fallback_models("base-a"),
            vec![
                "base-a".to_string(),
                "base-b".to_string(),
                "custom-a".to_string()
            ]
        );
        assert_eq!(
            overlay.find_fallback_models("base-b"),
            vec![
                "base-b".to_string(),
                "base-a".to_string(),
                "custom-a".to_string()
            ]
        );
    }

    #[test]
    fn overlay_equivalence_index_does_not_inherit_shadowed_base_edges() {
        let base = Arc::new(
            ModelCatalog::from_json_str(
                r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat",
    "fallback_models": ["model-b"]
  },
  "model-b": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
            )
            .expect("base catalog parses"),
        );
        let mut custom = ModelCatalog::empty();
        custom.insert(
            "model-b".to_string(),
            ModelSpec {
                model: "custom-model-b".to_string(),
                format: ProviderFormat::Anthropic,
                flavor: ModelFlavor::Chat,
                display_name: None,
                parent: None,
                input_cost_per_mil_tokens: None,
                output_cost_per_mil_tokens: None,
                input_cache_read_cost_per_mil_tokens: None,
                multimodal: None,
                reasoning: None,
                max_input_tokens: None,
                max_output_tokens: None,
                supports_streaming: true,
                extra: Default::default(),
                available_providers: vec!["custom-provider".to_string()],
            },
        );

        let overlay = OverlayModelCatalog::new(base, custom);

        assert_eq!(
            overlay.find_fallback_models("model-a"),
            vec!["model-a".to_string()]
        );
        assert_eq!(
            overlay.find_fallback_models("model-b"),
            vec!["model-b".to_string()]
        );
    }
}
