use llmir::universal::{Message, MessageRole, ContentBlock, ContentType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create some basic messages
    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("What is 2 + 2?"),
        Message::assistant("2 + 2 equals 4."),
    ];

    // Print the messages
    for (i, message) in messages.iter().enumerate() {
        println!("Message {}: {:?}", i + 1, message);
    }

    // Serialize to JSON to see the format
    let json = serde_json::to_string_pretty(&messages)?;
    println!("\nJSON representation:");
    println!("{}", json);

    Ok(())
}