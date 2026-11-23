use serde::Deserialize;

use crate::serde_json::Value;
use crate::universal::{ServedInfo, UniversalResponse, UniversalStreamChunk, UniversalUsage};

#[derive(Debug, Clone, Default)]
pub struct ResponseObservation {
    pub usage: Option<UniversalUsage>,
    pub served: ServedInfo,
    pub provider_cost_usd: Option<f64>,
    pub generation: Option<GenerationEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationEvidence {
    PreGenerationRejection(PreGenerationRejection),
    UsagePossible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreGenerationRejection {
    ContextLengthExceeded,
    BillingNotActive,
    InsufficientCredits,
    ProviderAtCapacity,
    OpenRouterCreditLimit,
}

impl PreGenerationRejection {
    pub fn reason(self, provider: Option<&str>) -> Option<&'static str> {
        Some(match self {
            Self::ContextLengthExceeded => "context_length_exceeded",
            Self::BillingNotActive => "billing_not_active",
            Self::InsufficientCredits => "insufficient_credits",
            Self::ProviderAtCapacity => "provider_at_capacity",
            Self::OpenRouterCreditLimit if provider == Some("openrouter") => {
                "openrouter_credit_limit"
            }
            Self::OpenRouterCreditLimit => return None,
        })
    }
}

impl ResponseObservation {
    pub(crate) fn from_value(value: &Value, event_type: Option<&str>) -> Self {
        let frame = match ObservationEnvelope::deserialize(value) {
            Ok(frame) => frame,
            Err(_) => {
                return Self {
                    provider_cost_usd: CostEnvelope::deserialize(value)
                        .ok()
                        .and_then(|envelope| envelope.cost()),
                    generation: Some(GenerationEvidence::UsagePossible),
                    ..Self::default()
                };
            }
        };
        Self {
            provider_cost_usd: frame.usage().and_then(UsageCost::cost),
            generation: frame.generation(event_type),
            ..Self::default()
        }
    }

    pub(crate) fn record_response(&mut self, response: &UniversalResponse) {
        self.usage.clone_from(&response.usage);
        self.served = ServedInfo {
            service_tier: response.served_service_tier,
            speed: response.served_speed,
            inference_geo: response.inference_geo.clone(),
        };
    }

