//! Property-based roundtrip tests for Lingua cross-provider conversions.
//!
//! Test cases come from two sources:
//! 1. **Saved cases** (`roundtrip_cases.json`): Hand-written provider JSON payloads in any
//!    format (OpenAI, Anthropic, Google, etc.). When proptest finds a novel failure, copy
//!    the provider JSON into this file to make it a permanent regression test.
//! 2. **Proptest strategies**: Random `UniversalRequest` generation for coverage.
//!
//! The roundtrip harness is format-agnostic: it accepts either a `UniversalRequest` directly
//! or a `(ProviderFormat, serde_json::Value)` pair and runs the same assertions.

use lingua::processing::adapter_for_format;
use lingua::serde_json::{self, json, Value};
use lingua::universal::message::*;
use lingua::{ProviderFormat, UniversalParams, UniversalRequest};
use std::path::PathBuf;

// ============================================================================
// Roundtrip harness (test-case-agnostic)
// ============================================================================

/// Result of a roundtrip conversion through a provider.
#[derive(Debug)]
struct RoundtripResult {
    /// The provider JSON produced by `request_from_universal`
    provider_json: Value,
    /// The universal request recovered by `request_to_universal`
    roundtripped: UniversalRequest,
}

/// Harness for testing roundtrip conversions. Operates on any `UniversalRequest`.
struct RoundtripHarness;

impl RoundtripHarness {
    /// Convert a provider-native JSON payload to Universal first, then roundtrip
    /// through the same or different providers.
    fn from_provider_json(
        source_format: ProviderFormat,
        payload: Value,
    ) -> Result<UniversalRequest, String> {
        let adapter = adapter_for_format(source_format)
            .ok_or_else(|| format!("No adapter for {:?}", source_format))?;
        adapter
            .request_to_universal(payload)
            .map_err(|e| format!("request_to_universal({:?}): {}", source_format, e))
    }

    /// Convert Universal -> Provider -> Universal and return both intermediate + result.
    fn self_roundtrip(
        req: &UniversalRequest,
        format: ProviderFormat,
    ) -> Result<RoundtripResult, String> {
        let adapter =
            adapter_for_format(format).ok_or_else(|| format!("No adapter for {:?}", format))?;

        let mut req = req.clone();
        if req.model.is_none() {
            req.model = Some(default_model(format).to_string());
        }
        adapter.apply_defaults(&mut req);

        let provider_json = adapter
            .request_from_universal(&req)
            .map_err(|e| format!("request_from_universal({:?}): {}", format, e))?;

        let roundtripped = adapter
            .request_to_universal(provider_json.clone())
            .map_err(|e| format!("request_to_universal({:?}): {}", format, e))?;

        Ok(RoundtripResult {
            provider_json,
            roundtripped,
        })
    }

    /// Convert Universal -> Source -> Universal -> Target -> Universal.
    fn cross_provider(
        req: &UniversalRequest,
        source: ProviderFormat,
        target: ProviderFormat,
    ) -> Result<RoundtripResult, String> {
        let source_result = Self::self_roundtrip(req, source)?;

        let target_adapter =
            adapter_for_format(target).ok_or_else(|| format!("No adapter for {:?}", target))?;

        let mut universal_for_target = source_result.roundtripped;
        universal_for_target.model = Some(default_model(target).to_string());
        target_adapter.apply_defaults(&mut universal_for_target);

        let target_json = target_adapter
            .request_from_universal(&universal_for_target)
            .map_err(|e| format!("request_from_universal({:?}): {}", target, e))?;

        let roundtripped = target_adapter
            .request_to_universal(target_json.clone())
            .map_err(|e| format!("request_to_universal({:?}): {}", target, e))?;

        Ok(RoundtripResult {
            provider_json: target_json,
            roundtripped,
        })
    }
}

// ============================================================================
// Content extraction (lossy-conversion-aware comparison)
// ============================================================================

/// A normalized per-message summary. Joins text so that `["a","b"]` and `["ab"]`
/// compare equal (providers may merge adjacent text blocks). Ignores `tool_name`
/// on tool results since OpenAI doesn't carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MessageSummary {
    role: &'static str,
    text: String,
    tool_call_ids: Vec<String>,
    tool_result_ids: Vec<String>,
}

