use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{ModelCatalog, ModelSpec};
use crate::error::{Error, Result};

/// A request-local catalog overlay.
///
/// Secret-defined custom models live in `custom` and shadow entries in the
/// shared `base` catalog. This avoids cloning the base catalog when adding
/// per-request model definitions.
#[derive(Debug, Clone)]
pub struct OverlayModelCatalog {
    base: Arc<ModelCatalog>,
    custom: ModelCatalog,
    custom_model_names: HashSet<String>,
    overlay_edges: HashMap<String, Vec<String>>,
}

impl OverlayModelCatalog {
    pub fn new(base: Arc<ModelCatalog>, custom: ModelCatalog) -> Self {
        let custom_model_names: HashSet<String> = custom.models.keys().cloned().collect();
        let mut overlay_edges: HashMap<String, Vec<String>> = HashMap::new();
        for (name, fallbacks) in &custom.fallback_models {
            if !custom.models.contains_key(name) {
                continue;
            }
            for fallback_model in fallbacks {
                let fallback_is_visible = custom.models.contains_key(fallback_model)
                    || (base.models.contains_key(fallback_model)
                        && !custom_model_names.contains(fallback_model));
                if !fallback_is_visible {
                    continue;
                }
                overlay_edges
                    .entry(name.clone())
                    .or_default()
                    .push(fallback_model.clone());
                overlay_edges
                    .entry(fallback_model.clone())
                    .or_default()
                    .push(name.clone());
            }
        }
        Self {
            base,
            custom,
            custom_model_names,
            overlay_edges,
        }
    }

    pub fn base_catalog(&self) -> Arc<ModelCatalog> {
        Arc::clone(&self.base)
    }

    pub fn get(&self, name: &str) -> Option<Arc<ModelSpec>> {
        self.custom.get(name).or_else(|| self.base.get(name))
    }

    pub fn find_fallback_models(&self, name: &str) -> Vec<String> {
        let Some(_) = self.get(name) else {
            return Vec::new();
        };

        let mut visited = HashSet::new();
        let mut stack = vec![name.to_string()];
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if !self.custom_model_names.contains(&current) {
                stack.extend(
                    self.base
                        .fallback_models(&current)
                        .into_iter()
                        .filter(|model_name| !self.custom_model_names.contains(model_name)),
                );
            }
            if let Some(neighbors) = self.overlay_edges.get(&current) {
                stack.extend(neighbors.iter().cloned());
            }
        }

        let mut names = vec![name.to_string()];
        visited.remove(name);
        let mut equivalent_names: Vec<String> = visited.into_iter().collect();
        equivalent_names.sort();
        names.extend(equivalent_names);
        names
    }
}

enum FallbackModelSource<'a> {
    Json(&'a str),
    Parsed(HashMap<String, Vec<String>>),
}

impl ModelCatalog {
    pub fn fallback_models(&self, name: &str) -> Vec<String> {
        let Some(_) = self.models.get(name) else {
            return Vec::new();
        };

        let mut names = vec![name.to_string()];
        if let Some(equivalent_names) = self.equivalence_index.get(name) {
            names.extend(equivalent_names.iter().cloned());
        }
        names
    }

    pub fn add_fallback_models<I>(&mut self, name: String, fallback_models: I) -> Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        if !self.models.contains_key(&name) {
            return Err(Error::InvalidRequest(format!(
                "model '{name}' references fallback_models but is missing from catalog"
            )));
        }

        let fallback_models: Vec<String> = fallback_models
            .into_iter()
            .filter(|fallback_model| !fallback_model.is_empty())
            .collect();
        for fallback_model in &fallback_models {
            if !self.models.contains_key(fallback_model) {
                return Err(Error::InvalidRequest(format!(
                    "model '{name}' references missing fallback model '{fallback_model}'"
                )));
            }
        }

