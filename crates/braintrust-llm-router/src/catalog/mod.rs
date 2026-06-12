mod resolver;
pub mod spec;

pub(crate) use resolver::is_gemini_api_model;
pub use resolver::ModelResolver;
pub use spec::{ModelFlavor, ModelSpec};

use lingua::ProviderFormat;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct ModelCatalog {
    models: HashMap<String, Arc<ModelSpec>>,
    by_format: HashMap<ProviderFormat, Vec<String>>,
    by_parent: HashMap<String, Vec<String>>,
    equivalent_models: HashMap<String, Vec<String>>,
    equivalence_index: HashMap<String, Vec<String>>,
}

/// A request-local catalog overlay.
///
/// Secret-defined custom models live in `custom` and shadow entries in the
/// shared `base` catalog. This avoids cloning the base catalog when adding
/// per-request model definitions.
#[derive(Debug, Clone)]
pub struct OverlayModelCatalog {
    pub base: Arc<ModelCatalog>,
    pub custom: ModelCatalog,
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
            Self::Overlay(overlay) => Arc::clone(&overlay.base),
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<ModelSpec>> {
        match self {
            Self::Base(catalog) => catalog.get(name),
            Self::Overlay(overlay) => overlay.custom.get(name).or_else(|| overlay.base.get(name)),
        }
    }

    pub fn equivalent_model_names(&self, name: &str) -> Vec<String> {
        match self {
            Self::Base(catalog) => catalog.equivalent_model_names(name),
            Self::Overlay(overlay) => {
                if overlay.custom.get(name).is_some() {
                    overlay.custom.equivalent_model_names(name)
                } else {
                    overlay.base.equivalent_model_names(name)
                }
            }
        }
    }
}

impl From<Arc<ModelCatalog>> for CatalogResolver {
    fn from(catalog: Arc<ModelCatalog>) -> Self {
        Self::Base(catalog)
    }
}

fn parse_equivalent_models(content: &str) -> Result<HashMap<String, Vec<String>>> {
    let raw: HashMap<String, serde_json::Value> = serde_json::from_str(content)?;
    let mut equivalent_models = HashMap::new();
    for (name, value) in raw {
        let Some(equivalents) = value.get("equivalent_models") else {
            continue;
        };
        let Some(equivalents) = equivalents.as_array() else {
            return Err(Error::InvalidRequest(format!(
                "model '{name}' has invalid equivalent_models"
            )));
        };
        let mut parsed = Vec::with_capacity(equivalents.len());
        for equivalent in equivalents {
            let Some(equivalent) = equivalent.as_str() else {
                return Err(Error::InvalidRequest(format!(
                    "model '{name}' has invalid equivalent_models"
                )));
            };
            parsed.push(equivalent.to_string());
        }
        if !parsed.is_empty() {
            equivalent_models.insert(name, parsed);
        }
    }
    Ok(equivalent_models)
}

impl ModelCatalog {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_json_str(content: &str) -> Result<Self> {
        let raw: HashMap<String, ModelSpec> = serde_json::from_str(content)?;
        let equivalent_models = parse_equivalent_models(content)?;
        let mut catalog = Self::empty();
        for (name, spec) in raw {
            catalog.insert(name, spec);
        }
        catalog.equivalent_models = equivalent_models;
        catalog.validate_equivalent_models()?;
        catalog.rebuild_equivalence_index();
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

    pub fn equivalent_model_names(&self, name: &str) -> Vec<String> {
        let Some(_) = self.models.get(name) else {
            return Vec::new();
        };

        let mut names = vec![name.to_string()];
        if let Some(equivalent_names) = self.equivalence_index.get(name) {
            names.extend(equivalent_names.iter().cloned());
        }
        names
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
        let mut out = Self {
            equivalent_models: self.equivalent_models.clone(),
            ..Self::empty()
        };
        for (name, spec) in &self.models {
            out.insert(name.clone(), f(name, spec.as_ref()));
        }
        out.rebuild_equivalence_index();
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

    fn validate_equivalent_models(&self) -> Result<()> {
        for (name, equivalents) in &self.equivalent_models {
            for equivalent_model in equivalents {
                if !self.models.contains_key(equivalent_model) {
                    return Err(Error::InvalidRequest(format!(
                        "model '{name}' references missing equivalent model '{equivalent_model}'"
                    )));
                }
            }
        }
        Ok(())
    }

    fn rebuild_equivalence_index(&mut self) {
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for name in self.models.keys() {
            adjacency.entry(name.clone()).or_default();
        }

        for (name, equivalents) in &self.equivalent_models {
            if !self.models.contains_key(name) {
                continue;
            }
            for equivalent_model in equivalents {
                if !self.models.contains_key(equivalent_model) {
                    continue;
                }
                adjacency
                    .entry(name.clone())
                    .or_default()
                    .push(equivalent_model.clone());
                adjacency
                    .entry(equivalent_model.clone())
                    .or_default()
                    .push(name.clone());
            }
        }

        let mut visited = std::collections::HashSet::new();
        let mut index = HashMap::new();
        for name in self.models.keys() {
            if visited.contains(name) {
                continue;
            }

            let mut stack = vec![name.clone()];
            let mut component = Vec::new();
            while let Some(current) = stack.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }
                component.push(current.clone());
                if let Some(neighbors) = adjacency.get(&current) {
                    stack.extend(neighbors.iter().cloned());
                }
            }

            if component.len() <= 1 {
                continue;
            }
            component.sort();
            for member in &component {
                index.insert(
                    member.clone(),
                    component
                        .iter()
                        .filter(|other| *other != member)
                        .cloned()
                        .collect(),
                );
            }
        }

        self.equivalence_index = index;
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

    #[test]
    fn equivalent_model_names_are_available_from_any_member() {
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat",
    "equivalent_models": [
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
            catalog.equivalent_model_names("claude-sonnet-4-6"),
            vec![
                "claude-sonnet-4-6".to_string(),
                "anthropic.claude-sonnet-4-6".to_string(),
                "publishers/anthropic/models/claude-sonnet-4-6".to_string(),
            ]
        );
        assert_eq!(
            catalog.equivalent_model_names("publishers/anthropic/models/claude-sonnet-4-6"),
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
    "equivalent_models": ["model-b"]
  },
  "model-b": {
    "format": "openai",
    "flavor": "chat",
    "equivalent_models": ["model-c"]
  },
  "model-c": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");

        assert_eq!(
            catalog.equivalent_model_names("model-a"),
            vec![
                "model-a".to_string(),
                "model-b".to_string(),
                "model-c".to_string(),
            ]
        );
    }

    #[test]
    fn missing_equivalent_model_reference_is_invalid() {
        let error = ModelCatalog::from_json_str(
            r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat",
    "equivalent_models": ["missing-model"]
  }
}"#,
        )
        .expect_err("missing equivalent model should fail");

        assert!(matches!(error, Error::InvalidRequest(_)));
    }

    #[test]
    fn map_specs_preserves_equivalent_model_index() {
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "model-a": {
    "format": "openai",
    "flavor": "chat",
    "equivalent_models": ["model-b"]
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
            mapped.equivalent_model_names("model-a"),
            vec!["model-a".to_string(), "model-b".to_string()]
        );
    }
}
