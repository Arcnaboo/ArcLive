//! WAV import for audio clips.

use crate::song::AudioClipData;
use crate::warp::WarpMode;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WavError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("wav: {0}")]
    Hound(#[from] hound::Error),
    #[error("unsupported wav format")]
    Unsupported,
}

/// Load a WAV file into an `AudioClipData` with default warp markers.
/// Length in beats is estimated from duration at `project_bpm` (default 120).
pub fn load_wav(path: &Path, project_bpm: f64) -> Result<AudioClipData, WavError> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let channels = spec.channels;
    if channels == 0 {
        return Err(WavError::Unsupported);
    }
    let sample_rate = spec.sample_rate as f32;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max = match spec.bits_per_sample {
                8 => i8::MAX as f32,
                16 => i16::MAX as f32,
                24 => 8388607.0,
                32 => i32::MAX as f32,
                _ => return Err(WavError::Unsupported),
            };
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()?,
    };

    let frames = samples.len() as f64 / f64::from(channels);
    let seconds = frames / f64::from(sample_rate).max(1.0);
    let bpm = project_bpm.max(1.0);
    let length_beats = seconds * bpm / 60.0;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Sample")
        .to_string();

    let mut clip = AudioClipData {
        id: crate::id::ClipId::new(),
        name,
        length_beats,
        samples,
        channels,
        sample_rate,
        asset_path: Some(path.to_string_lossy().into_owned()),
        missing_asset: false,
        warp_enabled: true,
        warp_mode: WarpMode::Beats,
        warp_markers: Vec::new(),
        transpose_semitones: 0,
        detune_cents: 0,
        loop_on: true,
        loop_start_beats: 0.0,
        loop_end_beats: length_beats,
        clip_start_beats: 0.0,
        clip_end_beats: length_beats,
    };
    clip.init_default_markers();
    Ok(clip)
}