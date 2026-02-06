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
use lingua::{ProviderFormat, UniversalRequest};
use std::path::PathBuf;

mod schema_strategy;

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
// Proptest strategies: generate provider-native JSON payloads
// ============================================================================

mod strategies {
    use super::schema_strategy::{load_openapi_definitions, strategy_for_schema_name};
    use super::*;
    use proptest::prelude::*;

    // -- Shared primitives --

    pub fn arb_text() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 .!?,]{1,80}"
    }

    fn arb_json_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i32>().prop_map(|i| json!(i)),
            arb_text().prop_map(Value::String),
        ]
    }

    fn arb_json_object() -> impl Strategy<Value = Value> {
        proptest::collection::hash_map("[a-z]{2,8}", arb_json_value(), 0..=3)
            .prop_map(|m| json!(m.into_iter().collect::<serde_json::Map<String, Value>>()))
    }

    fn arb_function_name() -> impl Strategy<Value = String> {
        "[a-z_]{3,12}"
    }

    // ========================================================================
    // Schema-driven strategies (OpenAI + Anthropic from OpenAPI specs)
    // ========================================================================

    fn specs_dir() -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        // specs/ is at the repo root, two levels up from crates/lingua/
        format!("{}/../..", manifest_dir)
    }

    pub fn arb_openai_payload() -> BoxedStrategy<Value> {
        let defs = load_openapi_definitions(&format!("{}/specs/openai/openapi.yml", specs_dir()));
        strategy_for_schema_name("CreateChatCompletionRequest", &defs)
    }

    pub fn arb_anthropic_payload() -> BoxedStrategy<Value> {
        let defs =
            load_openapi_definitions(&format!("{}/specs/anthropic/openapi.yml", specs_dir()));
        strategy_for_schema_name("CreateMessageParams", &defs)
    }

    // ========================================================================
    // Google GenerateContent JSON strategies (hand-written, no OpenAPI spec)
    // ========================================================================

    fn google_text_part() -> impl Strategy<Value = Value> {
        arb_text().prop_map(|t| json!({"text": t}))
    }

    fn google_function_call_part() -> impl Strategy<Value = Value> {
        (arb_function_name(), arb_json_object())
            .prop_map(|(name, args)| json!({"functionCall": {"name": name, "args": args}}))
    }

    fn google_function_response_part() -> impl Strategy<Value = Value> {
        (arb_function_name(), arb_json_object()).prop_map(
            |(name, response)| json!({"functionResponse": {"name": name, "response": response}}),
        )
    }

    fn google_user_content() -> impl Strategy<Value = Value> {
        proptest::collection::vec(google_text_part(), 1..=3)
            .prop_map(|parts| json!({"role": "user", "parts": parts}))
    }

    fn google_model_content() -> impl Strategy<Value = Value> {
        prop_oneof![
            // Text-only
            3 => proptest::collection::vec(google_text_part(), 1..=3)
                .prop_map(|parts| json!({"role": "model", "parts": parts})),
            // Function calls
            1 => proptest::collection::vec(google_function_call_part(), 1..=2)
                .prop_map(|parts| json!({"role": "model", "parts": parts})),
        ]
    }

    fn google_tool_response_content() -> impl Strategy<Value = Value> {
        proptest::collection::vec(google_function_response_part(), 1..=2)
            .prop_map(|parts| json!({"role": "user", "parts": parts}))
    }

    fn google_content() -> impl Strategy<Value = Value> {
        prop_oneof![
            3 => google_user_content(),
            3 => google_model_content(),
            1 => google_tool_response_content(),
        ]
    }

    pub fn arb_google_payload() -> impl Strategy<Value = Value> {
        (
            proptest::collection::vec(google_content(), 1..=6),
            prop::option::of(arb_text()), // optional systemInstruction
        )
            .prop_map(|(contents, system)| {
                let mut payload = json!({
                    "contents": contents,
                    "generationConfig": {"maxOutputTokens": 1024},
                });
                if let Some(s) = system {
                    payload["systemInstruction"] = json!({"parts": [{"text": s}]});
                }
                payload
            })
    }

    // ========================================================================
    // Combined: generate (format, payload) pairs
    // ========================================================================

    pub fn arb_provider_payload() -> impl Strategy<Value = (ProviderFormat, Value)> {
        prop_oneof![
            arb_openai_payload().prop_map(|p| (ProviderFormat::OpenAI, p)),
            arb_anthropic_payload().prop_map(|p| (ProviderFormat::Anthropic, p)),
            arb_google_payload().prop_map(|p| (ProviderFormat::Google, p)),
        ]
    }
}

// ============================================================================
// Proptest-driven tests
// ============================================================================

use proptest::prelude::*;

