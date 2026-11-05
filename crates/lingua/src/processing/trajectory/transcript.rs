//! A voice call as the caller heard it, for consumers that read messages instead of turns.

use super::{voice, Result, TrajectoryCollector, TrajectoryStream};
use crate::processing::import::{import_span, ImportValue, Span};
use crate::universal::Message;
use serde::de::IgnoredAny;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct VoiceMarkers {
    #[serde(rename = "audio.schema.version")]
    schema_version: Option<IgnoredAny>,
    #[serde(rename = "turn.id")]
    turn_id: Option<IgnoredAny>,
    #[serde(rename = "turn.reply_to")]
    reply_to: Option<IgnoredAny>,
}

/// Whether span metadata marks the span as part of a voice call. `voice_transcript`
/// needs only these spans.
pub fn is_voice_span(metadata: &impl ImportValue) -> bool {
    metadata.parse::<VoiceMarkers>().is_some_and(|markers| {
        markers.schema_version.is_some() || markers.turn_id.is_some() || markers.reply_to.is_some()
    })
}

/// What the caller heard on the voice calls among `spans`: for each turn, its request,
/// then its response as played. A cut-off response is cut off, and a turn where nothing
/// played has no response. Work, such as LLM text that never played, is left out.
/// `None` when no span is a supported voice session.
pub fn voice_transcript(spans: Vec<Span>) -> Result<Option<Vec<Message>>> {
    let spans = spans
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()?;
    if !spans.iter().any(|span| {
        matches!(
            &span.header.voice.schema_version,
            Some(Ok(version)) if voice::is_supported_version(version)
        )
    }) {
        return Ok(None);
    }
    let headers = spans.iter().map(|span| span.header_only()).collect();
    let mut stream = TrajectoryStream::new(headers, true)?;
    let mut by_id: HashMap<_, _> = spans
        .into_iter()
        .filter_map(|span| Some((span.header.id.clone()?, span)))
        .collect();
    let mut collector = TrajectoryCollector::default();
    loop {
        let ids = stream.pending_ids(usize::MAX);
        if ids.is_empty() {
            break;
        }
        for id in ids {
            let span = by_id
                .remove(&id)
                .ok_or_else(|| format!("Missing voice span {id}"))?;
            for event in stream.push(span)? {
                collector.push(event)?;
            }
        }
    }
    for event in stream.finish()? {
        collector.push(event)?;
    }
    let mut messages = Vec::new();
    for trajectory in collector.snapshot()? {
        for turn in trajectory.turns {
            messages.extend(turn.request.into_iter().flatten());
            if let Some(content) = turn.response.and_then(|response| response.response) {
                messages.push(Message::Assistant { content, id: None });
            }
        }
    }
    Ok(Some(messages))
}
