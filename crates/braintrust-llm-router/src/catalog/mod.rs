mod resolver;
pub mod spec;

pub use resolver::ModelResolver;
pub use spec::{ModelFlavor, ModelSpec};

use lingua::ProviderFormat;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::error::Result;

#[derive(Debug, Clone, Default)]
pub struct ModelCatalog {
    models: HashMap<String, Arc<ModelSpec>>,
    by_format: HashMap<ProviderFormat, Vec<String>>,
    by_parent: HashMap<String, Vec<String>>,
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

/// The bundled model catalog JSON string (model_list.json).
pub const BUNDLED_CATALOG_JSON: &str = include_str!("model_list.json");

static DEFAULT_CATALOG: OnceLock<Arc<ModelCatalog>> = OnceLock::new();

pub fn default_catalog() -> Arc<ModelCatalog> {
    DEFAULT_CATALOG
        .get_or_init(|| {
            let catalog = ModelCatalog::from_json_str(include_str!("model_list.json"))
                .expect("embedded model_list.json must be valid");
            Arc::new(catalog)
        })
        .clone()
}
