/*!
TypeSafe System One adapter for decision request and response transformation.

System One serves as a decision target: universal requests convert to System One
requests, and System One responses convert back. System One payloads cannot be
converted to other decision formats.
*/

use super::convert::system_one_response_to_universal;
use super::generated::{SystemOneRequest, SystemOneResponse};
use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{parse_payload, serialize_payload, DecisionAdapter};
use crate::processing::transform::TransformError;
use crate::serde_json::Value;
use crate::universal::convert::TryFromLLM;
use crate::universal::{DecisionQuestion, UniversalDecisionRequest, UniversalDecisionResponse};
use bytes::Bytes;

/// Adapter for the TypeSafe System One API.
pub struct TypeSafeAdapter;

impl DecisionAdapter for TypeSafeAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::TypeSafe
    }

    fn validate_request(&self, payload: Value) -> Result<(), TransformError> {
        parse_payload::<SystemOneRequest>(payload).map(drop)
    }

    fn validate_response(&self, payload: Value) -> Result<(), TransformError> {
        parse_payload::<SystemOneResponse>(payload).map(drop)
    }

    fn request_to_universal(
        &self,
        _payload: Value,
    ) -> Result<UniversalDecisionRequest, TransformError> {
        Err(TransformError::UnsupportedSourceFormat(
            ProviderFormat::TypeSafe,
        ))
    }

    fn request_from_universal(
        &self,
        request: UniversalDecisionRequest,
    ) -> Result<Bytes, TransformError> {
        let request: SystemOneRequest = TryFromLLM::try_from(request)?;
        serialize_payload(&request)
    }

    fn response_to_universal(
        &self,
        payload: Value,
        questions: &[DecisionQuestion],
    ) -> Result<UniversalDecisionResponse, TransformError> {
        system_one_response_to_universal(parse_payload(payload)?, questions)
            .map_err(|error| TransformError::ToUniversalFailed(error.to_string()))
            .and_then(|response| {
                crate::processing::decisions::validate_decision_response(response, questions)
            })
    }

    fn response_from_universal(
        &self,
        _response: UniversalDecisionResponse,
    ) -> Result<Bytes, TransformError> {
        Err(TransformError::UnsupportedSourceFormat(
            ProviderFormat::TypeSafe,
        ))
    }
}
