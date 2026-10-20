use crate::time::MusicalTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransportState {
    #[default]
    Stopped,
    Playing,
    Recording,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transport {
    pub state: TransportState,
    /// Absolute sample position of the playhead.
    pub sample_position: u64,
    pub bpm: f64,
    pub loop_enabled: bool,
    pub loop_start_beats: f64,
    pub loop_end_beats: f64,
    pub metronome: bool,
    pub record_armed: bool,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            state: TransportState::Stopped,
            sample_position: 0,
            bpm: 120.0,
            loop_enabled: false,
            loop_start_beats: 0.0,
            loop_end_beats: 16.0,
            metronome: false,
            record_armed: false,
        }
    }
}

impl Transport {
    pub fn play(&mut self) {
        self.state = if self.record_armed {
            TransportState::Recording
        } else {
            TransportState::Playing
        };
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
    }

    pub fn toggle_play_stop(&mut self) {
        match self.state {
            TransportState::Stopped => self.play(),
            TransportState::Playing | TransportState::Recording => self.stop(),
        }
    }

    pub fn seek_samples(&mut self, sample: u64) {
        self.sample_position = sample;
    }

    pub fn seek_beats(&mut self, beats: f64, sample_rate: f64) {
        self.sample_position = MusicalTime::new(beats).to_samples(sample_rate, self.bpm);
    }

    pub fn arm_record(&mut self, armed: bool) {
        self.record_armed = armed;
        if !armed && self.state == TransportState::Recording {
            self.state = TransportState::Playing;
        }
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(20.0, 999.0);
    }

    pub fn is_rolling(&self) -> bool {
        matches!(
            self.state,
            TransportState::Playing | TransportState::Recording
        )
    }

    pub fn musical_time(&self, sample_rate: f64) -> MusicalTime {
        MusicalTime::from_samples(self.sample_position, sample_rate, self.bpm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_stop_seek() {
        let mut t = Transport::default();
        assert_eq!(t.state, TransportState::Stopped);
        t.play();
        assert_eq!(t.state, TransportState::Playing);
        t.seek_samples(48000);
        assert_eq!(t.sample_position, 48000);
        t.stop();
        assert_eq!(t.state, TransportState::Stopped);
        assert_eq!(t.sample_position, 48000);
    }

    #[test]
    fn record_arm_enters_recording_on_play() {
        let mut t = Transport::default();
        t.arm_record(true);
        t.play();
        assert_eq!(t.state, TransportState::Recording);
        t.arm_record(false);
        assert_eq!(t.state, TransportState::Playing);
    }

    #[test]
    fn toggle_play_stop() {
        let mut t = Transport::default();
        t.toggle_play_stop();
        assert!(t.is_rolling());
        t.toggle_play_stop();
        assert!(!t.is_rolling());
    }
}
