use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use reqwest::{Client, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[cfg(feature = "tracing")]
use tracing::Instrument;

const TOKEN_ENDPOINT: &str = "https://login.microsoftonline.com";
const TOKEN_BUFFER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Deserialize)]
pub struct AzureEntraCredentials {
    pub client_id: String,
    pub tenant_id: String,
    pub scope: String,
    pub client_secret: String,
    #[serde(default)]
    pub token_url: Option<String>,
}

impl AzureEntraCredentials {
    pub fn from_json(payload: &str) -> Result<Self> {
        serde_json::from_str(payload).context("failed to parse Azure Entra credentials JSON")
    }

    fn cache_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.client_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.tenant_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.scope.as_bytes());
        if let Some(url) = &self.token_url {
            hasher.update(b"|");
            hasher.update(url.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    fn token_url(&self) -> String {
        if let Some(url) = &self.token_url {
            url.clone()
        } else {
            format!("{}/{}/oauth2/v2.0/token", TOKEN_ENDPOINT, self.tenant_id)
        }
    }
}

#[derive(Debug)]
struct CachedToken {
    value: String,
    expires_at: Instant,
}

#[derive(Debug, Default)]
pub struct AzureEntraTokenManager {
    cache: DashMap<String, CachedToken>,
}

impl AzureEntraTokenManager {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    pub async fn get_token(
        &self,
        client: &Client,
        credentials: &AzureEntraCredentials,
    ) -> Result<String> {
        let token_url = credentials.token_url();
        let token_host = token_host(&token_url);
        let token_future = async {
            let key = credentials.cache_key();
            if let Some(entry) = self.cache.get(&key) {
                if entry.expires_at > Instant::now() + TOKEN_BUFFER {
                    #[cfg(feature = "tracing")]
                    tracing::Span::current().record("cache.hit", true);
                    return Ok(entry.value.clone());
                }
            }

            #[cfg(feature = "tracing")]
            tracing::Span::current().record("cache.hit", false);
            let token = request_token(client, credentials, &token_url, token_host.clone()).await?;
            self.cache.insert(
                key,
                CachedToken {
                    value: token.value.clone(),
                    expires_at: token.expires_at,
                },
            );

            Ok(token.value)
        };
        #[cfg(feature = "tracing")]
        {
            return token_future
                .instrument(tracing::info_span!(
                    "bt.router.auth.token",
                    provider = "azure",
                    auth.host = token_host.as_deref().unwrap_or(""),
                    cache.hit = tracing::field::Empty,
                ))
                .await;
        }
        #[cfg(not(feature = "tracing"))]
        {
            token_future.await
        }
    }
}

struct TokenResponse {
    value: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct AzureTokenResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn request_token(
    client: &Client,
    credentials: &AzureEntraCredentials,
    token_url: &str,
    _token_host: Option<String>,
) -> Result<TokenResponse> {
    #[cfg(feature = "tracing")]
    let response = async {
        client
            .post(token_url)
            .form(&[
                ("client_id", credentials.client_id.as_str()),
                ("client_secret", credentials.client_secret.as_str()),
                ("scope", credentials.scope.as_str()),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await
    }
    .instrument(tracing::info_span!(
        "bt.router.auth.token.request",
        provider = "azure",
        auth.host = _token_host.as_deref().unwrap_or(""),
    ))
    .await
    .context("failed to send Azure Entra token request")?;
    #[cfg(not(feature = "tracing"))]
    let response = client
        .post(token_url)
        .form(&[
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
            ("scope", credentials.scope.as_str()),
            ("grant_type", "client_credentials"),
        ])
        .send()
        .await
        .context("failed to send Azure Entra token request")?;

    let status = response.status();
    let body = response
        .json::<AzureTokenResponse>()
        .await
        .context("failed to parse Azure Entra token response")?;

    if let (Some(token), Some(expires_in)) = (body.access_token, body.expires_in) {
        let expires_at = Instant::now() + Duration::from_secs(expires_in);
        Ok(TokenResponse {
            value: token,
            expires_at,
        })
    } else {
        let message = body
            .error_description
            .or(body.error)
            .unwrap_or_else(|| status.to_string());
        Err(anyhow!("Azure Entra error: {message}"))
    }
}

fn token_host(token_url: &str) -> Option<String> {
    Url::parse(token_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fetches_and_caches_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "test-token",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let credentials = AzureEntraCredentials {
            client_id: "client".into(),
            tenant_id: "tenant".into(),
            scope: "scope/.default".into(),
            client_secret: "secret".into(),
            token_url: Some(format!("{}/token", server.uri())),
        };

        let manager = AzureEntraTokenManager::new();
        let client = Client::builder().build().unwrap();

        let first = manager
            .get_token(&client, &credentials)
            .await
            .expect("token fetched");
        assert_eq!(first, "test-token");

        let second = manager
            .get_token(&client, &credentials)
            .await
            .expect("token cached");
        assert_eq!(second, "test-token");

        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
