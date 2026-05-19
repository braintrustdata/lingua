use braintrust_llm_router::ClientHeaders;
use http::HeaderMap;

fn apply_headers(cases: &[(&str, &str, bool)]) -> HeaderMap {
    let header_pairs = cases
        .iter()
        .map(|(name, value, _)| (name.to_string(), value.to_string()))
        .collect::<Vec<_>>();
    let client_headers: ClientHeaders = header_pairs.into_iter().collect();
    let mut headers = HeaderMap::new();
    client_headers.apply(&mut headers);
    headers
}

#[test]
fn client_headers_filter_and_host_behavior() {
    let cases = [
        ("x-amzn-trace-id", "1", false),
        ("x-bt-project-id", "1", false),
        ("sec-fetch-mode", "cors", false),
        ("content-length", "123", false),
        ("origin", "https://example.com", false),
        ("priority", "u=1", false),
        ("referer", "https://example.com", false),
        ("user-agent", "test", false),
        ("cache-control", "no-cache", false),
        ("host", "api.example.com", false),
        ("anthropic-beta", "tools-2024-05-16", true),
        ("accept", "application/json", true),
        ("x-custom-header", "1", true),
    ];

    let headers = apply_headers(&cases);
    for (name, _value, expected) in cases {
        assert_eq!(headers.contains_key(name), expected, "header {name}");
    }
}

#[test]
fn user_configured_headers_are_not_filtered() {
    let client_headers = ClientHeaders::with_user_configured_headers([
        ("authorization".to_string(), "Bearer configured".to_string()),
        ("host".to_string(), "configured.example.com".to_string()),
        (
            "x-bt-project-id".to_string(),
            "configured-project".to_string(),
        ),
    ])
    .expect("headers");
    let mut headers = HeaderMap::new();

    client_headers.apply(&mut headers);

    assert_eq!(
        headers.get("authorization").and_then(|v| v.to_str().ok()),
        Some("Bearer configured")
    );
    assert_eq!(
        headers.get("host").and_then(|v| v.to_str().ok()),
        Some("configured.example.com")
    );
    assert_eq!(
        headers.get("x-bt-project-id").and_then(|v| v.to_str().ok()),
        Some("configured-project")
    );
}
