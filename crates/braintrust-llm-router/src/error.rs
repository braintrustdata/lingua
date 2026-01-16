use std::time::Duration;

use http::HeaderMap;
use thiserror::Error;

use lingua::ProviderFormat;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct UpstreamHttpError {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl UpstreamHttpError {
    pub fn new(status: u16, headers: HeaderMap, body: String) -> Self {
        let headers = headers
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|v| (name.as_str().to_string(), v.to_string()))
            })
            .collect();
        Self {
            status,
            headers,
            body,
        }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn status_code(&self) -> Option<http::StatusCode> {
        http::StatusCode::from_u16(self.status).ok()
    }

    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    pub fn into_parts(self) -> (u16, Vec<(String, String)>, String) {
        (self.status, self.headers, self.body)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown model '{0}'")]
    UnknownModel(String),

    #[error("no provider registered for format '{0:?}'")]
    NoProvider(ProviderFormat),

    #[error("no authentication configured for provider '{0}'")]
    NoAuth(String),

    #[error("provider '{provider}' error: {source}")]
    Provider {
        provider: String,
        #[source]
        source: anyhow::Error,
        retry_after: Option<Duration>,
        http: Option<UpstreamHttpError>,
    },

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("lingua serialization error: {0}")]
    LinguaJson(#[from] lingua::serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("{0}")]
    Lingua(#[from] lingua::TransformError),

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("operation timed out")]
    Timeout,
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Error::Http(err) if err.is_timeout() || err.is_connect() || err.is_request() || err.status().map(|c| c.is_server_error()).unwrap_or(false))
            || matches!(self, Error::Provider { retry_after, .. } if retry_after.is_some())
            || matches!(self, Error::Timeout)
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Provider { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Returns true if this is a client-side error (400 Bad Request).
    ///
    /// Client errors indicate problems with the user's request that they
    /// should fix, such as unknown models, unsupported formats, or invalid payloads.
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Error::UnknownModel(_) | Error::NoProvider(_) | Error::InvalidRequest(_)
        ) || matches!(self, Error::Lingua(e) if e.is_client_error())
    }

    /// Returns true if this is an authentication error (401 Unauthorized).
    ///
    /// Auth errors indicate missing or invalid authentication credentials.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, Error::NoAuth(_) | Error::Auth(_))
    }

    /// Returns true if this is an upstream provider error with HTTP details.
    ///
    /// Upstream errors should be passed through to the client with the original
    /// status code, headers, and body from the provider.
    pub fn is_upstream_error(&self) -> bool {
        matches!(self, Error::Provider { http: Some(_), .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_http_error_preserves_parts() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test", "value".parse().unwrap());
        let error = UpstreamHttpError::new(404, headers.clone(), "not found".into());

        assert_eq!(error.status(), 404);
        assert_eq!(error.status_code(), Some(http::StatusCode::NOT_FOUND));
        assert_eq!(error.headers(), &[("x-test".into(), "value".into())]);

        let (status, returned_headers, body) = error.clone().into_parts();
        assert_eq!(status, 404);
        assert_eq!(body, "not found");
        assert_eq!(returned_headers, vec![("x-test".into(), "value".into())]);
    }

    #[test]
    fn transform_error_classification() {
        use lingua::TransformError;

        // Client errors
        assert!(TransformError::UnableToDetectFormat.is_client_error());
        assert!(TransformError::ValidationFailed {
            target: ProviderFormat::OpenAI,
            reason: "test".into()
        }
        .is_client_error());
        assert!(TransformError::DeserializationFailed("invalid json".into()).is_client_error());
        assert!(TransformError::UnsupportedTargetFormat(ProviderFormat::OpenAI).is_client_error());
        assert!(TransformError::UnsupportedSourceFormat(ProviderFormat::OpenAI).is_client_error());

        // Server errors
        assert!(!TransformError::SerializationFailed("test".into()).is_client_error());
        assert!(!TransformError::FromUniversalFailed("test".into()).is_client_error());
        assert!(!TransformError::ToUniversalFailed("test".into()).is_client_error());
        assert!(!TransformError::StreamingNotImplemented("test".into()).is_client_error());
    }

    #[test]
    fn router_error_classification() {
        // Client errors
        assert!(Error::UnknownModel("gpt-5".into()).is_client_error());
        assert!(Error::NoProvider(ProviderFormat::OpenAI).is_client_error());
        assert!(Error::InvalidRequest("bad".into()).is_client_error());
        assert!(Error::Lingua(lingua::TransformError::UnableToDetectFormat).is_client_error());

        // Auth errors
        assert!(Error::NoAuth("openai".into()).is_auth_error());
        assert!(Error::Auth("invalid".into()).is_auth_error());

        // Not client errors
        assert!(!Error::Timeout.is_client_error());
        assert!(
            !Error::Lingua(lingua::TransformError::SerializationFailed("test".into()))
                .is_client_error()
        );

        // Upstream errors
        let upstream_err = Error::Provider {
            provider: "openai".into(),
            source: anyhow::anyhow!("test"),
            retry_after: None,
            http: Some(UpstreamHttpError {
                status: 404,
                headers: vec![],
                body: "not found".into(),
            }),
        };
        assert!(upstream_err.is_upstream_error());

        let non_upstream_err = Error::Provider {
            provider: "openai".into(),
            source: anyhow::anyhow!("test"),
            retry_after: None,
            http: None,
        };
        assert!(!non_upstream_err.is_upstream_error());
    }
}
