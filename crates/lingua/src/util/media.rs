/*!
Media URL fetching and conversion utilities.

This module provides utilities for fetching media from URLs and converting
between base64 data URLs and binary data. It mirrors the proxy's behavior
in `packages/proxy/src/providers/util.ts`.
*/

use thiserror::Error;

/// A parsed media block containing the MIME type and base64-encoded data.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaBlock {
    /// The MIME type of the media (e.g., "image/png", "application/pdf").
    pub media_type: String,
    /// The base64-encoded data (without the data URL prefix).
    pub data: String,
}

/// Errors that can occur during media operations.
#[derive(Debug, Error)]
pub enum MediaError {
    /// Failed to fetch media from URL.
    #[error("failed to fetch media: {0}")]
    FetchError(String),
    /// The response did not include a content type.
    #[error("failed to get content type of the media")]
    MissingContentType,
    /// The media type is not in the allowed list.
    #[error("unsupported media type: {0}")]
    UnsupportedMediaType(String),
    /// The media size exceeds the maximum allowed.
    #[error("media size exceeds the {0} MB limit")]
    SizeExceeded(usize),
    /// Failed to decode base64 data.
    #[error("failed to decode base64: {0}")]
    Base64Error(String),
}

/// Parse a base64 data URL into its components.
///
/// Returns `None` if the URL is not a valid data URL.
///
/// # Example
///
/// ```
/// use lingua::util::media::parse_base64_data_url;
///
/// let url = "data:image/png;base64,iVBORw0KGgo=";
/// let block = parse_base64_data_url(url).unwrap();
/// assert_eq!(block.media_type, "image/png");
/// assert_eq!(block.data, "iVBORw0KGgo=");
/// ```
pub fn parse_base64_data_url(url: &str) -> Option<MediaBlock> {
    // Pattern: data:<media_type>;base64,<data>
    if !url.starts_with("data:") {
        return None;
    }

    let without_prefix = &url["data:".len()..];
    let (meta, data) = without_prefix.split_once(',')?;

    if data.is_empty() {
        return None;
    }

    // meta should be like "image/png;base64"
    let (media_type, encoding) = meta.split_once(';')?;
    if encoding != "base64" {
        return None;
    }

    Some(MediaBlock {
        media_type: media_type.to_string(),
        data: data.to_string(),
    })
}

/// Convert a MediaBlock back to a data URL.
///
/// # Example
///
/// ```
/// use lingua::util::media::{MediaBlock, media_block_to_url};
///
/// let block = MediaBlock {
///     media_type: "image/png".to_string(),
///     data: "iVBORw0KGgo=".to_string(),
/// };
/// let url = media_block_to_url(&block);
/// assert_eq!(url, "data:image/png;base64,iVBORw0KGgo=");
/// ```
pub fn media_block_to_url(block: &MediaBlock) -> String {
    format!("data:{};base64,{}", block.media_type, block.data)
}

/// Parse file metadata from a URL.
///
/// Returns the filename extracted from the URL path, and optionally the content type
/// if it can be determined from query parameters (e.g., S3 presigned URLs).
#[derive(Debug, Clone, PartialEq)]
pub struct FileMetadata {
    /// The filename extracted from the URL.
    pub filename: String,
    /// The content type, if available.
    pub content_type: Option<String>,
}

/// Return whether a media reference can be sent to a provider as a remote URI.
pub fn is_remote_media_uri(reference: &str) -> bool {
    url::Url::parse(reference)
        .map(|parsed| matches!(parsed.scheme(), "http" | "https" | "gs"))
        .unwrap_or(false)
}

/// Infer a MIME type from a filename or supported remote URI reference.
///
/// A content type encoded in a signed URL takes precedence over filename and
/// path-extension inference.
pub fn infer_mime_type_from_reference(filename: Option<&str>, url: Option<&str>) -> Option<String> {
    let url_metadata = url.and_then(parse_file_metadata_from_url);

    if let Some(content_type) = url_metadata
        .as_ref()
        .and_then(|metadata| metadata.content_type.clone())
    {
        return Some(content_type);
    }

    let url_filename = url_metadata
        .as_ref()
        .map(|metadata| metadata.filename.as_str());

    filename
        .into_iter()
        .chain(url_filename)
        .find_map(mime_type_from_filename)
        .map(str::to_string)
}

