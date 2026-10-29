use super::request::{SystemOneRequest, TypeSafeQuestion};
use super::response::SystemOneResponse;
use super::{decision_request_to_system_one, system_one_response_to_decision};
use crate::error::ConvertError;
use crate::providers::openai::decisions_generated::{DecisionRequest, DecisionResponse};
use crate::serde_json::value::RawValue;
use crate::serde_json::{self, json, Value};
use serde::Deserialize;
use std::collections::BTreeMap;

const SPEC: &str = include_str!("../../../../../specs/typesafe/openapi.json");

#[derive(Deserialize)]
struct SpecView {
    paths: PathsView,
}

#[derive(Deserialize)]
struct PathsView {
    #[serde(rename = "/v1/systemone")]
    system_one: PathItemView,
}

#[derive(Deserialize)]
struct PathItemView {
    post: OperationView,
}

#[derive(Deserialize)]
struct OperationView {
    #[serde(rename = "requestBody")]
    request_body: ContentView,
    responses: BTreeMap<String, ContentView>,
}

#[derive(Deserialize)]
struct ContentView {
    content: BTreeMap<String, MediaView>,
}

#[derive(Deserialize)]
struct MediaView {
    #[serde(default)]
    examples: BTreeMap<String, ExampleView>,
}

#[derive(Deserialize)]
struct ExampleView {
    value: Value,
}

fn spec_examples(content: &ContentView) -> Vec<(String, Value)> {
    content.content["application/json"]
        .examples
        .iter()
        .map(|(name, example)| (name.clone(), example.value.clone()))
        .collect()
}

fn spec() -> OperationView {
    serde_json::from_str::<SpecView>(SPEC)
        .expect("TypeSafe spec parses")
        .paths
        .system_one
        .post
}

#[test]
fn system_one_request_examples_round_trip() {
    let operation = spec();
    let examples = spec_examples(&operation.request_body);
    assert_eq!(examples.len(), 4);
    for (name, value) in examples {
        let request: SystemOneRequest = serde_json::from_value(value.clone())
            .unwrap_or_else(|error| panic!("{name} request parses: {error}"));
        assert_eq!(serde_json::to_value(&request).unwrap(), value, "{name}");
    }
}

#[test]
fn system_one_response_examples_round_trip() {
    let operation = spec();
    let examples = spec_examples(&operation.responses["200"]);
    assert_eq!(examples.len(), 4);
    for (name, value) in examples {
        let response: SystemOneResponse = serde_json::from_value(value.clone())
            .unwrap_or_else(|error| panic!("{name} response parses: {error}"));
        assert_eq!(serde_json::to_value(&response).unwrap(), value, "{name}");
    }
}

#[test]
fn choice_criteria_keep_caller_order() {
    let request: SystemOneRequest = serde_json::from_str(
        r#"{"model":"jev-latest","state":"s","questions":{"q":{"type":"choice","criteria":{"z":null,"a":"first"}}}}"#,
    )
    .unwrap();
    let TypeSafeQuestion::Choice { criteria, .. } = &request.questions[0].1 else {
        panic!("expected a choice question");
    };
    assert_eq!(
        criteria
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>(),
        ["z", "a"]
    );
    assert_eq!(
        serde_json::to_string(&request).unwrap(),
        r#"{"model":"jev-latest","state":"s","questions":{"q":{"type":"choice","criteria":{"z":null,"a":"first"}}}}"#
    );
}

fn decision_request(value: Value) -> DecisionRequest {
    serde_json::from_value(value).expect("decision request parses")
}

#[derive(Deserialize)]
struct RawInput<'a> {
    #[serde(borrow)]
    input: &'a RawValue,
}

fn convert_text(body: &str) -> Result<String, ConvertError> {
    let raw: RawInput<'_> = serde_json::from_str(body).unwrap();
    let request = decision_request(serde_json::from_str(body).unwrap());
    decision_request_to_system_one(&request, raw.input)
        .map(|(request, _)| serde_json::to_string(&request).unwrap())
}

fn convert(value: Value) -> Result<Value, ConvertError> {
    convert_text(&value.to_string()).map(|text| serde_json::from_str(&text).unwrap())
}

