//! LiveKit agent sessions as logged by the Braintrust TypeScript SDK.

use super::span::Playback;
use super::span::{SpanContext, Speaker, VoiceKind};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub(super) struct PlaybackEvent {
    name: String,
    attributes: PlaybackAttributes,
}

#[derive(Deserialize)]
struct PlaybackAttributes {
    interrupted: Option<bool>,
    #[serde(rename = "synchronizedTranscript")]
    synchronized_transcript: Option<String>,
}

/// The `playbackFinished` events LiveKit records on an agent turn, in order.
pub(super) fn playbacks(events: Vec<PlaybackEvent>) -> Vec<Playback> {
    events
        .into_iter()
        .filter(|event| event.name == "playbackFinished")
        .map(|event| Playback {
            interrupted: event.attributes.interrupted,
            transcript: event.attributes.synchronized_transcript,
        })
        .collect()
}

// Only use the last playback transcript when each speech has a matching playback event.
pub(crate) fn adapt_trace(spans: &mut [&mut SpanContext]) {
    let mut speech_by_turn = HashMap::<String, Vec<usize>>::new();
    for (index, span) in spans.iter().enumerate() {
        if let (Some(VoiceKind::Speech(Speaker::Agent)), Some(turn_id)) =
            (span.voice.kind, &span.voice.turn_id)
        {
            speech_by_turn
                .entry(turn_id.clone())
                .or_default()
                .push(index);
        }
    }
    for turn in 0..spans.len() {
        if spans[turn].voice.kind != Some(VoiceKind::Turn(Speaker::Agent)) {
            continue;
        }
        let Some(speech) = spans[turn]
            .span_id
            .as_ref()
            .and_then(|id| speech_by_turn.get(id))
        else {
            continue;
        };
        let playbacks = &spans[turn].voice.playbacks;
        let (Some(&last), Some(playback)) = (
            speech.iter().max_by_key(|&&index| {
                (
                    spans[index].start,
                    spans[index].exec_counter,
                    spans[index].id.as_deref(),
                )
            }),
            playbacks.last(),
        ) else {
            continue;
        };
        if speech.len() == playbacks.len()
            && playback.interrupted == Some(true)
            && spans[last].voice.interrupted == Some(true)
        {
            spans[last].voice.fallback_text = playback.transcript.clone();
        }
    }
}

pub(super) fn voice_kind(kind: Option<&str>, name: Option<&str>) -> Option<VoiceKind> {
    if kind.is_some_and(|kind| kind != "task") {
        return None;
    }
    match name? {
        "user_turn" => Some(VoiceKind::Turn(Speaker::User)),
        "agent_turn" => Some(VoiceKind::Turn(Speaker::Agent)),
        "user_speech" => Some(VoiceKind::Speech(Speaker::User)),
        "agent_speech" => Some(VoiceKind::Speech(Speaker::Agent)),
        "stt" => Some(VoiceKind::Stt),
        "tts" => Some(VoiceKind::Tts),
        _ => None,
    }
}
