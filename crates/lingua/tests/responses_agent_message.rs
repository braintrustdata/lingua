use lingua::providers::openai::{
    generated::{InputItemContent, InputItemContentListType, InputItemType, InputParam},
    try_parse_responses,
};
use lingua::{serde_json, Bytes, ProviderFormat, TransformResult};

const SNAPSHOT: &[u8] = include_bytes!("fixtures/codex-subagent-agent-message.json");

#[test]
fn codex_agent_message_snapshot_parses_and_passes_through() {
    let value = serde_json::from_slice(SNAPSHOT).expect("snapshot is JSON");
    let parsed =
        try_parse_responses(&value).expect("Codex agent messages are valid Responses input");
    let Some(InputParam::InputItemArray(items)) = parsed.input else {
        panic!("snapshot contains Responses input items");
    };
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item.input_item_type, Some(InputItemType::AgentMessage));
    assert_eq!(item.author.as_deref(), Some("/root"));
    assert_eq!(item.recipient.as_deref(), Some("/root/trace_summaries"));
    let Some(InputItemContent::InputContentArray(content)) = &item.content else {
        panic!("agent message contains typed content");
    };
    assert_eq!(content.len(), 2);
    assert_eq!(
        content[0].input_content_type,
        InputItemContentListType::InputText
    );
    assert_eq!(content[0].text.as_deref(), Some("anon_1"));
    assert_eq!(
        content[1].input_content_type,
        InputItemContentListType::EncryptedContent
    );
    assert_eq!(content[1].encrypted_content.as_deref(), Some("anon_2"));

    let body = Bytes::from_static(SNAPSHOT);
    let result = lingua::transform_request(body.clone(), ProviderFormat::Responses, None)
        .expect("native Responses request passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Responses request must pass through");
    };
    assert_eq!(actual, body);
}

#[test]
fn codex_agent_message_cross_provider_conversion_is_explicitly_unsupported() {
    for target in [
        ProviderFormat::ChatCompletions,
        ProviderFormat::Anthropic,
        ProviderFormat::Google,
    ] {
        let error = lingua::transform_request(Bytes::from_static(SNAPSHOT), target, None)
            .expect_err("agent routing and encrypted messages have no portable mapping");
        assert!(error.to_string().contains("agent_message"), "{error}");
    }
}
