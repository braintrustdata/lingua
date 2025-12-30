//! Helper functions for Braintrust SDK integration.
//!
//! This module provides transform functions that convert lingua JSON payloads
//! to universal format for logging to Braintrust. The helpers take lingua::serde_json::Value
//! directly to avoid unnecessary serialization round-trips.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use braintrust_sdk_rust::{SpanHandle, SpanLog};
use bytes::Bytes;
use futures::Stream;
use lingua::processing::ProviderAdapter;
use lingua::providers::openai::OpenAIAdapter;
use lingua::{ProviderFormat, TransformResult, UniversalStreamChunk, UniversalUsage};

use crate::error::Result as RouterResult;
use crate::streaming::RawResponseStream;
use tokio::sync::Mutex;

/// Convert UniversalUsage to HashMap<String, f64> for SpanLog.metrics.
pub fn universal_usage_to_map(usage: &UniversalUsage) -> HashMap<String, f64> {
    let mut map = HashMap::new();
    if let Some(v) = usage.prompt_tokens {
        map.insert("prompt_tokens".into(), v as f64);
    }
    if let Some(v) = usage.completion_tokens {
        map.insert("completion_tokens".into(), v as f64);
    }
    // Calculate total_tokens on the fly
    if let (Some(prompt), Some(completion)) = (usage.prompt_tokens, usage.completion_tokens) {
        map.insert("tokens".into(), (prompt + completion) as f64);
    }
    if let Some(v) = usage.prompt_cached_tokens {
        map.insert("prompt_cached_tokens".into(), v as f64);
    }
    if let Some(v) = usage.prompt_cache_creation_tokens {
        map.insert("prompt_cache_creation_tokens".into(), v as f64);
    }
    if let Some(v) = usage.completion_reasoning_tokens {
        map.insert("completion_reasoning_tokens".into(), v as f64);
    }
    map
}

/// Convert a request payload to universal format.
/// Returns (messages_value, metadata_map).
///
/// Uses bytes-based lingua transform to convert to OpenAI format (canonical),
/// then extracts messages and metadata from the result.
pub fn request_to_universal_value(
    payload: &[u8],
    _format: ProviderFormat,
) -> anyhow::Result<(
    serde_json::Value,
    serde_json::Map<String, serde_json::Value>,
)> {
    // Transform to OpenAI format (canonical) using bytes-based API
    let payload_bytes = Bytes::from(payload.to_vec());
    let transformed_bytes =
        match lingua::transform_request(payload_bytes.clone(), ProviderFormat::OpenAI, None) {
            Ok(TransformResult::PassThrough(bytes)) => bytes,
            Ok(TransformResult::Transformed { bytes, .. }) => bytes,
            Err(_) => payload_bytes, // Fallback to original if transform fails
        };

    // Parse transformed bytes into standard serde_json::Value
    let transformed: serde_json::Value = serde_json::from_slice(&transformed_bytes)
        .map_err(|e| anyhow::anyhow!("failed to parse transformed payload: {e}"))?;

    // Extract messages and metadata from OpenAI format
    let mut metadata = serde_json::Map::new();

    // Get messages array
    let messages = transformed
        .get("messages")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));

    // Extract model
    if let Some(model) = transformed.get("model").and_then(|v| v.as_str()) {
        metadata.insert("model".into(), serde_json::Value::String(model.to_string()));
    }

    // Extract common params into metadata
    let params = ["temperature", "top_p", "max_tokens", "stream"];
    for param in params {
        if let Some(v) = transformed.get(param) {
            metadata.insert(param.into(), v.clone());
        }
    }

    Ok((messages, metadata))
}

