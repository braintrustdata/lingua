use lingua::processing::import::{
    import_messages_from_values, import_messages_from_values_with_metadata,
    import_role_message_from_value, import_span_with_options, try_import_ai_sdk_prompt_message,
    ImportOptions, ImportValue, InputKind, OpaqueItem,
};
use lingua::processing::{import_messages_from_spans, Span};
use lingua::serde_json;
use lingua::universal::{AssistantContent, AssistantContentPart, UserContent};
use lingua::Message;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

struct BorrowedValue<'a> {
    value: &'a serde_json::Value,
    copies: &'a Cell<usize>,
}

impl ImportValue for BorrowedValue<'_> {
    type Item<'a>
        = BorrowedValue<'a>
    where
        Self: 'a;
    type Items<'a>
        = std::vec::IntoIter<BorrowedValue<'a>>
    where
        Self: 'a;

    fn kind(&self) -> InputKind {
        self.value.kind()
    }

    fn parse<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_str(&serde_json::to_string(self.value).unwrap()).ok()
    }

    fn array(&self) -> Option<Self::Items<'_>> {
        Some(
            self.value
                .as_array()?
                .iter()
                .map(|value| BorrowedValue {
                    value,
                    copies: self.copies,
                })
                .collect::<Vec<_>>()
                .into_iter(),
        )
    }

    fn to_value(&self) -> Cow<'_, serde_json::Value> {
        self.copies.set(self.copies.get() + 1);
        Cow::Owned(self.value.clone())
    }
}

struct UnreadableString;

impl ImportValue for UnreadableString {
    type Item<'a> = Self;
    type Items<'a> = std::iter::Empty<Self>;

    fn kind(&self) -> InputKind {
        InputKind::String
    }

    fn parse<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        None
    }

    fn array(&self) -> Option<Self::Items<'_>> {
        None
    }

    fn to_value(&self) -> Cow<'_, serde_json::Value> {
        panic!("unreadable strings must not be materialized")
    }
}

#[test]
fn borrowed_tool_chat_does_not_materialize_values() {
    let input = serde_json::json!([
        {"role": "system", "content": "Use tools"},
        {"role": "user", "content": "What is the weather?"},
        {"role": "assistant", "content": null, "tool_calls": [{
            "id": "call_weather", "type": "function",
            "function": {"name": "weather", "arguments": "{}"}
        }]},
        {"role": "tool", "tool_call_id": "call_weather", "content": "Sunny"},
        {"role": "assistant", "content": "It is sunny"}
    ]);
    let copies = Cell::new(0);
    let messages = import_messages_from_values(
        &BorrowedValue {
            value: &input,
            copies: &copies,
        },
        &serde_json::Value::Null,
        None,
    );
    assert_eq!(messages.len(), 5);
    assert_eq!(copies.get(), 0);
    assert_eq!(
        serde_json::to_value(&messages).unwrap(),
        serde_json::to_value(import_messages_from_spans(vec![Span {
            input: Some(input),
            output: None,
            other: Default::default(),
        }]))
        .unwrap(),
    );
}

#[test]
fn borrowed_message_only_import_discards_opaque_values_without_copying() {
    let input = serde_json::json!([
        {"role": "user", "content": "question"},
        {"type": "item_reference", "id": "msg_previous"}
    ]);
    let metadata = serde_json::json!({"instructions": "Use tools"});
    let copies = Cell::new(0);
    let messages = import_messages_from_values_with_metadata(
        &BorrowedValue {
            value: &input,
            copies: &copies,
        },
        &serde_json::Value::Null,
        Some(&BorrowedValue {
            value: &metadata,
            copies: &copies,
        }),
    );
    assert!(matches!(&messages[..], [Message::System {
        content: UserContent::String(instructions),
    }, Message::User { content: UserContent::String(question) }]
        if instructions == "Use tools" && question == "question"));
    assert_eq!(copies.get(), 0);
}

#[test]
fn unreadable_string_input_preserves_output() {
    let output = serde_json::json!("answer");
    let messages = import_messages_from_values(&UnreadableString, &output, None);
    assert!(matches!(&messages[..], [Message::Assistant {
        content: AssistantContent::String(text), ..
    }] if text == "answer"));
}

