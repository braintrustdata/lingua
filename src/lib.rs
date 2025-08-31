/*!
# Elmir - LLM Intermediate Representation

A universal message format for large language model APIs that compiles to provider-specific formats with zero runtime overhead.

## Example

```rust
use elmir::universal::{SimpleMessage, SimpleRole};

// Create messages using convenience methods
let messages = vec![
    SimpleMessage::user("You are a helpful assistant."),
    SimpleMessage::user("Hello, world!"),
];

// Or create manually
let message = SimpleMessage::user("Hello, world!");
assert_eq!(message.role, SimpleRole::User);
```
*/

pub mod capabilities;
pub mod providers;
pub mod translators;
pub mod universal;

// Re-export commonly used types
pub use universal::{SimpleMessage, SimpleRole};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let message = SimpleMessage::user("Hello, world!");
        assert_eq!(message.role, SimpleRole::User);
        assert_eq!(message.content, "Hello, world!");
    }

    #[test]
    fn test_assistant_message() {
        let message = SimpleMessage::assistant("I'm doing well!");
        assert_eq!(message.role, SimpleRole::Assistant);
        assert_eq!(message.content, "I'm doing well!");
    }
}
