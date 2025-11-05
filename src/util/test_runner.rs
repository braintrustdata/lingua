#[cfg(test)]
use crate::universal::Message;
#[cfg(test)]
use crate::util::testutil::TestCase;
#[cfg(test)]
use serde::{de::DeserializeOwned, Serialize};

/// Common test runner functions that take concrete types and run the test logic
#[cfg(test)]
pub fn run_roundtrip_test<Req, Resp, StreamResp, ProviderMessage, ResponseContent>(
    test_case: &TestCase<Req, Resp, StreamResp>,
    extract_messages: impl Fn(&Req) -> Result<&Vec<ProviderMessage>, String>,
    convert_to_universal: impl Fn(&Vec<ProviderMessage>) -> Result<Vec<Message>, String>,
    convert_from_universal: impl Fn(Vec<Message>) -> Result<Vec<ProviderMessage>, String>,
    extract_response_content: impl Fn(&Resp) -> Result<ResponseContent, String>,
    convert_response_to_universal: impl Fn(&ResponseContent) -> Result<Vec<Message>, String>,
    convert_universal_to_response: impl Fn(Vec<Message>) -> Result<ResponseContent, String>,
) -> Result<(), String>
where
    ProviderMessage: Clone + Serialize + DeserializeOwned,
    ResponseContent: Clone + Serialize + DeserializeOwned,
{
    use crate::util::testutil::diff_serializable;
    use log::{debug, info};

    // Initialize env_logger if not already done
    let _ = env_logger::try_init();

    info!("🧪 Testing roundtrip conversion for: {}", test_case.name);

    let messages = extract_messages(&test_case.request)?;

    // Log conversion steps
    debug!("📄 Original: {} Messages", messages.len());
    debug!("\n{}", serde_json::to_string_pretty(&messages).unwrap());

    debug!("🔄 Converting to universal format...");

    // Convert to universal format
    let universal_request = convert_to_universal(messages)?;

    debug!("✓ Universal: {} Messages", universal_request.len());
    debug!(
        "\n{}",
        serde_json::to_string_pretty(&universal_request).unwrap()
    );

    debug!("↩️  Converting back to provider format...");

    // Convert back to provider format
    let roundtripped = convert_from_universal(universal_request.clone())?;

    debug!("\n{}", serde_json::to_string_pretty(&roundtripped).unwrap());

    // Compare original and roundtripped messages
    let diff = diff_serializable(messages, &roundtripped, "messages");
    if !diff.starts_with("✅") {
        return Err(format!("Roundtrip conversion failed:\n{}", diff));
    }

    println!(
        "✅ {} - request roundtrip conversion passed",
        test_case.name
    );

    // Test response conversion if available
    if let Some(response) = &test_case.non_streaming_response {
        info!("🧪 Testing response conversion for: {}", test_case.name);

        let response_content = extract_response_content(response)?;

        debug!(
            "📄 Response Original: {} items",
            // This is a generic debug message since we don't know the exact structure
            serde_json::to_string_pretty(&response_content)
                .map(|s| s.lines().count())
                .unwrap_or(0)
        );
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&response_content).unwrap()
        );

        debug!("🔄 Converting response to universal format...");

        // Convert response to universal format
        let universal_response = convert_response_to_universal(&response_content)?;

        debug!(
            "✓ Universal Response: {} Messages",
            universal_response.len()
        );
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&universal_response).unwrap()
        );

        debug!("↩️  Converting response back to provider format...");

        // Convert back to provider response format
        let roundtripped_response = convert_universal_to_response(universal_response.clone())?;

        debug!(
            "\n{}",
            serde_json::to_string_pretty(&roundtripped_response).unwrap()
        );

        // Compare response using the same colored diff as request roundtrip
        let diff = diff_serializable(&[&response_content], &[&roundtripped_response], "response");
        if !diff.starts_with("✅") {
            return Err(format!("Response roundtrip conversion failed:\n{}", diff));
        }

        println!(
            "✅ {} - response roundtrip conversion passed",
            test_case.name
        );
    }

    println!("✅ {} - all conversions passed", test_case.name);
    Ok(())
}
