use crate::serde_json as json;
use crate::universal::trajectory::{AudioChannel, AudioRecording, AudioSelection};
use serde::{Deserialize, Serialize};

/// Voice evidence logged in a span's metadata. Kept on the span header so a trajectory can place
/// speech, audio, and observed relationships before payloads load.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpanVoice {
    /// `metadata["audio.recordings"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recordings: Vec<AudioRecording>,
    /// `metadata["audio.selections"]`, or `metadata["audio.selection"]` when that is all there is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selections: Vec<AudioSelection>,
    /// `metadata.links`, in logged order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<SpanLink>,
    /// Native message ids, like `metadata["contrib.livekit.message_ids"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_ids: Vec<String>,
    /// Native response id of a model operation, like `metadata["contrib.livekit.response_id"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    /// Native participant kind, like `metadata["contrib.livekit.participant_kind"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant_kind: Option<String>,
    /// `metadata["turn.id"]`: the framework turn that owns this span.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    /// `metadata["turn.reply_to"]`: the user turn an agent turn answers, when the framework knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

impl SpanVoice {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// An observed relationship logged by the source span, like `generated_by` or `input_context`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpanLink {
    pub relationship: String,
    pub span_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// Input of a speech synthesis (`tts`) span: the text sent to TTS.
#[derive(Deserialize)]
struct SynthesisInput {
    text: String,
}

/// Text a `tts` span was asked to synthesize, from its `{"text": ...}` input.
pub(crate) fn import_synthesis_text(input: Option<&json::Value>) -> Option<String> {
    SynthesisInput::deserialize(input?)
        .ok()
        .map(|input| input.text)
}

#[derive(Deserialize)]
struct LoggedRecording {
    id: String,
    attachment: Option<LoggedAttachment>,
    recording_group_id: Option<String>,
    mime_type: Option<String>,
    duration_ms: Option<f64>,
    channel_count: Option<u32>,
    #[serde(default)]
    sources: Vec<LoggedSource>,
    timeline: Option<LoggedTimeline>,
    state: Option<String>,
    truncated: Option<bool>,
    reason: Option<String>,
}

#[derive(Deserialize)]
struct LoggedAttachment {
    #[serde(rename = "ref")]
    pointer: String,
    span_id: String,
}

#[derive(Deserialize)]
struct LoggedSource {
    boundary: String,
    channel_index: u32,
}

#[derive(Deserialize)]
struct LoggedTimeline {
    origin_unix_ms: Option<f64>,
    recording_start_offset_ms: Option<f64>,
}

#[derive(Deserialize)]
struct LoggedLink {
    #[serde(default)]
    attributes: LoggedLinkAttributes,
    context: LoggedLinkContext,
}

#[derive(Default, Deserialize)]
struct LoggedLinkAttributes {
    #[serde(rename = "braintrust.relationship.type")]
    relationship: Option<String>,
}

/// Voice keys of span metadata, each parsed on its own so one malformed key keeps the rest.
#[derive(Default, Deserialize)]
struct VoiceMetadata {
    #[serde(rename = "audio.recordings")]
    recordings: Option<json::Value>,
    #[serde(rename = "audio.selection")]
    selection: Option<json::Value>,
    #[serde(rename = "audio.selections")]
    selections: Option<json::Value>,
    links: Option<json::Value>,
    #[serde(rename = "contrib.livekit.message_ids")]
    message_ids: Option<json::Value>,
    #[serde(rename = "contrib.livekit.response_id")]
    response_id: Option<json::Value>,
    #[serde(rename = "contrib.livekit.participant_kind")]
    participant_kind: Option<json::Value>,
    #[serde(rename = "turn.id")]
    turn_id: Option<json::Value>,
    #[serde(rename = "turn.reply_to")]
    reply_to: Option<json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoggedLinkContext {
    span_id: String,
    trace_id: Option<String>,
}

fn field<T: serde::de::DeserializeOwned>(
    value: Option<json::Value>,
    key: &str,
    warnings: &mut Vec<String>,
) -> Option<T> {
    let value = value.filter(|value| !value.is_null())?;
    T::deserialize(value)
        .map_err(|error| warnings.push(format!("Invalid metadata.{key}: {error}")))
        .ok()
}

pub(crate) fn import_voice(
    metadata: Option<&json::Value>,
    warnings: &mut Vec<String>,
) -> Option<SpanVoice> {
    let metadata = VoiceMetadata::deserialize(metadata?).ok()?;
    let recordings =
        field::<Vec<LoggedRecording>>(metadata.recordings, "audio.recordings", warnings)
            .unwrap_or_default()
            .into_iter()
            .map(|recording| AudioRecording {
                id: recording.id,
                attachment_span_id: recording.attachment.as_ref().map(|a| a.span_id.clone()),
                attachment_pointer: recording.attachment.map(|a| a.pointer),
                group_id: recording.recording_group_id,
                mime_type: recording.mime_type,
                duration_ms: recording.duration_ms,
                channel_count: recording.channel_count,
                channels: recording
                    .sources
                    .into_iter()
                    .map(|source| AudioChannel {
                        index: source.channel_index,
                        boundary: source.boundary,
                    })
                    .collect(),
                // Range-mapped timelines have no single offset, so they are left unplaced.
                origin_unix_ms: recording.timeline.and_then(|timeline| {
                    Some(timeline.origin_unix_ms? + timeline.recording_start_offset_ms?)
                }),
                state: recording.state,
                truncated: recording.truncated,
                reason: recording.reason,
            })
            .collect();
    let selections =
        field::<Vec<AudioSelection>>(metadata.selections, "audio.selections", warnings)
            .filter(|selections| !selections.is_empty())
            .or_else(|| {
                field::<AudioSelection>(metadata.selection, "audio.selection", warnings)
                    .map(|one| vec![one])
            })
            .unwrap_or_default();
    let links = field::<Vec<LoggedLink>>(metadata.links, "links", warnings)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|link| {
            Some(SpanLink {
                relationship: link.attributes.relationship?,
                span_id: link.context.span_id,
                trace_id: link.context.trace_id,
            })
        })
        .collect();
    let voice = SpanVoice {
        recordings,
        selections,
        links,
        message_ids: field(
            metadata.message_ids,
            "contrib.livekit.message_ids",
            warnings,
        )
        .unwrap_or_default(),
        response_id: field(
            metadata.response_id,
            "contrib.livekit.response_id",
            warnings,
        ),
        participant_kind: field(
            metadata.participant_kind,
            "contrib.livekit.participant_kind",
            warnings,
        ),
        turn_id: field(metadata.turn_id, "turn.id", warnings),
        reply_to: field(metadata.reply_to, "turn.reply_to", warnings),
    };
    (!voice.is_empty()).then_some(voice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn reads_livekit_agent_speech() {
        let metadata = json!({
            "audio.selection": null,
            "audio.selections": [
                {"channel_index": 1, "end_offset_ms": 3252, "recording_id": "call-0000", "recording_span_id": "root", "start_offset_ms": 1952},
                {"channel_index": 1, "end_offset_ms": 3407, "recording_id": "call-0000", "recording_span_id": "root", "start_offset_ms": 3307}
            ],
            "contrib.livekit.message_ids": ["item_1"],
            "contrib.livekit.participant_kind": "AGENT",
            "links": [{"attributes": {"braintrust.relationship.type": "generated_by"}, "context": {"spanId": "llm", "traceFlags": 1, "traceId": "trace"}}]
        });
        let mut warnings = Vec::new();
        let voice = import_voice(Some(&metadata), &mut warnings).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(voice.selections.len(), 2);
        assert_eq!(voice.selections[1].start_offset_ms, 3307.0);
        assert_eq!(voice.message_ids, vec!["item_1"]);
        assert_eq!(voice.participant_kind.as_deref(), Some("AGENT"));
        assert_eq!(
            voice.links,
            vec![SpanLink {
                relationship: "generated_by".into(),
                span_id: "llm".into(),
                trace_id: Some("trace".into()),
            }]
        );
    }

    #[test]
    fn reads_recording_channels_and_clock() {
        let metadata = json!({"audio.recordings": [{
            "attachment": {"ref": "/input/audio/call-0000", "span_id": "root"},
            "channel_count": 2, "duration_ms": 20589, "id": "call-0000", "mime_type": "audio/ogg",
            "sources": [{"boundary": "session_input", "channel_index": 0}, {"boundary": "local_audio_output", "channel_index": 1}],
            "state": "ready",
            "timeline": {"basis": "local_input_arrival_and_output_playout", "origin_unix_ms": 1000.0, "recording_start_offset_ms": 250.0}
        }]});
        let voice = import_voice(Some(&metadata), &mut Vec::new()).unwrap();
        let recording = &voice.recordings[0];
        assert_eq!(
            recording.attachment_pointer.as_deref(),
            Some("/input/audio/call-0000")
        );
        assert_eq!(recording.origin_unix_ms, Some(1250.0));
        assert_eq!(recording.channels[1].boundary, "local_audio_output");
    }

    #[test]
    fn keeps_pending_recordings_without_attachments() {
        let metadata = json!({"audio.recordings": [
            {"id": "call", "state": "pending"},
            {"id": "clip", "state": "omitted", "reason": "consent_withheld",
             "timeline": {"origin_unix_ms": 5.0, "basis": "x", "ranges": []}}
        ]});
        let voice = import_voice(Some(&metadata), &mut Vec::new()).unwrap();
        assert_eq!(voice.recordings[0].state.as_deref(), Some("pending"));
        assert_eq!(voice.recordings[0].attachment_span_id, None);
        assert_eq!(
            voice.recordings[1].reason.as_deref(),
            Some("consent_withheld")
        );
        assert_eq!(voice.recordings[1].origin_unix_ms, None);
    }

    #[test]
    fn falls_back_to_single_selection_and_keeps_unknown_relationships() {
        let metadata = json!({
            "audio.selection": {"recording_id": "r", "recording_span_id": "root", "start_offset_ms": 1, "end_offset_ms": 2},
            "links": [
                {"attributes": {"braintrust.relationship.type": "contrib.acme.heard_by"}, "context": {"spanId": "x"}},
                {"attributes": {}, "context": {"spanId": "untyped"}}
            ],
            "turn.id": "UT1"
        });
        let voice = import_voice(Some(&metadata), &mut Vec::new()).unwrap();
        assert_eq!(voice.selections.len(), 1);
        assert_eq!(voice.selections[0].channel_index, None);
        assert_eq!(voice.links.len(), 1);
        assert_eq!(voice.links[0].relationship, "contrib.acme.heard_by");
        assert_eq!(voice.turn_id.as_deref(), Some("UT1"));
    }

    #[test]
    fn reports_malformed_voice_metadata_without_failing() {
        let metadata = json!({"audio.selections": "nope", "contrib.livekit.message_ids": ["m"]});
        let mut warnings = Vec::new();
        let voice = import_voice(Some(&metadata), &mut warnings).unwrap();
        assert_eq!(voice.message_ids, vec!["m"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("Invalid metadata.audio.selections"));
    }

    #[test]
    fn ignores_spans_without_voice_metadata() {
        assert_eq!(
            import_voice(Some(&json!({"model": "x"})), &mut Vec::new()),
            None
        );
        assert_eq!(import_voice(None, &mut Vec::new()), None);
    }

    fn fixture_voices(fixture: &str) -> Vec<(String, Option<SpanVoice>, Vec<String>)> {
        let fixture: json::Value = json::from_str(fixture).unwrap();
        fixture["spans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let span: crate::processing::import::Span = json::from_value(row.clone()).unwrap();
                let imported = crate::processing::import::import_span(span).unwrap();
                (
                    imported.header.name.clone().unwrap_or_default(),
                    imported.header.voice,
                    imported.warnings,
                )
            })
            .collect()
    }

    #[test]
    fn reads_every_voice_field_in_the_saved_livekit_trace() {
        let spans = fixture_voices(include_str!(
            "../trajectory/fixtures/voice/livekit-realtime-order-lookup.json"
        ));
        assert_eq!(spans.len(), 24);
        assert!(spans.iter().all(|(_, _, warnings)| warnings.is_empty()));
        let speech: Vec<_> = spans
            .iter()
            .filter(|(name, _, _)| name.ends_with("_speech"))
            .collect();
        assert_eq!(speech.len(), 7);
        assert!(speech.iter().all(|(_, voice, _)| {
            let voice = voice.as_ref().unwrap();
            !voice.selections.is_empty() && voice.message_ids.len() == 1
        }));
        let links: usize = spans
            .iter()
            .filter_map(|(_, voice, _)| voice.as_ref())
            .map(|voice| voice.links.len())
            .sum();
        assert_eq!(links, 18);
        let root = spans
            .iter()
            .find(|(name, _, _)| name == "livekit.agent_session")
            .and_then(|(_, voice, _)| voice.as_ref())
            .unwrap();
        assert_eq!(root.recordings[0].channels.len(), 2);
        assert_eq!(root.recordings[0].origin_unix_ms, Some(1792089428434.0));
    }

    #[test]
    fn reads_fictional_fixtures_without_warnings() {
        for fixture in [
            include_str!("../trajectory/fixtures/voice/fictional-cascade-turns.json"),
            include_str!("../trajectory/fixtures/voice/fictional-edge-cases.json"),
        ] {
            for (name, _, warnings) in fixture_voices(fixture) {
                assert!(warnings.is_empty(), "{name}: {warnings:?}");
            }
        }
    }
}
