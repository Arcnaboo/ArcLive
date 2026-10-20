use arclive_core::{
    ArrangementClip, ClipSlot, DeviceSpec, MixerChannel, QuantizeGrid, Song, TimeSignature,
    TrackKind, WarpMarker, WarpMode,
};
use crate::warp::detect_onsets;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ClipPlayState {
    pub slot_index: usize,
    /// Fractional source frame (warp-off / neutral path).
    pub sample_pos: f64,
    /// Musical playhead inside the clip (beats).
    pub clip_beat: f64,
    /// Absolute sample time when playback should start (quantized).
    pub start_at_sample: u64,
    pub pending: bool,
}

#[derive(Clone, Debug)]
pub struct TrackRuntime {
    pub name: String,
    pub kind: TrackKind,
    pub mixer: MixerChannel,
    pub devices: Vec<DeviceSpec>,
    pub slots: Vec<SlotAudio>,
    pub playing: Option<ClipPlayState>,
}

#[derive(Clone, Debug)]
pub struct SlotAudio {
    pub name: String,
    pub samples: Arc<[f32]>,
    pub channels: u16,
    pub sample_rate: f32,
    pub length_beats: f64,
    pub is_midi: bool,
    /// For MIDI: simple square at note freqs rendered offline into samples when published.
    pub midi_notes: Vec<(u8, f64, f64)>, // pitch, start_beats, length_beats
    pub warp_enabled: bool,
    pub warp_mode: WarpMode,
    pub warp_markers: Arc<[WarpMarker]>,
    pub transpose_semitones: i8,
    pub detune_cents: i16,
    pub loop_on: bool,
    pub loop_start_beats: f64,
    pub loop_end_beats: f64,
    pub clip_tempo_bpm: f64,
    pub onsets: Arc<[u64]>,
}

#[derive(Clone, Debug)]
pub struct ArrClipRuntime {
    pub track_index: usize,
    pub name: String,
    pub start_beats: f64,
    pub length_beats: f64,
    pub samples: Arc<[f32]>,
    pub channels: u16,
    pub sample_rate: f32,
    pub warp_enabled: bool,
    pub warp_mode: WarpMode,
    pub warp_markers: Arc<[WarpMarker]>,
    pub transpose_semitones: i8,
    pub detune_cents: i16,
    pub clip_tempo_bpm: f64,
    pub onsets: Arc<[u64]>,
}

#[derive(Clone, Debug)]
pub struct PublishedGraph {
    pub tracks: Vec<TrackRuntime>,
    pub arrangement: Vec<ArrClipRuntime>,
    pub time_signature: TimeSignature,
    pub launch_quantize: QuantizeGrid,
    pub scene_count: usize,
}

impl PublishedGraph {
    pub fn from_song(song: &Song) -> Self {
        let tracks = song
            .tracks
            .iter()
            .map(|t| TrackRuntime {
                name: t.name.clone(),
                kind: t.kind,
                mixer: t.mixer.clone(),
                devices: t.devices.clone(),
                slots: t.slots.iter().map(slot_from).collect(),
                playing: t.playing_slot.map(|slot_index| ClipPlayState {
                    slot_index,
                    sample_pos: 0.0,
                    clip_beat: 0.0,
                    start_at_sample: 0,
                    pending: false,
                }),
            })
            .collect();

        let arrangement = song
            .arrangement
            .iter()
            .filter_map(|c| arr_from(c, song))
            .collect();

        Self {
            tracks,
            arrangement,
            time_signature: song.time_signature,
            launch_quantize: song.launch_quantize,
            scene_count: song.scenes.len(),
        }
    }
}

