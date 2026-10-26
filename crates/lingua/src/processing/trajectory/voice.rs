use super::PreparedSpan;
use crate::processing::import::VoiceKind;
use crate::universal::trajectory::{AudioClip, Speaker, Turn, Utterance};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap};

pub(super) struct VoiceWarning {
    pub span_id: String,
    pub message: String,
}

pub(super) struct TurnSpeech {
    pub id: String,
    pub speech: Vec<Utterance>,
    /// A turn for an agent turn without model work, or for a session without turns.
    pub created: Option<Turn>,
}

/// Attaches speech to `turns` (start and ID, per root) for traces with voice spans.
/// Owners come from `turn.id`; `turn_of` maps rows to the turns that hold them.
pub(super) fn build<'a>(
    spans: impl IntoIterator<Item = &'a PreparedSpan>,
    turns: &BTreeMap<String, Vec<(DateTime<Utc>, String)>>,
    turn_of: &HashMap<&str, &str>,
) -> BTreeMap<String, (Vec<TurnSpeech>, Vec<VoiceWarning>)> {
    let mut roots = BTreeMap::<&str, Vec<&PreparedSpan>>::new();
    for span in spans {
        roots
            .entry(span.root_span_id.as_str())
            .or_default()
            .push(span);
    }
    roots
        .into_iter()
        .filter(|(_, spans)| spans.iter().any(|span| span.source.voice.kind.is_some()))
        .map(|(root, spans)| {
            let turns = turns.get(root).cloned().unwrap_or_default();
            (root.to_string(), build_root(root, spans, turns, turn_of))
        })
        .collect()
}

fn build_root(
    root: &str,
    mut spans: Vec<&PreparedSpan>,
    mut targets: Vec<(DateTime<Utc>, String)>,
    turn_of: &HashMap<&str, &str>,
) -> (Vec<TurnSpeech>, Vec<VoiceWarning>) {
    spans.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.source.exec_counter.cmp(&b.source.exec_counter))
            .then(a.id.cmp(&b.id))
    });
    let by_span_id: HashMap<&str, &PreparedSpan> = spans
        .iter()
        .map(|span| (span.span_id.as_str(), *span))
        .collect();
    let mut warnings = Vec::new();
    let mut recordings = HashMap::new();
    for span in &spans {
        for error in &span.source.voice.errors {
            warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message: error.clone(),
            });
        }
        for recording in &span.source.voice.recordings {
            if recording
                .state
                .as_deref()
                .is_some_and(|state| state != "ready")
            {
                continue;
            }
            match recording.attachment.as_ref().and_then(|attachment| {
                Some((attachment, by_span_id.get(attachment.span_id.as_str())?))
            }) {
                Some((attachment, row)) => {
                    recordings.insert(
                        (attachment.span_id.as_str(), recording.id.as_str()),
                        (row.id.as_str(), attachment.reference.as_str(), recording),
                    );
                }
                None => warnings.push(VoiceWarning {
                    span_id: span.id.clone(),
                    message: format!(
                        "Unsupported audio recording {}: no attachment in this trace",
                        recording.id
                    ),
                }),
            }
        }
    }
    let speaker_turn = |span: &PreparedSpan| {
        let turn = by_span_id.get(span.source.voice.turn_id.as_deref()?)?;
        match turn.source.voice.kind {
            Some(VoiceKind::Turn(speaker)) => Some((*turn, speaker)),
            _ => None,
        }
    };
    let mut agent_targets = HashMap::<&str, String>::new();
    for span in &spans {
        if let (Some(id), Some((turn, Speaker::Agent))) =
            (turn_of.get(span.id.as_str()), speaker_turn(span))
        {
            agent_targets
                .entry(turn.id.as_str())
                .or_insert_with(|| id.to_string());
        }
    }
    let mut created = HashMap::new();
    for span in &spans {
        if span.source.voice.kind == Some(VoiceKind::Turn(Speaker::Agent))
            && !agent_targets.contains_key(span.id.as_str())
        {
            agent_targets.insert(span.id.as_str(), span.id.clone());
            targets.push((span.start, span.id.clone()));
            created.insert(span.id.clone(), new_turn(span));
        }
    }
    if targets.is_empty() {
        if let Some(session) = by_span_id.get(root) {
            targets.push((session.start, session.id.clone()));
            created.insert(session.id.clone(), new_turn(session));
        }
    }
    targets.sort_by_key(|(start, _)| *start);

    let mut speech_by_turn = HashMap::<&str, Vec<&PreparedSpan>>::new();
    for span in &spans {
        if let (Some(VoiceKind::Speech(Speaker::Agent)), Some(turn_id)) =
            (span.source.voice.kind, span.source.voice.turn_id.as_deref())
        {
            speech_by_turn.entry(turn_id).or_default().push(span);
        }
    }
    let mut speech = BTreeMap::<&str, Vec<Utterance>>::new();
    for span in &spans {
        let Some(VoiceKind::Speech(speaker)) = span.source.voice.kind else {
            continue;
        };
        let mut warn = |message: String| {
            warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message,
            })
        };
        let owner = speaker_turn(span);
        if let (Some(turn_id), None) = (&span.source.voice.turn_id, owner) {
            warn(format!(
                "Unresolved voice turn.id {turn_id}: no voice turn has that span ID"
            ));
        }
        let target = match owner {
            Some((turn, Speaker::Agent)) => agent_targets.get(turn.id.as_str()),
            Some((turn, Speaker::User)) => targets
                .iter()
                .find(|(start, _)| *start >= turn.start)
                .or(targets.last())
                .map(|(_, id)| id),
            None if speaker == Speaker::User => targets
                .iter()
                .find(|(start, _)| *start >= span.start)
                .or(targets.last())
                .map(|(_, id)| id),
            None => targets
                .iter()
                .rev()
                .find(|(start, _)| *start <= span.start)
                .or(targets.first())
                .map(|(_, id)| id),
        };
        let Some(target) = target else {
            warn("Unattached speech: the trace has no turns and no root span".to_string());
            continue;
        };
        let mut audio = Vec::new();
        for selection in &span.source.voice.selections {
            let key = (
                selection.recording_span_id.as_deref().unwrap_or_default(),
                selection.recording_id.as_str(),
            );
            match recordings.get(&key) {
                Some((row_id, attachment_ref, recording)) => audio.push(AudioClip {
                    row_id: row_id.to_string(),
                    attachment_ref: attachment_ref.to_string(),
                    mime_type: recording.mime_type.clone(),
                    channel_index: selection.channel_index,
                    start_offset_ms: selection.start_offset_ms,
                    end_offset_ms: selection.end_offset_ms,
                }),
                None => warn(format!(
                    "Unresolved audio selection for recording {}: no ready recording in this trace",
                    selection.recording_id
                )),
            }
        }
        let text = match speech_text(span, speaker) {
            Ok(Some(text)) if !text.is_empty() => Some(text),
            Ok(_) => playback_text(span, &by_span_id, &speech_by_turn),
            Err(message) => {
                warn(message);
                None
            }
        };
        speech.entry(target).or_default().push(Utterance {
            id: span.id.clone(),
            speaker,
            start_time: span.start,
            end_time: span.source.end,
            text,
            interrupted: span.source.voice.interrupted,
            audio,
        });
    }
    let turns = speech
        .into_iter()
        .map(|(id, speech)| TurnSpeech {
            id: id.to_string(),
            speech,
            created: created.remove(id),
        })
        .collect();
    (turns, warnings)
}

