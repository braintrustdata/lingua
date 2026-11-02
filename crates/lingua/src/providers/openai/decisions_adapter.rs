/*!
OpenAI Decisions adapter for decision request and response transformation.
*/

use super::decisions_generated::{DecisionRequest, DecisionResponse};
use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{parse_payload, serialize_payload, DecisionAdapter};
use crate::processing::transform::{parse_json_value, TransformError};
use crate::serde_json::Value;
use crate::universal::convert::TryFromLLM;
use crate::universal::{DecisionQuestion, UniversalDecisionRequest, UniversalDecisionResponse};
use bytes::Bytes;

/// Adapter for the OpenAI Decisions API.
pub struct DecisionsAdapter;

/// Parse an OpenAI Decisions response body.
pub fn parse_decision_response(body: &[u8]) -> Result<DecisionResponse, TransformError> {
    parse_payload(parse_json_value(body)?)
}

impl DecisionAdapter for DecisionsAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Decisions
    }

    fn validate_request(&self, payload: Value) -> Result<(), TransformError> {
        parse_payload::<DecisionRequest>(payload).map(drop)
    }

    fn validate_response(&self, payload: Value) -> Result<(), TransformError> {
        parse_payload::<DecisionResponse>(payload).map(drop)
    }

    fn request_to_universal(
        &self,
        payload: Value,
    ) -> Result<UniversalDecisionRequest, TransformError> {
        let request: DecisionRequest = parse_payload(payload)?;
        <UniversalDecisionRequest as TryFromLLM<_>>::try_from(request)
            .map_err(|error| TransformError::ToUniversalFailed(error.to_string()))
    }

    fn request_from_universal(
        &self,
        request: UniversalDecisionRequest,
    ) -> Result<Bytes, TransformError> {
        let request: DecisionRequest = TryFromLLM::try_from(request)?;
        serialize_payload(&request)
    }

    fn response_to_universal(
        &self,
        payload: Value,
        questions: &[DecisionQuestion],
    ) -> Result<UniversalDecisionResponse, TransformError> {
        let response: DecisionResponse = parse_payload(payload)?;
        <UniversalDecisionResponse as TryFromLLM<_>>::try_from(response)
            .map_err(|error| TransformError::ToUniversalFailed(error.to_string()))
            .and_then(|response| {
                crate::processing::decisions::validate_decision_response(response, questions)
            })
    }

    fn response_from_universal(
        &self,
        response: UniversalDecisionResponse,
    ) -> Result<Bytes, TransformError> {
        let response: DecisionResponse = TryFromLLM::try_from(response)?;
        serialize_payload(&response)
    }
}