/// Shared helper: parse provider JSON to Universal, roundtrip through each target,
/// and check assertions. Returns Err with saveable JSON on failure.
fn run_roundtrips(
    source_format: ProviderFormat,
    payload: &Value,
    targets: &[ProviderFormat],
) -> Result<(), String> {
    let universal = RoundtripHarness::from_provider_json(source_format, payload.clone())?;

    // Self-roundtrip
    let self_result = RoundtripHarness::self_roundtrip(&universal, source_format)?;
    let orig = summarize_messages(&universal.messages);
    let rt = summarize_messages(&self_result.roundtripped.messages);
    if non_system(&orig) != non_system(&rt) {
        return Err(format!(
            "Self-roundtrip through {:?} lost messages.\n\
             Original:     {:#?}\n\
             Roundtripped: {:#?}\n\
             To save:\n{}",
            source_format,
            non_system(&orig),
            non_system(&rt),
            format_as_saved_case(
                &format!("proptest: {:?} self-roundtrip failure", source_format),
                source_format,
                payload,
                &[&format!("{}", source_format).to_lowercase()],
            ),
        ));
    }

    // Cross-provider roundtrips
    for &target in targets {
        if let Ok(result) = RoundtripHarness::cross_provider(&universal, source_format, target) {
            let rt = summarize_messages(&result.roundtripped.messages);
            if non_system(&orig) != non_system(&rt) {
                return Err(format!(
                    "{:?} -> {:?} lost messages.\n\
                     Original:     {:#?}\n\
                     Roundtripped: {:#?}\n\
                     To save:\n{}",
                    source_format,
                    target,
                    non_system(&orig),
                    non_system(&rt),
                    format_as_saved_case(
                        &format!("proptest: {:?}->{:?} failure", source_format, target),
                        source_format,
                        payload,
                        &[
                            &format!("{}", source_format).to_lowercase(),
                            &format!("{}", target).to_lowercase(),
                        ],
                    ),
                ));
            }

            if target == ProviderFormat::Anthropic {
                let sys = extract_system_text(&universal.messages);
                if !sys.is_empty() {
                    let s = result.provider_json.get("system");
                    if s.is_none() || s.unwrap().as_str().unwrap_or("").is_empty() {
                        return Err(format!(
                            "Anthropic system dropped! Original: {:?}\nTo save:\n{}",
                            sys,
                            format_as_saved_case(
                                "proptest: Anthropic system dropped",
                                source_format,
                                payload,
                                &[&format!("{}", source_format).to_lowercase(), "anthropic",],
                            ),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        // Don't write .proptest-regressions files; we use roundtrip_cases.json instead.
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// OpenAI Chat Completions JSON -> roundtrip through all providers.
    #[test]
    fn prop_openai_payload(payload in strategies::arb_openai_payload()) {
        if let Ok(universal) = RoundtripHarness::from_provider_json(ProviderFormat::OpenAI, payload.clone()) {
            let _ = universal; // parsed ok
            if let Err(e) = run_roundtrips(
                ProviderFormat::OpenAI, &payload,
                &[ProviderFormat::Anthropic, ProviderFormat::Google],
            ) {
                prop_assert!(false, "{}", e);
            }
        }
    }

    /// Anthropic Messages JSON -> roundtrip through all providers.
    #[test]
    fn prop_anthropic_payload(payload in strategies::arb_anthropic_payload()) {
        if let Ok(universal) = RoundtripHarness::from_provider_json(ProviderFormat::Anthropic, payload.clone()) {
            let _ = universal;
            if let Err(e) = run_roundtrips(
                ProviderFormat::Anthropic, &payload,
                &[ProviderFormat::OpenAI, ProviderFormat::Google],
            ) {
                prop_assert!(false, "{}", e);
            }
        }
    }

    /// Google GenerateContent JSON -> roundtrip through all providers.
    #[test]
    fn prop_google_payload(payload in strategies::arb_google_payload()) {
        if let Ok(universal) = RoundtripHarness::from_provider_json(ProviderFormat::Google, payload.clone()) {
            let _ = universal;
            if let Err(e) = run_roundtrips(
                ProviderFormat::Google, &payload,
                &[ProviderFormat::OpenAI, ProviderFormat::Anthropic],
            ) {
                prop_assert!(false, "{}", e);
            }
        }
    }

    /// Any provider JSON -> all roundtrips (maximum coverage).
    #[test]
    fn prop_any_provider((format, payload) in strategies::arb_provider_payload()) {
        let all_targets = [ProviderFormat::OpenAI, ProviderFormat::Anthropic, ProviderFormat::Google];
        let targets: Vec<_> = all_targets.iter().copied().filter(|&t| t != format).collect();

        if let Ok(_) = RoundtripHarness::from_provider_json(format, payload.clone()) {
            if let Err(e) = run_roundtrips(format, &payload, &targets) {
                prop_assert!(false, "{}", e);
            }
        }
    }
}
