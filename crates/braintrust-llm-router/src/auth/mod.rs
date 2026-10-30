use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub mod azure;
pub mod databricks;
pub mod google;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ApiKey,
    OAuth,
    AwsSignatureV4,
    AzureEntra,
    Custom,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    ApiKey {
        key: String,
        #[serde(default)]
        header: Option<String>,
        #[serde(default)]
        prefix: Option<String>,
    },
    OAuth {
        access_token: String,
        #[serde(default)]
        token_type: Option<String>,
    },
    AwsSignatureV4 {
        access_key: String,
        secret_key: String,
        #[serde(default)]
        session_token: Option<String>,
        region: String,
        service: String,
    },
    AzureEntra {
        bearer_token: String,
    },
    Custom {
        headers: HashMap<String, String>,
    },
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("auth_type", &self.auth_type())
            .finish_non_exhaustive()
    }
}

fn credential_header_value(value: &str) -> Result<HeaderValue> {
    let mut value = HeaderValue::from_str(value)
        .map_err(|_| Error::Auth("invalid authentication header value".to_string()))?;
    value.set_sensitive(true);
    Ok(value)
}

/// Convenience function to create an API key authentication configuration.
pub fn api_key_auth(api_key: &str) -> AuthConfig {
    AuthConfig::ApiKey {
        key: api_key.to_string(),
        header: Some("authorization".into()),
        prefix: Some("Bearer".into()),
    }
}

impl AuthConfig {
    pub fn auth_type(&self) -> AuthType {
        match self {
            AuthConfig::ApiKey { .. } => AuthType::ApiKey,
            AuthConfig::OAuth { .. } => AuthType::OAuth,
            AuthConfig::AwsSignatureV4 { .. } => AuthType::AwsSignatureV4,
            AuthConfig::AzureEntra { .. } => AuthType::AzureEntra,
            AuthConfig::Custom { .. } => AuthType::Custom,
        }
    }

    pub fn apply_headers(&self, headers: &mut HeaderMap) -> Result<()> {
        match self {
            AuthConfig::ApiKey {
                key,
                header,
                prefix,
            } => {
                let header_name = header.as_deref().unwrap_or("authorization");
                let mut value = String::new();
                if let Some(prefix) = prefix {
                    value.push_str(prefix);
                    if !prefix.ends_with(' ') {
                        value.push(' ');
                    }
                }
                value.push_str(key);
                headers.insert(
                    header_name.parse::<HeaderName>().map_err(|e| {
                        Error::Auth(format!("invalid header name '{header_name}': {e}"))
                    })?,
                    credential_header_value(&value)?,
                );
                Ok(())
            }
            AuthConfig::OAuth {
                access_token,
                token_type,
            } => {
                let prefix = token_type.as_deref().unwrap_or("Bearer");
                let value = format!("{prefix} {access_token}");
                headers.insert(AUTHORIZATION, credential_header_value(&value)?);
                Ok(())
            }
            AuthConfig::AzureEntra { bearer_token } => {
                let value = format!("Bearer {bearer_token}");
                headers.insert(AUTHORIZATION, credential_header_value(&value)?);
                Ok(())
            }
            AuthConfig::Custom { headers: custom } => {
                for (key, value) in custom {
                    let name = key
                        .parse::<HeaderName>()
                        .map_err(|e| Error::Auth(format!("invalid header name '{key}' : {e}")))?;
                    headers.insert(name, credential_header_value(value)?);
                }
                Ok(())
            }
            AuthConfig::AwsSignatureV4 { .. } => {
                // Handled directly by the AWS provider
                Ok(())
            }
        }
    }

    pub fn api_key(&self) -> Option<&str> {
        match self {
            AuthConfig::ApiKey { key, .. } => Some(key.as_str()),
            _ => None,
        }
    }

    pub fn oauth_token(&self) -> Option<&str> {
        match self {
            AuthConfig::OAuth { access_token, .. } => Some(access_token.as_str()),
            _ => None,
        }
    }

    pub fn azure_token(&self) -> Option<&str> {
        match self {
            AuthConfig::AzureEntra { bearer_token } => Some(bearer_token.as_str()),
            _ => None,
        }
    }

    pub fn aws_credentials(&self) -> Option<(&str, &str, Option<&str>, &str, &str)> {
        match self {
            AuthConfig::AwsSignatureV4 {
                access_key,
                secret_key,
                session_token,
                region,
                service,
            } => Some((
                access_key,
                secret_key,
                session_token.as_deref(),
                region,
                service,
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_headers_preserve_bytes_and_redact_debug() {
        let key = "credential+/=&<>\"'";
        for prefix in [None, Some("Token")] {
            let auth = AuthConfig::ApiKey {
                key: key.into(),
                header: Some("apikey".into()),
                prefix: prefix.map(str::to_string),
            };
            assert!(!format!("{auth:?}").contains(key));
            let mut headers = HeaderMap::new();
            auth.apply_headers(&mut headers).unwrap();
            let expected = prefix.map(|prefix| format!("{prefix} {key}"));
            assert_eq!(
                headers["apikey"].to_str().unwrap(),
                expected.as_deref().unwrap_or(key)
            );
            assert!(headers["apikey"].is_sensitive());
            assert!(!format!("{headers:?}").contains(key));
        }
    }

    #[test]
    fn custom_auth_debug_redacts_header_values() {
        let auth = AuthConfig::Custom {
            headers: HashMap::from([("apikey".into(), "private-key".into())]),
        };
        assert!(!format!("{auth:?}").contains("private-key"));
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers).unwrap();
        assert!(headers["apikey"].is_sensitive());
    }
}