fn mime_type_from_filename(name: &str) -> Option<&'static str> {
    let extension = name.rsplit_once('.')?.1;

    if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        Some("image/jpeg")
    } else if extension.eq_ignore_ascii_case("png") {
        Some("image/png")
    } else if extension.eq_ignore_ascii_case("webp") {
        Some("image/webp")
    } else if extension.eq_ignore_ascii_case("heic") {
        Some("image/heic")
    } else if extension.eq_ignore_ascii_case("heif") {
        Some("image/heif")
    } else if extension.eq_ignore_ascii_case("pdf") {
        Some("application/pdf")
    } else if extension.eq_ignore_ascii_case("txt") {
        Some("text/plain")
    } else if extension.eq_ignore_ascii_case("flv") {
        Some("video/x-flv")
    } else if extension.eq_ignore_ascii_case("mov") {
        Some("video/quicktime")
    } else if extension.eq_ignore_ascii_case("mp4") {
        Some("video/mp4")
    } else if extension.eq_ignore_ascii_case("mpeg") {
        Some("video/mpeg")
    } else if extension.eq_ignore_ascii_case("mpegs") {
        Some("video/mpegs")
    } else if extension.eq_ignore_ascii_case("mpg") {
        Some("video/mpg")
    } else if extension.eq_ignore_ascii_case("wmv") {
        Some("video/wmv")
    } else if extension.eq_ignore_ascii_case("3gp") || extension.eq_ignore_ascii_case("3gpp") {
        Some("video/3gpp")
    } else if extension.eq_ignore_ascii_case("aac") {
        Some("audio/x-aac")
    } else if extension.eq_ignore_ascii_case("flac") {
        Some("audio/flac")
    } else if extension.eq_ignore_ascii_case("mp3") {
        Some("audio/mp3")
    } else if extension.eq_ignore_ascii_case("m4a") {
        Some("audio/m4a")
    } else if extension.eq_ignore_ascii_case("mpga") {
        Some("audio/mpga")
    } else if extension.eq_ignore_ascii_case("ogg") {
        Some("audio/ogg")
    } else if extension.eq_ignore_ascii_case("pcm") {
        Some("audio/pcm")
    } else if extension.eq_ignore_ascii_case("wav") {
        Some("audio/wav")
    } else {
        None
    }
}

/// Parse file metadata from a URL.
///
/// This handles:
/// - Regular HTTP(S) URLs and GCS URIs: extracts filename from path
/// - S3 presigned URLs: extracts filename from response-content-disposition
///
/// Returns `None` if the URL cannot be parsed or doesn't contain a filename.
pub fn parse_file_metadata_from_url(url: &str) -> Option<FileMetadata> {
    // Handle empty string
    if url.is_empty() || url.trim().is_empty() {
        return None;
    }

    // Try to parse as URL
    let parsed = url::Url::parse(url).ok()?;

    if !matches!(parsed.scheme(), "http" | "https" | "gs") {
        return None;
    }

    // If pathname is empty or ends with "/", there's no filename to extract
    let path = parsed.path();
    if path.is_empty() || path == "/" || path.ends_with('/') {
        return None;
    }

    // Get the last segment of the path
    let mut filename = path.split('/').next_back()?.to_string();
    if filename.is_empty() {
        return None;
    }

    let mut content_type = None;

    // Handle case where this is an S3 pre-signed URL
    if parsed.query_pairs().any(|(k, _)| k == "X-Amz-Expires") {
        // Try to extract filename from response-content-disposition
        if let Some((_, disposition)) = parsed
            .query_pairs()
            .find(|(k, _)| k == "response-content-disposition")
        {
            // Simple parsing: look for filename="..." or filename*=...
            if let Some(fname) = parse_content_disposition(&disposition) {
                filename = fname;
            }
        }

        // Try to extract content type
        if let Some((_, ct)) = parsed
            .query_pairs()
            .find(|(k, _)| k == "response-content-type")
        {
            content_type = Some(ct.to_string());
        }
    }

    // URL decode the filename
    if let Ok(decoded) = urlencoding::decode(&filename) {
        filename = decoded.into_owned();
    }

    Some(FileMetadata {
        filename,
        content_type,
    })
}

