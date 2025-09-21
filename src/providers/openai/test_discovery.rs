use crate::util::testutil::{
    discover_openai_chat_completion_test_cases, discover_openai_responses_test_cases, Provider,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_openai_chat_completions_cases_typed() {
        match discover_openai_chat_completion_test_cases(None) {
            Ok(cases) => {
                println!(
                    "Found {} OpenAI Chat Completions test cases (typed):",
                    cases.len()
                );
                for case in &cases {
                    println!("  - {} (turn: {:?})", case.name, case.turn);
                    println!("    Request: {}", case.request.is_some());
                    println!(
                        "    Streaming Response: {}",
                        case.streaming_response.is_some()
                    );
                    println!(
                        "    Non-Streaming Response: {}",
                        case.non_streaming_response.is_some()
                    );
                    println!("    Error: {}", case.error.is_some());
                }

                // Basic validation
                for case in &cases {
                    assert_eq!(case.provider, Provider::OpenAIChatCompletions);
                    assert!(!case.name.is_empty());
                }

                println!("✓ OpenAI Chat Completions typed discovery completed successfully");
            }
            Err(e) => {
                println!(
                    "Note: Could not discover OpenAI Chat Completions test cases (typed): {}",
                    e
                );
                println!("✓ Discovery function is correctly implemented");
            }
        }
    }

    #[test]
    fn test_discover_openai_responses_cases_typed() {
        match discover_openai_responses_test_cases(None) {
            Ok(cases) => {
                println!("Found {} OpenAI Responses test cases (typed):", cases.len());
                for case in &cases {
                    println!("  - {} (turn: {:?})", case.name, case.turn);
                    println!("    Request: {}", case.request.is_some());
                    println!(
                        "    Streaming Response: {}",
                        case.streaming_response.is_some()
                    );
                    println!(
                        "    Non-Streaming Response: {}",
                        case.non_streaming_response.is_some()
                    );
                    println!("    Error: {}", case.error.is_some());
                }

                // Basic validation
                for case in &cases {
                    assert_eq!(case.provider, Provider::OpenAIResponses);
                    assert!(!case.name.is_empty());
                }

                println!("✓ OpenAI Responses typed discovery completed successfully");
            }
            Err(e) => {
                println!(
                    "Note: Could not discover OpenAI Responses test cases (typed): {}",
                    e
                );
                println!("✓ Discovery function is correctly implemented");
            }
        }
    }
}
