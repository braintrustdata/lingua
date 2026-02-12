use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(feature = "provider-bedrock")]
use base64::Engine as _;
use bytes::{Bytes, BytesMut};
use futures::Stream;
use reqwest::Response;

use crate::error::{Error, Result};
use lingua::ProviderFormat;
use lingua::TransformResult;

/// A single chunk in a streaming response, carrying the JSON data and
/// an optional SSE event type (e.g. `"message_start"` for Anthropic).
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub data: Bytes,
    pub event_type: Option<String>,
}

impl StreamChunk {
    pub fn data(data: Bytes) -> Self {
        Self {
            data,
            event_type: None,
        }
    }

    pub fn with_event(data: Bytes, event_type: String) -> Self {
        Self {
            data,
            event_type: Some(event_type),
        }
    }
}

/// Stream of transformed chunks ready for output.
/// Serialized using lingua's serde_json at the boundary.
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

/// Stream of raw chunks from providers.
/// Each chunk contains the JSON bytes for a single SSE event.
pub type RawResponseStream = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

/// Create a raw SSE stream that yields JSON bytes without transformation.
///
/// Parses Server-Sent Events from the HTTP response and yields raw JSON bytes.
/// Use `transform_stream()` to convert to the desired output format.
pub fn sse_stream(response: Response) -> RawResponseStream {
    Box::pin(RawSseStream::new(response.bytes_stream()))
}

/// Transform a raw stream of bytes chunks to the specified output format.
///
/// This is the central transformation point for all streaming responses.
/// It takes raw bytes from any provider and transforms them using lingua.
/// The stream yields pre-serialized bytes.
pub fn transform_stream(raw: RawResponseStream, output_format: ProviderFormat) -> ResponseStream {
    use futures::StreamExt;
    Box::pin(raw.filter_map(move |result| {
        let output_format = output_format;
        async move {
            match result {
                Ok(chunk) => {
                    let StreamChunk { data, event_type } = chunk;
                    // Check for keep-alive marker (empty or whitespace-only bytes)
                    if data.is_empty() || data.iter().all(|b| b.is_ascii_whitespace()) {
                        return None;
                    }

                    // Transform with lingua (bytes-based)
                    match lingua::transform_stream_chunk(data.clone(), output_format) {
                        Ok(TransformResult::PassThrough(pass_bytes)) => {
                            Some(Ok(StreamChunk { data: pass_bytes, event_type }))
                        }
                        Ok(TransformResult::Transformed {
                            bytes: out_bytes, ..
                        }) => {
                            // Skip empty payloads (from terminal events like message_stop)
                            if out_bytes.as_ref() == b"{}" {
                                None
                            } else {
                                Some(Ok(StreamChunk { data: out_bytes, event_type }))
                            }
                        }
                        Err(lingua::TransformError::UnableToDetectFormat) => {
                            // Pass through unrecognized formats
                            Some(Ok(StreamChunk { data, event_type }))
                        }
                        Err(e) => Some(Err(Error::Lingua(e))),
                    }
                }
                Err(e) => Some(Err(e)),
            }
        }
    }))
}

/// Create a single-bytes stream from a raw response.
///
/// Used for fake streaming when a provider doesn't support native streaming.
/// The router's transform_stream will handle converting this to the output format.
pub fn single_bytes_stream(bytes: Bytes) -> RawResponseStream {
    Box::pin(SingleBytesStream { bytes: Some(bytes) })
}

struct SingleBytesStream {
    bytes: Option<Bytes>,
}

impl Stream for SingleBytesStream {
    type Item = Result<StreamChunk>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Poll::Ready(this.bytes.take().map(|b| Ok(StreamChunk::data(b))))
    }
}

/// Raw SSE stream that yields JSON bytes without parsing.
struct RawSseStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    inner: S,
    buffer: BytesMut,
    finished: bool,
}

impl<S> RawSseStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: BytesMut::new(),
            finished: false,
        }
    }
}