/// Simple content-disposition parser to extract filename.
fn parse_content_disposition(value: &str) -> Option<String> {
    // Look for filename="..." pattern
    if let Some(start) = value.find("filename=\"") {
        let rest = &value[start + 10..];
        if let Some(end) = rest.find('"') {
            let filename = &rest[..end];
            return Some(
                urlencoding::decode(filename)
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| filename.to_string()),
            );
        }
    }

    // Look for filename*=UTF-8''... pattern
    if let Some(start) = value.find("filename*=") {
        let rest = &value[start + 10..];
        // Skip encoding prefix like "UTF-8''"
        if let Some(quote_pos) = rest.find("''") {
            let encoded = &rest[quote_pos + 2..];
            // Take until semicolon or end
            let encoded = encoded.split(';').next().unwrap_or(encoded);
            return urlencoding::decode(encoded.trim())
                .map(|s| s.into_owned())
                .ok();
        }
    }

    None
}

// ============================================================================
// Async URL Fetching - WASM implementation
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_fetch {
    use super::*;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    /// Fetch a URL and return its content as a MediaBlock.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch
    /// * `allowed_types` - Optional list of allowed MIME types. If `None`, all types are allowed.
    /// * `max_bytes` - Optional maximum size in bytes. If `None`, no limit is applied.
    pub async fn fetch_url_to_base64(
        url: &str,
        allowed_types: Option<&[&str]>,
        max_bytes: Option<usize>,
    ) -> Result<MediaBlock, MediaError> {
        let window = web_sys::window().ok_or_else(|| MediaError::FetchError("no window".into()))?;

        // Create request
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| MediaError::FetchError(format!("{:?}", e)))?;

        // Fetch
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| MediaError::FetchError(format!("{:?}", e)))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| MediaError::FetchError("response is not a Response".into()))?;

        if !resp.ok() {
            return Err(MediaError::FetchError(format!(
                "HTTP {}",
                resp.status_text()
            )));
        }

        // Get content type
        let content_type = resp
            .headers()
            .get("content-type")
            .map_err(|e| MediaError::FetchError(format!("{:?}", e)))?
            .ok_or(MediaError::MissingContentType)?;

        // Extract base content type (before semicolon)
        let base_content_type = content_type
            .split(';')
            .next()
            .unwrap_or(&content_type)
            .trim()
            .to_string();

        // Check allowed types
        if let Some(allowed) = allowed_types {
            if !allowed.contains(&base_content_type.as_str()) {
                return Err(MediaError::UnsupportedMediaType(base_content_type));
            }
        }

        // Get array buffer
        let array_buffer = JsFuture::from(
            resp.array_buffer()
                .map_err(|e| MediaError::FetchError(format!("{:?}", e)))?,
        )
        .await
        .map_err(|e| MediaError::FetchError(format!("{:?}", e)))?;

        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        let bytes = uint8_array.to_vec();

        // Check size
        if let Some(max) = max_bytes {
            if bytes.len() > max {
                return Err(MediaError::SizeExceeded(max / 1024 / 1024));
            }
        }

        // Encode to base64
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.encode(&bytes);

        Ok(MediaBlock {
            media_type: base_content_type,
            data,
        })
    }

    /// Fetch a URL using the platform trust store plus an optional PEM CA bundle.
    pub async fn fetch_url_to_base64_with_additional_ca_bundle(
        url: &str,
        allowed_types: Option<&[&str]>,
        max_bytes: Option<usize>,
        additional_ca_bundle: Option<&str>,
    ) -> Result<MediaBlock, MediaError> {
        if additional_ca_bundle.is_some() {
            return Err(MediaError::FetchError(
                "additional CA bundles are not supported on wasm32".to_string(),
            ));
        }
        fetch_url_to_base64(url, allowed_types, max_bytes).await
    }
}

