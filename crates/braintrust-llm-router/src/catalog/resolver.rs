use std::collections::HashMap;
use std::sync::Arc;

use crate::catalog::{ModelCatalog, ModelSpec};
use crate::error::{Error, Result};
use lingua::ProviderFormat;

#[derive(Debug, Clone)]
pub struct ModelResolver {
    catalog: Arc<ModelCatalog>,
    aliases: HashMap<String, String>,
}

impl ModelResolver {
    pub fn new(catalog: Arc<ModelCatalog>) -> Self {
        Self {
            catalog,
            aliases: HashMap::new(),
        }
    }

    pub fn with_aliases(mut self, aliases: HashMap<String, String>) -> Self {
        self.aliases = aliases;
        self
    }

    pub fn catalog(&self) -> Arc<ModelCatalog> {
        Arc::clone(&self.catalog)
    }

    pub fn resolve(&self, model: &str) -> Result<(Arc<ModelSpec>, ProviderFormat, String)> {
        let spec = self
            .catalog
            .get(model)
            .ok_or_else(|| Error::UnknownModel(model.to_string()))?;
        let format = if spec.format == ProviderFormat::Anthropic
            && lingua::is_bedrock_anthropic_model(model)
        {
            ProviderFormat::BedrockAnthropic
        } else {
            spec.format
        };
        let provider_alias = self
            .aliases
            .get(model)
            .cloned()
            .unwrap_or_else(|| format_identifier(format));
        Ok((spec, format, provider_alias))
    }
}

fn format_identifier(format: ProviderFormat) -> String {
    match format {
        ProviderFormat::OpenAI => "openai",
        ProviderFormat::Anthropic => "anthropic",
        ProviderFormat::BedrockAnthropic => "bedrock",
        ProviderFormat::Google => "google",
        ProviderFormat::Mistral => "mistral",
        ProviderFormat::Converse => "bedrock",
        ProviderFormat::Responses => "openai", // Responses API uses OpenAI provider
        ProviderFormat::Unknown => "unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ModelCatalog, ModelFlavor, ModelSpec};
    use crate::error::Error;
    use lingua::ProviderFormat;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn spec(model: &str, format: ProviderFormat) -> ModelSpec {
        ModelSpec {
            model: model.to_string(),
            format,
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
        }
    }

    #[test]
    fn resolve_returns_default_alias() {
        let mut catalog = ModelCatalog::empty();
        catalog.insert("model".into(), spec("model", ProviderFormat::OpenAI));
        let resolver = ModelResolver::new(Arc::new(catalog));

        let (_, format, alias) = resolver.resolve("model").expect("resolves");
        assert_eq!(format, ProviderFormat::OpenAI);
        assert_eq!(alias, "openai");
    }

    #[test]
    fn resolve_uses_custom_alias() {
        let mut catalog = ModelCatalog::empty();
        catalog.insert("model".into(), spec("model", ProviderFormat::Anthropic));
        let resolver = ModelResolver::new(Arc::new(catalog))
            .with_aliases(HashMap::from([("model".into(), "custom".into())]));

        let (_, format, alias) = resolver.resolve("model").expect("resolves");
        assert_eq!(format, ProviderFormat::Anthropic);
        assert_eq!(alias, "custom");
    }

    #[test]
    fn resolve_unknown_model_errors() {
        let resolver = ModelResolver::new(Arc::new(ModelCatalog::empty()));
        let err = resolver.resolve("missing").expect_err("unknown model");
        assert!(matches!(err, Error::UnknownModel(name) if name == "missing"));
    }

    #[test]
    fn resolve_bedrock_anthropic_routes_to_bedrock() {
        let model = "us.anthropic.claude-haiku-4-5-20251001-v1:0";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), spec(model, ProviderFormat::Anthropic));
        let resolver = ModelResolver::new(Arc::new(catalog));

        let (_, format, alias) = resolver.resolve(model).expect("resolves");
        assert_eq!(format, ProviderFormat::BedrockAnthropic);
        assert_eq!(alias, "bedrock");
    }

    #[test]
    fn resolve_non_bedrock_anthropic_stays_anthropic() {
        let model = "claude-sonnet-4-20250514";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), spec(model, ProviderFormat::Anthropic));
        let resolver = ModelResolver::new(Arc::new(catalog));

        let (_, format, alias) = resolver.resolve(model).expect("resolves");
        assert_eq!(format, ProviderFormat::Anthropic);
        assert_eq!(alias, "anthropic");
    }
}
