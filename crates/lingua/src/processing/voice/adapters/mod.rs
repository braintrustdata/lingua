pub mod livekit;

use super::{Recording, Utterance, VoiceSpan};

pub trait Adapter {
    fn matches(&self, rows: &[VoiceSpan]) -> Result<bool, String>;
    fn recordings(&self, rows: &[VoiceSpan]) -> Result<Vec<Recording>, String>;
    fn utterances(&self, rows: &[VoiceSpan]) -> Result<Vec<Utterance>, String>;
}