// ============================================================================
// Async URL Fetching - Native implementation
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native_fetch {
    use super::*;
    use ipnet::IpNet;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
    use std::str::FromStr;
    use std::time::Duration;
    use url::{Host, Url};

    const MAX_REDIRECTS: usize = 3;
    const MEDIA_FETCH_TIMEOUT: Duration = Duration::from_secs(30);
    const ALLOW_CIDRS_ENV: &str = "BRAINTRUST_URL_SECURITY_ALLOW_CIDRS";
    const AWS_IMDS_IPV6: Ipv6Addr = Ipv6Addr::new(0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254);

    fn ipv4_in_cidr(address: Ipv4Addr, base: Ipv4Addr, prefix_len: u32) -> bool {
        let address = u32::from(address);
        let base = u32::from(base);
        let mask = if prefix_len == 0 {
            0
        } else {
            u32::MAX << (32 - prefix_len)
        };

        (address & mask) == (base & mask)
    }

    fn is_hard_blocked_ipv4(address: Ipv4Addr) -> bool {
        address.is_link_local()
            || address.is_multicast()
            || address.is_unspecified()
            || ipv4_in_cidr(address, Ipv4Addr::new(0, 0, 0, 0), 8)
            || ipv4_in_cidr(address, Ipv4Addr::new(224, 0, 0, 0), 4)
            || ipv4_in_cidr(address, Ipv4Addr::new(240, 0, 0, 0), 4)
    }

    fn is_blocked_ipv4(address: Ipv4Addr) -> bool {
        is_hard_blocked_ipv4(address)
            || address.is_loopback()
            || address.is_private()
            || address.is_link_local()
            || ipv4_in_cidr(address, Ipv4Addr::new(100, 64, 0, 0), 10)
            || ipv4_in_cidr(address, Ipv4Addr::new(192, 0, 0, 0), 24)
            || ipv4_in_cidr(address, Ipv4Addr::new(198, 18, 0, 0), 15)
    }

    fn is_hard_blocked_ipv6(address: Ipv6Addr) -> bool {
        address == AWS_IMDS_IPV6
            || address.is_unspecified()
            || address.is_multicast()
            || address.is_unicast_link_local()
    }

    fn is_blocked_ipv6(address: Ipv6Addr) -> bool {
        is_hard_blocked_ipv6(address)
            || address.is_loopback()
            || address.is_unique_local()
            || address.is_unicast_link_local()
            || address.to_ipv4_mapped().is_some_and(is_blocked_ipv4)
    }

    fn is_blocked_ip(address: IpAddr) -> bool {
        match address {
            IpAddr::V4(address) => is_blocked_ipv4(address),
            IpAddr::V6(address) => is_blocked_ipv6(address),
        }
    }

    fn is_hard_blocked_hostname(hostname: &str) -> bool {
        let hostname = hostname.trim_end_matches('.');
        hostname.eq_ignore_ascii_case("metadata.amazonaws.com")
            || hostname.eq_ignore_ascii_case("metadata.google.internal")
    }

    fn normalize_ip_for_policy(address: IpAddr) -> IpAddr {
        match address {
            IpAddr::V6(address) => address
                .to_ipv4_mapped()
                .map(IpAddr::V4)
                .unwrap_or(IpAddr::V6(address)),
            IpAddr::V4(_) => address,
        }
    }

    fn parse_allowed_cidrs() -> Result<Vec<IpNet>, MediaError> {
        let Ok(raw) = std::env::var(ALLOW_CIDRS_ENV) else {
            return Ok(Vec::new());
        };
        raw.split(',')
            .map(str::trim)
            .filter(|cidr| !cidr.is_empty())
            .map(|cidr| {
                IpNet::from_str(cidr).map_err(|error| {
                    MediaError::FetchError(format!("invalid {ALLOW_CIDRS_ENV}: {error}"))
                })
            })
            .collect()
    }

    fn is_allowed_ip(address: IpAddr, allowed_cidrs: &[IpNet]) -> bool {
        let address = normalize_ip_for_policy(address);
        let hard_blocked = match address {
            IpAddr::V4(address) => is_hard_blocked_ipv4(address),
            IpAddr::V6(address) => is_hard_blocked_ipv6(address),
        };
        if hard_blocked {
            return false;
        }

        !is_blocked_ip(address) || allowed_cidrs.iter().any(|cidr| cidr.contains(&address))
    }

    struct ValidatedMediaUrl {
        hostname: Option<String>,
        addresses: Vec<SocketAddr>,
    }

    fn validate_media_url(url: &Url) -> Result<ValidatedMediaUrl, MediaError> {
        let allowed_cidrs = parse_allowed_cidrs()?;
        validate_media_url_with_allowed_cidrs(url, &allowed_cidrs)
    }

    fn validate_media_url_with_allowed_cidrs(
        url: &Url,
        allowed_cidrs: &[IpNet],
    ) -> Result<ValidatedMediaUrl, MediaError> {
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(MediaError::FetchError(
                "media URL must use http or https".to_string(),
            ));
        }

        let host = url
            .host()
            .ok_or_else(|| MediaError::FetchError("media URL is missing a host".to_string()))?;
        match host {
            Host::Ipv4(address) => {
                if !is_allowed_ip(IpAddr::V4(address), allowed_cidrs) {
                    return Err(MediaError::FetchError(
                        "media URL resolves to a blocked address".to_string(),
                    ));
                }
                return Ok(ValidatedMediaUrl {
                    hostname: None,
                    addresses: Vec::new(),
                });
            }
            Host::Ipv6(address) => {
                if !is_allowed_ip(IpAddr::V6(address), allowed_cidrs) {
                    return Err(MediaError::FetchError(
                        "media URL resolves to a blocked address".to_string(),
                    ));
                }
                return Ok(ValidatedMediaUrl {
                    hostname: None,
                    addresses: Vec::new(),
                });
            }
            Host::Domain(host) => {
                if is_hard_blocked_hostname(host) {
                    return Err(MediaError::FetchError(
                        "media URL resolves to a blocked address".to_string(),
                    ));
                }
            }
        }

        let hostname = url
            .host_str()
            .ok_or_else(|| MediaError::FetchError("media URL is missing a host".to_string()))?;
        let port = url.port_or_known_default().ok_or_else(|| {
            MediaError::FetchError("media URL is missing a valid port".to_string())
        })?;
        let addresses = (hostname, port)
            .to_socket_addrs()
            .map_err(|e| MediaError::FetchError(format!("failed to resolve media URL: {e}")))?;

        let mut resolved_addresses = Vec::new();
        for address in addresses {
            if !is_allowed_ip(address.ip(), allowed_cidrs) {
                return Err(MediaError::FetchError(
                    "media URL resolves to a blocked address".to_string(),
                ));
            }
            resolved_addresses.push(address);
        }

        if resolved_addresses.is_empty() {
            return Err(MediaError::FetchError(
                "media URL did not resolve to any addresses".to_string(),
            ));
        }

        Ok(ValidatedMediaUrl {
            hostname: Some(hostname.to_string()),
            addresses: resolved_addresses,
        })
    }

    fn add_additional_ca_bundle(
        mut builder: reqwest::ClientBuilder,
        additional_ca_bundle: Option<&str>,
    ) -> Result<reqwest::ClientBuilder, MediaError> {
        let Some(additional_ca_bundle) = additional_ca_bundle else {
            return Ok(builder);
        };

        let certificates = reqwest::Certificate::from_pem_bundle(additional_ca_bundle.as_bytes())
            .map_err(|error| MediaError::FetchError(error.to_string()))?;
        if certificates.is_empty() {
            return Err(MediaError::FetchError(
                "additional CA bundle contains no certificates".to_string(),
            ));
        }
        for certificate in certificates {
            builder = builder.add_root_certificate(certificate);
        }
        Ok(builder)
    }

    async fn fetch_validated_url(
        url: &str,
        additional_ca_bundle: Option<&str>,
    ) -> Result<reqwest::Response, MediaError> {
        let mut current_url = Url::parse(url)
            .map_err(|e| MediaError::FetchError(format!("invalid media URL: {e}")))?;

        for redirect_count in 0..=MAX_REDIRECTS {
            let validated_url = validate_media_url(&current_url)?;
            let mut client_builder = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(MEDIA_FETCH_TIMEOUT);
            if let Some(hostname) = validated_url.hostname {
                client_builder =
                    client_builder.resolve_to_addrs(&hostname, &validated_url.addresses);
            }
            client_builder = add_additional_ca_bundle(client_builder, additional_ca_bundle)?;
            let client = client_builder
                .build()
                .map_err(|e| MediaError::FetchError(e.to_string()))?;
            let response = client
                .get(current_url.clone())
                .send()
                .await
                .map_err(|e| MediaError::FetchError(e.to_string()))?;

            if !response.status().is_redirection() {
                return Ok(response);
            }

            if redirect_count >= MAX_REDIRECTS {
                return Err(MediaError::FetchError(
                    "media URL exceeded redirect limit".to_string(),
                ));
            }

            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    MediaError::FetchError("media URL redirect missing location header".to_string())
                })?;
            current_url = current_url
                .join(location)
                .map_err(|e| MediaError::FetchError(format!("invalid media redirect URL: {e}")))?;
            validate_media_url(&current_url)?;
        }

        Err(MediaError::FetchError(
            "media URL exceeded redirect limit".to_string(),
        ))
    }

    async fn response_bytes_with_limit(
        response: &mut reqwest::Response,
        max_bytes: Option<usize>,
    ) -> Result<Vec<u8>, MediaError> {
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| MediaError::FetchError(e.to_string()))?
        {
            bytes.extend_from_slice(&chunk);
            if let Some(max) = max_bytes {
                if bytes.len() > max {
                    return Err(MediaError::SizeExceeded(max / 1024 / 1024));
                }
            }
        }

        Ok(bytes)
    }

    /// Fetch a URL and return its content as a MediaBlock.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch
    /// * `allowed_types` - Optional list of allowed MIME types. If `None`, all types are allowed.
    /// * `max_bytes` - Optional maximum size in bytes. If `None`, no limit is applied.
    pub async fn fetch_url_to_base64(
        url: &str,
        allowed_types: Option<&[&str]>,
        max_bytes: Option<usize>,
    ) -> Result<MediaBlock, MediaError> {
        fetch_url_to_base64_with_additional_ca_bundle(url, allowed_types, max_bytes, None).await
    }

    /// Fetch a URL using the default trust store plus an optional PEM CA bundle.
    pub async fn fetch_url_to_base64_with_additional_ca_bundle(
        url: &str,
        allowed_types: Option<&[&str]>,
        max_bytes: Option<usize>,
        additional_ca_bundle: Option<&str>,
    ) -> Result<MediaBlock, MediaError> {
        let mut response = fetch_validated_url(url, additional_ca_bundle).await?;

        if !response.status().is_success() {
            return Err(MediaError::FetchError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        // Get content type
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .ok_or(MediaError::MissingContentType)?
            .to_string();

        // Extract base content type (before semicolon)
        let base_content_type = content_type
            .split(';')
            .next()
            .unwrap_or(&content_type)
            .trim()
            .to_string();

        // Check allowed types
        if let Some(allowed) = allowed_types {
            if !allowed.contains(&base_content_type.as_str()) {
                return Err(MediaError::UnsupportedMediaType(base_content_type));
            }
        }

        // Get bytes
        let bytes = response_bytes_with_limit(&mut response, max_bytes).await?;

        // Encode to base64
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.encode(&bytes);

        Ok(MediaBlock {
            media_type: base_content_type,
            data,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn validate_media_url_rejects_non_http_schemes() {
            let url = Url::parse("file:///etc/passwd").unwrap();
            assert!(matches!(
                validate_media_url_with_allowed_cidrs(&url, &[]),
                Err(MediaError::FetchError(message))
                    if message.contains("http or https")
            ));
        }

        #[test]
        fn validate_media_url_rejects_localhost() {
            let url = Url::parse("http://localhost/image.png").unwrap();
            assert!(matches!(
                validate_media_url_with_allowed_cidrs(&url, &[]),
                Err(MediaError::FetchError(message))
                    if message.contains("blocked address")
            ));
        }

        #[test]
        fn validate_media_url_allows_configured_private_cidr() {
            let url = Url::parse("http://127.0.0.1/image.png").unwrap();
            let allowed_cidrs = [IpNet::from_str("127.0.0.0/8").unwrap()];
            assert!(validate_media_url_with_allowed_cidrs(&url, &allowed_cidrs).is_ok());
        }

        #[test]
        fn validate_media_url_rejects_dns_resolved_localhost() {
            let url = Url::parse("http://localhost./image.png").unwrap();
            assert!(matches!(
                validate_media_url_with_allowed_cidrs(&url, &[]),
                Err(MediaError::FetchError(message))
                    if message.contains("blocked address")
            ));
        }

        #[test]
        fn validate_media_url_rejects_metadata_ip() {
            let url = Url::parse("http://169.254.169.254/latest/meta-data").unwrap();
            let allowed_cidrs = [IpNet::from_str("169.254.0.0/16").unwrap()];
            assert!(matches!(
                validate_media_url_with_allowed_cidrs(&url, &allowed_cidrs),
                Err(MediaError::FetchError(message))
                    if message.contains("blocked address")
            ));
        }

        #[test]
        fn validate_media_url_rejects_ipv6_metadata_ip_despite_allowed_ula() {
            let url = Url::parse("http://[fd00:ec2::254]/latest/meta-data").unwrap();
            let allowed_cidrs = [IpNet::from_str("fd00::/8").unwrap()];
            assert!(matches!(
                validate_media_url_with_allowed_cidrs(&url, &allowed_cidrs),
                Err(MediaError::FetchError(message))
                    if message.contains("blocked address")
            ));
        }

        #[test]
        fn validate_media_url_rejects_cloud_metadata_hostnames() {
            for url in [
                "http://metadata.amazonaws.com/latest/meta-data",
                "http://metadata.google.internal/computeMetadata/v1",
            ] {
                let url = Url::parse(url).unwrap();
                assert!(matches!(
                    validate_media_url_with_allowed_cidrs(&url, &[]),
                    Err(MediaError::FetchError(message))
                        if message.contains("blocked address")
                ));
            }
        }

        #[test]
        fn redirect_from_public_url_to_localhost_or_private_ip_is_rejected() {
            let current_url = Url::parse("https://example.com/image.png").unwrap();

            for location in [
                "http://127.0.0.1:8080/private.png",
                "http://192.168.0.10/private.png",
            ] {
                let redirect_url = current_url.join(location).unwrap();

                assert!(matches!(
                    validate_media_url_with_allowed_cidrs(&redirect_url, &[]),
                    Err(MediaError::FetchError(message))
                        if message.contains("blocked address")
                ));
            }
        }

        #[test]
        fn validate_media_url_rejects_ipv4_mapped_ipv6_localhost() {
            let dotted_url = Url::parse("http://[::ffff:127.0.0.1]/image.png").unwrap();
            assert!(matches!(
                validate_media_url_with_allowed_cidrs(&dotted_url, &[]),
                Err(MediaError::FetchError(message))
                    if message.contains("blocked address")
            ));

            let hex_url = Url::parse("http://[::ffff:7f00:1]/image.png").unwrap();
            assert!(matches!(
                validate_media_url_with_allowed_cidrs(&hex_url, &[]),
                Err(MediaError::FetchError(message))
                    if message.contains("blocked address")
            ));
        }

        #[test]
        fn blocked_ip_ranges_include_metadata_local_private_and_multicast_addresses() {
            for address in [
                IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255)),
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                IpAddr::V4(Ipv4Addr::new(0, 255, 255, 255)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)),
                IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
                IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1)),
                IpAddr::V4(Ipv4Addr::new(239, 255, 255, 255)),
                IpAddr::V6(Ipv6Addr::LOCALHOST),
                IpAddr::V6("ff00::1".parse().unwrap()),
                IpAddr::V6("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff".parse().unwrap()),
                IpAddr::V6("::ffff:127.0.0.1".parse().unwrap()),
            ] {
                assert!(is_blocked_ip(address), "{address} should be blocked");
            }

            for address in [
                IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
                IpAddr::V6("2606:2800:220:1:248:1893:25c8:1946".parse().unwrap()),
            ] {
                assert!(!is_blocked_ip(address), "{address} should be allowed");
            }
        }

        #[test]
        fn additional_ca_bundle_rejects_content_without_certificates() {
            let error =
                add_additional_ca_bundle(reqwest::Client::builder(), Some("not a certificate"))
                    .expect_err("certificate-free bundle should fail");

            assert!(matches!(
                error,
                MediaError::FetchError(message)
                    if message.contains("contains no certificates")
            ));
        }
    }
}