fn new_turn(span: &PreparedSpan) -> Turn {
    Turn {
        request_id: span.id.clone(),
        response_id: None,
        request: None,
        opaque_request: Vec::new(),
        response: None,
        work: Vec::new(),
        model: None,
        params: None,
        start_time: span.start,
        end_time: span.source.end,
        interrupted: None,
        compaction: None,
        speech: None,
    }
}

// Only use the last playback transcript when each speech has a matching playback event.
fn playback_text(
    span: &PreparedSpan,
    by_span_id: &HashMap<&str, &PreparedSpan>,
    speech_by_turn: &HashMap<&str, Vec<&PreparedSpan>>,
) -> Option<String> {
    let turn_id = span.source.voice.turn_id.as_deref()?;
    let turn = by_span_id.get(turn_id)?;
    let speeches = speech_by_turn.get(turn_id)?;
    let playback = turn.source.voice.playbacks.last()?;
    if turn.source.voice.kind != Some(VoiceKind::Turn(Speaker::Agent))
        || speeches.last()?.id != span.id
        || speeches.len() != turn.source.voice.playbacks.len()
        || span.source.voice.interrupted != Some(true)
        || playback.interrupted != Some(true)
    {
        return None;
    }
    playback.transcript.clone()
}

/// The speech span's own recorded text: user input or agent output, never STT or model text.
fn speech_text(span: &PreparedSpan, speaker: Speaker) -> Result<Option<String>, String> {
    let texts: Vec<_> = match speaker {
        Speaker::User => span
            .input
            .iter()
            .filter_map(|input| match &input.message {
                Message::User { content } => Some(user_text(content)),
                _ => None,
            })
            .collect(),
        Speaker::Agent => span
            .output
            .iter()
            .filter_map(|message| match message {
                Message::Assistant { content, .. } => Some(assistant_text(content)),
                _ => None,
            })
            .collect(),
    };
    match texts.as_slice() {
        [] => Ok(None),
        [Some(text)] => Ok(Some(text.clone())),
        _ => Err("Unsupported speech payload: expected one text message".to_string()),
    }
}

fn user_text(content: &UserContent) -> Option<String> {
    match content {
        UserContent::String(text) => Some(text.clone()),
        UserContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                UserContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect(),
    }
}

fn assistant_text(content: &AssistantContent) -> Option<String> {
    match content {
        AssistantContent::String(text) => Some(text.clone()),
        AssistantContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                AssistantContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect(),
    }
}
