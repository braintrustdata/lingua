mod auth;
mod catalog;
mod client;
mod error;
mod providers;
mod retry;
mod router;
mod streaming;

// Re-export lingua's serde_json (big_serde_json with arbitrary_precision)
// This allows gateway and other consumers to use the same JSON types as lingua
pub use lingua::serde_json;

pub use auth::{
    azure::{AzureEntraCredentials, AzureEntraTokenManager},
    databricks::{DatabricksCredentials, DatabricksTokenManager},
    google::{GoogleServiceAccountConfig, GoogleTokenManager, ServiceAccountKey},
    AuthConfig, AuthType,
};
pub use catalog::{
    default_catalog, ModelCatalog, ModelFlavor, ModelResolver, ModelSpec, BUNDLED_CATALOG_JSON,
};
pub use error::{Error, Result, UpstreamHttpError};
pub use lingua::ProviderFormat;
pub use lingua::{FinishReason, UniversalStreamChoice, UniversalStreamChunk};
pub use providers::{
    is_openai_compatible, openai_compatible_endpoint, AnthropicConfig, AnthropicProvider,
    AzureConfig, AzureProvider, BedrockConfig, BedrockProvider, GoogleConfig, GoogleProvider,
    MistralConfig, MistralProvider, OpenAICompatibleEndpoint, OpenAIConfig, OpenAIProvider,
    OpenAIResponsesProvider, Provider, VertexConfig, VertexProvider,
};
pub use retry::{RetryPolicy, RetryStrategy};
pub use router::{create_provider, extract_request_hints, RequestHints, Router, RouterBuilder};
pub use streaming::{RawResponseStream, ResponseStream};

// Provider trait requirement (for custom provider implementations)
pub use lingua::{UniversalResponse, UniversalUsage};
