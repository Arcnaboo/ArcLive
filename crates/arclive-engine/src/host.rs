use crate::engine::{Engine, EngineConfig, EngineHandle};
use crate::graph::PublishedGraph;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HostError {
    #[error("no output device")]
    NoOutputDevice,
    #[error("cpal: {0}")]
    Cpal(String),
}

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Default)]
pub struct HostStatus {
    pub device_name: Option<String>,
    pub sample_rate: Option<f64>,
    pub buffer_frames: Option<u32>,
    pub channels: Option<u16>,
    pub running: bool,
    pub latency_ms: Option<f32>,
}

/// Owns the cpal stream. The `Engine` lives inside the audio callback closure
/// (exclusive owner). Control uses `handle` (rtrb + ArcSwap + atomics) only.
pub struct AudioHost {
    _stream: Option<Stream>,
    pub handle: EngineHandle,
    pub status: HostStatus,
}

impl AudioHost {
    pub fn enumerate_outputs() -> Result<Vec<DeviceInfo>, HostError> {
        let host = cpal::default_host();
        let default_name = host.default_output_device().map(|d| d.to_string());
        let mut out = Vec::new();
        let devices = host
            .output_devices()
            .map_err(|e| HostError::Cpal(e.to_string()))?;
        for d in devices {
            let name = d.to_string();
            let is_default = default_name.as_ref() == Some(&name);
            out.push(DeviceInfo { name, is_default });
        }
        Ok(out)
    }

    pub fn open_default(graph: PublishedGraph) -> Result<Self, HostError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(HostError::NoOutputDevice)?;
        Self::open_device(device, graph)
    }

    fn open_device(device: Device, graph: PublishedGraph) -> Result<Self, HostError> {
        let name = device.to_string();
        let supported = device
            .default_output_config()
            .map_err(|e| HostError::Cpal(e.to_string()))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.config();
        let sample_rate = f64::from(config.sample_rate);
        let channels = config.channels as usize;
        let buffer_hint = match config.buffer_size {
            cpal::BufferSize::Fixed(n) => n as usize,
            cpal::BufferSize::Default => 1024,
        };

        let eng_config = EngineConfig {
            sample_rate,
            max_buffer_frames: buffer_hint.max(4096),
            channels,
        };
        let (engine, handle) = Engine::create(eng_config, graph);
        let meters = handle.meters.clone();
        let fault = Arc::clone(&meters.fault);
        let xruns = Arc::clone(&meters.xruns);

        let err_fn = move |e: cpal::Error| {
            tracing::error!("audio stream error: {e}");
            fault.store(1, Ordering::Relaxed);
            xruns.fetch_add(1, Ordering::Relaxed);
        };

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(&device, &config, engine, err_fn)?,
            SampleFormat::I16 => build_stream::<i16>(&device, &config, engine, err_fn)?,
            SampleFormat::U16 => build_stream::<u16>(&device, &config, engine, err_fn)?,
            SampleFormat::I32 => build_stream::<i32>(&device, &config, engine, err_fn)?,
            SampleFormat::U32 => build_stream::<u32>(&device, &config, engine, err_fn)?,
            SampleFormat::F64 => build_stream::<f64>(&device, &config, engine, err_fn)?,
            SampleFormat::I8 => build_stream::<i8>(&device, &config, engine, err_fn)?,
            SampleFormat::U8 => build_stream::<u8>(&device, &config, engine, err_fn)?,
            SampleFormat::I64 => build_stream::<i64>(&device, &config, engine, err_fn)?,
            SampleFormat::U64 => build_stream::<u64>(&device, &config, engine, err_fn)?,
            other => {
                return Err(HostError::Cpal(format!(
                    "unsupported sample format {other:?}"
                )));
            }
        };
        stream.play().map_err(|e| HostError::Cpal(e.to_string()))?;

        let latency_ms = (buffer_hint as f32 / sample_rate as f32) * 1000.0;

        Ok(Self {
            _stream: Some(stream),
            handle,
            status: HostStatus {
                device_name: Some(name),
                sample_rate: Some(sample_rate),
                buffer_frames: Some(buffer_hint as u32),
                channels: Some(channels as u16),
                running: true,
                latency_ms: Some(latency_ms),
            },
        })
    }

    pub fn try_open(graph: PublishedGraph) -> Result<Self, HostError> {
        Self::open_default(graph)
    }

    pub fn offline(
        graph: PublishedGraph,
        config: EngineConfig,
    ) -> (Engine, EngineHandle, HostStatus) {
        let (engine, handle) = Engine::create(config.clone(), graph);
        let status = HostStatus {
            device_name: Some("Offline".into()),
            sample_rate: Some(config.sample_rate),
            buffer_frames: Some(config.max_buffer_frames as u32),
            channels: Some(config.channels as u16),
            running: false,
            latency_ms: None,
        };
        (engine, handle, status)
    }

    pub fn push(&mut self, cmd: crate::EngineCommand) -> bool {
        self.handle.command_tx.push(cmd).is_ok()
    }

    pub fn publish_graph(&self, graph: PublishedGraph) {
        self.handle.graph.store(Arc::new(graph));
    }
}

fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    mut engine: Engine,
    err_fn: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<Stream, HostError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let max_frames = engine.config.max_buffer_frames;
    let mut convert = vec![0.0_f32; max_frames * channels];

    let stream = device
        .build_output_stream(
            *config,
            move |data: &mut [T], _| {
                let frames = data.len() / channels;
                let need = frames * channels;
                if need > convert.len() {
                    for s in data.iter_mut() {
                        *s = T::from_sample(0.0);
                    }
                    engine.meters().xruns.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                engine.process(&mut convert[..need], frames, channels);
                for (i, sample) in convert[..need].iter().enumerate() {
                    data[i] = T::from_sample(*sample);
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| HostError::Cpal(e.to_string()))?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arclive_core::Song;

    #[test]
    fn enumerate_or_empty() {
        let _ = AudioHost::enumerate_outputs();
    }

    #[test]
    fn open_default_or_skip() {
        let g = PublishedGraph::from_song(&Song::new_demo());
        match AudioHost::try_open(g) {
            Ok(h) => assert!(h.status.device_name.is_some()),
            Err(HostError::NoOutputDevice) => {}
            Err(e) => panic!("unexpected: {e}"),
        }
    }
}