#[test]
fn unreadable_string_output_preserves_input() {
    let input = serde_json::json!("question");
    let messages = import_messages_from_values(&input, &UnreadableString, None);
    assert!(matches!(&messages[..], [Message::User {
        content: UserContent::String(text),
    }] if text == "question"));
}

#[test]
fn metadata_json_strings_preserve_duplicate_key_resolution() {
    let span: Span = serde_json::from_value(serde_json::json!({
        "input": "question",
        "metadata": "{\"instructions\":\"first\",\"instructions\":\"last\"}",
    }))
    .unwrap();
    let copies = Cell::new(0);
    let metadata = span.other.get("metadata").map(|value| BorrowedValue {
        value,
        copies: &copies,
    });
    for messages in [
        import_messages_from_spans(vec![span.clone()]),
        import_messages_from_values(&span.input, &span.output, span.other.get("metadata")),
        import_messages_from_values_with_metadata(&span.input, &span.output, metadata.as_ref()),
    ] {
        assert!(matches!(
            messages.first(),
            Some(Message::System { content: UserContent::String(instructions) }) if instructions == "last"
        ));
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportAssertionCase {
    expected_message_count: Option<usize>,
    expected_roles_in_order: Option<Vec<String>>,
    expected_assistant_content_part_types: Option<Vec<Vec<String>>>,
    expected_reasoning_encrypted_content: Option<Vec<Option<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_opaque_input: Option<Vec<Vec<OpaqueItem>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_opaque_output: Option<Vec<Vec<OpaqueItem>>>,
    must_contain_text: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    must_not_contain_text: Option<Vec<String>>,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory should exist")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn discover_import_case_paths() -> Vec<PathBuf> {
    let import_cases_dir = workspace_root().join("payloads/import-cases");
    let mut candidate_paths: Vec<PathBuf> = fs::read_dir(import_cases_dir)
        .expect("payloads/import-cases should be readable")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?;
            if name.ends_with(".json") && !name.ends_with(".assertions.json") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    candidate_paths.sort();

    let mut by_case_name: BTreeMap<String, PathBuf> = BTreeMap::new();
    for path in candidate_paths {
        let case_name = case_name_from_fixture_path(&path);
        let current_is_legacy = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".spans.json"))
            .expect("fixture filename must be valid utf-8");

        if let Some(existing_path) = by_case_name.get(&case_name) {
            let existing_is_legacy = existing_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with(".spans.json"))
                .expect("fixture filename must be valid utf-8");

            match (existing_is_legacy, current_is_legacy) {
                (true, false) => {
                    by_case_name.insert(case_name, path);
                }
                (false, true) => {}
                _ => {
                    panic!(
                        "ambiguous import fixtures for case '{}': '{}' and '{}'",
                        case_name,
                        existing_path.display(),
                        path.display()
                    );
                }
            }
        } else {
            by_case_name.insert(case_name, path);
        }
    }

    by_case_name.into_values().collect()
}

fn case_name_from_fixture_path(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("fixture filename must be valid utf-8");

    let without_json = name
        .strip_suffix(".json")
        .expect("fixture name must end with .json");

    without_json
        .strip_suffix(".spans")
        .unwrap_or(without_json)
        .to_string()
}

fn parse_spans_fixture(case_name: &str, spans_path: &Path, spans_json: &str) -> Vec<Span> {
    let parsed: serde_json::Value = serde_json::from_str(spans_json).unwrap_or_else(|e| {
        panic!(
            "failed to parse spans fixture json for case '{}': {} ({})",
            case_name,
            e,
            spans_path.display()
        )
    });

    match parsed {
        serde_json::Value::Array(_) => serde_json::from_value(parsed).unwrap_or_else(|e| {
            panic!(
                "failed to parse spans fixture array for case '{}': {} ({})",
                case_name,
                e,
                spans_path.display()
            )
        }),
        serde_json::Value::Object(_) => {
            let span: Span = serde_json::from_value(parsed).unwrap_or_else(|e| {
                panic!(
                    "failed to parse single span fixture for case '{}': {} ({})",
                    case_name,
                    e,
                    spans_path.display()
                )
            });
            vec![span]
        }
        _ => panic!(
            "spans fixture for case '{}' must be a span object or an array of spans ({})",
            case_name,
            spans_path.display()
        ),
    }
}

