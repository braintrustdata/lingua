use anonymize::{
    anonymize_json_value, anonymize_json_value_with_options,
    anonymize_json_value_with_options_and_filter, AnonymizeFilterContext, AnonymizeFilterKind,
    AnonymizeOptions,
};
use serde_json::json;

#[test]
fn anonymizes_content_and_metadata_strings_by_default() {
    let input = json!({
        "input": [
            { "role": "user", "content": "hello world", "id": "user-1" },
            {
                "role": "assistant",
                "content": [{ "type": "text", "text": "hello world" }],
                "finish_reason": "stop"
            }
        ],
        "metadata": "leave me alone"
    });

    let result = anonymize_json_value(input);
    assert_eq!(
        result.value,
        json!({
            "input": [
                { "role": "user", "content": "anon_1", "id": "user-1" },
                {
                    "role": "assistant",
                    "content": [{ "type": "text", "text": "anon_1" }],
                    "finish_reason": "stop"
                }
            ],
            "metadata": "anon_2"
        })
    );
    assert_eq!(result.replaced_string_count, 3);
    assert_eq!(result.unique_replacement_count, 2);
}

#[test]
fn anonymizes_all_strings_when_all_strings_is_enabled() {
    let input = json!({
        "role": "user",
        "content": "hello world",
        "type": "text"
    });

    let result = anonymize_json_value_with_options(
        input,
        AnonymizeOptions::default().with_all_strings(true),
    );
    assert_eq!(
        result.value,
        json!({
            "role": "anon_1",
            "content": "anon_2",
            "type": "anon_3"
        })
    );
    assert_eq!(result.replaced_string_count, 3);
    assert_eq!(result.unique_replacement_count, 3);
}

#[test]
fn preserves_configured_keys_inside_content() {
    let input = json!({
        "content": [{ "toolName": "bash", "text": "run ls", "type": "text" }]
    });

    let result = anonymize_json_value_with_options(
        input,
        AnonymizeOptions::default().with_preserve_keys(["type", "toolName"]),
    );
    assert_eq!(
        result.value,
        json!({
            "content": [{ "toolName": "bash", "text": "anon_1", "type": "text" }]
        })
    );
    assert_eq!(result.replaced_string_count, 1);
    assert_eq!(result.unique_replacement_count, 1);
}

#[test]
fn anonymizes_all_metadata_strings() {
    let input = json!({
        "metadata": {
            "model": "gpt-5.1-2025-11-13",
            "trace_id": "trace-1",
            "route": "base",
            "tool_definitions": [
                {
                    "name": "fetch_weather",
                    "description": "Get weather by city",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "city": { "type": "string", "description": "City name" }
                        }
                    }
                }
            ]
        }
    });

    let result = anonymize_json_value(input);
    assert_eq!(
        result.value,
        json!({
            "metadata": {
                "model": "gpt-5.1-2025-11-13",
                "trace_id": "anon_1",
                "route": "anon_2",
                "tool_definitions": [
                    {
                        "name": "anon_3",
                        "description": "anon_4",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "city": { "type": "string", "description": "anon_5" }
                            }
                        }
                    }
                ]
            }
        })
    );
}

#[test]
fn anonymizes_metadata_variants_like_metadata2() {
    let input = json!({
        "metadata2": {
            "chatChannel": "SOLO_TOLAN:usr_abc",
            "chatID": "cht_123",
            "isFirstMessage": false
        }
    });

    let result = anonymize_json_value(input);
    assert_eq!(
        result.value,
        json!({
            "metadata2": {
                "chatChannel": "anon_1",
                "chatID": "anon_2",
                "isFirstMessage": false
            }
        })
    );
}

#[test]
fn removes_metadata_prompt_subtree_entirely() {
    let input = json!({
        "metadata": {
            "prompt": {
                "id": "prm_123",
                "key": "chat",
                "variables": {
                    "activeChatType": { "CONVERSATION_DEFAULT": true },
                    "medium": "TEXT"
                }
            },
            "route": "base"
        }
    });

    let result = anonymize_json_value(input);
    assert_eq!(
        result.value,
        json!({
            "metadata": {
                "route": "anon_1"
            }
        })
    );
}

