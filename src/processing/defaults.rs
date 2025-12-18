/*!
Provider-specific request parameter defaults.

This module defines the `RequestDefaults` trait that providers implement
to specify required fields and their default values. This follows the same
plugin-style architecture as `FormatDetector`.

## Adding a New Provider

1. Create a defaults struct in your provider module (e.g., `providers/myprovider/defaults.rs`)
2. Implement `RequestDefaults` for your struct
3. Register it in `get_defaults_for_format()` in this module
*/

use crate::capabilities::ProviderFormat;
use crate::serde_json::Value;

/// Trait for provider-specific request parameter defaults.
///
/// Providers implement this to specify required fields and their defaults.
/// This enables automatic application of provider-specific requirements
/// when transforming requests between formats.
///
/// # Example
///
/// ```
/// use lingua::processing::defaults::RequestDefaults;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::Value;
///
/// pub struct MyProviderDefaults;
///
/// impl RequestDefaults for MyProviderDefaults {
///     fn format(&self) -> ProviderFormat {
///         ProviderFormat::OpenAI
///     }
///
///     fn default_max_tokens(&self) -> Option<i64> {
///         Some(4096)
///     }
///
///     fn apply_defaults(&self, payload: &mut Value) {
///         if let Value::Object(ref mut obj) = payload {
///             if !obj.contains_key("max_tokens") {
///                 obj.insert("max_tokens".into(), Value::Number(4096.into()));
///             }
///         }
///     }
/// }
/// ```
pub trait RequestDefaults: Send + Sync {
    /// Returns the provider format this applies to.
    fn format(&self) -> ProviderFormat;

    /// Returns default value for max_tokens if not specified.
    /// Returns None if the provider doesn't require this field.
    fn default_max_tokens(&self) -> Option<i64> {
        None
    }

    /// Apply provider-specific defaults to a request payload.
    ///
    /// This method modifies the payload in place, adding any required
    /// fields that are missing with their default values.
    fn apply_defaults(&self, payload: &mut Value);
}

/// Get the RequestDefaults implementation for a given provider format.
///
/// Returns None if the format doesn't have specific defaults.
#[cfg(feature = "anthropic")]
pub fn get_defaults_for_format(format: ProviderFormat) -> Option<Box<dyn RequestDefaults>> {
    match format {
        ProviderFormat::Anthropic => Some(Box::new(crate::providers::anthropic::AnthropicDefaults)),
        _ => None,
    }
}

#[cfg(not(feature = "anthropic"))]
pub fn get_defaults_for_format(_format: ProviderFormat) -> Option<Box<dyn RequestDefaults>> {
    None
}

/// Apply provider-specific defaults to a payload for the given target format.
///
/// This is a convenience function that looks up the appropriate defaults
/// implementation and applies it to the payload.
pub fn apply_provider_defaults(payload: &mut Value, target_format: ProviderFormat) {
    if let Some(defaults) = get_defaults_for_format(target_format) {
        defaults.apply_defaults(payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    struct TestDefaults;

    impl RequestDefaults for TestDefaults {
        fn format(&self) -> ProviderFormat {
            ProviderFormat::OpenAI
        }

        fn default_max_tokens(&self) -> Option<i64> {
            Some(1000)
        }

        fn apply_defaults(&self, payload: &mut Value) {
            if let Value::Object(ref mut obj) = payload {
                if !obj.contains_key("max_tokens") {
                    obj.insert("max_tokens".into(), Value::Number(1000.into()));
                }
            }
        }
    }

    #[test]
    fn test_apply_defaults_adds_missing_field() {
        let defaults = TestDefaults;
        let mut payload = json!({
            "model": "test",
            "messages": []
        });

        defaults.apply_defaults(&mut payload);

        assert_eq!(
            payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(1000)
        );
    }

    #[test]
    fn test_apply_defaults_preserves_existing_field() {
        let defaults = TestDefaults;
        let mut payload = json!({
            "model": "test",
            "messages": [],
            "max_tokens": 500
        });

        defaults.apply_defaults(&mut payload);

        assert_eq!(
            payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(500)
        );
    }

    #[test]
    fn test_default_max_tokens() {
        let defaults = TestDefaults;
        assert_eq!(defaults.default_max_tokens(), Some(1000));
    }
}
