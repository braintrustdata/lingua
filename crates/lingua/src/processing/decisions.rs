/*!
Decision request and response transformation.

Decision requests use the OpenAI Decisions format. They pass through unchanged to
providers that serve that format. For TypeSafe System One, requests convert
through universal decision types, and responses convert back the same way.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::transform::{parse_json_value, TransformError};
use crate::providers::openai::decisions_generated::{DecisionRequest, DecisionResponse};
use crate::serde_json;
use bytes::Bytes;
use serde::de::DeserializeOwned;

#[cfg(feature = "typesafe")]
use crate::providers::typesafe::{
    system_one_response_to_universal, universal_to_system_one, SystemOneDecisionContext,
    SystemOneResponse,
};
#[cfg(feature = "typesafe")]
use crate::universal::convert::TryFromLLM;
#[cfg(feature = "typesafe")]
use crate::universal::{UniversalDecisionRequest, UniversalDecisionResponse};

/// How to convert a provider's decision response back to the Decisions format.
#[derive(Debug, Clone, PartialEq)]
pub enum DecisionResponseContext {
    PassThrough,
    #[cfg(feature = "typesafe")]
    SystemOne(SystemOneDecisionContext),
}

/// A decision request prepared for a provider format.
#[derive(Debug, Clone)]
pub struct DecisionRequestTransform {
    pub payload: Bytes,
    pub response_context: DecisionResponseContext,
}

/// Parse a Decisions request body.
pub fn parse_decision_request(body: &[u8]) -> Result<DecisionRequest, TransformError> {
    parse_typed(body)
}

/// Parse a Decisions response body.
pub fn parse_decision_response(body: &[u8]) -> Result<DecisionResponse, TransformError> {
    parse_typed(body)
}

fn parse_typed<T: DeserializeOwned>(body: &[u8]) -> Result<T, TransformError> {
    serde_json::from_value(parse_json_value(body)?)
        .map_err(|error| TransformError::DeserializationFailed(error.to_string()))
}

/// Prepare a Decisions request body for `target`.
pub fn transform_decision_request(
    body: Bytes,
    target: ProviderFormat,
) -> Result<DecisionRequestTransform, TransformError> {
    match target {
        ProviderFormat::Decisions => {
            parse_decision_request(&body)?;
            Ok(DecisionRequestTransform {
                payload: body,
                response_context: DecisionResponseContext::PassThrough,
            })
        }
        #[cfg(feature = "typesafe")]
        ProviderFormat::TypeSafe => {
            let request = parse_decision_request(&body)?;
            let (system_one, context) =
                <UniversalDecisionRequest as TryFromLLM<_>>::try_from(request)
                    .and_then(universal_to_system_one)
                    .map_err(|error| TransformError::ValidationFailed {
                        target,
                        reason: error.to_string(),
                    })?;
            let payload = serde_json::to_vec(&system_one)
                .map_err(|error| TransformError::SerializationFailed(error.to_string()))?;
            Ok(DecisionRequestTransform {
                payload: Bytes::from(payload),
                response_context: DecisionResponseContext::SystemOne(context),
            })
        }
        other => Err(TransformError::UnsupportedTargetFormat(other)),
    }
}

/// Convert a provider's decision response body to the Decisions format.
pub fn transform_decision_response(
    body: Bytes,
    context: &DecisionResponseContext,
) -> Result<Bytes, TransformError> {
    match context {
        DecisionResponseContext::PassThrough => {
            parse_decision_response(&body)?;
            Ok(body)
        }
        #[cfg(feature = "typesafe")]
        DecisionResponseContext::SystemOne(context) => {
            let response: SystemOneResponse = parse_typed(&body)?;
            let decision: DecisionResponse = system_one_response_to_universal(response, context)
                .and_then(<DecisionResponse as TryFromLLM<UniversalDecisionResponse>>::try_from)
                .map_err(|error| TransformError::ValidationFailed {
                    target: ProviderFormat::Decisions,
                    reason: error.to_string(),
                })?;
            serde_json::to_vec(&decision)
                .map(Bytes::from)
                .map_err(|error| TransformError::SerializationFailed(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json;
    use crate::universal::UniversalUsage;

    const REQUEST: &str = r#"{"model":"gpt-6-luna","input":"I was charged twice.","questions":[{"type":"choice","name":"department","instructions":"Which department?","choices":[{"value":"billing"},{"value":"shipping"}]}]}"#;

    #[test]
    fn decisions_requests_pass_through_unchanged() {
        let transformed = transform_decision_request(
            Bytes::from_static(REQUEST.as_bytes()),
            ProviderFormat::Decisions,
        )
        .unwrap();
        assert_eq!(transformed.payload, REQUEST.as_bytes());
        assert_eq!(
            transformed.response_context,
            DecisionResponseContext::PassThrough
        );
    }

    #[test]
    fn rejects_bodies_that_are_not_decision_requests() {
        for target in [ProviderFormat::Decisions, ProviderFormat::TypeSafe] {
            assert!(matches!(
                transform_decision_request(
                    Bytes::from_static(br#"{"model":"m","messages":[]}"#),
                    target
                ),
                Err(TransformError::DeserializationFailed(_))
            ));
        }
        assert!(matches!(
            transform_decision_request(
                Bytes::from_static(REQUEST.as_bytes()),
                ProviderFormat::ChatCompletions
            ),
            Err(TransformError::UnsupportedTargetFormat(
                ProviderFormat::ChatCompletions
            ))
        ));
    }

    #[cfg(feature = "typesafe")]
    #[test]
    fn converts_decisions_through_system_one() {
        let transformed = transform_decision_request(
            Bytes::from_static(REQUEST.as_bytes()),
            ProviderFormat::TypeSafe,
        )
        .unwrap();
        assert_eq!(
            std::str::from_utf8(&transformed.payload).unwrap(),
            r#"{"model":"gpt-6-luna","state":"I was charged twice.","questions":{"department":{"type":"choice","instructions":"Which department?","criteria":{"billing":null,"shipping":null}}}}"#
        );
        let response = transform_decision_response(
            Bytes::from_static(
                br#"{"model":"jev-1.14.0","answers":{"department":{"type":"choice","choice":"billing","confidence":0.9,"probabilities":{"billing":0.95,"shipping":0.05}}},"usage":{"input_tokens":40,"output_tokens":2}}"#,
            ),
            &transformed.response_context,
        )
        .unwrap();
        let decision = parse_decision_response(&response).unwrap();
        assert_eq!(decision.model, "jev-1.14.0");
        assert_eq!(decision.usage.total_tokens, 42);
        assert!(matches!(
            transform_decision_response(
                Bytes::from_static(br#"{"answers":[]}"#),
                &transformed.response_context
            ),
            Err(TransformError::DeserializationFailed(_))
        ));
    }

    #[test]
    fn parses_decision_usage() {
        let usage = UniversalUsage::from_provider_value(
            &serde_json::json!({
                "input_tokens": 1200,
                "input_tokens_details": {"cached_tokens": 1024, "cache_write_tokens": 0},
                "output_tokens": 0,
                "output_tokens_details": {"reasoning_tokens": 0},
                "total_tokens": 1200
            }),
            ProviderFormat::Decisions,
        );
        assert_eq!(usage.prompt_tokens, Some(1200));
        assert_eq!(usage.prompt_cached_tokens, Some(1024));
        assert_eq!(usage.completion_tokens, Some(0));
        let usage = UniversalUsage::from_provider_value(
            &serde_json::json!({"input_tokens": 40, "output_tokens": 2}),
            ProviderFormat::TypeSafe,
        );
        assert_eq!(usage.prompt_tokens, Some(40));
        assert_eq!(usage.total_tokens, Some(42));
        assert_eq!(
            usage.to_provider_value(ProviderFormat::TypeSafe),
            serde_json::json!({"input_tokens": 40, "output_tokens": 2})
        );
    }
}
