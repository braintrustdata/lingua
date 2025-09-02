use super::message::*;

#[test]
fn test_basic_message() {
    let message_list = vec![
        Message::System {
            content: Content::Text("You are a helpful assistant.".to_string()),
        },
        Message::User {
            content: Content::ContentList(vec![ContentPart::Text {
                text: "Show me a picture of a cat.".to_string(),
                cache_control: None,
                citations: None,
            }]),
        },
        Message::Assistant {
            content: Content::ContentList(vec![ContentPart::Image {
                url: "https://example.com/cat.png".to_string(),
                detail: None,
                cache_control: None,
            }]),
        },
    ];

    let serialized = serde_json::to_string_pretty(&message_list).unwrap();
    eprintln!("{}", serialized);
}
