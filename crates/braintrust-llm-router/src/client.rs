use std::error::Error as StdError;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;

use dashmap::DashMap;
use http::Extensions;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::{redirect::Policy, Certificate, Client, ClientBuilder, Request, Response};
use reqwest_middleware::{ClientWithMiddleware, Middleware, Next};
use reqwest_retry::{
    default_on_request_failure, policies::ExponentialBackoff, RetryTransientMiddleware, Retryable,
    RetryableStrategy,
};

use crate::error::{Error, Result};

// The default number of retries for transient transport failures.
const DEFAULT_MAX_RETRIES: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DnsOverride {
    pub domain: String,
    pub addrs: Vec<SocketAddr>,
}

// Shared reqwest clients are cached by these settings. Keep this key
// low-cardinality and effectively process-wide; request-scoped values do not
// belong here because they fragment client reuse and connection pooling.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientSettings {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub pool_idle_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    // Force HTTP/1.1 for providers whose high-concurrency path performs better without HTTP/2 multiplexing.
    pub http1_only: bool,
    pub user_agent: String,
    pub dns_overrides: Vec<DnsOverride>,
    pub follow_redirects: bool,
    pub additional_ca_bundle: Option<String>,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(600),
            pool_idle_timeout: Duration::from_secs(90),
            pool_max_idle_per_host: 16,
            http1_only: false,
            user_agent: format!("braintrust-llm-router/{}", env!("CARGO_PKG_VERSION")),
            dns_overrides: Vec::new(),
            follow_redirects: true,
            additional_ca_bundle: None,
        }
    }
}

pub fn build_client(settings: &ClientSettings) -> Result<Client> {
    let mut builder = ClientBuilder::new()
        .connect_timeout(settings.connect_timeout)
        .timeout(settings.request_timeout)
        .pool_idle_timeout(settings.pool_idle_timeout)
        .pool_max_idle_per_host(settings.pool_max_idle_per_host)
        .user_agent(&settings.user_agent);

    #[cfg(target_os = "macos")]
    {
        // Hickory snapshots the DNS configuration and can become stale after a network change.
        builder = builder.no_hickory_dns();
    }

    if settings.http1_only {
        builder = builder.http1_only();
    }

    if !settings.follow_redirects {
        builder = builder.redirect(Policy::none());
    }

    for override_entry in &settings.dns_overrides {
        builder = builder.resolve_to_addrs(&override_entry.domain, &override_entry.addrs);
    }

    builder = add_additional_ca_bundle(builder, settings.additional_ca_bundle.as_deref())?;

    builder.build().map_err(Error::from)
}

pub fn add_additional_ca_bundle(
    mut builder: ClientBuilder,
    additional_ca_bundle: Option<&str>,
) -> Result<ClientBuilder> {
    let Some(additional_ca_bundle) = additional_ca_bundle else {
        return Ok(builder);
    };

    let certificates = Certificate::from_pem_bundle(additional_ca_bundle.as_bytes())?;
    if certificates.is_empty() {
        return Err(Error::InvalidRequest(
            "additional CA bundle contains no certificates".to_string(),
        ));
    }

    for certificate in certificates {
        builder = builder.add_root_certificate(certificate);
    }
    Ok(builder)
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
        client.http1_only = settings.http1_only,
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
        client.http1_only = settings.http1_only,
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
        .with(ResponseMetadataMiddleware)
        .with(retry_middleware)
        .build()
}

struct ResponseMetadataMiddleware;

#[async_trait::async_trait]
impl Middleware for ResponseMetadataMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        let response = next.run(req, extensions).await?;

        #[cfg(feature = "tracing")]
        tracing::Span::current().record(
            "http.response.version",
            http_version_label(response.version()),
        );

        Ok(response)
    }
}

