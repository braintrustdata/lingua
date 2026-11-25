use serde::de::IgnoredAny;

use crate::serde_json;

#[derive(serde::Deserialize)]
struct ProviderUsage {
    cost: Option<ProviderValue<serde_json::Number>>,
}

impl ProviderUsage {
    fn cost(&self) -> Option<f64> {
        self.cost
            .as_ref()?
            .typed()?
            .as_f64()
            .filter(|cost| cost.is_finite() && *cost >= 0.0)
    }
}

#[derive(serde::Deserialize)]
struct UsageField<T> {
    usage: Option<T>,
}

#[derive(serde::Deserialize)]
struct ProviderResponse {
    usage: Option<ProviderUsage>,
    error: Option<ProviderValue<ProviderError>>,
    output: Option<Vec<IgnoredAny>>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ProviderValue<T> {
    Typed(T),
    Other(IgnoredAny),
}

impl<T> ProviderValue<T> {
    fn typed(&self) -> Option<&T> {
        match self {
            Self::Typed(value) => Some(value),
            Self::Other(_) => None,
        }
    }
}

#[derive(serde::Deserialize)]
struct ProviderError {
    #[serde(default)]
    code: ErrorCode,
    message: Option<ProviderValue<String>>,
}

#[derive(Default, serde::Deserialize)]
#[serde(untagged)]
enum ErrorCode {
    Text(String),
    Number(u64),
    Other(IgnoredAny),
    #[default]
    #[serde(skip)]
    Missing,
}

#[derive(serde::Deserialize)]
pub struct ProviderFrame<'a> {
    #[serde(rename = "type")]
    kind: Option<&'a str>,
    sequence_number: Option<ProviderValue<u64>>,
    usage: Option<ProviderUsage>,
    response: Option<ProviderResponse>,
    message: Option<ProviderValue<UsageField<ProviderUsage>>>,
    metadata: Option<UsageField<IgnoredAny>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<IgnoredAny>,
    #[serde(rename = "amazon-bedrock-invocationMetrics")]
    invocation_metrics: Option<IgnoredAny>,
    error: Option<ProviderValue<ProviderError>>,
    code: Option<ErrorCode>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StreamEvidence {
    PreGenerationRejection(&'static str),
    UsagePossible,
}

impl ProviderFrame<'_> {
    fn usage(&self) -> Option<&ProviderUsage> {
        self.usage
            .as_ref()
            .or_else(|| self.response.as_ref()?.usage.as_ref())
            .or_else(|| self.message.as_ref()?.typed()?.usage.as_ref())
    }

    pub fn cost(&self) -> Option<f64> {
        self.usage()?.cost()
    }

    pub fn has_usage(&self) -> bool {
        self.usage().is_some()
            || self
                .metadata
                .as_ref()
                .is_some_and(|metadata| metadata.usage.is_some())
            || self.usage_metadata.is_some()
            || self.invocation_metrics.is_some()
    }

    pub fn stream_evidence(&self, event_type: Option<&str>) -> Option<StreamEvidence> {
        use StreamEvidence::*;
        let response = self.response.as_ref();
        if self.has_usage()
            || response
                .and_then(|response| response.output.as_ref())
                .is_some_and(|output| !output.is_empty())
        {
            return Some(UsagePossible);
        }
        let event_type = self.kind.or(event_type);
        let error = match event_type {
            Some("response.created" | "response.in_progress" | "ping") => return None,
            Some("response.failed") => response.and_then(|response| response.error.as_ref()),
            Some("error") => self.error.as_ref(),
            None if self.error.is_some() => self.error.as_ref(),
            _ => return Some(UsagePossible),
        }
        .and_then(ProviderValue::typed);
        let code = error
            .map(|error| &error.code)
            .filter(|code| !matches!(code, ErrorCode::Missing))
            .or(self.code.as_ref());
        let reason = match code {
            Some(ErrorCode::Text(code)) => match code.as_str() {
                "context_length_exceeded" => Some("context_length_exceeded"),
                "billing_not_active" => Some("billing_not_active"),
                "insufficient_quota"
                | "organization_spend_limit_exceeded"
                | "organization_usage_limit_exceeded"
                | "project_spend_limit_exceeded"
                | "credit_balance_exhausted" => Some("insufficient_credits"),
                "bio_policy" => Some("bio_policy"),
                "server_is_overloaded" => Some("provider_at_capacity"),
                "server_error"
                    if error
                        .and_then(|error| error.message.as_ref()?.typed())
                        .is_some_and(|message| {
                            message.starts_with(
                                "This request requires more credits, or fewer max_tokens.",
                            )
                        }) =>
                {
                    Some("openrouter_credit_limit")
                }
                _ => None,
            },
            Some(ErrorCode::Number(402)) => Some("openrouter_credit_limit"),
            _ => None,
        };
        let reason = reason.or_else(|| {
            (event_type == Some("error")
                && matches!(self.sequence_number, Some(ProviderValue::Typed(_))))
            .then_some("provider_stream_error")
        });
        Some(reason.map_or(UsagePossible, PreGenerationRejection))
    }
}

fn provider_cost(body: &[u8]) -> Option<f64> {
    #[derive(serde::Deserialize)]
    struct CostEnvelope {
        usage: Option<ProviderUsage>,
        response: Option<UsageField<ProviderUsage>>,
        message: Option<ProviderValue<UsageField<ProviderUsage>>>,
    }

    let envelope: CostEnvelope = serde_json::from_slice(body).ok()?;
    envelope
        .usage
        .as_ref()
        .or_else(|| envelope.response.as_ref()?.usage.as_ref())
        .or_else(|| envelope.message.as_ref()?.typed()?.usage.as_ref())?
        .cost()
}

pub struct UpstreamUsage {
    pub seen: bool,
    pub cost: Option<f64>,
}

pub fn inspect_upstream_usage(body: &[u8]) -> UpstreamUsage {
    match serde_json::from_slice::<ProviderFrame>(body) {
        Ok(frame) => UpstreamUsage {
            seen: frame.has_usage(),
            cost: frame.cost(),
        },
        Err(_) => inspect_unrecognized_usage(body),
    }
}

pub fn inspect_unrecognized_usage(body: &[u8]) -> UpstreamUsage {
    #[derive(serde::Deserialize)]
    struct UsagePresence {
        usage: Option<IgnoredAny>,
        response: Option<ProviderValue<UsageField<IgnoredAny>>>,
        message: Option<ProviderValue<UsageField<IgnoredAny>>>,
        metadata: Option<ProviderValue<UsageField<IgnoredAny>>>,
        #[serde(rename = "usageMetadata")]
        usage_metadata: Option<IgnoredAny>,
        #[serde(rename = "amazon-bedrock-invocationMetrics")]
        invocation_metrics: Option<IgnoredAny>,
    }

    let cost = provider_cost(body);
    let seen = serde_json::from_slice::<UsagePresence>(body).is_ok_and(|frame| {
        frame.usage.is_some()
            || [frame.response, frame.message, frame.metadata]
                .iter()
                .any(|container| {
                    container
                        .as_ref()
                        .and_then(ProviderValue::typed)
                        .is_some_and(|usage| usage.usage.is_some())
                })
            || frame.usage_metadata.is_some()
            || frame.invocation_metrics.is_some()
    });
    UpstreamUsage {
        seen: seen || cost.is_some(),
        cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Usage diagnostics must survive even when the provider's frame cannot be decoded.
    #[test]
    fn sees_usage_in_unrecognized_frames() {
        for (body, seen, cost) in [
            (
                r#"{"response":{"output":"extension","usage":{"cost":0.25}}}"#,
                true,
                Some(0.25),
            ),
            (r#"{"usage":"extension"}"#, true, None),
            (r#"{"type":1,"metadata":{"usage":{}}}"#, true, None),
            (r#"{"type":1,"usageMetadata":{}}"#, true, None),
            (
                r#"{"type":1,"amazon-bedrock-invocationMetrics":{}}"#,
                true,
                None,
            ),
            (r#"{"type":1,"usageMetadata":null}"#, false, None),
        ] {
            let usage = inspect_upstream_usage(body.as_bytes());
            assert_eq!(usage.seen, seen, "{body}");
            assert_eq!(usage.cost, cost, "{body}");
        }
    }

    // Partial provider fields must keep the existing cost and error-code precedence rules.
    #[test]
    fn partial_metadata_preserves_cost_and_rejection_semantics() {
        use StreamEvidence::{PreGenerationRejection as Rejected, UsagePossible};

        for (body, cost, evidence) in [
            (
                r#"{"usage":{"cost":0.25},"message":"provider extension"}"#,
                Some(0.25),
                UsagePossible,
            ),
            (
                r#"{"type":"error","code":"billing_not_active","error":{"code":null}}"#,
                None,
                UsagePossible,
            ),
            (
                r#"{"type":"error","code":"billing_not_active","error":{}}"#,
                None,
                Rejected("billing_not_active"),
            ),
            (
                r#"{"type":"error","error":{"code":"billing_not_active","message":1}}"#,
                None,
                Rejected("billing_not_active"),
            ),
            (
                r#"{"type":"error","error":{"code":402}}"#,
                None,
                Rejected("openrouter_credit_limit"),
            ),
            (
                r#"{"type":"error","error":{"code":402.0}}"#,
                None,
                UsagePossible,
            ),
            (
                r#"{"type":"error","sequence_number":2,"error":{"type":"too_many_requests","code":"no_capacity","message":"Capacity unavailable"}}"#,
                None,
                Rejected("provider_stream_error"),
            ),
            (
                r#"{"type":"error","sequence_number":2,"code":null,"message":"Request failed"}"#,
                None,
                Rejected("provider_stream_error"),
            ),
            (
                r#"{"type":"error","error":{"type":"server_error","message":"Request failed"}}"#,
                None,
                UsagePossible,
            ),
            (
                r#"{"type":"error","sequence_number":2,"error":{"type":"server_error","code":"internal_error"},"usage":{"cost":0.25}}"#,
                Some(0.25),
                UsagePossible,
            ),
            (
                r#"{"type":"response.failed","response":{"output":[],"usage":null,"error":{"code":"organization_spend_limit_exceeded","type":"insufficient_quota"}}}"#,
                None,
                Rejected("insufficient_credits"),
            ),
            (
                r#"{"type":"response.failed","response":{"output":[],"usage":null,"error":{"code":"organization_usage_limit_exceeded","type":"insufficient_quota"}}}"#,
                None,
                Rejected("insufficient_credits"),
            ),
            (
                r#"{"type":"response.failed","response":{"output":[],"usage":null,"error":{"code":"project_spend_limit_exceeded","type":"insufficient_quota"}}}"#,
                None,
                Rejected("insufficient_credits"),
            ),
            (
                r#"{"type":"response.failed","response":{"output":[],"usage":null,"error":{"code":"bio_policy","type":"invalid_request_error"}}}"#,
                None,
                Rejected("bio_policy"),
            ),
            (
                r#"{"type":"response.failed","response":{"output":[],"usage":null,"error":{"code":"server_error"}}}"#,
                None,
                UsagePossible,
            ),
        ] {
            let frame: ProviderFrame<'_> = serde_json::from_str(body).unwrap();
            assert_eq!(frame.cost(), cost);
            assert_eq!(frame.stream_evidence(None), Some(evidence));
        }
    }
}