    pub(crate) fn record_stream(&mut self, chunk: &UniversalStreamChunk) {
        self.usage.clone_from(&chunk.usage);
        self.served = chunk.served();
    }
}

#[derive(Deserialize)]
struct UsageCost {
    cost: Option<ReportedCost>,
}

impl UsageCost {
    fn cost(&self) -> Option<f64> {
        match &self.cost {
            Some(ReportedCost::Dollars(cost)) if cost.is_finite() && *cost >= 0.0 => Some(*cost),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ReportedCost {
    Dollars(f64),
    Other(serde::de::IgnoredAny),
}

#[derive(Deserialize)]
struct UsageEnvelope {
    usage: Option<UsageCost>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MessageEnvelope {
    Usage(UsageEnvelope),
    Other(serde::de::IgnoredAny),
}

impl MessageEnvelope {
    fn usage(&self) -> Option<&UsageCost> {
        match self {
            Self::Usage(message) => message.usage.as_ref(),
            Self::Other(_) => None,
        }
    }
}

#[derive(Deserialize)]
struct CostEnvelope {
    usage: Option<UsageCost>,
    response: Option<UsageEnvelope>,
    message: Option<MessageEnvelope>,
}

impl CostEnvelope {
    fn usage(&self) -> Option<&UsageCost> {
        self.usage
            .as_ref()
            .or_else(|| self.response.as_ref()?.usage.as_ref())
            .or_else(|| self.message.as_ref()?.usage())
    }

    fn cost(&self) -> Option<f64> {
        self.usage()?.cost()
    }
}

#[derive(Deserialize)]
struct ObservationEnvelope<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
    usage: Option<UsageCost>,
    #[serde(borrow)]
    response: Option<ResponseEnvelope<'a>>,
    message: Option<MessageEnvelope>,
    #[serde(borrow)]
    error: Option<ErrorEnvelope<'a>>,
    code: Option<ErrorCode>,
}

#[derive(Deserialize)]
struct ResponseEnvelope<'a> {
    usage: Option<UsageCost>,
    #[serde(borrow)]
    error: Option<ErrorEnvelope<'a>>,
    output: Option<Vec<serde::de::IgnoredAny>>,
}

#[derive(Deserialize)]
struct ErrorEnvelope<'a> {
    code: Option<ErrorCode>,
    #[serde(borrow)]
    message: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ErrorCode {
    Named(NamedErrorCode),
    Number(u64),
    Other(serde::de::IgnoredAny),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum NamedErrorCode {
    ContextLengthExceeded,
    BillingNotActive,
    InsufficientQuota,
    CreditBalanceExhausted,
    ServerIsOverloaded,
    ServerError,
    #[serde(other)]
    Other,
}

impl ObservationEnvelope<'_> {
    fn usage(&self) -> Option<&UsageCost> {
        self.usage
            .as_ref()
            .or_else(|| self.response.as_ref()?.usage.as_ref())
            .or_else(|| self.message.as_ref()?.usage())
    }

    fn generation(&self, event_type: Option<&str>) -> Option<GenerationEvidence> {
        use GenerationEvidence::UsagePossible;
        use PreGenerationRejection as Rejection;

        if self.usage().is_some()
            || self
                .response
                .as_ref()
                .and_then(|response| response.output.as_ref())
                .is_some_and(|output| !output.is_empty())
        {
            return Some(UsagePossible);
        }
        let error = match self.kind.or(event_type) {
            Some("response.created" | "response.in_progress" | "ping") => return None,
            Some("response.failed") => self
                .response
                .as_ref()
                .and_then(|response| response.error.as_ref()),
            Some("error") => self.error.as_ref(),
            None if self.error.is_some() => self.error.as_ref(),
            _ => return Some(UsagePossible),
        };
        let code = error
            .and_then(|error| error.code.as_ref())
            .or(self.code.as_ref());
        let rejection = match code {
            Some(ErrorCode::Named(NamedErrorCode::ContextLengthExceeded)) => {
                Rejection::ContextLengthExceeded
            }
            Some(ErrorCode::Named(NamedErrorCode::BillingNotActive)) => Rejection::BillingNotActive,
            Some(ErrorCode::Named(
                NamedErrorCode::InsufficientQuota | NamedErrorCode::CreditBalanceExhausted,
            )) => Rejection::InsufficientCredits,
            Some(ErrorCode::Named(NamedErrorCode::ServerIsOverloaded)) => {
                Rejection::ProviderAtCapacity
            }
            Some(ErrorCode::Named(NamedErrorCode::ServerError))
                if error
                    .and_then(|error| error.message)
                    .is_some_and(|message| {
                        message
                            .starts_with("This request requires more credits, or fewer max_tokens.")
                    }) =>
            {
                Rejection::OpenRouterCreditLimit
            }
            Some(ErrorCode::Number(402)) => Rejection::OpenRouterCreditLimit,
            _ => return Some(UsagePossible),
        };
        Some(GenerationEvidence::PreGenerationRejection(rejection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn stream_end_markers_are_neutral_observations() {
        for data in [b"[DONE]".as_slice(), b" [DONE]\n", b"\n"] {
            let observation = crate::observe_stream_event(
                bytes::Bytes::copy_from_slice(data),
                crate::ProviderFormat::ChatCompletions,
                None,
            )
            .unwrap();
            assert_eq!(observation.generation, None);
            assert_eq!(observation.usage, None);
        }
    }

    #[test]
    fn observes_cost_without_requiring_a_complete_response() {
        for value in [
            json!({"usage":{"cost":0.25}}),
            json!({"response":{"usage":{"cost":0.25}}}),
            json!({"message":{"usage":{"cost":0.25}}}),
            json!({"usage":{"cost":0.25},"response":{"output":"provider extension"}}),
        ] {
            let observation = ResponseObservation::from_value(&value, None);
            assert_eq!(observation.provider_cost_usd, Some(0.25));
            assert_eq!(
                observation.generation,
                Some(GenerationEvidence::UsagePossible)
            );
        }
        for cost in [json!(-1), json!("0.25"), Value::Null, json!({"cost":1})] {
            let value = json!({"usage":{"cost":cost}});
            assert_eq!(
                ResponseObservation::from_value(&value, None).provider_cost_usd,
                None
            );
        }
    }

    #[test]
    fn observes_rejections_only_without_generation_evidence() {
        use GenerationEvidence::{PreGenerationRejection as Rejected, UsagePossible};
        use PreGenerationRejection::*;

        for (value, event_type, expected) in [
            (
                json!({"type":"response.created","response":{"output":[],"usage":null}}),
                None,
                None,
            ),
            (json!({"type":"ping"}), None, None),
            (
                json!({"code":"context_length_exceeded"}),
                Some("error"),
                Some(Rejected(ContextLengthExceeded)),
            ),
            (
                json!({"type":"response.failed","response":{"error":{"code":"billing_not_active"}}}),
                None,
                Some(Rejected(BillingNotActive)),
            ),
            (
                json!({"error":{"code":"insufficient_quota"}}),
                None,
                Some(Rejected(InsufficientCredits)),
            ),
            (
                json!({"type":"error","error":{"code":"server_is_overloaded"}}),
                None,
                Some(Rejected(ProviderAtCapacity)),
            ),
            (
                json!({"type":"error","error":{"code":"unknown"}}),
                None,
                Some(UsagePossible),
            ),
            (
                json!({"type":"response.failed","response":{"usage":{},"error":{"code":"billing_not_active"}}}),
                None,
                Some(UsagePossible),
            ),
            (
                json!({"type":"response.failed","response":{"output":[{}],"error":{"code":"billing_not_active"}}}),
                None,
                Some(UsagePossible),
            ),
            (
                json!({"type":"response.output_text.delta","delta":"partial"}),
                None,
                Some(UsagePossible),
            ),
        ] {
            assert_eq!(
                ResponseObservation::from_value(&value, event_type).generation,
                expected,
                "{value}"
            );
        }
    }

    #[test]
    fn scopes_openrouter_credit_rejections_to_the_provider() {
        for error in [
            json!({"code":402}),
            json!({"code":"server_error","message":"This request requires more credits, or fewer max_tokens. You requested up to 65536 tokens."}),
        ] {
            let value = json!({"type":"response.failed","response":{"error":error}});
            let Some(GenerationEvidence::PreGenerationRejection(rejection)) =
                ResponseObservation::from_value(&value, None).generation
            else {
                panic!("expected credit rejection");
            };
            assert_eq!(
                rejection.reason(Some("openrouter")),
                Some("openrouter_credit_limit")
            );
            assert_eq!(rejection.reason(Some("openai")), None);
        }
    }
}
