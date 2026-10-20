use thiserror::Error;

#[derive(Debug, Error)]
pub enum MidiError {
    #[error("midi: {0}")]
    Init(String),
}

#[derive(Clone, Debug)]
pub struct MidiDeviceInfo {
    pub index: usize,
    pub name: String,
}

/// Placeholder MIDI hub.
///
/// `midir` is not linked yet: combining `midir` 0.9/0.10 with `cpal` 0.18
/// produces an `alsa-sys` `links` conflict in Cargo's multi-target resolver.
/// MIDI *clip playback* still works via pre-rendered session slots.
#[derive(Clone, Default)]
pub struct MidiInputHub;

impl MidiInputHub {
    pub fn list_inputs() -> Result<Vec<MidiDeviceInfo>, MidiError> {
        Ok(Vec::new())
    }

    pub fn open_first(&self) -> Result<Option<()>, MidiError> {
        Ok(None)
    }
}