#[cfg(feature = "tracing")]
fn http_version_label(version: reqwest::Version) -> &'static str {
    match version {
        reqwest::Version::HTTP_09 => "HTTP/0.9",
        reqwest::Version::HTTP_10 => "HTTP/1.0",
        reqwest::Version::HTTP_11 => "HTTP/1.1",
        reqwest::Version::HTTP_2 => "HTTP/2",
        reqwest::Version::HTTP_3 => "HTTP/3",
        _ => "unknown",
    }
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
fn has_cached_client(settings: &ClientSettings) -> bool {
    SHARED_CLIENTS.contains_key(settings)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
    use rustls::{ServerConfig, ServerConnection, StreamOwned};

    use super::*;
    use serial_test::serial;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    const TEST_CA_CERT: &str = include_str!("../testdata/custom-ca/ca-cert.pem");
    const UNRELATED_CA_CERT: &str = include_str!("../testdata/custom-ca/unrelated-ca-cert.pem");
    const TEST_SERVER_CERT: &str = include_str!("../testdata/custom-ca/server-cert.pem");
    const TEST_SERVER_KEY: &str = include_str!("../testdata/custom-ca/server-key.pem");

    fn spawn_https_server() -> (String, thread::JoinHandle<std::io::Result<()>>) {
        let certificates = CertificateDer::pem_slice_iter(TEST_SERVER_CERT.as_bytes())
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("server certificate");
        let private_key =
            PrivateKeyDer::from_pem_slice(TEST_SERVER_KEY.as_bytes()).expect("server private key");
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)
            .expect("TLS server config");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind TLS server");
        let address = listener.local_addr().expect("TLS server address");

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept()?;
            let connection =
                ServerConnection::new(Arc::new(config)).map_err(std::io::Error::other)?;
            let mut stream = StreamOwned::new(connection, stream);
            let mut request = [0; 1024];
            let bytes_read = stream.read(&mut request)?;
            if bytes_read == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            )?;
            stream.flush()
        });

        (format!("https://localhost:{}/", address.port()), handle)
    }

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

        assert!(has_cached_client(&settings));
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

        assert!(has_cached_client(&first_settings));
        assert!(has_cached_client(&second_settings));
    }

    #[test]
    #[serial]
    fn build_middleware_client_reuses_cached_client_for_same_dns_overrides() {
        clear_override_client();
        clear_cached_clients();

        let settings = ClientSettings {
            dns_overrides: vec![DnsOverride {
                domain: "example.com".to_string(),
                addrs: vec!["127.0.0.1:443".parse().expect("socket addr")],
            }],
            ..ClientSettings::default()
        };

        let first = build_middleware_client(&settings).expect("first client");
        let second = build_middleware_client(&settings).expect("second client");

        assert!(has_cached_client(&settings));
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    #[test]
    fn default_request_timeout_is_600_seconds() {
        assert_eq!(
            ClientSettings::default().request_timeout,
            Duration::from_secs(600)
        );
    }

    #[test]
    fn build_client_rejects_a_bundle_without_certificates() {
        let settings = ClientSettings {
            additional_ca_bundle: Some("not a certificate".to_string()),
            ..ClientSettings::default()
        };

        let error = build_client(&settings).expect_err("certificate-free bundle should fail");
        assert_eq!(
            error.to_string(),
            "invalid request: additional CA bundle contains no certificates"
        );
    }

    #[test]
    fn build_client_rejects_malformed_certificate_pem() {
        let settings = ClientSettings {
            additional_ca_bundle: Some(
                "-----BEGIN CERTIFICATE-----\nnot-base64\n-----END CERTIFICATE-----".to_string(),
            ),
            ..ClientSettings::default()
        };

        let error = build_client(&settings).expect_err("malformed certificate should fail");
        assert!(matches!(error, Error::Http(_)));
    }

    #[test]
    fn build_client_accepts_a_multi_certificate_bundle() {
        let settings = ClientSettings {
            additional_ca_bundle: Some(format!("{UNRELATED_CA_CERT}\n{TEST_CA_CERT}")),
            ..ClientSettings::default()
        };

        build_client(&settings).expect("multi-certificate CA bundle");
    }

    #[tokio::test]
    async fn additional_ca_bundle_enables_a_private_tls_connection() {
        let (url, server) = spawn_https_server();
        let default_client = build_client(&ClientSettings::default()).expect("default client");

        default_client
            .get(&url)
            .send()
            .await
            .expect_err("private CA should not be trusted by default");
        let _ = server.join();

        let (url, server) = spawn_https_server();
        let settings = ClientSettings {
            additional_ca_bundle: Some(format!("{UNRELATED_CA_CERT}\n{TEST_CA_CERT}")),
            ..ClientSettings::default()
        };
        let client = build_client(&settings).expect("client with additional CA");
        let response = client.get(url).send().await.expect("private TLS request");

        assert_eq!(response.text().await.expect("response body"), "ok");
        server
            .join()
            .expect("TLS server thread")
            .expect("TLS server request");
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
