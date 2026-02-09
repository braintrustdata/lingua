use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::Stream;
use reqwest::Response;

use crate::error::{Error, Result};
#[cfg(feature = "provider-bedrock")]
use lingua::serde_json::Value;
use lingua::ProviderFormat;
use lingua::TransformResult;

/// Stream of transformed chunks ready for output (yields pre-serialized bytes).
/// Serialized using lingua's serde_json at the boundary.
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>;

/// Stream of raw bytes chunks from providers.
/// Each chunk contains the JSON bytes for a single SSE event (no parsing done).
pub type RawResponseStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>;

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
                Ok(bytes) => {
                    // Check for keep-alive marker (empty or whitespace-only bytes)
                    if bytes.is_empty() || bytes.iter().all(|b| b.is_ascii_whitespace()) {
                        return None;
                    }

                    // Transform with lingua (bytes-based)
                    match lingua::transform_stream_chunk(bytes.clone(), output_format) {
                        Ok(TransformResult::PassThrough(pass_bytes)) => Some(Ok(pass_bytes)),
                        Ok(TransformResult::Transformed {
                            bytes: out_bytes, ..
                        }) => {
                            // Skip empty payloads (from terminal events like message_stop)
                            if out_bytes.as_ref() == b"{}" {
                                None
                            } else {
                                Some(Ok(out_bytes))
                            }
                        }
                        Err(lingua::TransformError::UnableToDetectFormat) => {
                            // Pass through unrecognized formats
                            Some(Ok(bytes))
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
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Poll::Ready(this.bytes.take().map(Ok))
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
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.finished {
            return Poll::Ready(None);
        }

        loop {
            if let Some((event, rest)) = split_event(&this.buffer) {
                this.buffer = rest;
                match extract_json_bytes_from_sse(event) {
                    Ok(Some(json_bytes)) => return Poll::Ready(Some(Ok(json_bytes))),
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
                        Ok(Some(json_bytes)) => return Poll::Ready(Some(Ok(json_bytes))),
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

/// Extract JSON bytes from an SSE event without parsing.
/// Returns None for [DONE] signal, Some(Bytes) for data events.
fn extract_json_bytes_from_sse(event: Bytes) -> Result<Option<Bytes>> {
    let raw = String::from_utf8_lossy(&event);
    let mut data = String::new();

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
        }
    }

    if data.is_empty() {
        // Empty event - skip (keep-alive)
        return Ok(Some(Bytes::new()));
    }

    // Return the JSON as bytes without parsing
    Ok(Some(Bytes::from(data)))
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
    type Item = Result<Bytes>;

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

                    // Wrap the payload with event type as JSON bytes
                    // For Bedrock, we need to wrap: { "eventType": <payload> }
                    let json_bytes = if let Some(event_type) = event_type {
                        // Create wrapped JSON manually without parsing the payload
                        let payload_str = String::from_utf8_lossy(payload);
                        Bytes::from(format!(r#"{{"{}": {}}}"#, event_type, payload_str))
                    } else {
                        Bytes::copy_from_slice(payload)
                    };

                    return Poll::Ready(Some(Ok(json_bytes)));
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

/// Bedrock Messages API event stream that yields raw Anthropic JSON payloads.
///
/// Uses the same AWS binary event stream decoder as the Converse stream but
/// emits the payload bytes directly without wrapping in `{"eventType": payload}`.
/// The payloads are already valid Anthropic streaming JSON events.
#[cfg(feature = "provider-bedrock")]
struct RawBedrockMessagesEventStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    inner: S,
    buffer: BytesMut,
    decoder: aws_smithy_eventstream::frame::MessageFrameDecoder,
    finished: bool,
}

#[cfg(feature = "provider-bedrock")]
impl<S> RawBedrockMessagesEventStream<S>
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
impl<S> Stream for RawBedrockMessagesEventStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use aws_smithy_eventstream::frame::DecodedFrame;

        let this = self.get_mut();

        if this.finished {
            return Poll::Ready(None);
        }

        loop {
            match this.decoder.decode_frame(&mut this.buffer) {
                Ok(DecodedFrame::Complete(message)) => {
                    let payload = message.payload();
                    if payload.is_empty() {
                        continue;
                    }

                    // The invoke-with-response-stream payload is
                    // {"bytes":"<base64-encoded Anthropic JSON>"}
                    // We need to extract and decode the bytes field.
                    let json_bytes = match extract_bedrock_invoke_payload(payload) {
                        Ok(Some(decoded)) => decoded,
                        Ok(None) => continue,
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    };

                    return Poll::Ready(Some(Ok(json_bytes)));
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

/// Create a Bedrock Messages API event stream that yields raw Anthropic JSON payloads.
///
/// Uses the AWS binary event stream decoder but emits payloads directly
/// (no `{"eventType": payload}` wrapping). For use with Anthropic models on Bedrock.
#[cfg(feature = "provider-bedrock")]
pub fn bedrock_messages_event_stream(response: Response) -> RawResponseStream {
    Box::pin(RawBedrockMessagesEventStream::new(response.bytes_stream()))
}

/// Extract the Anthropic JSON payload from a Bedrock invoke-with-response-stream event.
///
/// The event payload has the shape `{"bytes":"<base64-encoded JSON>"}`.
/// Returns `Ok(Some(decoded_bytes))` on success, `Ok(None)` if the payload
/// should be skipped, or `Err` on decode failure.
#[cfg(feature = "provider-bedrock")]
fn extract_bedrock_invoke_payload(raw: &[u8]) -> Result<Option<Bytes>> {
    use base64::Engine;

    let wrapper: Value = lingua::serde_json::from_slice(raw).map_err(|e| Error::Provider {
        provider: "bedrock".to_string(),
        source: anyhow::anyhow!("failed to parse invoke stream event: {}", e),
        retry_after: None,
        http: None,
    })?;

    let b64 = match wrapper.get("bytes").and_then(Value::as_str) {
        Some(s) => s,
        None => return Ok(None),
    };

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| Error::Provider {
            provider: "bedrock".to_string(),
            source: anyhow::anyhow!("failed to base64-decode invoke stream event: {}", e),
            retry_after: None,
            http: None,
        })?;

    Ok(Some(Bytes::from(decoded)))
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
        assert_eq!(result.unwrap().as_ref(), b"{\"test\": 1}");
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

    #[cfg(feature = "provider-bedrock")]
    mod bedrock_messages_stream {
        use super::*;

        #[test]
        fn extract_bedrock_invoke_payload_decodes_base64() {
            use base64::Engine;

            let inner_json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
            let encoded = base64::engine::general_purpose::STANDARD.encode(inner_json);
            let wrapper = format!(r#"{{"bytes":"{}"}}"#, encoded);

            let result = extract_bedrock_invoke_payload(wrapper.as_bytes()).unwrap();
            assert!(result.is_some());
            let decoded = result.unwrap();
            assert_eq!(decoded.as_ref(), inner_json.as_bytes());
        }

        #[test]
        fn extract_bedrock_invoke_payload_returns_none_without_bytes_field() {
            let payload = br#"{"other_field": "value"}"#;
            let result = extract_bedrock_invoke_payload(payload).unwrap();
            assert!(result.is_none());
        }

        #[test]
        fn extract_bedrock_invoke_payload_errors_on_invalid_json() {
            let payload = b"not json";
            let result = extract_bedrock_invoke_payload(payload);
            assert!(result.is_err());
        }

        #[test]
        fn extract_bedrock_invoke_payload_errors_on_invalid_base64() {
            let payload = br#"{"bytes": "!!!not-valid-base64!!!"}"#;
            let result = extract_bedrock_invoke_payload(payload);
            assert!(result.is_err());
        }

        #[test]
        fn extract_bedrock_invoke_payload_handles_message_start_event() {
            use base64::Engine;

            let inner_json = r#"{"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":1}}}"#;
            let encoded = base64::engine::general_purpose::STANDARD.encode(inner_json);
            let wrapper = format!(r#"{{"bytes":"{}"}}"#, encoded);

            let result = extract_bedrock_invoke_payload(wrapper.as_bytes()).unwrap();
            assert!(result.is_some());
            let decoded = result.unwrap();
            let decoded_str = std::str::from_utf8(&decoded).unwrap();
            assert!(decoded_str.contains("message_start"));
            assert!(decoded_str.contains("msg_123"));
        }

        #[test]
        fn extract_bedrock_invoke_payload_handles_message_stop_event() {
            use base64::Engine;

            let inner_json = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}"#;
            let encoded = base64::engine::general_purpose::STANDARD.encode(inner_json);
            let wrapper = format!(r#"{{"bytes":"{}"}}"#, encoded);

            let result = extract_bedrock_invoke_payload(wrapper.as_bytes()).unwrap();
            assert!(result.is_some());
            let decoded = result.unwrap();
            let decoded_str = std::str::from_utf8(&decoded).unwrap();
            assert!(decoded_str.contains("message_delta"));
            assert!(decoded_str.contains("end_turn"));
        }
    }
}