impl<S> Stream for RawSseStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    type Item = Result<StreamChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.finished {
            return Poll::Ready(None);
        }

        loop {
            if let Some((event, rest)) = split_event(&this.buffer) {
                this.buffer = rest;
                match extract_json_bytes_from_sse(event) {
                    Ok(Some(chunk)) => return Poll::Ready(Some(Ok(chunk))),
                    Ok(None) => {
                        // [DONE] signal
                        this.finished = true;
                        return Poll::Ready(None);
                    }
                    Err(err) => return Poll::Ready(Some(Err(err))),
                }
            }

            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    this.buffer.extend_from_slice(&bytes);
                }
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err.into()))),
                Poll::Ready(None) => {
                    if this.buffer.is_empty() {
                        this.finished = true;
                        return Poll::Ready(None);
                    }

                    let remaining = this.buffer.split().freeze();
                    match extract_json_bytes_from_sse(remaining) {
                        Ok(Some(chunk)) => return Poll::Ready(Some(Ok(chunk))),
                        Ok(None) => {
                            this.finished = true;
                            return Poll::Ready(None);
                        }
                        Err(err) => return Poll::Ready(Some(Err(err))),
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Extract JSON bytes and optional event type from an SSE event without parsing.
/// Returns None for [DONE] signal, Some(StreamChunk) for data events.
fn extract_json_bytes_from_sse(event: Bytes) -> Result<Option<StreamChunk>> {
    let raw = String::from_utf8_lossy(&event);
    let mut data = String::new();
    let mut event_type: Option<String> = None;

    for line in raw.lines() {
        if let Some(payload) = line.strip_prefix("data:") {
            let payload = payload.trim_start();
            if payload == "[DONE]" {
                return Ok(None);
            }
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(payload);
        } else if let Some(name) = line.strip_prefix("event:") {
            event_type = Some(name.trim().to_string());
        }
    }

    if data.is_empty() {
        return Ok(Some(StreamChunk::data(Bytes::new())));
    }

    let chunk = match event_type {
        Some(et) => StreamChunk::with_event(Bytes::from(data), et),
        None => StreamChunk::data(Bytes::from(data)),
    };
    Ok(Some(chunk))
}

/// Raw Bedrock event stream that yields JSON bytes without transformation.
#[cfg(feature = "provider-bedrock")]
struct RawBedrockEventStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    inner: S,
    buffer: BytesMut,
    decoder: aws_smithy_eventstream::frame::MessageFrameDecoder,
    finished: bool,
}

#[cfg(feature = "provider-bedrock")]
impl<S> RawBedrockEventStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: BytesMut::new(),
            decoder: aws_smithy_eventstream::frame::MessageFrameDecoder::new(),
            finished: false,
        }
    }
}

