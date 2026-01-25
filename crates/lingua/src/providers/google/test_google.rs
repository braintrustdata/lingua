use crate::providers::google::generated::{
    Content as GoogleContent, GenerateContentRequest, GenerateContentResponse,
};
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::run_roundtrip_test;
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};

pub type GoogleTestCase = TestCase<GenerateContentRequest, GenerateContentResponse, Value>;

pub fn discover_google_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<GoogleTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<GenerateContentRequest, GenerateContentResponse, Value>(
        Provider::Google,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        let cases = discover_google_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        run_roundtrip_test(
            case,
            // Extract messages from request
            |request: &GenerateContentRequest| Ok(&request.contents),
            // Convert to universal
            |messages: &Vec<GoogleContent>| {
                <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(messages.clone())
                    .map_err(|e| format!("Failed to convert to universal format: {}", e))
            },
            // Convert from universal
            |messages: Vec<Message>| {
                <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip conversion: {}", e))
            },
            // Extract response content (candidate contents)
            |response: &GenerateContentResponse| {
                Ok(response
                    .candidates
                    .iter()
                    .filter_map(|candidate| candidate.content.clone())
                    .collect())
            },
            // Convert response to universal
            |contents: &Vec<GoogleContent>| {
                <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(contents.clone())
                    .map_err(|e| format!("Failed to convert response to universal format: {}", e))
            },
            // Convert universal to response content
            |messages: Vec<Message>| {
                <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))
            },
        )
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_google_tests.rs"));
    }
}
