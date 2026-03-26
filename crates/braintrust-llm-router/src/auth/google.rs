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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Fixed test-only RSA key used to generate signed JWT assertions locally.
    // This is not a real credential and is only valid for unit test fixtures.
    const TEST_RSA_PRIVATE_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCcMJbm9jjoOrvY
z6G8Pa4hNwoKuujMd5ivtiyNxC7/S/ALi94t14klKSt9nTQfUmjM8CzA+EPn/2HW
mjODsyDtXOARbwS1jCrlEadRe6uSRL9gXsCHhmgPocT6Ss5z6IoOiwAicnQG5kKv
Uw0Fdnj385fYC9R5MkqO0/OCi0xdPnImcCEUZPwrqB7zBmkOnTOg248+eESzsEav
tlIdC44glglPLFyKWCp05lsoSIoLoH6qrNk92qivs1r9vWrO6wjRUvUXUKlIyPoJ
OS9zjPJujrsEak329V7d/GYv2dgGInAEm8vWmRFN49iNu+Ph6SyBZb4pEi7ocX9F
pdBrNsedAgMBAAECggEADpiZ2Y6kBcvLVzkcH7fR6Ie4uAT8kXMRwUXwhvURAUmq
7qFNC5KrXd4pks0YnF66rYA6ZnQtAGbE0WXKr6GTT6tQw0BRO9gUACE0tjAs+ffT
vKFOM7wTSHaxLkTEY1+VW0ORKSbyAd0N2U2VF3AZYO6SP53nZsYU4qEbDhWPdt0k
Dfap1negwec4akZd0MfpvmPYe0xg6VY+O0bQEmj2G8Vl0a9hIn4rAxMUvkRUDO3u
t4sCdnnYrH1lEhdnZd2GV3o1ZwAF0mvAyT9Jne/GIEiUgTWvTEDQg7Rin9rAHoQK
ix0Dib3ly1SggNoflMufhSUsdmNKr6lje1AGbmYWYQKBgQDa4Qy5yjq5MRn1wxhT
mnYUlB9L0eRWPVUSZhFC7TBWrSZ9KRDKI5UbLpV3Tpp4z2qJjCB6cMtqzd0m0ECu
NWcY7Eodn5oB/TTpap4wr8ycKOBUiJWgfDpH0MIB25E8HqlwHl3J+3D0YSymJpde
N+8znEEwgUJI4DlzZu4pOkrkkQKBgQC2rcs9xwrBpnoVeqJxstz33wDR1BQvGJKw
o83XRWGvLOYrkWtPx1OfWMmSg+JC7IAKasfkbx+e7RlKChA54cTNDgmet2CDNDBC
p0E8nkhkJazc049FASx1pIu9lOkemX1CuuTXy5bRJ2JfQSRLhkVy2rSsRV0ytwVG
bCUoOpOITQKBgFeYgXNJT78Vu4HzliS/SEpsDSpW0b8BxK4cUwQp0JKfsSud564+
F0pNllutBX0b5VMu1UCrK32O7da+uWP+00fSKMc6PHRXVXmkxbJOaOCGK2EpWFhl
3x0mmr4LlVAuJTlNrdNL4aSrzyafgyydzgklm6FB2bk4o0VgCChPv/FBAoGAXjKr
9MUoVMcFeQHttfdnXiGOCKT1a3ueWJt+zxylzHC4l4q67T55bleYSYbcK2pMdBKv
1KlAgvD782PRDifPFXXBnCgvCjjlEdmxGBL+fTW4N36YCBsc0+TvcejRdMftAXXh
/yyqLlvCrB+pGZC5Swpf091Iu5gIjlHBr0bVQJkCgYAw+gdDMq/eXH3OAZcCSXFl
LlP8bOXhMTiDzwDVqnW2qmtDiZOGEPOkVejVBvGxv9bV4ivdTQDO/qLzsWbP54LR
IaDWu5YTkzCysoEwOqOEjoAyIwVi6g2wBcKbA/yajNTiun2pd0e2Y+xyydVUkIur
AuWIGWj6mq+yKlKUjA2WGQ==
-----END PRIVATE KEY-----"#;

    #[tokio::test]
    async fn refreshes_stale_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "stale-token",
                "expires_in": 1,
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "fresh-token",
                "expires_in": 3600,
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let config = GoogleServiceAccountConfig {
            key: ServiceAccountKey {
                client_email: "test@example.com".into(),
                private_key: TEST_RSA_PRIVATE_KEY_PEM.into(),
                token_uri: format!("{}/token", server.uri()),
            },
            scopes: vec![DEFAULT_SCOPE.to_string()],
        };

        let manager = GoogleTokenManager::new();
        let client = Client::builder().build().unwrap();

        let first = manager
            .get_token(&client, &config)
            .await
            .expect("stale token fetched");
        assert_eq!(first, "stale-token");

        let second = manager
            .get_token(&client, &config)
            .await
            .expect("fresh token fetched");
        assert_eq!(second, "fresh-token");

        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }
}
