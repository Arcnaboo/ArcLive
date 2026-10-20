use arclive_core::{QuantizeGrid, TrackId};

/// Copy commands for the lock-free ring into the audio thread.
#[derive(Clone, Copy, Debug)]
pub enum EngineCommand {
    Play,
    Stop,
    SeekSamples(u64),
    SetBpm(f64),
    SetMetronome(bool),
    SetLoop {
        enabled: bool,
        start_beats: f64,
        end_beats: f64,
    },
    SetArrangementMode(bool),
    /// Launch session slot; quantization applied on audio thread.
    LaunchClip {
        track_index: u16,
        slot_index: u16,
        quantize: QuantizeGrid,
    },
    StopClip {
        track_index: u16,
    },
    LaunchScene {
        scene_index: u16,
        quantize: QuantizeGrid,
    },
    StopAllClips,
    /// Clear Session overrides so tracks follow Arrangement again.
    BackToArrangement,
    SetArmed {
        track_index: u16,
        armed: bool,
    },
    SetActivator {
        track_index: u16,
        activator: bool,
    },
    /// Gain in dB for track index.
    SetGain {
        track_index: u16,
        gain_db: f32,
    },
    SetPan {
        track_index: u16,
        pan: f32,
    },
    SetMute {
        track_index: u16,
        mute: bool,
    },
    SetSolo {
        track_index: u16,
        solo: bool,
    },
    /// Enable test tone on master when playing and no clips (legacy/demo).
    SetTestTone(bool),
    /// Notify engine that a new graph was published (reload ArcSwap).
    GraphChanged,
    /// Dummy to keep TrackId visible for future typed commands.
    _Reserved(TrackId),
}
