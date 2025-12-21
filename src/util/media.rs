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

/// Parse file metadata from a URL.
///
/// This handles:
/// - Regular HTTP(S) URLs: extracts filename from path
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

    // If the URL is not http(s), file cannot be accessed
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
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
}

// ============================================================================
// Async URL Fetching - Native implementation
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native_fetch {
    use super::*;

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
        let response = reqwest::get(url)
            .await
            .map_err(|e| MediaError::FetchError(e.to_string()))?;

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
        let bytes = response
            .bytes()
            .await
            .map_err(|e| MediaError::FetchError(e.to_string()))?;

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
}

// Re-export the appropriate implementation
#[cfg(target_arch = "wasm32")]
pub use wasm_fetch::fetch_url_to_base64;

#[cfg(not(target_arch = "wasm32"))]
pub use native_fetch::fetch_url_to_base64;

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
    // Try to parse as data URL first
    if let Some(block) = parse_base64_data_url(media) {
        return Ok(block);
    }

    // Otherwise fetch the URL
    fetch_url_to_base64(media, allowed_types, max_bytes).await
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
    fn test_parse_file_metadata_from_url_invalid() {
        assert!(parse_file_metadata_from_url("").is_none());
        assert!(parse_file_metadata_from_url("not a url").is_none());
        assert!(parse_file_metadata_from_url("ftp://example.com/file").is_none());
        assert!(parse_file_metadata_from_url("https://example.com/").is_none());
    }
}