fn summarize_messages(messages: &[Message]) -> Vec<MessageSummary> {
    messages
        .iter()
        .map(|msg| match msg {
            Message::System { content } => MessageSummary {
                role: "system",
                text: join_user_content(content),
                tool_call_ids: vec![],
                tool_result_ids: vec![],
            },
            Message::User { content } => MessageSummary {
                role: "user",
                text: join_user_content(content),
                tool_call_ids: vec![],
                tool_result_ids: vec![],
            },
            Message::Assistant { content, .. } => {
                let (text, tool_call_ids) = summarize_assistant_content(content);
                MessageSummary {
                    role: "assistant",
                    text,
                    tool_call_ids,
                    tool_result_ids: vec![],
                }
            }
            Message::Tool { content } => MessageSummary {
                role: "tool",
                text: String::new(),
                tool_call_ids: vec![],
                tool_result_ids: content
                    .iter()
                    .map(|p| {
                        let ToolContentPart::ToolResult(tr) = p;
                        tr.tool_call_id.clone()
                    })
                    .collect(),
            },
        })
        .collect()
}

fn join_user_content(content: &UserContent) -> String {
    match content {
        UserContent::String(s) => s.clone(),
        UserContent::Array(parts) => parts
            .iter()
            .filter_map(|p| match p {
                UserContentPart::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn summarize_assistant_content(content: &AssistantContent) -> (String, Vec<String>) {
    match content {
        AssistantContent::String(s) => (s.clone(), vec![]),
        AssistantContent::Array(parts) => {
            let mut text = String::new();
            let mut tool_ids = Vec::new();
            for p in parts {
                match p {
                    AssistantContentPart::Text(t) => text.push_str(&t.text),
                    AssistantContentPart::ToolCall { tool_call_id, .. } => {
                        tool_ids.push(tool_call_id.clone());
                    }
                    AssistantContentPart::Reasoning { text: r, .. } => {
                        text.push_str(r);
                    }
                    _ => {}
                }
            }
            (text, tool_ids)
        }
    }
}

fn non_system(summaries: &[MessageSummary]) -> Vec<&MessageSummary> {
    summaries.iter().filter(|s| s.role != "system").collect()
}

fn extract_system_text(messages: &[Message]) -> String {
    let summaries = summarize_messages(messages);
    summaries
        .iter()
        .filter(|s| s.role == "system")
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn default_model(format: ProviderFormat) -> &'static str {
    match format {
        ProviderFormat::OpenAI => "gpt-4",
        ProviderFormat::Anthropic => "claude-3-5-sonnet-20241022",
        ProviderFormat::Google => "gemini-1.5-flash",
        ProviderFormat::Responses => "gpt-4",
        _ => "test-model",
    }
}

fn parse_format(s: &str) -> ProviderFormat {
    match s {
        "openai" => ProviderFormat::OpenAI,
        "anthropic" => ProviderFormat::Anthropic,
        "google" => ProviderFormat::Google,
        "responses" => ProviderFormat::Responses,
        other => panic!("Unknown format in test case: {:?}", other),
    }
}

// ============================================================================
// Assertion helpers
// ============================================================================

fn assert_anthropic_system_preserved(original: &UniversalRequest, anthropic_json: &Value) {
    let original_system = extract_system_text(&original.messages);
    if original_system.is_empty() {
        return;
    }

    let system_field = anthropic_json.get("system");
    assert!(
        system_field.is_some(),
        "Anthropic JSON should have 'system' field.\n\
         Original system text: {:?}\n\
         Anthropic JSON: {}",
        original_system,
        serde_json::to_string_pretty(anthropic_json).unwrap_or_default()
    );

    let system_text = system_field.unwrap().as_str().unwrap_or("");
    assert!(
        !system_text.is_empty(),
        "Anthropic 'system' field should not be empty.\n\
         Original system text: {:?}\n\
         Anthropic JSON: {}",
        original_system,
        serde_json::to_string_pretty(anthropic_json).unwrap_or_default()
    );
}

fn assert_messages_preserved(
    original: &UniversalRequest,
    roundtripped: &UniversalRequest,
    context: &str,
) {
    let orig = summarize_messages(&original.messages);
    let rt = summarize_messages(&roundtripped.messages);

    assert_eq!(
        non_system(&orig),
        non_system(&rt),
        "Non-system messages should be preserved in {}.\n\
         Original:     {:#?}\n\
         Roundtripped: {:#?}",
        context,
        non_system(&orig),
        non_system(&rt)
    );
}

// ============================================================================
// Saved test case loading (roundtrip_cases.json)
// ============================================================================

#[derive(serde::Deserialize, Debug)]
struct SavedTestCase {
    description: String,
    source_format: String,
    payload: Value,
    /// Provider formats to roundtrip through. Each one runs:
    ///   source -> Universal -> target -> Universal
    roundtrip_through: Vec<String>,
}

fn load_saved_cases() -> Vec<SavedTestCase> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/roundtrip_cases.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
}

/// When proptest finds a failure, call this to get a JSON snippet you can paste
/// into `roundtrip_cases.json`.
fn format_as_saved_case(
    description: &str,
    source_format: ProviderFormat,
    provider_json: &Value,
    roundtrip_through: &[&str],
) -> String {
    let case = json!({
        "description": description,
        "source_format": format!("{}", source_format).to_lowercase(),
        "payload": provider_json,
        "roundtrip_through": roundtrip_through,
    });
    serde_json::to_string_pretty(&case).unwrap()
}

// ============================================================================
// Saved case tests
// ============================================================================

#[test]
fn saved_cases_self_roundtrip() {
    let cases = load_saved_cases();
    for case in &cases {
        let source_format = parse_format(&case.source_format);

        // Parse from source format to Universal
        let universal = RoundtripHarness::from_provider_json(source_format, case.payload.clone())
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] Failed to parse {} payload: {}",
                    case.description, case.source_format, e
                )
            });

        // Self-roundtrip through source format
        let result =
            RoundtripHarness::self_roundtrip(&universal, source_format).unwrap_or_else(|e| {
                panic!(
                    "[{}] Self-roundtrip through {} failed: {}",
                    case.description, case.source_format, e
                )
            });

        assert_messages_preserved(
            &universal,
            &result.roundtripped,
            &format!(
                "[{}] self-roundtrip through {}",
                case.description, case.source_format
            ),
        );

        // For Anthropic, also check system preservation
        if source_format == ProviderFormat::Anthropic {
            assert_anthropic_system_preserved(&universal, &result.provider_json);
        }
    }
}

