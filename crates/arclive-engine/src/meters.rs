use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Peak meters shared between audio and UI threads.
#[derive(Clone, Debug)]
pub struct MeterBank {
    /// Packed f32 bits for master L/R peaks.
    pub master_l: Arc<AtomicU32>,
    pub master_r: Arc<AtomicU32>,
    pub track_peaks: Arc<Vec<AtomicU32>>,
    pub xruns: Arc<AtomicU64>,
    pub sample_position: Arc<AtomicU64>,
    pub fault: Arc<AtomicU32>,
}

impl MeterBank {
    pub fn new(track_count: usize) -> Self {
        let mut peaks = Vec::with_capacity(track_count);
        for _ in 0..track_count {
            peaks.push(AtomicU32::new(0));
        }
        Self {
            master_l: Arc::new(AtomicU32::new(0)),
            master_r: Arc::new(AtomicU32::new(0)),
            track_peaks: Arc::new(peaks),
            xruns: Arc::new(AtomicU64::new(0)),
            sample_position: Arc::new(AtomicU64::new(0)),
            fault: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn store_f32(atom: &AtomicU32, v: f32) {
        atom.store(v.to_bits(), Ordering::Relaxed);
    }

    pub fn load_f32(atom: &AtomicU32) -> f32 {
        f32::from_bits(atom.load(Ordering::Relaxed))
    }

    pub fn master_peaks(&self) -> (f32, f32) {
        (
            Self::load_f32(&self.master_l),
            Self::load_f32(&self.master_r),
        )
    }

    pub fn track_peak(&self, index: usize) -> f32 {
        self.track_peaks
            .get(index)
            .map(Self::load_f32)
            .unwrap_or(0.0)
    }

    pub fn resize_tracks(&mut self, count: usize) {
        let mut peaks = Vec::with_capacity(count);
        for _ in 0..count {
            peaks.push(AtomicU32::new(0));
        }
        self.track_peaks = Arc::new(peaks);
    }
}
