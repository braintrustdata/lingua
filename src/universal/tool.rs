/*!
Universal tool type definitions for Lingua.

Tools come in two categories:
- **Client tools**: Functions defined and executed by the client application
- **Provider tools**: Capabilities built into the provider's infrastructure

This module provides a unified representation that can be converted to/from
provider-specific formats using the `TryFromLLM` trait.
*/

use crate::serde_json;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

/// Universal tool type - union of client-defined and provider-native tools
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Tool {
    /// Client-defined function tool (client implements the execution)
    #[serde(rename = "function")]
    Client(ClientTool),

    /// Provider-native tool (provider implements the execution)
    #[serde(rename = "provider")]
    Provider(ProviderTool),
}

/// Client-defined function tool
///
/// Client tools are executed by your application code, not by the provider.
/// When the model calls a client tool:
/// 1. You receive a tool call with the function name and arguments
/// 2. Your application executes the function
/// 3. You return the result to the model
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
pub struct ClientTool {
    /// Unique identifier for the tool
    pub name: String,

    /// Detailed description of what the tool does
    ///
    /// Tool descriptions are critical for LLM performance. The more information
    /// the model has about what the tool does and how to use it, the better it
    /// will perform.
    pub description: String,

    /// JSON Schema defining the tool's input parameters
    ///
    /// This should be a valid JSON Schema object (usually with `type: "object"`).
    #[ts(type = "Record<string, any>")]
    pub input_schema: serde_json::Value,

    /// Optional provider-specific options
    ///
    /// This is an escape hatch for provider-specific features like OpenAI's
    /// `strict` mode. The contents depend on the target provider.
    #[ts(type = "Record<string, any> | undefined")]
    pub provider_options: Option<serde_json::Value>,
}

/// Provider-native tool (executed by the provider, not client code)
///
/// Provider tools are executed by the LLM provider's infrastructure.
/// These tools may have additional costs and are provider-specific.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
pub struct ProviderTool {
    /// Provider-specific tool type identifier
    ///
    /// Examples:
    /// - `"web_search_20250305"` (Anthropic)
    /// - `"bash_20250124"` (Anthropic)
    /// - `"code_execution"` (Google)
    /// - `"computer_20250124"` (OpenAI)
    pub tool_type: String,

    /// Optional name override
    ///
    /// If not provided, defaults to `tool_type` when converting to provider format.
    pub name: Option<String>,

    /// Tool-specific configuration (provider-dependent)
    ///
    /// The structure of this configuration depends on the specific tool type
    /// and provider. For example, Anthropic's web search tool accepts:
    /// - `max_uses`: number
    /// - `allowed_domains`: string[]
    /// - `blocked_domains`: string[]
    /// - `user_location`: object
    #[ts(type = "Record<string, any> | undefined")]
    pub config: Option<serde_json::Value>,
}
