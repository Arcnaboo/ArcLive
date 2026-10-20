//! Original ArcLive warp DSP (Live 7–inspired public model).

use arclive_core::{beat_to_sample, is_neutral_playback, WarpMarker, WarpMode};

/// Linear interpolate interleaved PCM at fractional frame.
#[inline]
pub fn sample_at(samples: &[f32], channels: usize, frame: f64) -> (f32, f32) {
    if samples.is_empty() || channels == 0 {
        return (0.0, 0.0);
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return (0.0, 0.0);
    }
    let max_f = (frames - 1) as f64;
    let f = frame.clamp(0.0, max_f);
    let i0 = f.floor() as usize;
    let i1 = (i0 + 1).min(frames - 1);
    let frac = (f - i0 as f64) as f32;
    let l0 = samples[i0 * channels];
    let r0 = samples[i0 * channels + (channels - 1).min(1)];
    let l1 = samples[i1 * channels];
    let r1 = samples[i1 * channels + (channels - 1).min(1)];
    (l0 + (l1 - l0) * frac, r0 + (r1 - r0) * frac)
}

pub fn pitch_ratio(transpose_semitones: i8, detune_cents: i16) -> f64 {
    let semis = f64::from(transpose_semitones) + f64::from(detune_cents) / 100.0;
    2f64.powf(semis / 12.0)
}

/// Offline onset frames (energy flux) for Beats WSOLA hop guidance.
pub fn detect_onsets(samples: &[f32], channels: u16, sample_rate: f32) -> Vec<u64> {
    let ch = channels.max(1) as usize;
    let frames = samples.len() / ch;
    if frames < 2 {
        return vec![0];
    }
    let hop = ((sample_rate * 0.01) as usize).max(64); // ~10ms
    let mut onsets = vec![0u64];
    let mut prev_e = 0.0f32;
    let mut i = 0usize;
    while i + hop < frames {
        let mut e = 0.0f32;
        for f in i..i + hop {
            let s = samples[f * ch];
            e += s * s;
        }
        e /= hop as f32;
        if e > prev_e * 2.5 && e > 1e-4 {
            let frame = i as u64;
            if onsets.last().copied().unwrap_or(0) + (hop as u64 / 2) < frame {
                onsets.push(frame);
            }
        }
        prev_e = e * 0.8 + prev_e * 0.2;
        i += hop;
    }
    onsets
}

pub struct WarpPlaybackParams<'a> {
    pub samples: &'a [f32],
    pub channels: u16,
    pub sample_rate: f32,
    pub markers: &'a [WarpMarker],
    pub warp_enabled: bool,
    pub warp_mode: WarpMode,
    pub set_bpm: f64,
    pub clip_seg_bpm: f64,
    pub transpose_semitones: i8,
    pub detune_cents: i16,
    pub onsets: &'a [u64],
}

/// Advance one output frame; returns (L, R) and updates playhead.
/// `clip_beat` is musical position in the clip; `sample_pos` is fractional source frame (warp-off).
pub fn render_frame(
    p: &WarpPlaybackParams<'_>,
    clip_beat: &mut f64,
    sample_pos: &mut f64,
    host_sr: f64,
) -> (f32, f32) {
    let ch = p.channels.max(1) as usize;
    let frames = (p.samples.len() / ch) as f64;
    let pitch = pitch_ratio(p.transpose_semitones, p.detune_cents);
    let mode = if p.warp_mode == WarpMode::Complex {
        WarpMode::Beats
    } else {
        p.warp_mode
    };

    let neutral = is_neutral_playback(
        p.warp_enabled,
        mode,
        p.set_bpm,
        p.clip_seg_bpm,
        p.transpose_semitones,
        p.detune_cents,
    );

    if !p.warp_enabled || neutral {
        // Sequential copy / simple varispeed for transpose only.
        let rate = if p.warp_enabled {
            1.0
        } else {
            // warp off: sample-rate match assumed; transpose via rate
            pitch
        };
        let (l, r) = sample_at(p.samples, ch, *sample_pos);
        *sample_pos += rate;
        if *sample_pos >= frames {
            *sample_pos = 0.0;
            *clip_beat = 0.0;
        }
        // keep beat roughly in sync for UI
        let spb = host_sr * 60.0 / p.set_bpm.max(1.0);
        *clip_beat += 1.0 / spb;
        return (l, r);
    }

    // Musical advance per host sample at set BPM.
    let beats_per_sample = p.set_bpm / (60.0 * host_sr);
    *clip_beat += beats_per_sample;

    match mode {
        WarpMode::RePitch => {
            let src = beat_to_sample(p.markers, *clip_beat) * pitch;
            let (l, r) = sample_at(p.samples, ch, src);
            *sample_pos = src;
            if src >= frames {
                *clip_beat = 0.0;
                *sample_pos = 0.0;
            }
            (l, r)
        }
        WarpMode::Beats | WarpMode::Tones | WarpMode::Texture | WarpMode::Complex => {
            // WSOLA-ish: read from mapped position with grain around nearest onset.
            let ideal = beat_to_sample(p.markers, *clip_beat);
            let grain = match mode {
                WarpMode::Tones => (p.sample_rate * 0.04) as f64,
                WarpMode::Texture => (p.sample_rate * 0.08) as f64,
                _ => (p.sample_rate * 0.02) as f64,
            };
            let src = snap_to_onset(ideal, p.onsets, grain);
            let (l, r) = sample_at(p.samples, ch, src);
            *sample_pos = src;
            if ideal >= frames {
                *clip_beat = 0.0;
                *sample_pos = 0.0;
            }
            (l, r)
        }
    }
}