// Re-export the appropriate implementation
#[cfg(target_arch = "wasm32")]
pub use wasm_fetch::{fetch_url_to_base64, fetch_url_to_base64_with_additional_ca_bundle};

#[cfg(not(target_arch = "wasm32"))]
pub use native_fetch::{fetch_url_to_base64, fetch_url_to_base64_with_additional_ca_bundle};

/// Convert media (URL or data URL) to a MediaBlock.
///
/// If the input is already a base64 data URL, it is parsed directly.
/// Otherwise, the URL is fetched and the content is converted to base64.
///
/// # Arguments
///
/// * `media` - A URL or data URL
/// * `allowed_types` - Optional list of allowed MIME types for fetched URLs
/// * `max_bytes` - Optional maximum size in bytes for fetched URLs
pub async fn convert_media_to_base64(
    media: &str,
    allowed_types: Option<&[&str]>,
    max_bytes: Option<usize>,
) -> Result<MediaBlock, MediaError> {
    convert_media_to_base64_with_additional_ca_bundle(media, allowed_types, max_bytes, None).await
}

/// Convert media to base64, trusting an optional additional PEM CA bundle for URL fetches.
pub async fn convert_media_to_base64_with_additional_ca_bundle(
    media: &str,
    allowed_types: Option<&[&str]>,
    max_bytes: Option<usize>,
    additional_ca_bundle: Option<&str>,
) -> Result<MediaBlock, MediaError> {
    // Try to parse as data URL first
    if let Some(block) = parse_base64_data_url(media) {
        return Ok(block);
    }

    // Otherwise fetch the URL
    fetch_url_to_base64_with_additional_ca_bundle(
        media,
        allowed_types,
        max_bytes,
        additional_ca_bundle,
    )
    .await
}

