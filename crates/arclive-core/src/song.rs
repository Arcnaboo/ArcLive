use crate::id::{ClipId, SceneId, TrackId};
use crate::time::{QuantizeGrid, TimeSignature};
use crate::transport::Transport;
use crate::warp::{clip_bpm_from_length, WarpMarker, WarpMode};
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}
fn default_returns() -> usize {
    2
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkspaceView {
    #[default]
    Session,
    Arrangement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Audio,
    Midi,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MixerChannel {
    pub gain_db: f32,
    /// -1.0 left … 1.0 right
    pub pan: f32,
    /// Live Track Activator: false silences the track (like mute).
    #[serde(default = "default_true")]
    pub activator: bool,
    pub solo: bool,
    #[serde(default)]
    pub armed: bool,
    #[serde(default)]
    pub send_a: f32,
    #[serde(default)]
    pub send_b: f32,
    /// 0 = none, 1 = A, 2 = B
    #[serde(default)]
    pub xfade_assign: u8,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            pan: 0.0,
            activator: true,
            solo: false,
            armed: false,
            send_a: 0.0,
            send_b: 0.0,
            xfade_assign: 0,
        }
    }
}

impl MixerChannel {
    /// Live Activator off == muted.
    pub fn is_audible(&self) -> bool {
        self.activator
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    Gain,
    Filter,
    Eq,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceSpec {
    pub kind: DeviceKind,
    pub bypass: bool,
    /// Normalized 0..1 parameter slots (gain amount, cutoff, etc.).
    pub params: [f32; 4],
}

impl DeviceSpec {
    pub fn gain() -> Self {
        Self {
            kind: DeviceKind::Gain,
            bypass: false,
            params: [0.75, 0.0, 0.0, 0.0],
        }
    }

    pub fn filter() -> Self {
        Self {
            kind: DeviceKind::Filter,
            bypass: false,
            params: [0.7, 0.5, 0.0, 0.0],
        }
    }

    pub fn eq_device() -> Self {
        Self {
            kind: DeviceKind::Eq,
            bypass: false,
            params: [0.5, 0.5, 0.5, 0.0],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioClipData {
    pub id: ClipId,
    pub name: String,
    /// Length in beats for session display / arrangement.
    pub length_beats: f64,
    /// In-memory PCM (mono interleaved as mono; stereo interleaved LRLR).
    /// Not serialized in project JSON (assets referenced separately).
    #[serde(skip)]
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: f32,
    /// Optional asset path relative to project root.
    pub asset_path: Option<String>,
    pub missing_asset: bool,
    pub warp_enabled: bool,
    pub warp_mode: WarpMode,
    pub warp_markers: Vec<WarpMarker>,
    pub transpose_semitones: i8,
    pub detune_cents: i16,
    pub loop_on: bool,
    pub loop_start_beats: f64,
    pub loop_end_beats: f64,
    pub clip_start_beats: f64,
    pub clip_end_beats: f64,
}

impl AudioClipData {
    pub fn silence(name: impl Into<String>, length_beats: f64, sample_rate: f32) -> Self {
        Self {
            id: ClipId::new(),
            name: name.into(),
            length_beats,
            samples: Vec::new(),
            channels: 2,
            sample_rate,
            asset_path: None,
            missing_asset: false,
            warp_enabled: false,
            warp_mode: WarpMode::Beats,
            warp_markers: vec![WarpMarker::new(0, 0.0)],
            transpose_semitones: 0,
            detune_cents: 0,
            loop_on: false,
            loop_start_beats: 0.0,
            loop_end_beats: length_beats,
            clip_start_beats: 0.0,
            clip_end_beats: length_beats,
        }
    }

    pub fn test_tone(
        name: impl Into<String>,
        length_beats: f64,
        sample_rate: f32,
        bpm: f64,
        freq_hz: f32,
    ) -> Self {
        let seconds = length_beats * 60.0 / bpm.max(1.0);
        let frames = (seconds * f64::from(sample_rate)) as usize;
        let mut samples = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let t = i as f32 / sample_rate;
            let s = (t * freq_hz * std::f32::consts::TAU).sin() * 0.2;
            samples.push(s);
            samples.push(s);
        }
        let mut clip = Self {
            id: ClipId::new(),
            name: name.into(),
            length_beats,
            samples,
            channels: 2,
            sample_rate,
            asset_path: None,
            missing_asset: false,
            warp_enabled: true,
            warp_mode: WarpMode::Beats,
            warp_markers: Vec::new(),
            transpose_semitones: 0,
            detune_cents: 0,
            loop_on: false,
            loop_start_beats: 0.0,
            loop_end_beats: length_beats,
            clip_start_beats: 0.0,
            clip_end_beats: length_beats,
        };
        clip.init_default_markers();
        clip
    }

    pub fn frame_count(&self) -> u64 {
        let ch = u64::from(self.channels.max(1));
        (self.samples.len() as u64) / ch
    }

    pub fn init_default_markers(&mut self) {
        let frames = self.frame_count();
        let end_beat = self.length_beats.max(0.001);
        self.warp_markers = vec![
            WarpMarker::new(0, 0.0),
            WarpMarker::new(frames, end_beat),
        ];
        self.clip_end_beats = end_beat;
        self.loop_end_beats = end_beat;
    }

    pub fn segment_bpm_at(&self, marker_index: usize) -> f64 {
        if self.warp_markers.len() < 2 {
            return clip_bpm_from_length(
                self.samples.len() as u64,
                self.channels,
                self.length_beats,
                self.sample_rate,
            );
        }
        let idx = marker_index.min(self.warp_markers.len() - 1).max(1);
        crate::warp::segment_bpm(
            self.warp_markers[idx - 1],
            self.warp_markers[idx],
            self.sample_rate,
        )
    }

    pub fn clip_tempo_bpm(&self) -> f64 {
        if self.warp_markers.len() >= 2 {
            let a = self.warp_markers[0];
            let b = *self.warp_markers.last().unwrap();
            return crate::warp::segment_bpm(a, b, self.sample_rate);
        }
        clip_bpm_from_length(
            self.samples.len() as u64,
            self.channels,
            self.length_beats,
            self.sample_rate,
        )
    }

    pub fn sort_markers(&mut self) {
        self.warp_markers.sort_by(|a, b| {
            a.beat
                .partial_cmp(&b.beat)
                .unwrap()
                .then(a.sample_frame.cmp(&b.sample_frame))
        });
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiNote {
    pub pitch: u8,
    pub velocity: u8,
    pub start_beats: f64,
    pub length_beats: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MidiClipData {
    pub id: ClipId,
    pub name: String,
    pub length_beats: f64,
    pub notes: Vec<MidiNote>,
}

impl MidiClipData {
    pub fn empty(name: impl Into<String>, length_beats: f64) -> Self {
        Self {
            id: ClipId::new(),
            name: name.into(),
            length_beats,
            notes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClipSlot {
    Empty,
    Audio(AudioClipData),
    Midi(MidiClipData),
}

impl ClipSlot {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::Audio(c) => &c.name,
            Self::Midi(c) => &c.name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub kind: TrackKind,
    pub mixer: MixerChannel,
    pub devices: Vec<DeviceSpec>,
    /// Session clip slots (one per scene column index).
    pub slots: Vec<ClipSlot>,
    /// Index of currently playing session slot, if any.
    pub playing_slot: Option<usize>,
}

impl Track {
    pub fn audio(name: impl Into<String>, scenes: usize) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::Audio,
            mixer: MixerChannel::default(),
            devices: vec![DeviceSpec::gain()],
            slots: (0..scenes).map(|_| ClipSlot::Empty).collect(),
            playing_slot: None,
        }
    }

    pub fn midi(name: impl Into<String>, scenes: usize) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            kind: TrackKind::Midi,
            mixer: MixerChannel::default(),
            devices: vec![DeviceSpec::gain()],
            slots: (0..scenes).map(|_| ClipSlot::Empty).collect(),
            playing_slot: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub id: SceneId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArrangementClip {
    pub id: ClipId,
    pub track_id: TrackId,
    pub name: String,
    pub start_beats: f64,
    pub length_beats: f64,
    /// Embedded audio for engine (skipped in serde for project assets).
    #[serde(skip)]
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: f32,
    pub asset_path: Option<String>,
    pub missing_asset: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub name: String,
    pub time_signature: TimeSignature,
    pub transport: Transport,
    pub tracks: Vec<Track>,
    pub scenes: Vec<Scene>,
    pub arrangement: Vec<ArrangementClip>,
    pub launch_quantize: QuantizeGrid,
    pub workspace: WorkspaceView,
    /// When true, arrangement playhead drives audio; else session clips.
    pub arrangement_mode: bool,
    /// Global Groove amount 0..1 (Live Control Bar).
    #[serde(default)]
    pub global_groove: f64,
    /// Number of return track stubs shown in Session.
    #[serde(default = "default_returns")]
    pub return_track_count: usize,
}

impl Default for Song {
    fn default() -> Self {
        Self::new_demo()
    }
}

impl Song {
    pub fn new_empty(name: impl Into<String>) -> Self {
        let scenes: Vec<Scene> = (1..=4)
            .map(|i| Scene {
                id: SceneId::new(),
                name: format!("Scene {i}"),
            })
            .collect();
        let n = scenes.len();
        Self {
            name: name.into(),
            time_signature: TimeSignature::default(),
            transport: Transport::default(),
            tracks: vec![
                Track::audio("Audio 1", n),
                Track::audio("Audio 2", n),
                Track::midi("MIDI 1", n),
            ],
            scenes,
            arrangement: Vec::new(),
            launch_quantize: QuantizeGrid::Bar,
            workspace: WorkspaceView::Session,
            arrangement_mode: false,
            global_groove: 0.0,
            return_track_count: 2,
        }
    }

    /// Demo song with tones in session slots for early milestones.
    pub fn new_demo() -> Self {
        let mut song = Self::new_empty("Untitled");
        let bpm = song.transport.bpm;
        let sr = 48_000.0_f32;
        if let Some(track) = song.tracks.get_mut(0) {
            track.slots[0] =
                ClipSlot::Audio(AudioClipData::test_tone("Tone A", 4.0, sr, bpm, 440.0));
            track.slots[1] =
                ClipSlot::Audio(AudioClipData::test_tone("Tone B", 4.0, sr, bpm, 554.37));
        }
        if let Some(track) = song.tracks.get_mut(1) {
            track.slots[0] = ClipSlot::Audio(AudioClipData::test_tone("Bass", 4.0, sr, bpm, 110.0));
        }
        if let Some(track) = song.tracks.get_mut(2) {
            let mut clip = MidiClipData::empty("Notes", 4.0);
            clip.notes = vec![
                MidiNote {
                    pitch: 60,
                    velocity: 100,
                    start_beats: 0.0,
                    length_beats: 0.5,
                },
                MidiNote {
                    pitch: 64,
                    velocity: 90,
                    start_beats: 1.0,
                    length_beats: 0.5,
                },
                MidiNote {
                    pitch: 67,
                    velocity: 90,
                    start_beats: 2.0,
                    length_beats: 1.0,
                },
            ];
            track.slots[0] = ClipSlot::Midi(clip);
        }
        // Arrangement clip on track 0
        if let Some(t0) = song.tracks.first() {
            let clip = ArrangementClip {
                id: ClipId::new(),
                track_id: t0.id,
                name: "Arr Clip".into(),
                start_beats: 0.0,
                length_beats: 8.0,
                samples: AudioClipData::test_tone("Arr", 8.0, sr, bpm, 330.0).samples,
                channels: 2,
                sample_rate: sr,
                asset_path: None,
                missing_asset: false,
            };
            song.arrangement.push(clip);
        }
        song
    }

    pub fn add_audio_track(&mut self) {
        let n = self.scenes.len();
        let idx = self
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Audio)
            .count()
            + 1;
        self.tracks.push(Track::audio(format!("Audio {idx}"), n));
    }

    pub fn remove_track(&mut self, id: TrackId) {
        self.tracks.retain(|t| t.id != id);
        self.arrangement.retain(|c| c.track_id != id);
    }

    pub fn any_solo(&self) -> bool {
        self.tracks.iter().any(|t| t.mixer.solo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_has_tracks_and_scenes() {
        let s = Song::new_demo();
        assert!(s.tracks.len() >= 2);
        assert!(!s.scenes.is_empty());
        assert!(!s.tracks[0].slots[0].is_empty());
    }

    #[test]
    fn add_remove_track() {
        let mut s = Song::new_empty("t");
        let n = s.tracks.len();
        s.add_audio_track();
        assert_eq!(s.tracks.len(), n + 1);
        let id = s.tracks.last().unwrap().id;
        s.remove_track(id);
        assert_eq!(s.tracks.len(), n);
    }
}
