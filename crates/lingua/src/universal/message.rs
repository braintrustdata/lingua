use crate::serde_json;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

pub type Thread = Vec<Message>;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: UserContent,
    },
    User {
        content: UserContent,
    },
    Assistant {
        content: AssistantContent,
        #[ts(optional)]
        id: Option<String>,
    },
    Tool {
        content: ToolContent,
    },
}

#[derive(Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct UserContent(#[ts(type = "UserContentPart[]")] Vec<UserContentPart>);

impl UserContent {
    /// Create from a vector of content parts
    pub fn new(parts: Vec<UserContentPart>) -> Self {
        Self(parts)
    }

    /// Get a reference to the content parts
    pub fn parts(&self) -> &[UserContentPart] {
        &self.0
    }

    /// Consume and return the inner parts
    pub fn into_parts(self) -> Vec<UserContentPart> {
        self.0
    }

    /// Create from a simple text string
    pub fn text(text: impl Into<String>) -> Self {
        Self(vec![UserContentPart::Text(TextContentPart {
            text: text.into(),
            provider_options: None,
        })])
    }

    /// Get as simple text if this is a single text part with no provider options
    pub fn as_text(&self) -> Option<&str> {
        match self.0.as_slice() {
            [UserContentPart::Text(t)] if t.provider_options.is_none() => Some(&t.text),
            _ => None,
        }
    }
}

impl Serialize for UserContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UserContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Accept both string and array, canonicalize immediately
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            String(String),
            Array(Vec<UserContentPart>),
        }

        match Helper::deserialize(deserializer)? {
            Helper::String(s) => Ok(UserContent::text(s)),
            Helper::Array(parts) => Ok(UserContent(parts)),
        }
    }
}

impl IntoIterator for UserContent {
    type Item = UserContentPart;
    type IntoIter = std::vec::IntoIter<UserContentPart>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a UserContent {
    type Item = &'a UserContentPart;
    type IntoIter = std::slice::Iter<'a, UserContentPart>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<String> for UserContent {
    fn from(s: String) -> Self {
        Self::text(s)
    }
}

impl From<&str> for UserContent {
    fn from(s: &str) -> Self {
        Self::text(s)
    }
}

impl From<Vec<UserContentPart>> for UserContent {
    fn from(parts: Vec<UserContentPart>) -> Self {
        Self(parts)
    }
}

/// User content parts - text, image, and file parts allowed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(tag = "type", rename_all = "snake_case")]
#[skip_serializing_none]
pub enum UserContentPart {
    Text(TextContentPart),
    Image {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        image: serde_json::Value,
        #[ts(optional)]
        media_type: Option<String>,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[ts(optional)]
        filename: Option<String>,
        media_type: String,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
}

#[derive(Debug, Clone, PartialEq, TS)]
#[ts(export)]
pub struct AssistantContent(#[ts(type = "AssistantContentPart[]")] Vec<AssistantContentPart>);

impl AssistantContent {
    /// Create from a vector of content parts
    pub fn new(parts: Vec<AssistantContentPart>) -> Self {
        Self(parts)
    }

    /// Get a reference to the content parts
    pub fn parts(&self) -> &[AssistantContentPart] {
        &self.0
    }

    /// Consume and return the inner parts
    pub fn into_parts(self) -> Vec<AssistantContentPart> {
        self.0
    }

    /// Create from a simple text string
    pub fn text(text: impl Into<String>) -> Self {
        Self(vec![AssistantContentPart::Text(TextContentPart {
            text: text.into(),
            provider_options: None,
        })])
    }

    /// Get as simple text if this is a single text part with no provider options
    pub fn as_text(&self) -> Option<&str> {
        match self.0.as_slice() {
            [AssistantContentPart::Text(t)] if t.provider_options.is_none() => Some(&t.text),
            _ => None,
        }
    }
}

impl Serialize for AssistantContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AssistantContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Accept both string and array, canonicalize immediately
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            String(String),
            Array(Vec<AssistantContentPart>),
        }

        match Helper::deserialize(deserializer)? {
            Helper::String(s) => Ok(AssistantContent::text(s)),
            Helper::Array(parts) => Ok(AssistantContent(parts)),
        }
    }
}

