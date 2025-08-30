use llmir::universal::SimpleMessage;
use llmir::translators::to_openai_format;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create some simple messages
    let messages = vec![
        SimpleMessage::user("Hello, how are you?"),
        SimpleMessage::assistant("I'm doing well, thank you! How can I help you today?"),
        SimpleMessage::user("What is 2 + 2?"),
    ];

    // Convert to OpenAI format
    let openai_request = to_openai_format(messages)?;

    // Print the OpenAI request
    let json = serde_json::to_string_pretty(&openai_request)?;
    println!("OpenAI Request:");
    println!("{}", json);

    Ok(())
}