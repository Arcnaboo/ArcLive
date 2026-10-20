//! Domain types shared by the UI, engine, and project crates.
//! No audio I/O or GUI dependencies.

mod id;
mod song;
mod time;
mod transport;
mod warp;
mod wav;

pub use id::{ClipId, SceneId, TrackId};
pub use song::{
    ArrangementClip, AudioClipData, ClipSlot, DeviceKind, DeviceSpec, MidiClipData, MidiNote,
    MixerChannel, Scene, Song, Track, TrackKind, WorkspaceView,
};
pub use time::{MusicalTime, QuantizeGrid, TimeSignature, samples_per_beat};
pub use transport::{Transport, TransportState};

pub use warp::{beat_to_sample, is_neutral_playback, sample_to_beat, segment_bpm, WarpMarker, WarpMode};

pub use wav::{load_wav, WavError};
