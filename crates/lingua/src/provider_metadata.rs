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
struct ProviderUsageEnvelope {
    usage: Option<ProviderUsage>,
}

#[derive(serde::Deserialize)]
struct ProviderResponse {
    usage: Option<ProviderUsage>,
    error: Option<ProviderValue<ProviderError>>,
    output: Option<Vec<serde::de::IgnoredAny>>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ProviderValue<T> {
    Typed(T),
    Other(serde::de::IgnoredAny),
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
    Other(serde::de::IgnoredAny),
    #[default]
    #[serde(skip)]
    Missing,
}

#[derive(serde::Deserialize)]
pub struct ProviderFrame<'a> {
    #[serde(rename = "type")]
    kind: Option<&'a str>,
    usage: Option<ProviderUsage>,
    response: Option<ProviderResponse>,
    message: Option<ProviderValue<ProviderUsageEnvelope>>,
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

    pub fn stream_evidence(&self, event_type: Option<&str>) -> Option<StreamEvidence> {
        use StreamEvidence::*;
        let response = self.response.as_ref();
        if self.usage().is_some()
            || response
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
            .and_then(ProviderValue::typed)
            .map(|error| &error.code)
            .filter(|code| !matches!(code, ErrorCode::Missing))
            .or(self.code.as_ref());
        let reason = match code {
            Some(ErrorCode::Text(code)) => match code.as_str() {
                "context_length_exceeded" => "context_length_exceeded",
                "billing_not_active" => "billing_not_active",
                "insufficient_quota" | "credit_balance_exhausted" => "insufficient_credits",
                "server_is_overloaded" => "provider_at_capacity",
                "server_error"
                    if error
                        .and_then(ProviderValue::typed)
                        .and_then(|error| error.message.as_ref()?.typed())
                        .is_some_and(|message| {
                            message.starts_with(
                                "This request requires more credits, or fewer max_tokens.",
                            )
                        }) =>
                {
                    "openrouter_credit_limit"
                }
                _ => return Some(UsagePossible),
            },
            Some(ErrorCode::Number(402)) => "openrouter_credit_limit",
            _ => return Some(UsagePossible),
        };
        Some(PreGenerationRejection(reason))
    }
}

pub fn provider_cost(body: &[u8]) -> Option<f64> {
    #[derive(serde::Deserialize)]
    struct CostEnvelope {
        usage: Option<ProviderUsage>,
        response: Option<ProviderUsageEnvelope>,
        message: Option<ProviderValue<ProviderUsageEnvelope>>,
    }

    let envelope: CostEnvelope = serde_json::from_slice(body).ok()?;
    envelope
        .usage
        .as_ref()
        .or_else(|| envelope.response.as_ref()?.usage.as_ref())
        .or_else(|| envelope.message.as_ref()?.typed()?.usage.as_ref())?
        .cost()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        ] {
            let frame: ProviderFrame<'_> = serde_json::from_str(body).unwrap();
            assert_eq!(frame.cost(), cost);
            assert_eq!(frame.stream_evidence(None), Some(evidence));
        }
    }
}