fn message_role(message: &Message) -> &'static str {
    match message {
        Message::User { .. } => "user",
        Message::System { .. } => "system",
        Message::Developer { .. } => "developer",
        Message::Assistant { .. } => "assistant",
        Message::Tool { .. } => "tool",
        Message::AdditionalTools { .. } => "additional_tools",
    }
}

fn infer_assertions_from_messages(messages: &[Message]) -> ImportAssertionCase {
    let roles = messages
        .iter()
        .map(|message| message_role(message).to_string())
        .collect();

    ImportAssertionCase {
        expected_message_count: Some(messages.len()),
        expected_roles_in_order: Some(roles),
        expected_assistant_content_part_types: None,
        expected_reasoning_encrypted_content: None,
        expected_opaque_input: None,
        expected_opaque_output: None,
        must_contain_text: Some(vec![]),
        must_not_contain_text: None,
    }
}

fn assistant_content_part_type(part: &AssistantContentPart) -> &'static str {
    match part {
        AssistantContentPart::Text(_) => "text",
        AssistantContentPart::File { .. } => "file",
        AssistantContentPart::Reasoning { .. } => "reasoning",
        AssistantContentPart::ToolCall { .. } => "tool_call",
        AssistantContentPart::Program { .. } => "program",
        AssistantContentPart::ProgramOutput { .. } => "program_output",
        AssistantContentPart::ToolDiscoveryCall { .. } => "tool_discovery_call",
        AssistantContentPart::ToolResult { .. } => "tool_result",
    }
}

fn assistant_content_part_types(messages: &[Message]) -> Vec<Vec<String>> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::Assistant { content, .. } => Some(match content {
                AssistantContent::String(_) => vec!["string".to_string()],
                AssistantContent::Array(parts) => parts
                    .iter()
                    .map(|part| assistant_content_part_type(part).to_string())
                    .collect(),
            }),
            _ => None,
        })
        .collect()
}

fn reasoning_encrypted_content(messages: &[Message]) -> Vec<Option<String>> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            } => Some(parts),
            _ => None,
        })
        .flatten()
        .filter_map(|part| match part {
            AssistantContentPart::Reasoning {
                encrypted_content, ..
            } => Some(encrypted_content.clone()),
            _ => None,
        })
        .collect()
}

fn write_assertions_fixture(assertions_path: &Path, assertions: &ImportAssertionCase) {
    let json = serde_json::to_string_pretty(assertions)
        .expect("assertions fixture should serialize")
        + "\n";
    fs::write(assertions_path, json).unwrap_or_else(|e| {
        panic!(
            "failed to write assertions fixture: {} ({})",
            e,
            assertions_path.display()
        )
    });
}