/// Convert a response payload to universal format.
/// Returns (messages_value, usage, metadata_map).
///
/// Parses the response to UniversalResponse (preserving all usage details),
/// then converts to OpenAI format for the output.
pub fn response_to_universal_value(
    payload: &[u8],
    _format: ProviderFormat,
) -> anyhow::Result<(
    serde_json::Value,
    Option<UniversalUsage>,
    serde_json::Map<String, serde_json::Value>,
)> {
    // Parse to UniversalResponse (preserves all usage fields)
    let resp = lingua::response_to_universal(Bytes::from(payload.to_vec()))
        .map_err(|e| anyhow::anyhow!("failed to parse response: {e}"))?;

    // Get usage directly from UniversalResponse (all fields preserved!)
    let usage = resp.usage.clone();

    // Convert to OpenAI format for output
    let output = OpenAIAdapter
        .response_from_universal(&resp)
        .map_err(|e| anyhow::anyhow!("failed to convert to OpenAI format: {e}"))?;

    // Convert lingua::serde_json::Value to serde_json::Value via serialization
    let output: serde_json::Value = serde_json::from_str(
        &lingua::serde_json::to_string(&output)
            .map_err(|e| anyhow::anyhow!("failed to serialize output: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("failed to deserialize output: {e}"))?;

    // Extract messages from OpenAI output
    let messages: Vec<serde_json::Value> = output
        .get("choices")
        .and_then(|c| c.as_array())
        .map(|choices| {
            choices
                .iter()
                .filter_map(|choice| choice.get("message").cloned())
                .collect()
        })
        .unwrap_or_default();

    // Build metadata from UniversalResponse
    let mut metadata = serde_json::Map::new();
    if let Some(model) = &resp.model {
        metadata.insert("model".into(), serde_json::Value::String(model.clone()));
    }
    if let Some(ref reason) = resp.finish_reason {
        metadata.insert(
            "finish_reason".into(),
            serde_json::Value::String(format!("{:?}", reason).to_lowercase()),
        );
    }

    Ok((serde_json::Value::Array(messages), usage, metadata))
}

/// Log request payload to span with universal format transformation.
/// Uses the known provider format for accurate transformation.
pub async fn log_request_to_span(span: &SpanHandle, payload: &[u8], format: ProviderFormat) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let start_metrics: HashMap<String, f64> =
        [("start".to_string(), start_time)].into_iter().collect();

    match request_to_universal_value(payload, format) {
        Ok((messages, metadata)) => {
            span.log(SpanLog {
                input: Some(messages),
                metadata: if metadata.is_empty() {
                    None
                } else {
                    Some(metadata)
                },
                metrics: Some(start_metrics),
                ..Default::default()
            })
            .await;
        }
        Err(_e) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(error = %_e, "Failed to transform request to universal format, logging raw payload");
            // Parse bytes to serde_json::Value for fallback logging
            if let Ok(raw_value) = serde_json::from_slice::<serde_json::Value>(payload) {
                span.log(SpanLog {
                    input: Some(raw_value),
                    metrics: Some(start_metrics),
                    ..Default::default()
                })
                .await;
            }
        }
    }
}

/// Log response payload to span with universal format transformation.
/// Uses the known provider format for accurate transformation.
/// Also records metrics.end timestamp and usage metrics.
pub async fn log_response_to_span(span: &SpanHandle, payload: &[u8], format: ProviderFormat) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let end_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    match response_to_universal_value(payload, format) {
        Ok((messages, usage, metadata)) => {
            let mut metrics = usage
                .as_ref()
                .map(universal_usage_to_map)
                .unwrap_or_default();
            metrics.insert("end".to_string(), end_time);

            span.log(SpanLog {
                output: Some(messages),
                metadata: if metadata.is_empty() {
                    None
                } else {
                    Some(metadata)
                },
                metrics: Some(metrics),
                ..Default::default()
            })
            .await;
        }
        Err(_e) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(error = %_e, "Failed to transform response to universal format, logging raw payload");
            let metrics: HashMap<String, f64> =
                [("end".to_string(), end_time)].into_iter().collect();
            // Parse bytes to serde_json::Value for fallback logging
            if let Ok(raw_value) = serde_json::from_slice::<serde_json::Value>(payload) {
                span.log(SpanLog {
                    output: Some(raw_value),
                    metrics: Some(metrics),
                    ..Default::default()
                })
                .await;
            }
        }
    }
}

// ============================================================================
// Streaming Support
// ============================================================================

/// Aggregated result from a streaming response.
#[derive(Clone)]
pub struct FinalizedStream {
    /// The output value (messages array)
    pub output: serde_json::Value,
    /// Usage extracted from the stream
    pub usage: Option<UniversalUsage>,
    /// Metadata (model, finish_reason)
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// A stream aggregator that collects bytes chunks and transforms them for logging.
/// Works with any provider format (OpenAI, Anthropic, Google, etc.)
#[derive(Clone)]
pub struct LinguaStreamAggregator {
    raw_chunks: Vec<Bytes>,
    finalized: Option<FinalizedStream>,
}

impl Default for LinguaStreamAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl LinguaStreamAggregator {
    /// Create a new empty stream aggregator.
    /// Chunks will be transformed to OpenAI format (lingua's canonical streaming format).
    pub fn new() -> Self {
        Self {
            raw_chunks: Vec::new(),
            finalized: None,
        }
    }

    /// Add a raw bytes chunk to the stream.
    /// Stores the raw chunk as-is for later aggregation.
    pub fn push(&mut self, chunk: Bytes) {
        // Skip empty chunks (keep-alive markers)
        if chunk.is_empty() || chunk.iter().all(|b| b.is_ascii_whitespace()) {
            return;
        }
        self.raw_chunks.push(chunk);
    }