        let mut next_fallback_models = self.fallback_models.clone();
        let entry = next_fallback_models.entry(name).or_default();
        for fallback_model in fallback_models {
            if entry.contains(&fallback_model) {
                continue;
            }
            entry.push(fallback_model);
        }
        self.set_fallback_models(FallbackModelSource::Parsed(next_fallback_models), false)?;
        Ok(())
    }

    pub fn add_external_fallback_models<I>(
        &mut self,
        name: String,
        fallback_models: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = String>,
    {
        if !self.models.contains_key(&name) {
            return Err(Error::InvalidRequest(format!(
                "model '{name}' references fallback_models but is missing from catalog"
            )));
        }

        let mut next_fallback_models = self.fallback_models.clone();
        let entry = next_fallback_models.entry(name).or_default();
        for fallback_model in fallback_models {
            if fallback_model.is_empty() || entry.contains(&fallback_model) {
                continue;
            }
            entry.push(fallback_model);
        }
        self.set_fallback_models(FallbackModelSource::Parsed(next_fallback_models), false)?;
        Ok(())
    }

    pub(super) fn set_fallback_models_from_json(
        &mut self,
        content: &str,
        validate_targets: bool,
    ) -> Result<()> {
        self.set_fallback_models(FallbackModelSource::Json(content), validate_targets)
    }

    pub(super) fn set_fallback_models_from_parsed(
        &mut self,
        fallback_models: HashMap<String, Vec<String>>,
        validate_targets: bool,
    ) -> Result<()> {
        self.set_fallback_models(
            FallbackModelSource::Parsed(fallback_models),
            validate_targets,
        )
    }

    fn set_fallback_models(
        &mut self,
        source: FallbackModelSource<'_>,
        validate_targets: bool,
    ) -> Result<()> {
        let fallback_models = match source {
            FallbackModelSource::Json(content) => parse_fallback_models(content)?,
            FallbackModelSource::Parsed(fallback_models) => fallback_models,
        };

        if validate_targets {
            validate_fallback_models(&self.models, &fallback_models)?;
        }
        self.equivalence_index =
            build_equivalence_index(self.models.keys().cloned().collect(), &fallback_models);
        self.fallback_models = fallback_models;
        Ok(())
    }
}

fn parse_fallback_models(content: &str) -> Result<HashMap<String, Vec<String>>> {
    let raw: HashMap<String, serde_json::Value> = serde_json::from_str(content)?;
    let mut fallback_models = HashMap::new();
    for (name, value) in raw {
        let Some(fallbacks) = value.get("fallback_models") else {
            continue;
        };
        let Some(fallbacks) = fallbacks.as_array() else {
            return Err(Error::InvalidRequest(format!(
                "model '{name}' has invalid fallback_models"
            )));
        };
        let mut parsed = Vec::with_capacity(fallbacks.len());
        for fallback_model in fallbacks {
            let Some(fallback_model) = fallback_model.as_str() else {
                return Err(Error::InvalidRequest(format!(
                    "model '{name}' has invalid fallback_models"
                )));
            };
            parsed.push(fallback_model.to_string());
        }
        if !parsed.is_empty() {
            fallback_models.insert(name, parsed);
        }
    }
    Ok(fallback_models)
}

fn validate_fallback_models(
    models: &HashMap<String, Arc<ModelSpec>>,
    fallback_models: &HashMap<String, Vec<String>>,
) -> Result<()> {
    for (name, fallback_models) in fallback_models {
        for fallback_model in fallback_models {
            if !models.contains_key(fallback_model) {
                return Err(Error::InvalidRequest(format!(
                    "model '{name}' references missing fallback model '{fallback_model}'"
                )));
            }
        }
    }
    Ok(())
}

fn build_equivalence_index(
    model_names: HashSet<String>,
    fallback_models: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for name in &model_names {
        adjacency.entry(name.clone()).or_default();
    }

    for (name, fallbacks) in fallback_models {
        if !model_names.contains(name) {
            continue;
        }
        for fallback_model in fallbacks {
            if !model_names.contains(fallback_model) {
                continue;
            }
            adjacency
                .entry(name.clone())
                .or_default()
                .push(fallback_model.clone());
            adjacency
                .entry(fallback_model.clone())
                .or_default()
                .push(name.clone());
        }
    }

    let mut visited = HashSet::new();
    let mut index = HashMap::new();
    for name in model_names {
        if visited.contains(&name) {
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

    index
}
