use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(feature = "tracing")]
use tracing::Instrument;

const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const TOKEN_BUFFER: Duration = Duration::from_secs(60);
const DEFAULT_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountKey {
    pub client_email: String,
    pub private_key: String,
    #[serde(default = "default_token_uri")]
    pub token_uri: String,
}

fn default_token_uri() -> String {
    DEFAULT_TOKEN_URI.to_string()
}

#[derive(Debug, Clone)]
pub struct GoogleServiceAccountConfig {
    pub key: ServiceAccountKey,
    pub scopes: Vec<String>,
}

impl GoogleServiceAccountConfig {
    pub fn from_json(payload: &str, scopes: Option<Vec<String>>) -> Result<Self> {
        let mut key: ServiceAccountKey =
            serde_json::from_str(payload).context("failed to parse Google service account JSON")?;
        if key.token_uri.is_empty() {
            key.token_uri = DEFAULT_TOKEN_URI.to_string();
        }
        let scopes = scopes.unwrap_or_else(|| vec![DEFAULT_SCOPE.to_string()]);
        Ok(Self { key, scopes })
    }

    fn scope_string(&self) -> String {
        if self.scopes.is_empty() {
            DEFAULT_SCOPE.to_string()
        } else {
            self.scopes.join(" ")
        }
    }

    fn cache_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.key.client_email.as_bytes());
        hasher.update(b"|");
        hasher.update(self.key.private_key.as_bytes());
        hasher.update(b"|");
        hasher.update(self.scope_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug)]
struct CachedToken {
    value: String,
    expires_at: Instant,
}

#[derive(Debug, Default)]
pub struct GoogleTokenManager {
    cache: DashMap<String, CachedToken>,
}

impl GoogleTokenManager {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    pub async fn get_token(
        &self,
        client: &Client,
        config: &GoogleServiceAccountConfig,
    ) -> Result<String> {
        let token_host = token_host(&config.key.token_uri);
        let token_future = async {
            let key = config.cache_key();
            if let Some(entry) = self.cache.get(&key) {
                if entry.expires_at > Instant::now() + TOKEN_BUFFER {
                    #[cfg(feature = "tracing")]
                    tracing::Span::current().record("cache.hit", true);
                    return Ok(entry.value.clone());
                }
            }

            #[cfg(feature = "tracing")]
            tracing::Span::current().record("cache.hit", false);
            let token = request_token(client, config, token_host.clone()).await?;
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
                    provider = "google",
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

#[derive(Debug, Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    exp: usize,
    iat: usize,
    sub: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn request_token(
    client: &Client,
    config: &GoogleServiceAccountConfig,
    _token_host: Option<String>,
) -> Result<TokenResponse> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before UNIX_EPOCH")?;
    let iat = now.as_secs() as usize;
    let exp = (now + Duration::from_secs(3600)).as_secs() as usize;
    let scopes = config.scope_string();

    let claims = JwtClaims {
        iss: &config.key.client_email,
        scope: &scopes,
        aud: &config.key.token_uri,
        exp,
        iat,
        sub: None,
    };

    let header = Header::new(Algorithm::RS256);
    let encoding_key = EncodingKey::from_rsa_pem(config.key.private_key.as_bytes())
        .context("failed to parse Google service account private key")?;
    let assertion = encode(&header, &claims, &encoding_key)
        .context("failed to encode Google service account JWT")?;

    #[cfg(feature = "tracing")]
    let response = async {
        client
            .post(&config.key.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
    }
    .instrument(tracing::info_span!(
        "bt.router.auth.token.request",
        provider = "google",
        auth.host = _token_host.as_deref().unwrap_or(""),
    ))
    .await
    .context("failed to send Google OAuth token request")?;
    #[cfg(not(feature = "tracing"))]
    let response = client
        .post(&config.key.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", assertion.as_str()),
        ])
        .send()
        .await
        .context("failed to send Google OAuth token request")?;

    let status = response.status();
    let body = response
        .json::<GoogleTokenResponse>()
        .await
        .context("failed to parse Google OAuth token response")?;

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
        Err(anyhow!("Google OAuth error: {message}"))
    }
}

fn token_host(token_url: &str) -> Option<String> {
    Url::parse(token_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
}