    /// Get the final aggregated value.
    /// Aggregates all chunks into a final response. The result is cached.
    pub fn final_value(&mut self) -> anyhow::Result<&FinalizedStream> {
        if self.finalized.is_none() {
            self.finalized = Some(self.aggregate()?);
        }
        Ok(self.finalized.as_ref().unwrap())
    }

    /// Check if the stream has any chunks.
    pub fn is_empty(&self) -> bool {
        self.raw_chunks.is_empty()
    }

    fn aggregate(&self) -> anyhow::Result<FinalizedStream> {
        let mut metadata = serde_json::Map::new();
        let mut usage: Option<UniversalUsage> = None;
        let mut model: Option<String> = None;
        let mut finish_reason: Option<String> = None;
        let mut aggregated_content = String::new();
        let mut role: Option<String> = None;

        for raw_bytes in &self.raw_chunks {
            // Transform to OpenAI format (lingua's canonical streaming format)
            // transform_stream_chunk auto-detects source format and transforms to target
            let transformed_bytes =
                match lingua::transform_stream_chunk(raw_bytes.clone(), ProviderFormat::OpenAI) {
                    Ok(TransformResult::Transformed { bytes, .. }) => bytes,
                    Ok(TransformResult::PassThrough(bytes)) => bytes,
                    Err(_) => raw_bytes.clone(), // Fallback to original
                };

            // Parse bytes to UniversalStreamChunk
            let chunk: UniversalStreamChunk = match serde_json::from_slice(&transformed_bytes) {
                Ok(c) => c,
                Err(_) => continue, // Skip unparseable chunks
            };

            // Skip keep-alive chunks
            if chunk.is_keep_alive() {
                continue;
            }

            // Extract model (first non-None)
            if model.is_none() {
                model = chunk.model.clone();
            }

            // Extract usage (last non-None)
            if let Some(ref u) = chunk.usage {
                usage = Some(u.clone());
            }

            // Process choices
            for choice in &chunk.choices {
                if let Some(ref reason) = choice.finish_reason {
                    finish_reason = Some(reason.clone());
                }
                if let Some(ref delta) = choice.delta {
                    // delta is a lingua_json::Value, extract role and content
                    if role.is_none() {
                        if let Some(r) = delta.get("role").and_then(|v| v.as_str()) {
                            role = Some(r.to_string());
                        }
                    }
                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                        aggregated_content.push_str(content);
                    }
                }
            }
        }

        // Build metadata
        if let Some(m) = model {
            metadata.insert("model".into(), serde_json::Value::String(m));
        }
        if let Some(fr) = finish_reason {
            metadata.insert("finish_reason".into(), serde_json::Value::String(fr));
        }

        // Build output as messages array
        let mut message = serde_json::Map::new();
        message.insert(
            "role".into(),
            serde_json::Value::String(role.unwrap_or_else(|| "assistant".into())),
        );
        message.insert(
            "content".into(),
            serde_json::Value::String(aggregated_content),
        );

        Ok(FinalizedStream {
            output: serde_json::Value::Array(vec![serde_json::Value::Object(message)]),
            usage,
            metadata,
        })
    }
}

/// Wrap a raw bytes stream with span logging using universal format transformation.
///
/// This creates a new stream that yields the same chunks as the original, but also:
/// - Records metrics.start timestamp at stream creation
/// - Records time-to-first-token on first meaningful content
/// - Accumulates chunks for aggregation using universal format transformation
/// - On stream completion, logs the aggregated output/usage/metadata with metrics.end via `span.log()`
///
/// Chunks are automatically transformed to OpenAI format (lingua's canonical streaming format)
/// regardless of the original provider format.
pub fn wrap_stream_with_span_lingua(
    stream: RawResponseStream,
    span: SpanHandle,
) -> RawResponseStream {
    use futures::StreamExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let start_time = Instant::now();
    let start_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let start_timestamp = Arc::new(start_timestamp);
    let ttft_recorded = Arc::new(AtomicBool::new(false));
    let aggregator = Arc::new(Mutex::new(LinguaStreamAggregator::new()));
    let span_for_complete = span.clone();
    let aggregator_for_complete = Arc::clone(&aggregator);
    let start_timestamp_for_complete = Arc::clone(&start_timestamp);

    let logged_stream = stream.then(move |result| {
        let span = span.clone();
        let ttft_recorded = ttft_recorded.clone();
        let aggregator = aggregator.clone();
        let start_timestamp = start_timestamp.clone();
        async move {
            if let Ok(ref chunk_bytes) = result {
                if !chunk_bytes.is_empty() && !chunk_bytes.iter().all(|b| b.is_ascii_whitespace()) {
                    // Record TTFT and start timestamp on first meaningful chunk
                    // We skip empty keep-alive markers
                    if bytes_have_content(chunk_bytes)
                        && ttft_recorded
                            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                            .is_ok()
                    {
                        let ttft_secs = start_time.elapsed().as_secs_f64();
                        span.log(SpanLog {
                            metrics: Some(
                                [
                                    ("start".to_string(), *start_timestamp),
                                    ("time_to_first_token".to_string(), ttft_secs),
                                ]
                                .into_iter()
                                .collect(),
                            ),
                            ..Default::default()
                        })
                        .await;
                    }
                    // Accumulate chunk for final aggregation
                    aggregator.lock().await.push(chunk_bytes.clone());
                }
            }
            result
        }
    });

    // Wrap in a stream that finalizes on completion
    Box::pin(BytesSpanCompleteWrapper {
        inner: Box::pin(logged_stream),
        span: Some(span_for_complete),
        aggregator: Some(aggregator_for_complete),
        start_timestamp: Some(start_timestamp_for_complete),
    })
}

