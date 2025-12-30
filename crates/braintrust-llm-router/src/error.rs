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

    #[error("lingua conversion failed: {0}")]
    Lingua(String),

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
}
