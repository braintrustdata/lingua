/*!
Decision request and response transformation.

A request passes through unchanged when the provider serves the client's format.
Otherwise it converts to universal decision types with the client format's
[`DecisionAdapter`] and from them with the provider's, and the response converts
back the same way.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{decision_adapter_for_format, DecisionAdapter};
use crate::processing::transform::{parse_json_value, TransformError};
use crate::universal::DecisionQuestion;
use bytes::Bytes;

/// How to convert a provider's decision response back to the client's format.
#[derive(Debug, Clone)]
pub enum DecisionResponseContext {
    /// The provider answers in the client's format.
    PassThrough { format: ProviderFormat },
    /// The provider answers in `target`, converted to `source` for the request's
    /// `questions`.
    Convert {
        source: ProviderFormat,
        target: ProviderFormat,
        questions: Vec<DecisionQuestion>,
    },
}

/// A decision request prepared for a provider format.
#[derive(Debug, Clone)]
pub struct DecisionRequestTransform {
    pub payload: Bytes,
    pub response_context: DecisionResponseContext,
}

fn source_adapter(format: ProviderFormat) -> Result<&'static dyn DecisionAdapter, TransformError> {
    decision_adapter_for_format(format).ok_or(TransformError::UnsupportedSourceFormat(format))
}

fn target_adapter(format: ProviderFormat) -> Result<&'static dyn DecisionAdapter, TransformError> {
    decision_adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))
}

/// Prepare a decision request body in `source` format for a provider serving `target`.
pub fn transform_decision_request(
    body: Bytes,
    source: ProviderFormat,
    target: ProviderFormat,
) -> Result<DecisionRequestTransform, TransformError> {
    let source_adapter = source_adapter(source)?;
    let target_adapter = target_adapter(target)?;
    let payload = parse_json_value(&body)?;
    if source == target {
        source_adapter.validate_request(payload)?;
        return Ok(DecisionRequestTransform {
            payload: body,
            response_context: DecisionResponseContext::PassThrough { format: source },
        });
    }
    let request = source_adapter.request_to_universal(payload)?;
    let questions = request.questions.clone();
    Ok(DecisionRequestTransform {
        payload: target_adapter.request_from_universal(request)?,
        response_context: DecisionResponseContext::Convert {
            source,
            target,
            questions,
        },
    })
}

/// Convert a provider's decision response body back to the client's format.
pub fn transform_decision_response(
    body: Bytes,
    context: &DecisionResponseContext,
) -> Result<Bytes, TransformError> {
    match context {
        DecisionResponseContext::PassThrough { format } => {
            source_adapter(*format)?.validate_response(parse_json_value(&body)?)?;
            Ok(body)
        }
        DecisionResponseContext::Convert {
            source,
            target,
            questions,
        } => {
            let response = target_adapter(*target)?
                .response_to_universal(parse_json_value(&body)?, questions)?;
            source_adapter(*source)?.response_from_universal(response)
        }
    }
}

#[cfg(all(test, feature = "openai"))]
mod tests {
    use super::*;
    use crate::serde_json;
    use crate::universal::UniversalUsage;

    const REQUEST: &str = r#"{"model":"gpt-6-luna","input":"I was charged twice.","questions":[{"type":"choice","name":"department","instructions":"Which department?","choices":[{"value":"billing"},{"value":"shipping"}]}]}"#;

    fn transform(target: ProviderFormat) -> Result<DecisionRequestTransform, TransformError> {
        transform_decision_request(
            Bytes::from_static(REQUEST.as_bytes()),
            ProviderFormat::Decisions,
            target,
        )
    }

    #[test]
    fn decisions_requests_pass_through_unchanged() {
        let transformed = transform(ProviderFormat::Decisions).unwrap();
        assert_eq!(transformed.payload, REQUEST.as_bytes());
        assert!(matches!(
            transformed.response_context,
            DecisionResponseContext::PassThrough {
                format: ProviderFormat::Decisions
            }
        ));
    }

    #[cfg(feature = "typesafe")]
    #[test]
    fn converts_decisions_through_system_one() {
        let transformed = transform(ProviderFormat::TypeSafe).unwrap();
        assert_eq!(
            std::str::from_utf8(&transformed.payload).unwrap(),
            r#"{"model":"gpt-6-luna","questions":{"department":{"type":"choice","criteria":{"billing":null,"shipping":null},"instructions":"Which department?"}},"state":"I was charged twice."}"#
        );
        let response = transform_decision_response(
            Bytes::from_static(
                br#"{"model":"jev-1.14.0","answers":{"department":{"type":"choice","choice":"billing","confidence":0.9,"probabilities":{"billing":0.95,"shipping":0.05}}},"usage":{"input_tokens":40,"output_tokens":2}}"#,
            ),
            &transformed.response_context,
        )
        .unwrap();
        let decision = crate::providers::openai::parse_decision_response(&response).unwrap();
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