/// Check if a URL is a localhost URL (for special handling).
pub fn is_localhost_url(url: &str) -> bool {
    url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base64_data_url_valid() {
        let url = "data:image/png;base64,iVBORw0KGgo=";
        let block = parse_base64_data_url(url).unwrap();
        assert_eq!(block.media_type, "image/png");
        assert_eq!(block.data, "iVBORw0KGgo=");
    }

    #[test]
    fn test_parse_base64_data_url_pdf() {
        let url = "data:application/pdf;base64,JVBERi0xLjQ=";
        let block = parse_base64_data_url(url).unwrap();
        assert_eq!(block.media_type, "application/pdf");
        assert_eq!(block.data, "JVBERi0xLjQ=");
    }

    #[test]
    fn test_parse_base64_data_url_invalid() {
        assert!(parse_base64_data_url("not a data url").is_none());
        assert!(parse_base64_data_url("data:image/png,nobase64").is_none());
        assert!(parse_base64_data_url("data:image/png;base64,").is_none());
    }

    #[test]
    fn test_media_block_to_url() {
        let block = MediaBlock {
            media_type: "image/jpeg".to_string(),
            data: "abcd1234".to_string(),
        };
        assert_eq!(
            media_block_to_url(&block),
            "data:image/jpeg;base64,abcd1234"
        );
    }

    #[test]
    fn test_is_localhost_url() {
        assert!(is_localhost_url("http://localhost:3000/image.png"));
        assert!(is_localhost_url("http://127.0.0.1:8080/file.pdf"));
        assert!(!is_localhost_url("https://example.com/image.png"));
        // Note: This matches proxy behavior - starts_with check doesn't prevent this
        // but in practice this URL pattern is unlikely to be a problem
        assert!(is_localhost_url("http://localhostfake.com/x"));
    }

    #[test]
    fn test_parse_file_metadata_from_url_simple() {
        let metadata =
            parse_file_metadata_from_url("https://example.com/path/to/file.pdf").unwrap();
        assert_eq!(metadata.filename, "file.pdf");
        assert!(metadata.content_type.is_none());
    }

    #[test]
    fn test_parse_file_metadata_from_url_encoded() {
        let metadata = parse_file_metadata_from_url("https://example.com/my%20file.pdf").unwrap();
        assert_eq!(metadata.filename, "my file.pdf");
    }

    #[test]
    fn test_parse_file_metadata_from_gcs_uri() {
        let metadata =
            parse_file_metadata_from_url("gs://lingua-test-bucket/media/sample-200mb.mp4").unwrap();
        assert_eq!(metadata.filename, "sample-200mb.mp4");
        assert!(metadata.content_type.is_none());
        assert!(is_remote_media_uri(
            "gs://lingua-test-bucket/media/sample-200mb.mp4"
        ));
    }

    #[test]
    fn test_parse_file_metadata_from_url_invalid() {
        assert!(parse_file_metadata_from_url("").is_none());
        assert!(parse_file_metadata_from_url("not a url").is_none());
        assert!(parse_file_metadata_from_url("ftp://example.com/file").is_none());
        assert!(parse_file_metadata_from_url("https://example.com/").is_none());
    }

    #[test]
    fn test_infer_mime_type_from_reference_uses_filename_and_url() {
        assert_eq!(
            infer_mime_type_from_reference(Some("sample-3s.MP3"), Some("https://example.com/file")),
            Some("audio/mp3".to_string())
        );
        assert_eq!(
            infer_mime_type_from_reference(
                None,
                Some("https://example.com/video/sample-5s.mp4?signature=abc")
            ),
            Some("video/mp4".to_string())
        );
        assert_eq!(
            infer_mime_type_from_reference(
                Some("audio"),
                Some("https://example.com/audio/sample-3s.mp3")
            ),
            Some("audio/mp3".to_string())
        );
        assert_eq!(
            infer_mime_type_from_reference(
                None,
                Some("gs://lingua-test-bucket/video/sample-200mb.mp4")
            ),
            Some("video/mp4".to_string())
        );
    }

    #[test]
    fn test_infer_mime_type_from_reference_uses_signed_url_content_type() {
        assert_eq!(
            infer_mime_type_from_reference(
                Some("sample.mp3"),
                Some(
                    "https://example.com/file?X-Amz-Expires=60&response-content-type=audio%2Fmpeg"
                )
            ),
            Some("audio/mpeg".to_string())
        );
    }
}