#[test]
fn saved_cases_cross_provider() {
    let cases = load_saved_cases();
    for case in &cases {
        let source_format = parse_format(&case.source_format);

        let universal = RoundtripHarness::from_provider_json(source_format, case.payload.clone())
            .unwrap_or_else(|e| {
                panic!(
                    "[{}] Failed to parse {} payload: {}",
                    case.description, case.source_format, e
                )
            });

        for target_name in &case.roundtrip_through {
            let target_format = parse_format(target_name);

            let result = RoundtripHarness::cross_provider(&universal, source_format, target_format)
                .unwrap_or_else(|e| {
                    panic!(
                        "[{}] {} -> {} failed: {}",
                        case.description, case.source_format, target_name, e
                    )
                });

            assert_messages_preserved(
                &universal,
                &result.roundtripped,
                &format!(
                    "[{}] {} -> {}",
                    case.description, case.source_format, target_name
                ),
            );

            // For Anthropic targets, verify system not dropped
            if target_format == ProviderFormat::Anthropic {
                assert_anthropic_system_preserved(&universal, &result.provider_json);
            }
        }
    }
}

// ============================================================================
// Proptest strategies
// ============================================================================

mod strategies {
    use super::*;
    use proptest::prelude::*;

    pub fn arb_text() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 .!?,]{1,80}"
    }

    fn arb_text_part() -> impl Strategy<Value = TextContentPart> {
        arb_text().prop_map(|text| TextContentPart {
            text,
            provider_options: None,
        })
    }

    pub fn arb_user_content() -> impl Strategy<Value = UserContent> {
        prop_oneof![
            arb_text().prop_map(UserContent::String),
            proptest::collection::vec(arb_text_part().prop_map(UserContentPart::Text), 1..=3)
                .prop_map(UserContent::Array),
        ]
    }

    fn arb_tool_call_arguments() -> impl Strategy<Value = ToolCallArguments> {
        prop_oneof![
            proptest::collection::hash_map("[a-z]{2,8}", arb_json_value(), 0..=3).prop_map(|m| {
                let map: serde_json::Map<String, Value> = m.into_iter().collect();
                ToolCallArguments::Valid(map)
            }),
            "[^{}]{1,30}".prop_map(ToolCallArguments::Invalid),
        ]
    }

    fn arb_json_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i32>().prop_map(|i| Value::Number(i.into())),
            arb_text().prop_map(Value::String),
        ]
    }

    fn arb_assistant_content() -> impl Strategy<Value = AssistantContent> {
        prop_oneof![
            3 => arb_text().prop_map(AssistantContent::String),
            1 => proptest::collection::vec(
                prop_oneof![
                    3 => arb_text_part().prop_map(AssistantContentPart::Text),
                    1 => (
                        "call_[a-zA-Z0-9]{8}",
                        "[a-z_]{3,12}",
                        arb_tool_call_arguments(),
                    ).prop_map(|(id, name, args)| AssistantContentPart::ToolCall {
                        tool_call_id: id,
                        tool_name: name,
                        arguments: args,
                        provider_options: None,
                        provider_executed: None,
                    }),
                ],
                1..=3,
            ).prop_map(AssistantContent::Array),
        ]
    }

    fn arb_tool_content() -> impl Strategy<Value = ToolContent> {
        proptest::collection::vec(
            ("call_[a-zA-Z0-9]{8}", "[a-z_]{3,12}", arb_json_value()).prop_map(
                |(id, name, output)| {
                    ToolContentPart::ToolResult(ToolResultContentPart {
                        tool_call_id: id,
                        tool_name: name,
                        output,
                        provider_options: None,
                    })
                },
            ),
            1..=2,
        )
    }

    pub fn arb_message() -> impl Strategy<Value = Message> {
        prop_oneof![
            2 => arb_user_content().prop_map(|c| Message::System { content: c }),
            3 => arb_user_content().prop_map(|c| Message::User { content: c }),
            3 => arb_assistant_content().prop_map(|c| Message::Assistant {
                content: c,
                id: None,
            }),
            1 => arb_tool_content().prop_map(|c| Message::Tool { content: c }),
        ]
    }

    /// Generate a valid-ish message thread (doesn't enforce strict role alternation).
    pub fn arb_message_thread() -> impl Strategy<Value = Vec<Message>> {
        proptest::collection::vec(arb_message(), 1..=8)
    }

    pub fn arb_universal_request() -> impl Strategy<Value = UniversalRequest> {
        (
            arb_message_thread(),
            prop::option::of(0.0..=2.0_f64),
            prop::option::of(1i64..=8192),
        )
            .prop_map(|(messages, temperature, max_tokens)| UniversalRequest {
                model: None,
                messages,
                params: UniversalParams {
                    temperature,
                    max_tokens,
                    ..Default::default()
                },
            })
    }
}

