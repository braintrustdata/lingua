use crate::catalog::ModelCatalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailoverRouteCandidate {
    pub alias: String,
    pub provider_match_aliases: Vec<String>,
    pub is_configured: bool,
    pub custom_model_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailoverRoutePlan {
    pub source_alias: String,
    pub fallback_aliases: Vec<String>,
}

pub fn plan_provider_failover_routes(
    catalog: &ModelCatalog,
    model: &str,
    candidates: Vec<FailoverRouteCandidate>,
) -> Option<FailoverRoutePlan> {
    let routes = candidates
        .into_iter()
        .filter(|candidate| {
            candidate.is_configured
                && (candidate.custom_model_supported
                    || model_supports_any_provider_alias(
                        catalog,
                        model,
                        &candidate.provider_match_aliases,
                    ))
        })
        .map(|candidate| candidate.alias)
        .collect::<Vec<_>>();

    let mut routes = routes.into_iter();
    let source_alias = routes.next()?;
    let fallback_aliases = routes.collect::<Vec<_>>();
    if fallback_aliases.is_empty() {
        return None;
    }

    Some(FailoverRoutePlan {
        source_alias,
        fallback_aliases,
    })
}

fn model_supports_any_provider_alias(
    catalog: &ModelCatalog,
    model: &str,
    provider_aliases: &[String],
) -> bool {
    let Some(spec) = catalog.get(model) else {
        return true;
    };
    !spec.available_providers.is_empty()
        && spec.available_providers.iter().any(|available| {
            provider_aliases
                .iter()
                .any(|alias| available.eq_ignore_ascii_case(alias))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ModelCatalog, ModelFlavor, ModelSpec};
    use lingua::ProviderFormat;

    fn candidate(
        alias: &str,
        match_aliases: &[&str],
        is_configured: bool,
    ) -> FailoverRouteCandidate {
        FailoverRouteCandidate {
            alias: alias.to_string(),
            provider_match_aliases: match_aliases
                .iter()
                .map(|alias| alias.to_string())
                .collect(),
            is_configured,
            custom_model_supported: false,
        }
    }

    fn custom_candidate(alias: &str) -> FailoverRouteCandidate {
        FailoverRouteCandidate {
            alias: alias.to_string(),
            provider_match_aliases: vec![alias.to_string()],
            is_configured: true,
            custom_model_supported: true,
        }
    }

    fn catalog() -> ModelCatalog {
        let mut catalog = ModelCatalog::empty();
        catalog.insert(
            "model".to_string(),
            ModelSpec {
                model: "model".to_string(),
                format: ProviderFormat::ChatCompletions,
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
                available_providers: vec![
                    "OPENAI_API_KEY".to_string(),
                    "ANTHROPIC_API_KEY".to_string(),
                ],
            },
        );
        catalog
    }

    #[test]
    fn plan_skips_unconfigured_aliases_until_configured_fallback() {
        let plan = plan_provider_failover_routes(
            &catalog(),
            "model",
            vec![
                candidate("openai", &["openai", "OPENAI_API_KEY"], true),
                candidate("missing", &["missing"], false),
                candidate("anthropic", &["anthropic", "ANTHROPIC_API_KEY"], true),
            ],
        )
        .expect("plan");

        assert_eq!(plan.source_alias, "openai");
        assert_eq!(plan.fallback_aliases, vec!["anthropic".to_string()]);
    }

    #[test]
    fn plan_returns_none_without_usable_fallback() {
        assert!(plan_provider_failover_routes(
            &catalog(),
            "model",
            vec![
                candidate("openai", &["openai", "OPENAI_API_KEY"], true),
                candidate("missing", &["missing"], false),
            ],
        )
        .is_none());
    }

    #[test]
    fn plan_allows_secret_defined_custom_model() {
        let plan = plan_provider_failover_routes(
            &catalog(),
            "not-in-catalog",
            vec![
                custom_candidate("custom-primary"),
                candidate("anthropic", &["anthropic", "ANTHROPIC_API_KEY"], true),
            ],
        )
        .expect("plan");

        assert_eq!(plan.source_alias, "custom-primary");
        assert_eq!(plan.fallback_aliases, vec!["anthropic".to_string()]);
    }
}