impl IntoIterator for AssistantContent {
    type Item = AssistantContentPart;
    type IntoIter = std::vec::IntoIter<AssistantContentPart>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a AssistantContent {
    type Item = &'a AssistantContentPart;
    type IntoIter = std::slice::Iter<'a, AssistantContentPart>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<String> for AssistantContent {
    fn from(s: String) -> Self {
        Self::text(s)
    }
}

impl From<&str> for AssistantContent {
    fn from(s: &str) -> Self {
        Self::text(s)
    }
}

impl From<Vec<AssistantContentPart>> for AssistantContent {
    fn from(parts: Vec<AssistantContentPart>) -> Self {
        Self(parts)
    }
}

/// Assistant content parts - text, file, reasoning, tool calls, and tool results allowed
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContentPart {
    Text(TextContentPart),
    File {
        #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
        data: serde_json::Value,
        #[ts(optional)]
        filename: Option<String>,
        media_type: String,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
    Reasoning {
        text: String,
        /// Providers will occasionally return encrypted content for reasoning parts which can
        /// be useful when you send a follow up message.
        #[ts(optional)]
        encrypted_content: Option<String>,
    },
    ToolCall {
        tool_call_id: String,
        tool_name: String,
        arguments: ToolCallArguments,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
        #[ts(optional)]
        provider_executed: Option<bool>,
    },
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        #[ts(type = "unknown")]
        output: serde_json::Value,
        #[ts(optional)]
        provider_options: Option<ProviderOptions>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ToolCallArguments {
    Valid(#[ts(type = "Record<string, unknown>")] serde_json::Map<String, serde_json::Value>),
    Invalid(String),
}

impl From<String> for ToolCallArguments {
    fn from(s: String) -> Self {
        match serde_json::from_str(&s) {
            Ok(serde_json::Value::Object(map)) => ToolCallArguments::Valid(map),
            _ => ToolCallArguments::Invalid(s),
        }
    }
}

impl std::fmt::Display for ToolCallArguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCallArguments::Valid(map) => write!(
                f,
                "{}",
                serde_json::to_string(map).map_err(|_| std::fmt::Error)?
            ),
            ToolCallArguments::Invalid(s) => write!(f, "{}", s),
        }
    }
}

/// Tool content - array of tool content parts
pub type ToolContent = Vec<ToolContentPart>;

/// Reusable tool result content part for tagged unions
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case", optional_fields)]
pub struct ToolResultContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "any")]
    pub output: serde_json::Value,
    pub provider_options: Option<ProviderOptions>,
}

/// Reusable text content part for tagged unions
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case", optional_fields)]
pub struct TextContentPart {
    pub text: String,
    pub provider_options: Option<ProviderOptions>,
}

/// Tool content parts - only tool results allowed
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContentPart {
    ToolResult(ToolResultContentPart),
}

/// Source type enum - matches AI SDK Source sourceType
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Url,
    Document,
}

/// Provider options - matching AI SDK Message format
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(type = "Record<string, any>")]
pub struct ProviderOptions {
    #[ts(type = "any")]
    #[serde(flatten)]
    pub options: serde_json::Map<String, serde_json::Value>,
}

/// Provider metadata
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(type = "Record<string, unknown>")]
pub struct ProviderMetadata {
    #[ts(type = "unknown")]
    #[serde(flatten)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// Source content part - matching AI SDK Source type
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case")]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum SourceContentPart {
    Url {
        id: String,
        url: String,
        #[ts(optional)]
        title: Option<String>,
        #[ts(optional)]
        provider_metadata: Option<ProviderMetadata>,
    },
    Document {
        id: String,
        media_type: String,
        title: String,
        #[ts(optional)]
        filename: Option<String>,
        #[ts(optional)]
        provider_metadata: Option<ProviderMetadata>,
    },
}

/// Generated file content part - matching AI SDK GeneratedFile
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case", optional_fields)]
pub struct GeneratedFileContentPart {
    #[ts(type = "string | Uint8Array | ArrayBuffer | Buffer | URL")]
    pub file: serde_json::Value,
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool call content part for response messages
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case", optional_fields)]
pub struct ToolCallContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "any")]
    pub input: serde_json::Value,
    pub provider_executed: Option<bool>,
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool result content part for response messages
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case", optional_fields)]
pub struct ToolResultResponsePart {
    pub tool_call_id: String,
    pub tool_name: String,
    #[ts(type = "any")]
    pub output: serde_json::Value,
    pub provider_metadata: Option<ProviderMetadata>,
}

/// Tool error content part for response messages
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "snake_case", optional_fields)]
pub struct ToolErrorContentPart {
    pub tool_call_id: String,
    pub tool_name: String,
    pub error: String,
    pub provider_metadata: Option<ProviderMetadata>,
}