// ============================================================================
// Proptest-driven tests
// ============================================================================

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// OpenAI self-roundtrip: Universal -> OpenAI -> Universal preserves non-system messages.
    #[test]
    fn prop_openai_self_roundtrip(req in strategies::arb_universal_request()) {
        if let Ok(result) = RoundtripHarness::self_roundtrip(&req, ProviderFormat::OpenAI) {
            let orig = summarize_messages(&req.messages);
            let rt = summarize_messages(&result.roundtripped.messages);
            prop_assert_eq!(
                non_system(&orig), non_system(&rt),
                "OpenAI self-roundtrip lost messages.\n\
                 To save as a test case, add to roundtrip_cases.json:\n{}",
                format_as_saved_case(
                    "proptest: OpenAI roundtrip failure",
                    ProviderFormat::OpenAI,
                    &result.provider_json,
                    &["openai"],
                )
            );
        }
    }

    /// Anthropic self-roundtrip: system messages must not be silently dropped.
    #[test]
    fn prop_anthropic_self_roundtrip(req in strategies::arb_universal_request()) {
        if let Ok(result) = RoundtripHarness::self_roundtrip(&req, ProviderFormat::Anthropic) {
            let system_text = extract_system_text(&req.messages);
            if !system_text.is_empty() {
                let system_field = result.provider_json.get("system");
                let saved = format_as_saved_case(
                    "proptest: Anthropic system dropped",
                    ProviderFormat::Anthropic,
                    &result.provider_json,
                    &["anthropic"],
                );
                prop_assert!(
                    system_field.is_some(),
                    "Anthropic should have 'system' field. Original: {:?}\n\
                     To save:\n{}", system_text, saved
                );
                let field_text = system_field.unwrap().as_str().unwrap_or("");
                prop_assert!(
                    !field_text.is_empty(),
                    "Anthropic 'system' should not be empty. Original: {:?}\n\
                     To save:\n{}", system_text, saved
                );
            }

            let orig = summarize_messages(&req.messages);
            let rt = summarize_messages(&result.roundtripped.messages);
            prop_assert_eq!(
                non_system(&orig), non_system(&rt),
                "Anthropic self-roundtrip lost non-system messages.\n\
                 To save:\n{}",
                format_as_saved_case(
                    "proptest: Anthropic roundtrip failure",
                    ProviderFormat::Anthropic,
                    &result.provider_json,
                    &["anthropic"],
                )
            );
        }
    }

    /// Cross-provider: OpenAI -> Anthropic preserves non-system message content.
    #[test]
    fn prop_openai_to_anthropic(req in strategies::arb_universal_request()) {
        if let Ok(result) = RoundtripHarness::cross_provider(
            &req, ProviderFormat::OpenAI, ProviderFormat::Anthropic,
        ) {
            let orig = summarize_messages(&req.messages);
            let rt = summarize_messages(&result.roundtripped.messages);
            prop_assert_eq!(
                non_system(&orig), non_system(&rt),
                "OpenAI->Anthropic lost non-system messages.\n\
                 To save:\n{}",
                format_as_saved_case(
                    "proptest: OpenAI->Anthropic failure",
                    ProviderFormat::Anthropic,
                    &result.provider_json,
                    &["anthropic", "openai"],
                )
            );
        }
    }

    /// Cross-provider: Anthropic -> OpenAI preserves non-system message content.
    #[test]
    fn prop_anthropic_to_openai(req in strategies::arb_universal_request()) {
        if let Ok(result) = RoundtripHarness::cross_provider(
            &req, ProviderFormat::Anthropic, ProviderFormat::OpenAI,
        ) {
            let orig = summarize_messages(&req.messages);
            let rt = summarize_messages(&result.roundtripped.messages);
            prop_assert_eq!(
                non_system(&orig), non_system(&rt),
                "Anthropic->OpenAI lost non-system messages.\n\
                 To save:\n{}",
                format_as_saved_case(
                    "proptest: Anthropic->OpenAI failure",
                    ProviderFormat::OpenAI,
                    &result.provider_json,
                    &["openai", "anthropic"],
                )
            );
        }
    }

    /// Targeted: random UserContent variants for system messages are never dropped by Anthropic.
    #[test]
    fn prop_system_message_variants_not_dropped(
        content in strategies::arb_user_content()
    ) {
        let req = UniversalRequest {
            model: Some("claude-3-5-sonnet-20241022".into()),
            messages: vec![
                Message::System { content },
                Message::User {
                    content: UserContent::String("Hello".into()),
                },
            ],
            params: UniversalParams {
                max_tokens: Some(1024),
                ..Default::default()
            },
        };

        let result = RoundtripHarness::self_roundtrip(&req, ProviderFormat::Anthropic).unwrap();
        let system_text = extract_system_text(&req.messages);
        let saved = format_as_saved_case(
            "proptest: system variant dropped",
            ProviderFormat::Anthropic,
            &result.provider_json,
            &["anthropic"],
        );
        prop_assert!(
            result.provider_json.get("system").is_some(),
            "Anthropic should have 'system' field. Original: {:?}\nTo save:\n{}", system_text, saved
        );
        let field_text = result.provider_json.get("system").unwrap().as_str().unwrap_or("");
        prop_assert!(
            !field_text.is_empty(),
            "Anthropic 'system' should not be empty. Original: {:?}\nTo save:\n{}", system_text, saved
        );
    }
}