#[cfg(feature = "provider-bedrock")]
impl<S> Stream for RawBedrockEventStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    type Item = Result<StreamChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use aws_smithy_eventstream::frame::DecodedFrame;

        let this = self.get_mut();

        if this.finished {
            return Poll::Ready(None);
        }

        loop {
            match this.decoder.decode_frame(&mut this.buffer) {
                Ok(DecodedFrame::Complete(message)) => {
                    // Extract event type from headers
                    let event_type = message
                        .headers()
                        .iter()
                        .find(|h| h.name().as_str() == ":event-type")
                        .and_then(|h| h.value().as_string().ok())
                        .map(|s| s.as_str().to_string());

                    // Get payload bytes
                    let payload = message.payload();
                    if payload.is_empty() {
                        continue;
                    }

                    if event_type.as_deref() == Some("chunk") {
                        if let Some(decoded_chunk) = decode_bedrock_chunk_payload(payload) {
                            return Poll::Ready(Some(Ok(decoded_chunk)));
                        }
                    }

                    // Wrap the payload with event type as JSON bytes
                    // For Bedrock, we need to wrap: { "eventType": <payload> }
                    let json_bytes = if let Some(event_type) = event_type {
                        // Create wrapped JSON manually without parsing the payload
                        let payload_str = String::from_utf8_lossy(payload);
                        Bytes::from(format!(r#"{{"{}": {}}}"#, event_type, payload_str))
                    } else {
                        Bytes::copy_from_slice(payload)
                    };

                    return Poll::Ready(Some(Ok(StreamChunk::data(json_bytes))));
                }
                Ok(DecodedFrame::Incomplete) => {
                    // Need more data, fall through to poll inner stream
                }
                Err(e) => {
                    return Poll::Ready(Some(Err(Error::Provider {
                        provider: "bedrock".to_string(),
                        source: anyhow::anyhow!("Event stream decode error: {}", e),
                        retry_after: None,
                        http: None,
                    })));
                }
            }

            // Poll for more data
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    this.buffer.extend_from_slice(&bytes);
                }
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err.into()))),
                Poll::Ready(None) => {
                    this.finished = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Create a raw Bedrock event stream that yields JSON bytes without transformation.
///
/// Parses AWS binary event stream format and yields raw JSON bytes.
/// Use `transform_stream()` to convert to the desired output format.
/// NOTE: This will be moved to bedrock.rs in a future refactor.
#[cfg(feature = "provider-bedrock")]
pub fn bedrock_event_stream(response: Response) -> RawResponseStream {
    Box::pin(RawBedrockEventStream::new(response.bytes_stream()))
}

#[cfg(feature = "provider-bedrock")]
fn decode_bedrock_chunk_payload(payload: &[u8]) -> Option<Bytes> {
    let chunk_envelope: lingua::serde_json::Value = lingua::serde_json::from_slice(payload).ok()?;
    let encoded_chunk = chunk_envelope.get("bytes")?.as_str()?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded_chunk)
        .ok()?;
    Some(Bytes::from(decoded_bytes))
}

fn split_event(buffer: &BytesMut) -> Option<(Bytes, BytesMut)> {
    // Check for \r\n\r\n first (4-byte CRLF delimiter)
    if let Some(index) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
        let split_at = index + 4;
        let event = buffer[..split_at].to_vec();
        let remaining = buffer[split_at..].to_vec();
        return Some((Bytes::from(event), BytesMut::from(&remaining[..])));
    }
    // Fall back to \n\n (2-byte LF delimiter)
    if let Some(index) = buffer.windows(2).position(|w| w == b"\n\n") {
        let split_at = index + 2;
        let event = buffer[..split_at].to_vec();
        let remaining = buffer[split_at..].to_vec();
        return Some((Bytes::from(event), BytesMut::from(&remaining[..])));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_bytes_from_sse_extracts_data() {
        let event = Bytes::from("data: {\"test\": 1}\n\n");
        let result = extract_json_bytes_from_sse(event).unwrap();
        assert!(result.is_some());
        let chunk = result.unwrap();
        assert_eq!(chunk.data.as_ref(), b"{\"test\": 1}");
        assert!(chunk.event_type.is_none());
    }

    #[test]
    fn extract_json_bytes_preserves_event_type() {
        let event = Bytes::from("event: message_start\ndata: {\"type\":\"message_start\"}\n\n");
        let result = extract_json_bytes_from_sse(event).unwrap();
        assert!(result.is_some());
        let chunk = result.unwrap();
        assert_eq!(chunk.data.as_ref(), b"{\"type\":\"message_start\"}");
        assert_eq!(chunk.event_type.as_deref(), Some("message_start"));
    }

    #[test]
    fn extract_json_bytes_returns_none_for_done() {
        let event = Bytes::from("data: [DONE]\n\n");
        let result = extract_json_bytes_from_sse(event).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn split_event_handles_lf_delimiter() {
        let mut buffer = BytesMut::from("data: {\"test\": 1}\n\ndata: {\"test\": 2}\n\n");
        let (event, rest) = split_event(&buffer).expect("should split");
        assert!(String::from_utf8_lossy(&event).contains("test"));
        buffer = rest;
        assert!(!buffer.is_empty());
    }
}
