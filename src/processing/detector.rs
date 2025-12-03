/*!
Format detector trait for provider payload detection.

This module defines the `FormatDetector` trait that each provider implements
to detect if a payload matches their format. This enables a plugin-style
architecture where detection logic lives in provider modules.

## Adding a New Provider

1. Create a detector struct in your provider module (e.g., `providers/myprovider/detect.rs`)
2. Implement `FormatDetector` for your struct
3. Register it in the `detectors()` function in `processing/detect.rs`

## Priority Guidelines

Higher priority = checked first. Use these ranges:
- 90-100: Highly distinctive formats (e.g., Bedrock with `modelId`)
- 70-89: Distinctive formats (e.g., Google with `contents/parts`, Anthropic with `max_tokens`)
- 50-69: Common formats with some signals (e.g., Mistral with model prefix)
- 30-49: Fallback formats (e.g., OpenAI as the most permissive)
*/

use crate::capabilities::ProviderFormat;
use crate::serde_json::Value;

/// Trait for detecting if a JSON payload matches a specific provider format.
///
/// Implementations should use heuristics to quickly determine if a payload
/// is likely in their format. The detection should be fast (no full parsing)
/// and err on the side of false negatives rather than false positives.
///
/// # Example
///
/// ```ignore
/// use lingua::processing::FormatDetector;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::Value;
///
/// pub struct MyProviderDetector;
///
/// impl FormatDetector for MyProviderDetector {
///     fn format(&self) -> ProviderFormat {
///         ProviderFormat::MyProvider
///     }
///
///     fn detect(&self, payload: &Value) -> bool {
///         // Check for provider-specific fields
///         payload.get("my_specific_field").is_some()
///     }
///
///     fn priority(&self) -> u8 {
///         75 // Checked after highly distinctive formats
///     }
///
///     fn confidence(&self) -> f32 {
///         0.85 // High confidence when detected
///     }
/// }
/// ```
pub trait FormatDetector: Send + Sync {
    /// Returns the provider format this detector identifies.
    fn format(&self) -> ProviderFormat;

    /// Checks if the payload matches this provider's format.
    ///
    /// This should be a fast heuristic check, not a full parse.
    /// Return `true` if the payload appears to match this format.
    fn detect(&self, payload: &Value) -> bool;

    /// Returns the detection priority (higher = checked first).
    ///
    /// Use higher values for more distinctive formats to avoid
    /// false positives from more permissive detectors.
    ///
    /// Recommended ranges:
    /// - 90-100: Highly distinctive (unique field names)
    /// - 70-89: Distinctive (specific structure)
    /// - 50-69: Common with some signals
    /// - 30-49: Fallback/permissive
    fn priority(&self) -> u8;

    /// Returns the confidence level when this format is detected (0.0-1.0).
    ///
    /// - 1.0: Certain (e.g., catalog lookup)
    /// - 0.8-0.99: High confidence (distinctive signals)
    /// - 0.5-0.79: Medium confidence (some signals)
    /// - Below 0.5: Low confidence (mostly a guess)
    fn confidence(&self) -> f32;
}