fn snap_to_onset(ideal: f64, onsets: &[u64], grain: f64) -> f64 {
    if onsets.is_empty() {
        return ideal;
    }
    let mut best = onsets[0] as f64;
    let mut best_d = (ideal - best).abs();
    for &o in onsets {
        let d = (ideal - o as f64).abs();
        if d < best_d {
            best_d = d;
            best = o as f64;
        }
    }
    if best_d < grain {
        // blend toward onset for transient lock
        ideal * 0.35 + best * 0.65
    } else {
        ideal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arclive_core::WarpMarker;

    #[test]
    fn neutrality_matched_tempo_copies() {
        let sr = 48_000.0f32;
        let frames = 480;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            samples[i * 2] = (i as f32) / frames as f32;
            samples[i * 2 + 1] = samples[i * 2];
        }
        let markers = [
            WarpMarker::new(0, 0.0),
            WarpMarker::new(frames as u64, 1.0), // 480 frames @ 48k = 0.01s = 1 beat → 6000 BPM nonsense;
        ];
        // Use clip_seg_bpm == set_bpm for neutrality
        let p = WarpPlaybackParams {
            samples: &samples,
            channels: 2,
            sample_rate: sr,
            markers: &markers,
            warp_enabled: true,
            warp_mode: WarpMode::Beats,
            set_bpm: 120.0,
            clip_seg_bpm: 120.0,
            transpose_semitones: 0,
            detune_cents: 0,
            onsets: &[],
        };
        let mut beat = 0.0;
        let mut pos = 0.0;
        let (l0, _) = render_frame(&p, &mut beat, &mut pos, 48_000.0);
        assert!((l0 - 0.0).abs() < 1e-6);
        let (l1, _) = render_frame(&p, &mut beat, &mut pos, 48_000.0);
        assert!((l1 - samples[2]).abs() < 1e-5, "l1={l1}");
        assert!((pos - 2.0).abs() < 1e-6);
    }

    #[test]
    fn repitch_advances_by_marker_map() {
        let sr = 48_000.0f32;
        let frames = 48_000u64;
        let samples = vec![0.5f32; (frames as usize) * 2];
        let markers = [
            WarpMarker::new(0, 0.0),
            WarpMarker::new(frames, 4.0),
        ];
        let p = WarpPlaybackParams {
            samples: &samples,
            channels: 2,
            sample_rate: sr,
            markers: &markers,
            warp_enabled: true,
            warp_mode: WarpMode::RePitch,
            set_bpm: 240.0, // 2x
            clip_seg_bpm: 120.0,
            transpose_semitones: 0,
            detune_cents: 0,
            onsets: &[],
        };
        let mut beat = 0.0;
        let mut pos = 0.0;
        let mut max_beat: f64 = 0.0;
        for _ in 0..24_000 {
            let _ = render_frame(&p, &mut beat, &mut pos, 48_000.0);
            max_beat = max_beat.max(beat);
        }
        assert!(max_beat > 1.5, "max_beat={max_beat} beat={beat}");
        assert!(pos > 10_000.0, "pos={pos}");
    }
}