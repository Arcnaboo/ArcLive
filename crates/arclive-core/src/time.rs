use serde::{Deserialize, Serialize};

/// Beats per bar / beat unit (e.g. 4/4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub numerator: u16,
    pub denominator: u16,
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }
}

/// Musical position in beats from song start (floating for sub-beat accuracy).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicalTime {
    pub beats: f64,
}

impl MusicalTime {
    pub const ZERO: Self = Self { beats: 0.0 };

    pub fn new(beats: f64) -> Self {
        Self { beats }
    }

    pub fn from_samples(samples: u64, sample_rate: f64, bpm: f64) -> Self {
        let spb = samples_per_beat(sample_rate, bpm);
        Self {
            beats: samples as f64 / spb,
        }
    }

    pub fn to_samples(self, sample_rate: f64, bpm: f64) -> u64 {
        let spb = samples_per_beat(sample_rate, bpm);
        (self.beats * spb).max(0.0).round() as u64
    }

    pub fn bar_beat_tick(self, sig: TimeSignature) -> (u32, u32, u32) {
        let beats_per_bar = f64::from(sig.numerator);
        let total = self.beats.max(0.0);
        let bar = (total / beats_per_bar).floor() as u32 + 1;
        let beat_in_bar = (total % beats_per_bar).floor() as u32 + 1;
        let frac = total.fract();
        let tick = (frac * 480.0).round() as u32;
        (bar, beat_in_bar, tick)
    }

    pub fn format_display(self, sig: TimeSignature) -> String {
        let (bar, beat, tick) = self.bar_beat_tick(sig);
        format!("{bar}.{beat}.{tick:03}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantizeGrid {
    Bar,
    Beat,
    Eighth,
    Sixteenth,
    None,
}

impl QuantizeGrid {
    pub fn beats(self, sig: TimeSignature) -> f64 {
        match self {
            Self::Bar => f64::from(sig.numerator),
            Self::Beat => 1.0,
            Self::Eighth => 0.5,
            Self::Sixteenth => 0.25,
            Self::None => 0.0,
        }
    }

    /// Next grid boundary at or after `beats`.
    pub fn quantize_forward(self, beats: f64, sig: TimeSignature) -> f64 {
        let grid = self.beats(sig);
        if grid <= 0.0 {
            return beats;
        }
        let n = (beats / grid).ceil();
        let q = n * grid;
        if (q - beats).abs() < 1e-9 { beats } else { q }
    }
}

pub fn samples_per_beat(sample_rate: f64, bpm: f64) -> f64 {
    let bpm = bpm.max(1.0);
    sample_rate * 60.0 / bpm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_samples() {
        let t = MusicalTime::new(4.0);
        let samples = t.to_samples(48_000.0, 120.0);
        let back = MusicalTime::from_samples(samples, 48_000.0, 120.0);
        assert!((back.beats - 4.0).abs() < 1e-6);
    }

    #[test]
    fn quantize_bar() {
        let sig = TimeSignature::default();
        let q = QuantizeGrid::Bar.quantize_forward(1.5, sig);
        assert!((q - 4.0).abs() < 1e-9);
    }
}
