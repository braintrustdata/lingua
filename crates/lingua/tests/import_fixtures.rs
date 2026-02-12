use lingua::processing::{import_messages_from_spans, Span};
use lingua::serde_json;
use lingua::Message;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportAssertionCase {
    expected_message_count: Option<usize>,
    expected_roles_in_order: Option<Vec<String>>,
    must_contain_text: Option<Vec<String>>,
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
    let mut paths: Vec<PathBuf> = fs::read_dir(import_cases_dir)
        .expect("payloads/import-cases should be readable")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?;
            if name.ends_with(".spans.json") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    paths.sort();
    paths
}

fn case_name_from_spans_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("fixture filename must be valid utf-8");
    stem.strip_suffix(".spans")
        .expect("fixture name must end with .spans")
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
        Message::Assistant { .. } => "assistant",
        Message::Tool { .. } => "tool",
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
        must_contain_text: Some(vec![]),
    }
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
        let case_name = case_name_from_spans_path(&spans_path);
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
        let messages = import_messages_from_spans(spans);
        let serialized_messages =
            serde_json::to_string(&messages).expect("messages should serialize to json");
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
                    must_contain_text: existing.must_contain_text.clone(),
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
