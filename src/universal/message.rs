use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    System { content: Content },
    User { content: Content },
    Assistant { content: Content },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    ContentList(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControlEphemeral>,
        // XXX TODO
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<Vec<String>>,

        /// By convention, you can forward along information specific to a particular provider
        /// in this field.
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_specific: Option<serde_json::Value>,
    },
    Image {
        data: FileData,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<ImageDetail>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControlEphemeral>,

        /// By convention, you can forward along information specific to a particular provider
        /// in this field.
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_specific: Option<serde_json::Value>,
    },
    Document {
        data: FileData,

        // Anthropic-specific params
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControlEphemeral>,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<bool>,

        /// By convention, you can forward along information specific to a particular provider
        /// in this field.
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_specific: Option<serde_json::Value>,
    },
}

impl ContentPart {
    /// Create a text content part with just the text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: None,
            citations: None,
            provider_specific: None,
        }
    }

    /// Create an image content part with just the file data
    pub fn image(data: FileData) -> Self {
        Self::Image {
            data,
            detail: None,
            cache_control: None,
            provider_specific: None,
        }
    }

    /// Create a document content part with just the file data
    pub fn document(data: FileData) -> Self {
        Self::Document {
            data,
            filename: None,
            cache_control: None,
            citations: None,
            provider_specific: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageDetail {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileData {
    Url(String),
    Base64(Base64Data),
    FileId(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Base64Data {
    pub mime_type: String,
    pub data: Vec<u8>,
}

// Cache breakpoints are only used by Anthropic models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CacheControlEphemeral {
    ttl: Option<CacheTtl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheTtl {
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "1h")]
    OneHour,
}
