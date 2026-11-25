mod auth;
mod catalog;
mod client;
pub use client::{
    ClientSettings, DnsOverride, HttpRequestStats, add_additional_ca_bundle,
    build_middleware_client, clear_override_client, rustls_client_builder, set_override_client,
};
pub use reqwest_middleware::ClientWithMiddleware;
mod error;
mod providers;
mod retry;
mod router;
mod streaming;

// Re-export lingua's serde_json (big_serde_json with arbitrary_precision)
// This allows gateway and other consumers to use the same JSON types as lingua
pub use lingua::serde_json;

pub use auth::{
    AuthConfig, AuthType, api_key_auth,
    azure::{AzureEntraCredentials, AzureEntraTokenManager},
    databricks::{DatabricksCredentials, DatabricksTokenManager},
    google::{GoogleServiceAccountConfig, GoogleTokenManager, ServiceAccountKey},
};
pub use catalog::{
    ModelCatalog, ModelFlavor, ModelResolver, ModelSpec, PricingRate, ProviderMetadata, TierPricing,
};
pub use error::{Error, Result, UpstreamHttpError};
pub use lingua::{FinishReason, UniversalStreamChoice, UniversalStreamChunk};
pub use lingua::{ProviderFormat, providers::openai::capabilities::normalize_openai_model_name};
pub use providers::{
    AnthropicConfig, AnthropicProvider, AzureConfig, AzureProvider, BedrockConfig, BedrockProvider,
    ClientHeaders, DatabricksConfig, DatabricksProvider, GoogleConfig, GoogleProvider,
    MistralConfig, MistralProvider, OpenAICompatibleEndpoint, OpenAIConfig, OpenAIProvider,
    Provider, TypeSafeConfig, TypeSafeProvider, VertexConfig, VertexProvider, is_openai_compatible,
    openai_compatible_endpoint,
};
pub use retry::{RetryPolicy, RetryStrategy};
pub use router::{
    CompleteResponseWithRaw, PreparedDecisionRequest, PreparedRequest, PreparedStreamRequest,
    ProviderRoute, RequestHints, Router, RouterBuilder, RouterMetadata, StreamCaptures,
    create_provider, extract_request_hints,
};
pub use streaming::{
    RawResponseStream, RawStreamChunkCapture, ResponseStream, ServedInfoCapture, StreamChunk,
    sse_stream, transform_provider_stream,
};

// Provider trait requirement (for custom provider implementations)
pub use lingua::{UniversalResponse, UniversalUsage};
