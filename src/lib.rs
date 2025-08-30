/*!
# LLMIR - LLM Intermediate Representation

A universal message format for large language model APIs that compiles to provider-specific formats with zero runtime overhead.

## Example

```rust
use llmir::universal::{Message, MessageRole};

// Create messages using convenience methods
let messages = vec![
    Message::system("You are a helpful assistant."),
    Message::user("Hello, world!"),
];

// Or create manually
let message = Message::user("Hello, world!");
assert_eq!(message.role, MessageRole::User);
```
*/

pub mod universal;
pub mod providers;
pub mod capabilities;
pub mod translators;

// Re-export commonly used types
pub use universal::{Message, MessageRole, ContentBlock, ContentType};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let message = Message {
            role: MessageRole::User,
            content: vec![ContentBlock {
                content_type: ContentType::Text,
                data: "Hello, world!".to_string(),
                metadata: None,
            }],
            metadata: None,
        };
        
        assert_eq!(message.role, MessageRole::User);
    }
}