fn slot_from(slot: &ClipSlot) -> SlotAudio {
    match slot {
        ClipSlot::Empty => SlotAudio {
            name: String::new(),
            samples: Arc::from([]),
            channels: 2,
            sample_rate: 48_000.0,
            length_beats: 0.0,
            is_midi: false,
            midi_notes: Vec::new(),
            warp_enabled: false,
            warp_mode: WarpMode::Beats,
            warp_markers: Arc::from([]),
            transpose_semitones: 0,
            detune_cents: 0,
            loop_on: false,
            loop_start_beats: 0.0,
            loop_end_beats: 0.0,
            clip_tempo_bpm: 120.0,
            onsets: Arc::from([]),
        },
        ClipSlot::Audio(c) => {
            let onsets = detect_onsets(&c.samples, c.channels, c.sample_rate);
            SlotAudio {
                name: c.name.clone(),
                samples: Arc::from(c.samples.as_slice()),
                channels: c.channels,
                sample_rate: c.sample_rate,
                length_beats: c.length_beats,
                is_midi: false,
                midi_notes: Vec::new(),
                warp_enabled: c.warp_enabled,
                warp_mode: c.warp_mode,
                warp_markers: Arc::from(c.warp_markers.as_slice()),
                transpose_semitones: c.transpose_semitones,
                detune_cents: c.detune_cents,
                loop_on: c.loop_on,
                loop_start_beats: c.loop_start_beats,
                loop_end_beats: c.loop_end_beats,
                clip_tempo_bpm: c.clip_tempo_bpm(),
                onsets: Arc::from(onsets.as_slice()),
            }
        },
        ClipSlot::Midi(c) => {
            let notes = c
                .notes
                .iter()
                .map(|n| (n.pitch, n.start_beats, n.length_beats))
                .collect();
            // Pre-render a simple beep sequence for audition without a synth plugin.
            let samples = render_midi_preview(&c.notes, c.length_beats, 48_000.0, 120.0);
            SlotAudio {
                name: c.name.clone(),
                samples: Arc::from(samples.as_slice()),
                channels: 2,
                sample_rate: 48_000.0,
                length_beats: c.length_beats,
                is_midi: true,
                midi_notes: notes,
                warp_enabled: false,
                warp_mode: WarpMode::Beats,
                warp_markers: Arc::from([]),
                transpose_semitones: 0,
                detune_cents: 0,
                loop_on: false,
                loop_start_beats: 0.0,
                loop_end_beats: c.length_beats,
                clip_tempo_bpm: 120.0,
                onsets: Arc::from([]),
            }
        }
    }
}

fn arr_from(c: &ArrangementClip, song: &Song) -> Option<ArrClipRuntime> {
    let track_index = song.tracks.iter().position(|t| t.id == c.track_id)?;
    let onsets = detect_onsets(&c.samples, c.channels, c.sample_rate);
    // Arrangement clips reuse audio fields; warp defaults until ArrangementClip gains them.
    Some(ArrClipRuntime {
        track_index,
        name: c.name.clone(),
        start_beats: c.start_beats,
        length_beats: c.length_beats,
        samples: Arc::from(c.samples.as_slice()),
        channels: c.channels,
        sample_rate: c.sample_rate,
        warp_enabled: false,
        warp_mode: WarpMode::Beats,
        warp_markers: Arc::from([]),
        transpose_semitones: 0,
        detune_cents: 0,
        clip_tempo_bpm: 120.0,
        onsets: Arc::from(onsets.as_slice()),
    })
}

fn render_midi_preview(
    notes: &[arclive_core::MidiNote],
    length_beats: f64,
    sample_rate: f32,
    bpm: f64,
) -> Vec<f32> {
    let seconds = length_beats * 60.0 / bpm.max(1.0);
    let frames = (seconds * f64::from(sample_rate)) as usize;
    let mut out = vec![0.0_f32; frames * 2];
    for n in notes {
        let start = (n.start_beats * 60.0 / bpm * f64::from(sample_rate)) as usize;
        let len = (n.length_beats * 60.0 / bpm * f64::from(sample_rate)) as usize;
        let freq = 440.0 * 2f32.powf((i32::from(n.pitch) - 69) as f32 / 12.0);
        let vel = f32::from(n.velocity) / 127.0 * 0.15;
        for i in 0..len {
            let idx = start + i;
            if idx >= frames {
                break;
            }
            let t = i as f32 / sample_rate;
            let s = (t * freq * std::f32::consts::TAU).sin() * vel;
            out[idx * 2] += s;
            out[idx * 2 + 1] += s;
        }
    }
    out
}