#[test]
fn converts_scorer_conversation_to_system_one() {
    let converted = convert(json!({
        "model": "jev-latest",
        "input": [
            {"role": "system", "content": "Grade the answer."},
            {"role": "user", "content": [{"type": "input_text", "text": "2 + 2 = 4"}]},
            {"type": "function_call", "call_id": "c1", "name": "lookup", "arguments": "{}"},
            {"type": "function_call_output", "call_id": "c1", "output": "4"}
        ],
        "questions": [{
            "type": "choice",
            "name": "score",
            "instructions": "Pick the grade.",
            "choices": [{"value": "B"}, {"value": "A", "description": "Correct"}]
        }]
    }))
    .unwrap();
    assert_eq!(
        converted,
        json!({
            "model": "jev-latest",
            "state": {"messages": [
                {"role": "system", "content": "Grade the answer."},
                {"role": "user", "content": [{"type": "input_text", "text": "2 + 2 = 4"}]},
                {"type": "function_call", "call_id": "c1", "name": "lookup", "arguments": "{}"},
                {"type": "function_call_output", "call_id": "c1", "output": "4"}
            ]},
            "questions": {"score": {
                "type": "choice",
                "instructions": "Pick the grade.",
                "criteria": {"B": null, "A": "Correct"}
            }}
        })
    );
}

#[test]
fn copies_input_items_as_sent() {
    let converted = convert_text(
        r#"{"model":"jev-latest","questions":[{"type":"predicate","name":"q","instructions":"?"}],"input":[{"role":"user","content":"x = 1.50","type":"message"},{"type":"function_call_output","output":"{\"z\":1}","call_id":"c1"}]}"#,
    )
    .unwrap();
    assert_eq!(
        converted,
        r#"{"model":"jev-latest","state":{"messages":[{"role":"user","content":"x = 1.50","type":"message"},{"type":"function_call_output","output":"{\"z\":1}","call_id":"c1"}]},"questions":{"q":{"type":"noul","instructions":"?"}}}"#
    );
}

#[test]
fn converts_string_input_predicates_and_scores() {
    let converted = convert(json!({
        "model": "jev-latest",
        "input": "Export fails in Safari.",
        "questions": [
            {"type": "predicate", "instructions": "Is this a bug report?"},
            {
                "type": "score",
                "name": "severity",
                "instructions": [
                    {"type": "input_text", "text": "Rate severity."},
                    {"type": "input_json", "data": {"product": "web"}}
                ],
                "levels": [{"label": "low"}, {"label": "high", "description": "Blocks work"}]
            }
        ]
    }))
    .unwrap();
    assert_eq!(
        converted,
        json!({
            "model": "jev-latest",
            "state": "Export fails in Safari.",
            "questions": {
                "0": {"type": "noul", "instructions": "Is this a bug report?"},
                "severity": {
                    "type": "score",
                    "instructions": ["Rate severity.", {"product": "web"}],
                    "criteria": ["low", {"label": "high", "description": "Blocks work"}]
                }
            }
        })
    );
}

#[test]
fn rejects_requests_system_one_cannot_represent() {
    let base = || {
        json!({
            "model": "jev-latest",
            "input": "x",
            "questions": [{"type": "predicate", "name": "q", "instructions": "?"}]
        })
    };
    let cases: Vec<(&str, Value)> = vec![
        ("prompt caching", {
            let mut request = base();
            request["prompt_cache_key"] = json!("k");
            request
        }),
        ("safety_identifier", {
            let mut request = base();
            request["safety_identifier"] = json!("user-1");
            request
        }),
        ("input_image content", {
            let mut request = base();
            request["input"] = json!([{"role": "user", "content": [
                {"type": "input_image", "image_url": "https://example.com/a.png"}
            ]}]);
            request
        }),
        ("prompt_cache_breakpoint", {
            let mut request = base();
            request["input"] = json!([{"role": "user", "content": [
                {"type": "input_text", "text": "x", "prompt_cache_breakpoint": {"mode": "explicit"}}
            ]}]);
            request
        }),
        ("input_image instructions", {
            let mut request = base();
            request["questions"][0]["instructions"] =
                json!([{"type": "input_image", "image_url": "data:image/png;base64,AA=="}]);
            request
        }),
        ("not unique", {
            let mut request = base();
            request["questions"] = json!([
                {"type": "predicate", "name": "q", "instructions": "?"},
                {"type": "predicate", "name": "q", "instructions": "!"}
            ]);
            request
        }),
        ("more than once", {
            let mut request = base();
            request["questions"] = json!([{"type": "choice", "name": "q", "instructions": "?",
                "choices": [{"value": "true"}, {"value": true}]}]);
            request
        }),
        ("input[0].role", {
            let mut request = base();
            request["input"] = json!([{"content": "x"}]);
            request
        }),
    ];
    for (expected, request) in cases {
        let error = convert(request).expect_err(expected).to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    }
}

