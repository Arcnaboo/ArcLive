//! Warp markers and modes (Live 7–inspired public model; original ArcLive math).

use serde::{Deserialize, Serialize};

/// Time-stretch algorithm family. Complex is experimental (falls back to Beats).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WarpMode {
    #[default]
    Beats,
    Tones,
    Texture,
    RePitch,
    Complex,
}

impl WarpMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Beats => "Beats",
            Self::Tones => "Tones",
            Self::Texture => "Texture",
            Self::RePitch => "Re-Pitch",
            Self::Complex => "Complex",
        }
    }

    pub const ALL: [WarpMode; 5] = [
        Self::Beats,
        Self::Tones,
        Self::Texture,
        Self::RePitch,
        Self::Complex,
    ];
}

/// Locks a sample frame to a musical beat position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WarpMarker {
    pub sample_frame: u64,
    pub beat: f64,
}

impl WarpMarker {
    pub fn new(sample_frame: u64, beat: f64) -> Self {
        Self {
            sample_frame,
            beat,
        }
    }
}

/// Segment BPM between `prev` and `curr` given sample rate.
/// Seg.BPM = (sample_delta / sample_rate) * 60 / beat_delta
pub fn segment_bpm(prev: WarpMarker, curr: WarpMarker, sample_rate: f32) -> f64 {
    let sr = f64::from(sample_rate).max(1.0);
    let sample_delta = curr.sample_frame.saturating_sub(prev.sample_frame) as f64;
    let beat_delta = (curr.beat - prev.beat).abs().max(1e-9);
    let seconds = sample_delta / sr;
    if seconds <= 0.0 {
        return 120.0;
    }
    (beat_delta * 60.0) / seconds
}

/// Whole-clip BPM from a single marker + total frames / length in beats.
pub fn clip_bpm_from_length(frames: u64, channels: u16, length_beats: f64, sample_rate: f32) -> f64 {
    let ch = u64::from(channels.max(1));
    let frame_count = frames / ch;
    let seconds = frame_count as f64 / f64::from(sample_rate).max(1.0);
    let beats = length_beats.max(1e-9);
    if seconds <= 0.0 {
        return 120.0;
    }
    (beats / seconds) * 60.0
}

/// Map a musical beat to a sample frame via piecewise-linear markers.
pub fn beat_to_sample(markers: &[WarpMarker], beat: f64) -> f64 {
    if markers.is_empty() {
        return 0.0;
    }
    if markers.len() == 1 || beat <= markers[0].beat {
        let m = markers[0];
        // Extrapolate using implicit seg bpm from next marker if any
        if markers.len() >= 2 {
            let a = markers[0];
            let b = markers[1];
            let db = (b.beat - a.beat).max(1e-9);
            let ds = b.sample_frame as f64 - a.sample_frame as f64;
            return a.sample_frame as f64 + (beat - a.beat) * (ds / db);
        }
        return m.sample_frame as f64;
    }
    for w in markers.windows(2) {
        let a = w[0];
        let b = w[1];
        if beat <= b.beat {
            let db = (b.beat - a.beat).max(1e-9);
            let t = (beat - a.beat) / db;
            return a.sample_frame as f64
                + t * (b.sample_frame as f64 - a.sample_frame as f64);
        }
    }
    let last = *markers.last().unwrap();
    let prev = markers[markers.len() - 2];
    let db = (last.beat - prev.beat).max(1e-9);
    let ds = last.sample_frame as f64 - prev.sample_frame as f64;
    last.sample_frame as f64 + (beat - last.beat) * (ds / db)
}

/// Map a sample frame to a musical beat via markers.
pub fn sample_to_beat(markers: &[WarpMarker], sample_frame: f64) -> f64 {
    if markers.is_empty() {
        return 0.0;
    }
    if markers.len() == 1 || sample_frame <= markers[0].sample_frame as f64 {
        if markers.len() >= 2 {
            let a = markers[0];
            let b = markers[1];
            let ds = (b.sample_frame as f64 - a.sample_frame as f64).max(1e-9);
            let db = b.beat - a.beat;
            return a.beat + (sample_frame - a.sample_frame as f64) * (db / ds);
        }
        return markers[0].beat;
    }
    for w in markers.windows(2) {
        let a = w[0];
        let b = w[1];
        if sample_frame <= b.sample_frame as f64 {
            let ds = (b.sample_frame as f64 - a.sample_frame as f64).max(1e-9);
            let t = (sample_frame - a.sample_frame as f64) / ds;
            return a.beat + t * (b.beat - a.beat);
        }
    }
    let last = *markers.last().unwrap();
    let prev = markers[markers.len() - 2];
    let ds = (last.sample_frame as f64 - prev.sample_frame as f64).max(1e-9);
    let db = last.beat - prev.beat;
    last.beat + (sample_frame - last.sample_frame as f64) * (db / ds)
}

/// Whether stretch can be skipped (Live 7 “neutral” unstretched case, approx).
pub fn is_neutral_playback(
    warp_enabled: bool,
    mode: WarpMode,
    set_bpm: f64,
    clip_seg_bpm: f64,
    transpose_semitones: i8,
    detune_cents: i16,
) -> bool {
    if !warp_enabled {
        return transpose_semitones == 0 && detune_cents == 0;
    }
    if mode == WarpMode::Complex {
        return false;
    }
    if transpose_semitones != 0 || detune_cents != 0 {
        return false;
    }
    (set_bpm - clip_seg_bpm).abs() < 0.05
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_bpm_four_beats_one_second_at_48k() {
        let a = WarpMarker::new(0, 0.0);
        let b = WarpMarker::new(48_000, 4.0); // 1 second = 4 beats → 240 BPM
        let bpm = segment_bpm(a, b, 48_000.0);
        assert!((bpm - 240.0).abs() < 0.01, "bpm={bpm}");
    }

    #[test]
    fn beat_sample_roundtrip() {
        let markers = vec![
            WarpMarker::new(0, 0.0),
            WarpMarker::new(48_000, 4.0),
            WarpMarker::new(96_000, 8.0),
        ];
        let s = beat_to_sample(&markers, 2.0);
        assert!((s - 24_000.0).abs() < 1.0);
        let b = sample_to_beat(&markers, 24_000.0);
        assert!((b - 2.0).abs() < 1e-6);
    }

    #[test]
    fn neutrality_matched_tempo() {
        assert!(is_neutral_playback(
            true,
            WarpMode::Beats,
            120.0,
            120.0,
            0,
            0
        ));
        assert!(!is_neutral_playback(
            true,
            WarpMode::Complex,
            120.0,
            120.0,
            0,
            0
        ));
        assert!(!is_neutral_playback(
            true,
            WarpMode::Beats,
            140.0,
            120.0,
            0,
            0
        ));
    }
}
