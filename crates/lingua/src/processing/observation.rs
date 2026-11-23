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
    PreGenerationRejection(&'static str),
    UsagePossible,
}

impl GenerationEvidence {
    pub fn rejection_reason(self, provider: Option<&str>) -> Option<&'static str> {
        match self {
            Self::PreGenerationRejection(reason)
                if reason != "openrouter_credit_limit" || provider == Some("openrouter") =>
            {
                Some(reason)
            }
            _ => None,
        }
    }
}

impl ResponseObservation {
    pub(crate) fn unparsed() -> Self {
        Self {
            generation: Some(GenerationEvidence::UsagePossible),
            ..Self::default()
        }
    }

    pub(crate) fn from_value(value: &Value, event_type: Option<&str>) -> Self {
        let cost = CostEnvelope::deserialize(value).ok();
        let generation = if cost.as_ref().is_some_and(|cost| cost.usage().is_none()) {
            match ObservationEnvelope::deserialize(value) {
                Ok(frame) => frame.generation(event_type),
                Err(_) => Some(GenerationEvidence::UsagePossible),
            }
        } else {
            Some(GenerationEvidence::UsagePossible)
        };
        Self {
            provider_cost_usd: cost.as_ref().and_then(|cost| cost.usage()?.cost()),
            generation,
            ..Self::default()
        }
    }

    pub(crate) fn record_response(&mut self, response: &UniversalResponse) {
        self.usage.clone_from(&response.usage);
        self.served = response.served();
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
            .or_else(|| match self.message.as_ref()? {
                MessageEnvelope::Usage(message) => message.usage.as_ref(),
                MessageEnvelope::Other(_) => None,
            })
    }
}

#[derive(Deserialize)]
struct ObservationEnvelope<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
    #[serde(borrow)]
    response: Option<ResponseEnvelope<'a>>,
    #[serde(borrow)]
    error: Option<ErrorEnvelope<'a>>,
    code: Option<ErrorCode>,
}

#[derive(Deserialize)]
struct ResponseEnvelope<'a> {
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
    Named(String),
    Number(u64),
    Other(serde::de::IgnoredAny),
}

impl ObservationEnvelope<'_> {
    fn generation(&self, event_type: Option<&str>) -> Option<GenerationEvidence> {
        use GenerationEvidence::UsagePossible;

        let response = self.response.as_ref();
        if response
            .and_then(|response| response.output.as_ref())
            .is_some_and(|output| !output.is_empty())
        {
            return Some(UsagePossible);
        }
        let error = match self.kind.or(event_type) {
            Some("response.created" | "response.in_progress" | "ping") => return None,
            Some("response.failed") => response.and_then(|response| response.error.as_ref()),
            Some("error") => self.error.as_ref(),
            None if self.error.is_some() => self.error.as_ref(),
            _ => return Some(UsagePossible),
        };
        let code = error
            .and_then(|error| error.code.as_ref())
            .or(self.code.as_ref());
        let openrouter_credit_limit =
            error
                .and_then(|error| error.message)
                .is_some_and(|message| {
                    message.starts_with("This request requires more credits, or fewer max_tokens.")
                });
        let rejection = match code {
            Some(ErrorCode::Named(code)) => match code.as_str() {
                "context_length_exceeded" => "context_length_exceeded",
                "billing_not_active" => "billing_not_active",
                "insufficient_quota" | "credit_balance_exhausted" => "insufficient_credits",
                "server_is_overloaded" => "provider_at_capacity",
                "server_error" if openrouter_credit_limit => "openrouter_credit_limit",
                _ => return Some(UsagePossible),
            },
            Some(ErrorCode::Number(402)) => "openrouter_credit_limit",
            _ => return Some(UsagePossible),
        };
        Some(GenerationEvidence::PreGenerationRejection(rejection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    // SSE terminators are neutral, but unreadable provider events cannot prove rejection.
    #[test]
    fn stream_end_markers_are_neutral_observations() {
        for (data, expected) in [
            (b"[DONE]".as_slice(), None),
            (b" [DONE]\n", None),
            (b"\n", None),
            (b"invalid", Some(GenerationEvidence::UsagePossible)),
        ] {
            let observation = crate::observe_stream_event(
                bytes::Bytes::copy_from_slice(data),
                crate::ProviderFormat::ChatCompletions,
                None,
            );
            assert_eq!(observation.generation, expected);
            assert_eq!(observation.usage, None);
        }
    }

    // Read partial cost envelopes without requiring a convertible provider response.
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

    // Lifecycle events are neutral; usage or output must override a reported rejection.
    #[test]
    fn observes_rejections_only_without_generation_evidence() {
        use GenerationEvidence::{PreGenerationRejection as Rejected, UsagePossible};

        for value in [
            json!({"type":"response.created","response":{"output":[],"usage":null}}),
            json!({"type":"ping"}),
        ] {
            assert_eq!(
                ResponseObservation::from_value(&value, None).generation,
                None
            );
        }
        for (value, event_type, reason) in [
            (
                json!({"code":"context_length_exceeded"}),
                Some("error"),
                "context_length_exceeded",
            ),
            (
                json!({"type":"response.failed","response":{"error":{"code":"billing_not_active"}}}),
                None,
                "billing_not_active",
            ),
            (
                json!({"error":{"code":"insufficient_quota"}}),
                None,
                "insufficient_credits",
            ),
            (
                json!({"type":"error","error":{"code":"server_is_overloaded"}}),
                None,
                "provider_at_capacity",
            ),
        ] {
            assert_eq!(
                ResponseObservation::from_value(&value, event_type).generation,
                Some(Rejected(reason))
            );
        }
        for value in [
            json!({"type":"error","error":{"code":"unknown"}}),
            json!({"type":"response.failed","response":{"usage":{},"error":{"code":"billing_not_active"}}}),
            json!({"type":"response.failed","response":{"output":[{}],"error":{"code":"billing_not_active"}}}),
            json!({"type":"response.output_text.delta","delta":"partial"}),
        ] {
            assert_eq!(
                ResponseObservation::from_value(&value, None).generation,
                Some(UsagePossible)
            );
        }
    }

    // OpenRouter-specific failures must not suppress unknown usage from other providers.
    #[test]
    fn scopes_openrouter_credit_rejections_to_the_provider() {
        for error in [
            json!({"code":402}),
            json!({"code":"server_error","message":"This request requires more credits, or fewer max_tokens. You requested up to 65536 tokens."}),
        ] {
            let value = json!({"type":"response.failed","response":{"error":error}});
            let Some(evidence) = ResponseObservation::from_value(&value, None).generation else {
                panic!("expected credit rejection");
            };
            assert_eq!(
                evidence.rejection_reason(Some("openrouter")),
                Some("openrouter_credit_limit")
            );
            assert_eq!(evidence.rejection_reason(Some("openai")), None);
        }
    }
}
