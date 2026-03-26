use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use dashmap::DashMap;
use reqwest::{Client, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[cfg(feature = "tracing")]
use tracing::Instrument;

const TOKEN_BUFFER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Deserialize)]
pub struct DatabricksCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl DatabricksCredentials {
    pub fn from_json(payload: &str) -> Result<Self> {
        serde_json::from_str(payload).context("failed to parse Databricks OAuth credentials JSON")
    }

    fn cache_key(&self, api_base: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.client_id.as_bytes());
        hasher.update(b"|");
        hasher.update(api_base.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug)]
struct CachedToken {
    value: String,
    expires_at: Instant,
    token_type: String,
}

#[derive(Debug, Default)]
pub struct DatabricksTokenManager {
    cache: DashMap<String, CachedToken>,
}

impl DatabricksTokenManager {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    pub async fn get_token(
        &self,
        client: &Client,
        credentials: &DatabricksCredentials,
        api_base: &str,
    ) -> Result<(String, String)> {
        let token_url = format!("{}/oidc/v1/token", api_base.trim_end_matches('/'));
        let token_host = token_host(&token_url);
        let token_future = async {
            let key = credentials.cache_key(api_base);
            if let Some(entry) = self.cache.get(&key) {
                if entry.expires_at > Instant::now() + TOKEN_BUFFER {
                    #[cfg(feature = "tracing")]
                    tracing::Span::current().record("cache.hit", true);
                    return Ok((entry.value.clone(), entry.token_type.clone()));
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
                    token_type: token.token_type.clone(),
                },
            );

            Ok((token.value, token.token_type))
        };
        #[cfg(feature = "tracing")]
        {
            return token_future
                .instrument(tracing::info_span!(
                    "bt.router.auth.token",
                    provider = "databricks",
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
    token_type: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct DatabricksTokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn request_token(
    client: &Client,
    credentials: &DatabricksCredentials,
    token_url: &str,
    _token_host: Option<String>,
) -> Result<TokenResponse> {
    let auth_header = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            credentials.client_id, credentials.client_secret
        ))
    );

    #[cfg(feature = "tracing")]
    let response = async {
        client
            .post(token_url)
            .header("Authorization", auth_header.clone())
            .form(&[("grant_type", "client_credentials"), ("scope", "all-apis")])
            .send()
            .await
    }
    .instrument(tracing::info_span!(
        "bt.router.auth.token.request",
        provider = "databricks",
        auth.host = _token_host.as_deref().unwrap_or(""),
    ))
    .await
    .context("failed to send Databricks OAuth token request")?;
    #[cfg(not(feature = "tracing"))]
    let response = client
        .post(token_url)
        .header("Authorization", auth_header)
        .form(&[("grant_type", "client_credentials"), ("scope", "all-apis")])
        .send()
        .await
        .context("failed to send Databricks OAuth token request")?;

    let status = response.status();
    let body = response
        .json::<DatabricksTokenResponse>()
        .await
        .context("failed to parse Databricks OAuth token response")?;

    if let (Some(token), Some(token_type), Some(expires_in)) =
        (body.access_token, body.token_type, body.expires_in)
    {
        let expires_at = Instant::now() + Duration::from_secs(expires_in);
        Ok(TokenResponse {
            value: token,
            token_type,
            expires_at,
        })
    } else {
        let message = body
            .error_description
            .or(body.error)
            .unwrap_or_else(|| status.to_string());
        Err(anyhow!("Databricks OAuth error: {message}"))
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
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fetches_and_caches_token() {
        let server = MockServer::start().await;
        let expected_auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("client:secret")
        );
        Mock::given(method("POST"))
            .and(path("/oidc/v1/token"))
            .and(header("Authorization", expected_auth.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "db-token",
                "token_type": "Bearer",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let credentials = DatabricksCredentials {
            client_id: "client".into(),
            client_secret: "secret".into(),
        };

        let manager = DatabricksTokenManager::new();
        let client = Client::builder().build().unwrap();
        let api_base = server.uri();

        let (token, token_type) = manager
            .get_token(&client, &credentials, &api_base)
            .await
            .expect("token fetched");
        assert_eq!(token, "db-token");
        assert_eq!(token_type, "Bearer");

        let (cached_token, cached_type) = manager
            .get_token(&client, &credentials, &api_base)
            .await
            .expect("token cached");
        assert_eq!(cached_token, "db-token");
        assert_eq!(cached_type, "Bearer");

        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
