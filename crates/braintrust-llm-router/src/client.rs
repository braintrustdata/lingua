use std::time::Duration;

use dashmap::DashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::{Client, ClientBuilder};
use reqwest_middleware::ClientWithMiddleware;

use crate::error::{Error, Result};

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
            request_timeout: Duration::from_secs(300),
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
        Ok::<ClientWithMiddleware, Error>(reqwest_middleware::ClientBuilder::new(client).build())
    })?;

    #[cfg(not(feature = "tracing"))]
    let client = {
        let client = build_client(settings)?;
        reqwest_middleware::ClientBuilder::new(client).build()
    };

    if let Some(existing) = SHARED_CLIENTS.get(settings) {
        return Ok(existing.clone());
    }
    SHARED_CLIENTS.insert(settings.clone(), client.clone());
    Ok(client)
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

    #[test]
    fn build_middleware_client_with_no_override() {
        clear_override_client();
        clear_cached_clients();
        let client = build_middleware_client(&ClientSettings::default());
        assert!(client.is_ok());
    }

    #[test]
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
}