fn env_flag_enabled(flag: &str) -> bool {
    env::var(flag)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[test]
fn test_import_cases_from_shared_fixtures() {
    let generate_missing = env_flag_enabled("GENERATE_MISSING");
    let accept = env_flag_enabled("ACCEPT");
    let case_filter = env::var("CASE_FILTER").ok();

    assert!(
        !(env_flag_enabled("CI") && (generate_missing || accept)),
        "GENERATE_MISSING/ACCEPT are disabled in CI"
    );

    let case_paths = discover_import_case_paths();
    assert!(
        !case_paths.is_empty(),
        "no import case fixtures found in payloads/import-cases"
    );

    let mut generated_count = 0usize;
    let mut checked_count = 0usize;

    for spans_path in case_paths {
        let case_name = case_name_from_fixture_path(&spans_path);
        if let Some(filter) = &case_filter {
            if !case_name.contains(filter) {
                continue;
            }
        }

        let spans_json = fs::read_to_string(&spans_path).unwrap_or_else(|e| {
            panic!(
                "failed to read spans fixture for case '{}': {} ({})",
                case_name,
                e,
                spans_path.display()
            )
        });

        let spans = parse_spans_fixture(&case_name, &spans_path, &spans_json);

        let assertions_path = spans_path.with_file_name(format!("{}.assertions.json", case_name));
        let messages = import_messages_from_spans(spans.clone());
        let serialized_messages =
            serde_json::to_string(&messages).expect("messages should serialize to json");
        let copies = Cell::new(0);
        let borrowed_messages: Vec<_> = spans
            .iter()
            .flat_map(|span| {
                let input = span.input.as_ref().map(|value| BorrowedValue {
                    value,
                    copies: &copies,
                });
                let output = span.output.as_ref().map(|value| BorrowedValue {
                    value,
                    copies: &copies,
                });
                let metadata = span.other.get("metadata").map(|value| BorrowedValue {
                    value,
                    copies: &copies,
                });
                import_messages_from_values_with_metadata(&input, &output, metadata.as_ref())
            })
            .collect();
        assert_eq!(
            serde_json::to_string(&borrowed_messages).unwrap(),
            serialized_messages,
            "borrowed import mismatch for case '{case_name}'",
        );
        for span in &spans {
            if let Some(items) = span
                .input
                .as_ref()
                .and_then(serde_json::Value::as_array)
                .filter(|items| {
                    !items.is_empty()
                        && items
                            .iter()
                            .all(|item| item.get("role").is_some_and(serde_json::Value::is_string))
                })
            {
                let imported: Vec<_> = items
                    .iter()
                    .flat_map(import_role_message_from_value)
                    .collect();
                let expected = import_messages_from_values(
                    &span.input,
                    &None::<serde_json::Value>,
                    None::<&serde_json::Value>,
                );
                assert_eq!(
                    serde_json::to_value(imported).unwrap(),
                    serde_json::to_value(expected).unwrap(),
                    "per-item role import mismatch for '{case_name}'"
                );
            }
        }
        let inferred = infer_assertions_from_messages(&messages);

        let assertions = if assertions_path.exists() {
            let assertions_json = fs::read_to_string(&assertions_path).unwrap_or_else(|e| {
                panic!(
                    "failed to read assertions fixture for case '{}': {} ({})",
                    case_name,
                    e,
                    assertions_path.display()
                )
            });
            let existing: ImportAssertionCase = serde_json::from_str(&assertions_json)
                .unwrap_or_else(|e| {
                    panic!(
                        "failed to parse assertions fixture for case '{}': {} ({})",
                        case_name,
                        e,
                        assertions_path.display()
                    )
                });

            if accept {
                let updated = ImportAssertionCase {
                    expected_message_count: inferred.expected_message_count,
                    expected_roles_in_order: inferred.expected_roles_in_order,
                    expected_assistant_content_part_types: existing
                        .expected_assistant_content_part_types
                        .clone(),
                    expected_reasoning_encrypted_content: existing
                        .expected_reasoning_encrypted_content
                        .clone(),
                    expected_opaque_input: existing.expected_opaque_input.clone(),
                    expected_opaque_output: existing.expected_opaque_output.clone(),
                    must_contain_text: existing.must_contain_text.clone(),
                    must_not_contain_text: existing.must_not_contain_text.clone(),
                };
                write_assertions_fixture(&assertions_path, &updated);
                generated_count += 1;
                println!("updated assertions fixture for case '{}'", case_name);
                updated
            } else {
                existing
            }
        } else if generate_missing || accept {
            write_assertions_fixture(&assertions_path, &inferred);
            generated_count += 1;
            println!("generated assertions fixture for case '{}'", case_name);
            inferred
        } else {
            panic!(
                "missing assertions fixture for case '{}' at {}.\n\
                 Re-run with GENERATE_MISSING=1 to create it.",
                case_name,
                assertions_path.display()
            );
        };

        if assertions.expected_opaque_input.is_some() || assertions.expected_opaque_output.is_some()
        {
            let imported = spans
                .into_iter()
                .map(|span| {
                    import_span_with_options(
                        span,
                        ImportOptions {
                            preserve_unsupported: true,
                        },
                    )
                    .expect("fixture spans should import")
                })
                .collect::<Vec<_>>();
            for (field, expected, actual) in [
                (
                    "input",
                    assertions.expected_opaque_input,
                    imported
                        .iter()
                        .map(|span| &span.opaque_input)
                        .collect::<Vec<_>>(),
                ),
                (
                    "output",
                    assertions.expected_opaque_output,
                    imported
                        .iter()
                        .map(|span| &span.opaque_output)
                        .collect::<Vec<_>>(),
                ),
            ] {
                if let Some(expected) = expected {
                    assert_eq!(
                        serde_json::to_value(actual).unwrap(),
                        serde_json::to_value(expected).unwrap(),
                        "opaque {field} mismatch for case '{case_name}'",
                    );
                }
            }
        }

        if let Some(expected_count) = assertions.expected_message_count {
            assert_eq!(
                messages.len(),
                expected_count,
                "message count mismatch for case '{}'",
                case_name
            );
        }

        if let Some(expected_roles) = assertions.expected_roles_in_order {
            let actual_roles: Vec<String> = messages
                .iter()
                .map(|message| message_role(message).to_string())
                .collect();
            assert_eq!(
                actual_roles, expected_roles,
                "message roles mismatch for case '{}'",
                case_name
            );
        }

        if let Some(expected_part_types) = assertions.expected_assistant_content_part_types {
            assert_eq!(
                assistant_content_part_types(&messages),
                expected_part_types,
                "assistant content part types mismatch for case '{}'",
                case_name
            );
        }

        if let Some(expected_encrypted_content) = assertions.expected_reasoning_encrypted_content {
            assert_eq!(
                reasoning_encrypted_content(&messages),
                expected_encrypted_content,
                "reasoning encrypted content mismatch for case '{}'",
                case_name
            );
        }

        if let Some(required_texts) = assertions.must_contain_text {
            for required_text in required_texts {
                assert!(
                    serialized_messages.contains(&required_text),
                    "missing required text '{}' for case '{}'",
                    required_text,
                    case_name
                );
            }
        }

        if let Some(disallowed_texts) = assertions.must_not_contain_text {
            for disallowed_text in disallowed_texts {
                assert!(
                    !serialized_messages.contains(&disallowed_text),
                    "found disallowed text '{}' for case '{}'",
                    disallowed_text,
                    case_name
                );
            }
        }
        checked_count += 1;
    }

    assert!(
        checked_count > 0,
        "no fixtures matched CASE_FILTER={:?}",
        case_filter
    );

    if generated_count > 0 {
        println!(
            "wrote {} assertions fixture file(s) (generate_missing={}, accept={})",
            generated_count, generate_missing, accept
        );
    }
}

#[test]
fn ai_sdk_item_import_preserves_sequence_rejection() {
    for items in [
        serde_json::json!([]),
        serde_json::json!([{"role":"user", "content":"hello"}, {"role":"assistant", "content":[{"type":"text", "text":"answer"}], "id":"discarded"}]),
        serde_json::json!([{"role":"tool", "content":"result", "tool_call_id":"call", "name":"lookup"}]),
        serde_json::json!([{"role":"user", "content":"hello"}, {"role":"developer", "content":"invalid for AI SDK"}]),
        serde_json::json!([{"role":"assistant", "content":{"malformed":true}}]),
    ] {
        let cached: Option<Vec<_>> = items
            .as_array()
            .unwrap()
            .iter()
            .map(try_import_ai_sdk_prompt_message)
            .collect();
        let probe = serde_json::json!({"system":"probe", "prompt":items});
        let expected = import_messages_from_values(
            &probe,
            &None::<serde_json::Value>,
            None::<&serde_json::Value>,
        );
        let mut messages = vec![Message::System {
            content: UserContent::String("probe".into()),
        }];
        if let Some(cached) = cached {
            messages.extend(cached);
        }
        assert_eq!(
            serde_json::to_value(messages).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}

#[test]
fn cached_history_formats_preserve_tool_result_differences() {
    let value = serde_json::json!({"role":"tool", "content":"result", "tool_call_id":"call", "name":"lookup"});
    let direct = import_role_message_from_value(&value);
    let ai_sdk = try_import_ai_sdk_prompt_message(&value).unwrap();
    assert_eq!(
        serde_json::to_value(direct).unwrap(),
        serde_json::json!([{
            "role": "tool",
            "content": [{
                "type": "tool_result",
                "tool_call_id": "call",
                "tool_name": "",
                "output": "result",
            }],
        }])
    );
    assert_eq!(
        serde_json::to_value(ai_sdk).unwrap(),
        serde_json::json!({
            "role": "tool",
            "content": [{
                "type": "tool_result",
                "tool_call_id": "call",
                "tool_name": "lookup",
                "output": null,
            }],
        })
    );
}