fn round_trip(request: Value, system_one_response: Value) -> Result<Value, ConvertError> {
    let body = request.to_string();
    let raw: RawInput<'_> = serde_json::from_str(&body).unwrap();
    let (_, context) =
        decision_request_to_system_one(&decision_request(request), raw.input).unwrap();
    let response: SystemOneResponse = serde_json::from_value(system_one_response).unwrap();
    system_one_response_to_decision(response, &context)
        .map(|decision| serde_json::to_value(decision).unwrap())
}

#[test]
fn converts_system_one_answers_in_question_order() {
    let decision = round_trip(
        json!({
            "model": "jev-latest",
            "input": "x",
            "questions": [
                {"type": "choice", "instructions": "?",
                    "choices": [{"value": false}, {"value": true}, {"value": "unsure"}]},
                {"type": "score", "name": "severity", "instructions": "?",
                    "levels": [{"label": "low"}, {"label": "high"}]},
                {"type": "predicate", "name": "bug", "instructions": "?"}
            ]
        }),
        json!({
            "model": "jev-1.14.0",
            "answers": {
                "bug": {"type": "noul", "noul": 0.25},
                "severity": {"type": "score", "score": 0.75, "confidence": 0.5,
                    "legend": {"0": "low", "1": "high"},
                    "probabilities": {"0": 0.25, "1": 0.75}},
                "0": {"type": "choice", "choice": "true", "confidence": 0.5,
                    "probabilities": {"unsure": 0.125, "true": 0.75, "false": 0.125}}
            },
            "usage": {"input_tokens": 120, "output_tokens": 3}
        }),
    )
    .unwrap();
    assert_eq!(
        decision,
        json!({
            "model": "jev-1.14.0",
            "answers": [
                {"type": "choice", "name": null, "choice": true, "confidence": 0.5,
                    "probabilities": [
                        {"value": false, "probability": 0.125},
                        {"value": true, "probability": 0.75},
                        {"value": "unsure", "probability": 0.125}
                    ]},
                {"type": "score", "name": "severity", "score": 0.75, "confidence": 0.5,
                    "probabilities": [
                        {"label": "low", "value": 0, "probability": 0.25},
                        {"label": "high", "value": 1, "probability": 0.75}
                    ]},
                {"type": "predicate", "name": "bug", "probability": 0.25}
            ],
            "usage": {
                "input_tokens": 120,
                "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
                "output_tokens": 3,
                "output_tokens_details": {"reasoning_tokens": 0},
                "total_tokens": 123
            }
        })
    );
    let parsed: DecisionResponse = serde_json::from_value(decision).unwrap();
    assert_eq!(parsed.answers.len(), 3);
}

#[test]
fn rejects_system_one_answers_that_do_not_match_the_request() {
    let request = json!({
        "model": "jev-latest",
        "input": "x",
        "questions": [{"type": "choice", "name": "q", "instructions": "?",
            "choices": [{"value": "a"}, {"value": "b"}]}]
    });
    let usage = json!({"input_tokens": 1, "output_tokens": 0});
    let cases = [
        (
            "answers.q",
            json!({"model": "m", "answers": {"other": {"type": "noul", "noul": 1.0}}, "usage": usage}),
        ),
        (
            "unknown choice \"c\"",
            json!({"model": "m", "answers": {"q": {"type": "choice", "choice": "c", "confidence": 1.0,
                "probabilities": {"a": 0.0, "b": 0.0}}}, "usage": usage}),
        ),
        (
            "answers.q.probabilities.b",
            json!({"model": "m", "answers": {"q": {"type": "choice", "choice": "a", "confidence": 1.0,
                "probabilities": {"a": 1.0}}}, "usage": usage}),
        ),
        (
            "noul answer for choice question",
            json!({"model": "m", "answers": {"q": {"type": "noul", "noul": 1.0}}, "usage": usage}),
        ),
    ];
    for (expected, response) in cases {
        let error = round_trip(request.clone(), response)
            .expect_err(expected)
            .to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    }
}
