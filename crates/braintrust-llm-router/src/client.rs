use std::error::Error as StdError;
use std::io::ErrorKind;
use std::time::Duration;

use dashmap::DashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::{Client, ClientBuilder};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{
    default_on_request_failure, policies::ExponentialBackoff, RetryTransientMiddleware, Retryable,
    RetryableStrategy,
};

use crate::error::{Error, Result};

// The default number of retries for transient transport failures.
const DEFAULT_MAX_RETRIES: u32 = 2;

// Shared reqwest clients are cached by these settings. Keep this key
// low-cardinality and effectively process-wide; request-scoped values do not
// belong here because they fragment client reuse and connection pooling.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientSettings {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub pool_idle_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    pub user_agent: String,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(600),
            pool_idle_timeout: Duration::from_secs(90),
            pool_max_idle_per_host: 16,
            user_agent: format!("braintrust-llm-router/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

pub fn build_client(settings: &ClientSettings) -> Result<Client> {
    ClientBuilder::new()
        .connect_timeout(settings.connect_timeout)
        .timeout(settings.request_timeout)
        .pool_idle_timeout(settings.pool_idle_timeout)
        .pool_max_idle_per_host(settings.pool_max_idle_per_host)
        .user_agent(&settings.user_agent)
        .build()
        .map_err(Error::from)
}

pub fn build_middleware_client(settings: &ClientSettings) -> Result<ClientWithMiddleware> {
    if let Some(existing) = OVERRIDE_CLIENT.read().clone() {
        return Ok(existing);
    }

    #[cfg(feature = "tracing")]
    let client_span = tracing::info_span!(
        "bt.router.http_client",
        cache.hit = tracing::field::Empty,
        client.request_timeout_ms = settings.request_timeout.as_millis() as u64,
        client.connect_timeout_ms = settings.connect_timeout.as_millis() as u64,
        client.pool_idle_timeout_ms = settings.pool_idle_timeout.as_millis() as u64,
        client.pool_max_idle_per_host = settings.pool_max_idle_per_host as u64,
    );

    #[cfg(feature = "tracing")]
    return client_span.in_scope(|| build_middleware_client_inner(settings));

    #[cfg(not(feature = "tracing"))]
    {
        build_middleware_client_inner(settings)
    }
}

fn build_middleware_client_inner(settings: &ClientSettings) -> Result<ClientWithMiddleware> {
    #[cfg(feature = "tracing")]
    {
        if let Some(existing) = SHARED_CLIENTS.get(settings) {
            tracing::Span::current().record("cache.hit", true);
            return Ok(existing.clone());
        }
        tracing::Span::current().record("cache.hit", false);
    }

    #[cfg(not(feature = "tracing"))]
    {
        if let Some(existing) = SHARED_CLIENTS.get(settings) {
            return Ok(existing.clone());
        }
    }

    #[cfg(feature = "tracing")]
    let client = tracing::info_span!(
        "bt.router.http_client.build",
        client.request_timeout_ms = settings.request_timeout.as_millis() as u64,
        client.connect_timeout_ms = settings.connect_timeout.as_millis() as u64,
        client.pool_idle_timeout_ms = settings.pool_idle_timeout.as_millis() as u64,
        client.pool_max_idle_per_host = settings.pool_max_idle_per_host as u64,
    )
    .in_scope(|| {
        let client = build_client(settings)?;
        Ok::<ClientWithMiddleware, Error>(build_retrying_middleware_client(client))
    })?;

    #[cfg(not(feature = "tracing"))]
    let client = {
        let client = build_client(settings)?;
        build_retrying_middleware_client(client)
    };

    if let Some(existing) = SHARED_CLIENTS.get(settings) {
        return Ok(existing.clone());
    }
    SHARED_CLIENTS.insert(settings.clone(), client.clone());
    Ok(client)
}

fn build_retrying_middleware_client(client: Client) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(DEFAULT_MAX_RETRIES);
    let retry_middleware = RetryTransientMiddleware::new_with_policy_and_strategy(
        retry_policy,
        ConnectionRetryStrategy,
    );

    reqwest_middleware::ClientBuilder::new(client)
        .with(retry_middleware)
        .build()
}

