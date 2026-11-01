//! Checks captured decision-model payloads in `payloads/snapshots`.

#![cfg(all(feature = "openai", feature = "typesafe"))]

use lingua::processing::decision_adapter_for_format;
use lingua::providers::openai::decisions_generated::{
    DecisionInput, DecisionInputType, DecisionRequest, DecisionResponse,
};
use lingua::serde_json;
use lingua::universal::convert::TryFromLLM;
use lingua::universal::{UniversalDecisionRequest, UniversalDecisionResponse};
use lingua::ProviderFormat;
use std::fs;
use std::path::{Path, PathBuf};

/// Case names and snapshot directories that have captures for `provider`.
fn snapshot_dirs(provider: &str) -> Vec<(String, PathBuf)> {
    let snapshots = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../payloads/snapshots");
    let mut dirs: Vec<_> = fs::read_dir(&snapshots)
        .expect("payloads/snapshots should be readable")
        .flatten()
        .filter_map(|entry| {
            let dir = entry.path().join(provider);
            dir.join("response.json")
                .exists()
                .then(|| (entry.file_name().to_string_lossy().into_owned(), dir))
        })
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty(), "no {provider} snapshots found");
    dirs
}

fn read<T: serde::de::DeserializeOwned>(path: PathBuf) -> T {
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `request` with input items that omit `type` set to its default, `message`.
fn with_default_item_types(mut request: DecisionRequest) -> DecisionRequest {
    if let DecisionInput::DecisionInputMessageArray(items) = &mut request.input {
        for item in items {
            item.decision_input_message_type
                .get_or_insert(DecisionInputType::Message);
        }
    }
    request
}

#[test]
fn openai_decisions_snapshots_round_trip_through_universal() {
    for (case, dir) in snapshot_dirs("decisions") {
        let request: DecisionRequest = read(dir.join("request.json"));
        let universal = <UniversalDecisionRequest as TryFromLLM<_>>::try_from(request.clone())
            .unwrap_or_else(|e| panic!("{case} request to universal: {e}"));
        let round_trip: DecisionRequest = TryFromLLM::try_from(universal)
            .unwrap_or_else(|e| panic!("{case} request from universal: {e}"));
        assert_eq!(
            round_trip,
            with_default_item_types(request),
            "{case} request"
        );

        let response: DecisionResponse = read(dir.join("response.json"));
        let universal = <UniversalDecisionResponse as TryFromLLM<_>>::try_from(response.clone())
            .unwrap_or_else(|e| panic!("{case} response to universal: {e}"));
        let round_trip: DecisionResponse = TryFromLLM::try_from(universal)
            .unwrap_or_else(|e| panic!("{case} response from universal: {e}"));
        assert_eq!(round_trip, response, "{case} response");
    }
}

#[test]
fn typesafe_snapshots_are_valid_system_one_payloads() {
    let adapter = decision_adapter_for_format(ProviderFormat::TypeSafe).unwrap();
    for (case, dir) in snapshot_dirs("typesafe") {
        adapter
            .validate_request(read(dir.join("request.json")))
            .unwrap_or_else(|e| panic!("{case} request: {e}"));
        adapter
            .validate_response(read(dir.join("response.json")))
            .unwrap_or_else(|e| panic!("{case} response: {e}"));
    }
}
