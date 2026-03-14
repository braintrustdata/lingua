use std::time::Duration;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::{Client, ClientBuilder};
use reqwest_middleware::ClientWithMiddleware;

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
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
    let client = build_client(settings)?;
    Ok(reqwest_middleware::ClientBuilder::new(client).build())
}

static OVERRIDE_CLIENT: Lazy<RwLock<Option<ClientWithMiddleware>>> =
    Lazy::new(|| RwLock::new(None));

pub fn set_override_client(client: ClientWithMiddleware) {
    *OVERRIDE_CLIENT.write() = Some(client);
}

pub fn clear_override_client() {
    *OVERRIDE_CLIENT.write() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_middleware_client_with_no_override() {
        clear_override_client();
        let client = build_middleware_client(&ClientSettings::default());
        assert!(client.is_ok());
    }
}