/// Check if bytes chunk contains meaningful output (for TTFT detection).
/// Transforms the chunk to OpenAI format (lingua's canonical streaming format) first,
/// then checks for content.
fn bytes_have_content(chunk: &Bytes) -> bool {
    // Transform to OpenAI format using bytes-based API
    let transformed_bytes =
        match lingua::transform_stream_chunk(chunk.clone(), ProviderFormat::OpenAI) {
            Ok(TransformResult::Transformed { bytes, .. }) => bytes,
            Ok(TransformResult::PassThrough(bytes)) => bytes,
            Err(_) => chunk.clone(),
        };

    // Parse to check content
    let value: serde_json::Value = match serde_json::from_slice(&transformed_bytes) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Check for choices array with content (OpenAI format)
    if let Some(choices) = value.get("choices").and_then(|c| c.as_array()) {
        if !choices.is_empty() {
            return true;
        }
    }
    // Check for usage with tokens
    if let Some(usage) = value.get("usage").and_then(|u| u.as_object()) {
        let has_tokens = usage
            .get("completion_tokens")
            .and_then(|v| v.as_i64())
            .map(|t| t > 0)
            .unwrap_or(false)
            || usage
                .get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .map(|t| t > 0)
                .unwrap_or(false);
        if has_tokens {
            return true;
        }
    }
    false
}

/// A wrapper stream that logs aggregated output when the stream is exhausted.
struct BytesSpanCompleteWrapper<S> {
    inner: S,
    span: Option<SpanHandle>,
    aggregator: Option<Arc<Mutex<LinguaStreamAggregator>>>,
    start_timestamp: Option<Arc<f64>>,
}

impl<S> Stream for BytesSpanCompleteWrapper<S>
where
    S: Stream<Item = RouterResult<Bytes>> + Unpin,
{
    type Item = RouterResult<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let this = self.get_mut();
        let result = Pin::new(&mut this.inner).poll_next(cx);

        // If stream is done, spawn task to finalize and log
        if matches!(result, Poll::Ready(None)) {
            if let (Some(span), Some(aggregator), Some(start_ts)) = (
                this.span.take(),
                this.aggregator.take(),
                this.start_timestamp.take(),
            ) {
                let end_timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);

                tokio::spawn(async move {
                    let mut agg = aggregator.lock().await;
                    if !agg.is_empty() {
                        match agg.final_value() {
                            Ok(finalized) => {
                                // Build metrics from usage + start/end timestamps
                                let mut metrics = finalized
                                    .usage
                                    .as_ref()
                                    .map(universal_usage_to_map)
                                    .unwrap_or_default();
                                metrics.insert("start".to_string(), *start_ts);
                                metrics.insert("end".to_string(), end_timestamp);

                                // Convert metadata Map to Option<Map>
                                let metadata = if finalized.metadata.is_empty() {
                                    None
                                } else {
                                    Some(finalized.metadata.clone())
                                };

                                span.log(SpanLog {
                                    output: Some(finalized.output.clone()),
                                    metadata,
                                    metrics: Some(metrics),
                                    ..Default::default()
                                })
                                .await;
                            }
                            Err(_e) => {
                                #[cfg(feature = "tracing")]
                                tracing::warn!(error = %_e, "Failed to finalize stream");
                            }
                        }
                    }
                    // Flush span with aggregated output
                    if let Err(_e) = span.flush().await {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(error = %_e, "Failed to flush span");
                    }
                });
            }
        }

        result
    }
}