fn retryable_transport_failure(err: &reqwest_middleware::Error) -> Option<Retryable> {
    if is_request_timeout(err) {
        return None;
    }

    let retryable = match default_on_request_failure(err) {
        Some(Retryable::Transient) => Some(Retryable::Transient),
        default_retryability => match err {
            reqwest_middleware::Error::Reqwest(err) if chain_has_connection_io_error(err) => {
                Some(Retryable::Transient)
            }
            reqwest_middleware::Error::Middleware(err)
                if chain_has_connection_io_error(err.as_ref()) =>
            {
                Some(Retryable::Transient)
            }
            _ => default_retryability,
        },
    };

    #[cfg(feature = "tracing")]
    if matches!(retryable, Some(Retryable::Transient)) {
        tracing::warn!(error = %err, "retrying middleware request after transient error");
    }

    retryable
}

fn is_request_timeout(err: &reqwest_middleware::Error) -> bool {
    match err {
        reqwest_middleware::Error::Reqwest(err) => err.is_timeout() && !err.is_connect(),
        reqwest_middleware::Error::Middleware(err) => err.chain().any(|source| {
            source
                .downcast_ref::<reqwest::Error>()
                .is_some_and(|err| err.is_timeout() && !err.is_connect())
        }),
    }
}

fn chain_has_connection_io_error(err: &(dyn StdError + 'static)) -> bool {
    // Reqwest does not always classify mid-flight resets as `is_connect()`, so
    // inspect the source chain for concrete socket teardown errors as well.
    let mut current: Option<&(dyn StdError + 'static)> = Some(err);
    while let Some(source) = current {
        if let Some(io_err) = source.downcast_ref::<std::io::Error>() {
            if matches!(
                io_err.kind(),
                ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::BrokenPipe
                    | ErrorKind::NotConnected
            ) {
                return true;
            }
        }
        current = source.source();
    }
    false
}

#[derive(Clone, Copy, Debug)]
struct ConnectionRetryStrategy;

impl RetryableStrategy for ConnectionRetryStrategy {
    fn handle(
        &self,
        result: &std::result::Result<reqwest::Response, reqwest_middleware::Error>,
    ) -> Option<Retryable> {
        match result {
            Ok(_) => None,
            Err(err) => retryable_transport_failure(err).or(Some(Retryable::Fatal)),
        }
    }
}

static OVERRIDE_CLIENT: Lazy<RwLock<Option<ClientWithMiddleware>>> =
    Lazy::new(|| RwLock::new(None));
static SHARED_CLIENTS: Lazy<DashMap<ClientSettings, ClientWithMiddleware>> =
    Lazy::new(DashMap::new);

pub fn set_override_client(client: ClientWithMiddleware) {
    *OVERRIDE_CLIENT.write() = Some(client);
}

pub fn clear_override_client() {
    *OVERRIDE_CLIENT.write() = None;
}

#[cfg(test)]
fn clear_cached_clients() {
    SHARED_CLIENTS.clear();
}

#[cfg(test)]
fn cached_client_count() -> usize {
    SHARED_CLIENTS.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[test]
    #[serial]
    fn build_middleware_client_with_no_override() {
        clear_override_client();
        clear_cached_clients();
        let client = build_middleware_client(&ClientSettings::default());
        assert!(client.is_ok());
    }

    #[test]
    #[serial]
    fn build_middleware_client_reuses_cached_client_for_same_settings() {
        clear_override_client();
        clear_cached_clients();

        let settings = ClientSettings::default();
        let first = build_middleware_client(&settings).expect("first client");
        let second = build_middleware_client(&settings).expect("second client");

        assert_eq!(cached_client_count(), 1);
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    #[test]
    #[serial]
    fn build_middleware_client_creates_distinct_cached_clients_for_distinct_settings() {
        clear_override_client();
        clear_cached_clients();

        let first_settings = ClientSettings::default();
        let second_settings = ClientSettings {
            request_timeout: Duration::from_secs(30),
            ..ClientSettings::default()
        };

        build_middleware_client(&first_settings).expect("first client");
        build_middleware_client(&second_settings).expect("second client");

        assert_eq!(cached_client_count(), 2);
    }

    #[test]
    fn default_request_timeout_is_600_seconds() {
        assert_eq!(
            ClientSettings::default().request_timeout,
            Duration::from_secs(600)
        );
    }

    #[tokio::test]
    async fn request_timeout_is_not_retryable_transport_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(100)))
            .mount(&server)
            .await;

        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(10))
            .build()
            .expect("client");
        let err = client
            .get(format!("{}/slow", server.uri()))
            .send()
            .await
            .expect_err("request should time out");

        assert!(err.is_timeout());
        assert!(retryable_transport_failure(&reqwest_middleware::Error::Reqwest(err)).is_none());
    }
}