#[test]
fn anonymizes_strings_under_context_and_output() {
    let input = json!({
        "context": {
            "caller_filename": "file:///tmp/project/src/main.ts",
            "caller_functionname": "runJob",
            "caller_lineno": 42
        },
        "output": "Final assistant response text",
        "model": "gpt-5.1-2025-11-13"
    });

    let result = anonymize_json_value(input);
    assert_eq!(
        result.value,
        json!({
            "context": {
                "caller_filename": "anon_1",
                "caller_functionname": "anon_2",
                "caller_lineno": 42
            },
            "output": "anon_3",
            "model": "gpt-5.1-2025-11-13"
        })
    );
}

#[test]
fn anonymizes_json_encoded_tool_call_arguments_strings() {
    let input = json!({
        "input": [
            {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "id": "toolu_123",
                        "type": "function",
                        "function": {
                            "name": "CreateTodoList",
                            "arguments": "{\"items\":[\"task_alpha\",\"task_beta\"],\"status\":\"queued\"}"
                        }
                    }
                ]
            }
        ]
    });

    let result = anonymize_json_value(input);
    assert_eq!(
        result.value,
        json!({
            "input": [
                {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [
                        {
                            "id": "toolu_123",
                            "type": "function",
                            "function": {
                                "name": "CreateTodoList",
                                "arguments": "{\"items\":[\"anon_1\",\"anon_2\"],\"status\":\"anon_3\"}"
                            }
                        }
                    ]
                }
            ]
        })
    );
}

#[test]
fn anonymizes_non_json_tool_call_arguments_strings_as_plain_strings() {
    let input = json!({
        "input": [
            {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "id": "toolu_123",
                        "type": "function",
                        "function": {
                            "name": "CreateTodoList",
                            "arguments": "not-json-content"
                        }
                    }
                ]
            }
        ]
    });

    let result = anonymize_json_value(input);
    assert_eq!(
        result.value,
        json!({
            "input": [
                {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [
                        {
                            "id": "toolu_123",
                            "type": "function",
                            "function": {
                                "name": "CreateTodoList",
                                "arguments": "anon_1"
                            }
                        }
                    ]
                }
            ]
        })
    );
}

#[test]
fn filter_sees_every_unanonymized_key_and_field_in_json() {
    let input = json!({
        "userEmail": "person@example.com",
        "count": 42,
        "enabled": true,
        "content": "sensitive content",
        "items": [
            { "name": "alpha", "qty": 2 },
            { "name": "beta", "price": 12.5 }
        ],
        "details": {
            "nullable": null,
            "tags": ["x", "y"]
        },
        "output": "final response",
        "role": "user"
    });

    let mut next = 1;
    let mut filter = |context: AnonymizeFilterContext<'_>, _value: &serde_json::Value| {
        let replacement = json!(format!("custom_{next}"));
        next += 1;

        match context.kind {
            AnonymizeFilterKind::Key | AnonymizeFilterKind::Value => Some(replacement),
        }
    };

    let result = anonymize_json_value_with_options_and_filter(
        input,
        AnonymizeOptions::default(),
        Some(&mut filter),
    );

    assert_eq!(
        result.value,
        json!({
            "custom_1": "custom_2",
            "custom_3": "custom_4",
            "custom_5": "custom_6",
            "custom_7": "anon_1",
            "custom_8": [
                { "custom_9": "custom_10", "custom_11": "custom_12" },
                { "custom_13": "custom_14", "custom_15": "custom_16" }
            ],
            "custom_17": {
                "custom_18": "custom_19",
                "custom_20": ["custom_21", "custom_22"]
            },
            "custom_23": "anon_2",
            "custom_24": "custom_25"
        })
    );
}
