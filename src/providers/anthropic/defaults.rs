/*!
Anthropic-specific request parameter defaults.

Anthropic's Messages API requires `max_tokens` to be specified in every request.
This module provides the default value to use when it's not specified in the
source request (matching the legacy proxy behavior).
*/

use crate::capabilities::ProviderFormat;
use crate::processing::defaults::RequestDefaults;
use crate::serde_json::Value;

/// Default max_tokens for Anthropic requests.
///
/// This matches the legacy proxy behavior (see `packages/proxy/src/providers/anthropic.ts`).
pub const DEFAULT_MAX_TOKENS: i64 = 4096;

/// Anthropic-specific request defaults.
///
/// Ensures that required fields like `max_tokens` have sensible defaults
/// when not provided in the source request.
pub struct AnthropicDefaults;

impl RequestDefaults for AnthropicDefaults {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Anthropic
    }

    fn default_max_tokens(&self) -> Option<i64> {
        Some(DEFAULT_MAX_TOKENS)
    }

    fn apply_defaults(&self, payload: &mut Value) {
        if let Value::Object(ref mut obj) = payload {
            // Anthropic requires max_tokens - add default if missing
            if !obj.contains_key("max_tokens") {
                obj.insert(
                    "max_tokens".into(),
                    Value::Number(DEFAULT_MAX_TOKENS.into()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_anthropic_defaults_adds_max_tokens() {
        let defaults = AnthropicDefaults;
        let mut payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        defaults.apply_defaults(&mut payload);

        assert_eq!(
            payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(DEFAULT_MAX_TOKENS)
        );
    }

    #[test]
    fn test_anthropic_defaults_preserves_existing_max_tokens() {
        let defaults = AnthropicDefaults;
        let mut payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 8192
        });

        defaults.apply_defaults(&mut payload);

        assert_eq!(
            payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(8192)
        );
    }

    #[test]
    fn test_anthropic_defaults_format() {
        let defaults = AnthropicDefaults;
        assert_eq!(defaults.format(), ProviderFormat::Anthropic);
    }

    #[test]
    fn test_anthropic_default_max_tokens_value() {
        let defaults = AnthropicDefaults;
        assert_eq!(defaults.default_max_tokens(), Some(4096));
    }
}
