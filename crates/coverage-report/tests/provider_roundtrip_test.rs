use lingua::processing::adapters::ProviderAdapter;
use lingua::providers::openai::ResponsesAdapter;
use lingua::serde_json::{self, Value};

#[test]
fn responses_request_provider_roundtrip_preserves_reasoning_provider_fields() {
    let original: Value = serde_json::from_slice(include_bytes!(
        "../../../payloads/provider-roundtrip/responses-reasoning-provider-extras-request.json"
    ))
    .expect("fixture should parse");

    let adapter = ResponsesAdapter;
    let universal = adapter
        .request_to_universal(original.clone())
        .expect("fixture should convert to universal");
    let reconstructed = adapter
        .request_from_universal(&universal)
        .expect("fixture should convert back to Responses");

    assert_eq!(reconstructed, original);
}
