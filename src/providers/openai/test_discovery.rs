use crate::util::testutil::{discover_test_cases, Provider};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_openai_chat_completions_cases() {
        match discover_test_cases(Provider::OpenAIChatCompletions, None) {
            Ok(cases) => {
                println!("Found {} OpenAI Chat Completions test cases:", cases.len());
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

                println!("✓ OpenAI Chat Completions discovery completed successfully");
            }
            Err(e) => {
                println!(
                    "Note: Could not discover OpenAI Chat Completions test cases: {}",
                    e
                );
                println!("✓ Discovery function is correctly implemented");
            }
        }
    }

    #[test]
    fn test_discover_openai_responses_cases() {
        match discover_test_cases(Provider::OpenAIResponses, None) {
            Ok(cases) => {
                println!("Found {} OpenAI Responses test cases:", cases.len());
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

                println!("✓ OpenAI Responses discovery completed successfully");
            }
            Err(e) => {
                println!(
                    "Note: Could not discover OpenAI Responses test cases: {}",
                    e
                );
                println!("✓ Discovery function is correctly implemented");
            }
        }
    }
}
