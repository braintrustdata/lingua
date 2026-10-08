#![cfg(feature = "anthropic")]

use lingua::processing::adapters::ProviderAdapter;
use lingua::providers::anthropic::generated::{
    CreateMessageParams, Thinking, ThinkingDisplayMode, ThinkingType,
};
use lingua::providers::anthropic::AnthropicAdapter;
use lingua::providers::bedrock_anthropic::BedrockAnthropicAdapter;
use lingua::serde_json;
use serde::Deserialize;

#[derive(Deserialize)]
struct ThinkingBodyView {
    thinking: Thinking,
    anthropic_version: String,
}

fn assert_captured_thinking_display_survives_adapters(input: &str, bedrock_model: &str) {
    let source: CreateMessageParams = serde_json::from_str(input).expect("valid captured request");
    let expected = source.thinking.expect("captured adaptive thinking");
    assert_eq!(expected.thinking_type, ThinkingType::Adaptive);
    assert_eq!(expected.display, Some(ThinkingDisplayMode::Summarized));

    let mut universal = AnthropicAdapter
        .request_to_universal(serde_json::from_str(input).expect("captured request JSON"))
        .expect("import captured Anthropic request");
    let round_trip: CreateMessageParams = serde_json::from_value(
        AnthropicAdapter
            .request_from_universal(&universal)
            .expect("Anthropic round-trip"),
    )
    .expect("typed Anthropic request");
    assert_eq!(round_trip.thinking, Some(expected.clone()));

    universal.model = Some(bedrock_model.to_string());
    let bedrock: ThinkingBodyView = serde_json::from_value(
        BedrockAnthropicAdapter::new()
            .request_from_universal(&universal)
            .expect("adapt captured request to Bedrock Anthropic"),
    )
    .expect("typed Bedrock thinking body");
    assert_eq!(bedrock.thinking, expected);
    assert_eq!(bedrock.anthropic_version, "bedrock-2023-05-31");
}

#[test]
fn captured_sonnet_5_adaptive_thinking_display_survives_adapters() {
    assert_captured_thinking_display_survives_adapters(
        include_str!("../../../payloads/snapshots/anthropicSonnet5AdaptiveThinkingDisplaySummarizedParam/anthropic/request.json"),
        "us.anthropic.claude-sonnet-5",
    );
}

#[test]
fn captured_opus_5_adaptive_thinking_display_survives_adapters() {
    assert_captured_thinking_display_survives_adapters(
        include_str!("../../../payloads/snapshots/anthropicOpus5AdaptiveThinkingDisplaySummarizedParam/anthropic/request.json"),
        "us.anthropic.claude-opus-5",
    );
}
