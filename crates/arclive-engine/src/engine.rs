use crate::command::EngineCommand;
use crate::devices::process_device_chain;
use crate::graph::{ClipPlayState, PublishedGraph};
use crate::meters::MeterBank;
use crate::warp::{render_frame, WarpPlaybackParams};
use arc_swap::ArcSwap;
use arclive_core::{QuantizeGrid, TimeSignature, TransportState, samples_per_beat};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub sample_rate: f64,
    pub max_buffer_frames: usize,
    pub channels: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            max_buffer_frames: 2048,
            channels: 2,
        }
    }
}

pub struct Engine {
    pub config: EngineConfig,
    pub state: TransportState,
    pub sample_position: u64,
    pub bpm: f64,
    pub loop_enabled: bool,
    pub loop_start_beats: f64,
    pub loop_end_beats: f64,
    pub metronome: bool,
    pub arrangement_mode: bool,
    pub test_tone: bool,
    phase: f64,
    graph: Arc<ArcSwap<PublishedGraph>>,
    local_graph: Arc<PublishedGraph>,
    meters: MeterBank,
    command_rx: Consumer<EngineCommand>,
    scratch: Vec<f32>,
    time_signature: TimeSignature,
}

pub struct EngineHandle {
    pub command_tx: Producer<EngineCommand>,
    pub graph: Arc<ArcSwap<PublishedGraph>>,
    pub meters: MeterBank,
}

impl Engine {
    pub fn create(config: EngineConfig, graph: PublishedGraph) -> (Self, EngineHandle) {
        let (command_tx, command_rx) = RingBuffer::<EngineCommand>::new(256);
        let meters = MeterBank::new(graph.tracks.len().max(8));
        let graph_slot = Arc::new(ArcSwap::from_pointee(graph.clone()));
        let engine = Self {
            config: config.clone(),
            state: TransportState::Stopped,
            sample_position: 0,
            bpm: 120.0,
            loop_enabled: false,
            loop_start_beats: 0.0,
            loop_end_beats: 16.0,
            metronome: false,
            arrangement_mode: false,
            test_tone: true,
            phase: 0.0,
            graph: Arc::clone(&graph_slot),
            local_graph: Arc::new(graph),
            meters: meters.clone(),
            command_rx,
            scratch: vec![0.0; config.max_buffer_frames * config.channels],
            time_signature: TimeSignature::default(),
        };
        let handle = EngineHandle {
            command_tx,
            graph: graph_slot,
            meters,
        };
        (engine, handle)
    }

    pub fn meters(&self) -> &MeterBank {
        &self.meters
    }

    /// Real-time process entry. Interleaved f32 output.
    pub fn process(&mut self, output: &mut [f32], frames: usize, channels: usize) {
        debug_assert!(channels > 0);
        let needed = frames * channels;
        if output.len() < needed {
            return;
        }
        for s in &mut output[..needed] {
            *s = 0.0;
        }

        self.drain_commands();
        self.reload_graph_if_needed();

        let rolling = matches!(
            self.state,
            TransportState::Playing | TransportState::Recording
        );

        if !rolling {
            self.write_meters(output, frames, channels, &[]);
            self.meters
                .sample_position
                .store(self.sample_position, std::sync::atomic::Ordering::Relaxed);
            return;
        }

        let track_count = self.local_graph.tracks.len();
        let mut track_peaks = vec![0.0_f32; track_count];

        if self.arrangement_mode {
            self.render_arrangement(output, frames, channels, &mut track_peaks);
        } else {
            self.render_session(output, frames, channels, &mut track_peaks);
        }

        if self.test_tone && !self.any_session_playing() && !self.arrangement_mode {
            self.mix_test_tone(output, frames, channels);
        }

        if self.metronome {
            self.mix_metronome(output, frames, channels);
        }

        self.apply_loop(frames);
        self.sample_position = self.sample_position.saturating_add(frames as u64);

        self.write_meters(output, frames, channels, &track_peaks);
        self.meters
            .sample_position
            .store(self.sample_position, std::sync::atomic::Ordering::Relaxed);
    }

    fn drain_commands(&mut self) {
        while let Ok(cmd) = self.command_rx.pop() {
            self.apply_command(cmd);
        }
    }

    fn apply_command(&mut self, cmd: EngineCommand) {
        match cmd {
            EngineCommand::Play => self.state = TransportState::Playing,
            EngineCommand::Stop => {
                self.state = TransportState::Stopped;
                for t in &mut Arc::make_mut(&mut self.local_graph).tracks {
                    t.playing = None;
                }
            }
            EngineCommand::SeekSamples(s) => self.sample_position = s,
            EngineCommand::SetBpm(b) => self.bpm = b.clamp(20.0, 999.0),
            EngineCommand::SetMetronome(v) => self.metronome = v,
            EngineCommand::SetLoop {
                enabled,
                start_beats,
                end_beats,
            } => {
                self.loop_enabled = enabled;
                self.loop_start_beats = start_beats;
                self.loop_end_beats = end_beats.max(start_beats + 0.25);
            }
            EngineCommand::SetArrangementMode(v) => self.arrangement_mode = v,
            EngineCommand::LaunchClip {
                track_index,
                slot_index,
                quantize,
            } => self.launch_clip(track_index as usize, slot_index as usize, quantize),
            EngineCommand::StopClip { track_index } => {
                if let Some(t) = Arc::make_mut(&mut self.local_graph)
                    .tracks
                    .get_mut(track_index as usize)
                {
                    t.playing = None;
                }
            }
            EngineCommand::LaunchScene {
                scene_index,
                quantize,
            } => {
                let n = self.local_graph.tracks.len();
                for ti in 0..n {
                    self.launch_clip(ti, scene_index as usize, quantize);
                }
            }
            EngineCommand::StopAllClips => {
                for t in &mut Arc::make_mut(&mut self.local_graph).tracks {
                    t.playing = None;
                }
            }
            EngineCommand::SetGain {
                track_index,
                gain_db,
            } => {
                if let Some(t) = Arc::make_mut(&mut self.local_graph)
                    .tracks
                    .get_mut(track_index as usize)
                {
                    t.mixer.gain_db = gain_db;
                }
            }
            EngineCommand::SetPan { track_index, pan } => {
                if let Some(t) = Arc::make_mut(&mut self.local_graph)
                    .tracks
                    .get_mut(track_index as usize)
                {
                    t.mixer.pan = pan.clamp(-1.0, 1.0);
                }
            }
            EngineCommand::SetMute { track_index, mute } => {
                if let Some(t) = Arc::make_mut(&mut self.local_graph)
                    .tracks
                    .get_mut(track_index as usize)
                {
                    t.mixer.mute = mute;
                }
            }
            EngineCommand::SetSolo { track_index, solo } => {
                if let Some(t) = Arc::make_mut(&mut self.local_graph)
                    .tracks
                    .get_mut(track_index as usize)
                {
                    t.mixer.solo = solo;
                }
            }
            EngineCommand::SetTestTone(v) => self.test_tone = v,
            EngineCommand::GraphChanged => {
                self.local_graph = self.graph.load_full();
                self.time_signature = self.local_graph.time_signature;
            }
            EngineCommand::_Reserved(_) => {}
        }
    }

    fn reload_graph_if_needed(&mut self) {
        // Cheap pointer check: if ArcSwap published a new arc, refresh.
        let current = self.graph.load_full();
        if !Arc::ptr_eq(&current, &self.local_graph) {
            // Preserve playing state by track index where possible.
            let old = Arc::clone(&self.local_graph);
            let mut new_g = (*current).clone();
            for (i, t) in new_g.tracks.iter_mut().enumerate() {
                if let Some(old_t) = old.tracks.get(i) {
                    t.playing = old_t.playing.clone();
                    t.mixer = old_t.mixer.clone();
                }
            }
            self.local_graph = Arc::new(new_g);
            self.time_signature = self.local_graph.time_signature;
        }
    }

    fn launch_clip(&mut self, track_index: usize, slot_index: usize, quantize: QuantizeGrid) {
        let sig = self.time_signature;
        let spb = samples_per_beat(self.config.sample_rate, self.bpm);
        let beats = self.sample_position as f64 / spb;
        let q_beats = quantize.quantize_forward(beats, sig);
        let start_at = (q_beats * spb).round() as u64;

        let graph = Arc::make_mut(&mut self.local_graph);
        let Some(track) = graph.tracks.get_mut(track_index) else {
            return;
        };
        if slot_index >= track.slots.len() || track.slots[slot_index].samples.is_empty() {
            return;
        }
        let pending = start_at > self.sample_position;
        track.playing = Some(ClipPlayState {
            slot_index,
            sample_pos: 0.0,
                    clip_beat: 0.0,
            start_at_sample: start_at,
            pending,
        });
    }

    fn any_session_playing(&self) -> bool {
        self.local_graph.tracks.iter().any(|t| t.playing.is_some())
    }

    fn any_solo(&self) -> bool {
        self.local_graph.tracks.iter().any(|t| t.mixer.solo)
    }

    fn track_audible(&self, index: usize) -> bool {
        let Some(t) = self.local_graph.tracks.get(index) else {
            return false;
        };
        if t.mixer.mute {
            return false;
        }
        if self.any_solo() && !t.mixer.solo {
            return false;
        }
        true
    }

    fn render_session(
        &mut self,
        output: &mut [f32],
        frames: usize,
        channels: usize,
        track_peaks: &mut [f32],
    ) {
        let scratch_len = frames * channels;
        if self.scratch.len() < scratch_len {
            // Should be preallocated; grow only if misconfigured (avoid in RT ideally).
            self.scratch.resize(scratch_len, 0.0);
        }

        let track_count = self.local_graph.tracks.len();
        for ti in 0..track_count {
            if !self.track_audible(ti) {
                continue;
            }
            for s in &mut self.scratch[..scratch_len] {
                *s = 0.0;
            }

            // Clip borrow: extract play info then samples
            let play_info = self.local_graph.tracks[ti].playing.clone();
            let Some(mut play) = play_info else {
                continue;
            };

            let slot_index = play.slot_index;
            let samples = {
                let slot = &self.local_graph.tracks[ti].slots[slot_index];
                Arc::clone(&slot.samples)
            };
            let clip_ch = self.local_graph.tracks[ti].slots[slot_index].channels as usize;
            if samples.is_empty() {
                continue;
            }

            let slot_meta = {
                let slot = &self.local_graph.tracks[ti].slots[slot_index];
                (
                    slot.warp_enabled,
                    slot.warp_mode,
                    Arc::clone(&slot.warp_markers),
                    slot.transpose_semitones,
                    slot.detune_cents,
                    slot.clip_tempo_bpm,
                    Arc::clone(&slot.onsets),
                    slot.sample_rate,
                    slot.loop_on,
                    slot.loop_end_beats,
                )
            };

            for frame in 0..frames {
                let abs = self.sample_position + frame as u64;
                if play.pending {
                    if abs < play.start_at_sample {
                        continue;
                    }
                    play.pending = false;
                    play.sample_pos = 0.0;
                    play.clip_beat = 0.0;
                }
                let (
                    warp_enabled,
                    warp_mode,
                    ref markers,
                    transpose_semitones,
                    detune_cents,
                    clip_tempo_bpm,
                    ref onsets,
                    sample_rate,
                    loop_on,
                    loop_end_beats,
                ) = slot_meta;
                let params = WarpPlaybackParams {
                    samples: &samples,
                    channels: clip_ch as u16,
                    sample_rate,
                    markers,
                    warp_enabled,
                    warp_mode,
                    set_bpm: self.bpm,
                    clip_seg_bpm: clip_tempo_bpm,
                    transpose_semitones,
                    detune_cents,
                    onsets,
                };
                let (l, r) = render_frame(
                    &params,
                    &mut play.clip_beat,
                    &mut play.sample_pos,
                    self.config.sample_rate,
                );
                if loop_on && play.clip_beat >= loop_end_beats && loop_end_beats > 0.0 {
                    play.clip_beat = 0.0;
                    play.sample_pos = 0.0;
                }
                if channels >= 1 {
                    self.scratch[frame * channels] = l;
                }
                if channels >= 2 {
                    self.scratch[frame * channels + 1] = r;
                }
                for c in 2..channels {
                    self.scratch[frame * channels + c] = r;
                }
            }

            let devices = self.local_graph.tracks[ti].devices.clone();
            process_device_chain(&devices, &mut self.scratch[..scratch_len], channels);

            let gain = db_to_lin(self.local_graph.tracks[ti].mixer.gain_db);
            let pan = self.local_graph.tracks[ti].mixer.pan;
            let (gl, gr) = equal_power_pan(pan);
            let mut peak = 0.0_f32;
            for frame in 0..frames {
                let base = frame * channels;
                let l = self.scratch[base] * gain * gl;
                let r = if channels > 1 {
                    self.scratch[base + 1] * gain * gr
                } else {
                    l
                };
                output[base] += l;
                if channels > 1 {
                    output[base + 1] += r;
                }
                peak = peak.max(l.abs()).max(r.abs());
            }
            if let Some(p) = track_peaks.get_mut(ti) {
                *p = peak;
            }

            if let Some(t) = Arc::make_mut(&mut self.local_graph).tracks.get_mut(ti) {
                t.playing = Some(play);
            }
        }
    }

    fn render_arrangement(
        &mut self,
        output: &mut [f32],
        frames: usize,
        channels: usize,
        track_peaks: &mut [f32],
    ) {
        let spb = samples_per_beat(self.config.sample_rate, self.bpm);
        let scratch_len = frames * channels;
        if self.scratch.len() < scratch_len {
            self.scratch.resize(scratch_len, 0.0);
        }

        let clips: Vec<_> = self.local_graph.arrangement.clone();
        for clip in clips {
            if !self.track_audible(clip.track_index) {
                continue;
            }
            for s in &mut self.scratch[..scratch_len] {
                *s = 0.0;
            }
            let clip_ch = clip.channels as usize;
            let start_sample = (clip.start_beats * spb).round() as u64;
            let end_sample = ((clip.start_beats + clip.length_beats) * spb).round() as u64;

            for frame in 0..frames {
                let abs = self.sample_position + frame as u64;
                if abs < start_sample || abs >= end_sample {
                    continue;
                }
                let src_frame = (abs - start_sample) as usize;
                let src_i = src_frame * clip_ch;
                if src_i + clip_ch > clip.samples.len() {
                    continue;
                }
                for c in 0..channels {
                    let src_c = c.min(clip_ch.saturating_sub(1));
                    self.scratch[frame * channels + c] = clip.samples[src_i + src_c];
                }
            }

            let devices = self
                .local_graph
                .tracks
                .get(clip.track_index)
                .map(|t| t.devices.clone())
                .unwrap_or_default();
            process_device_chain(&devices, &mut self.scratch[..scratch_len], channels);

            let mixer = self
                .local_graph
                .tracks
                .get(clip.track_index)
                .map(|t| t.mixer.clone())
                .unwrap_or_default();
            let gain = db_to_lin(mixer.gain_db);
            let (gl, gr) = equal_power_pan(mixer.pan);
            let mut peak = 0.0_f32;
            for frame in 0..frames {
                let base = frame * channels;
                let l = self.scratch[base] * gain * gl;
                let r = if channels > 1 {
                    self.scratch[base + 1] * gain * gr
                } else {
                    l
                };
                output[base] += l;
                if channels > 1 {
                    output[base + 1] += r;
                }
                peak = peak.max(l.abs()).max(r.abs());
            }
            if let Some(p) = track_peaks.get_mut(clip.track_index) {
                *p = (*p).max(peak);
            }
        }
    }

    fn mix_test_tone(&mut self, output: &mut [f32], frames: usize, channels: usize) {
        let freq = 440.0;
        let sr = self.config.sample_rate;
        for frame in 0..frames {
            let s = (self.phase * std::f64::consts::TAU).sin() as f32 * 0.1;
            self.phase += freq / sr;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let base = frame * channels;
            output[base] += s;
            if channels > 1 {
                output[base + 1] += s;
            }
        }
    }

    fn mix_metronome(&mut self, output: &mut [f32], frames: usize, channels: usize) {
        let spb = samples_per_beat(self.config.sample_rate, self.bpm);
        let beats_per_bar = f64::from(self.time_signature.numerator);
        for frame in 0..frames {
            let abs = self.sample_position + frame as u64;
            let beat_f = abs as f64 / spb;
            let beat_i = beat_f.floor() as i64;
            let phase_in_beat = beat_f - beat_i as f64;
            if phase_in_beat < 0.02 {
                let bar_beat = beat_i.rem_euclid(beats_per_bar as i64);
                let amp = if bar_beat == 0 { 0.25 } else { 0.12 };
                let s = amp * (1.0 - phase_in_beat as f32 / 0.02);
                let base = frame * channels;
                output[base] += s;
                if channels > 1 {
                    output[base + 1] += s;
                }
            }
        }
    }

    fn apply_loop(&mut self, frames: usize) {
        if !self.loop_enabled {
            return;
        }
        let spb = samples_per_beat(self.config.sample_rate, self.bpm);
        let end = (self.loop_end_beats * spb).round() as u64;
        let start = (self.loop_start_beats * spb).round() as u64;
        let next = self.sample_position.saturating_add(frames as u64);
        if next >= end && end > start {
            self.sample_position = start;
        }
    }

    fn write_meters(&self, output: &[f32], frames: usize, channels: usize, track_peaks: &[f32]) {
        let mut ml = 0.0_f32;
        let mut mr = 0.0_f32;
        for frame in 0..frames {
            let base = frame * channels;
            ml = ml.max(output[base].abs());
            if channels > 1 {
                mr = mr.max(output[base + 1].abs());
            } else {
                mr = ml;
            }
        }
        MeterBank::store_f32(&self.meters.master_l, ml);
        MeterBank::store_f32(&self.meters.master_r, mr);
        for (i, p) in track_peaks.iter().enumerate() {
            if let Some(atom) = self.meters.track_peaks.get(i) {
                MeterBank::store_f32(atom, *p);
            }
        }
    }
}

fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

fn equal_power_pan(pan: f32) -> (f32, f32) {
    let p = ((pan + 1.0) * 0.5).clamp(0.0, 1.0);
    let angle = p * std::f32::consts::FRAC_PI_2;
    (angle.cos(), angle.sin())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PublishedGraph;
    use arclive_core::Song;

    fn eng() -> (Engine, EngineHandle) {
        let g = PublishedGraph::from_song(&Song::new_demo());
        Engine::create(EngineConfig::default(), g)
    }

    #[test]
    fn silence_when_stopped() {
        let (mut e, _) = eng();
        e.test_tone = false;
        let mut out = vec![1.0_f32; 256 * 2];
        e.process(&mut out, 256, 2);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_tone_when_playing() {
        let (mut e, mut h) = eng();
        e.test_tone = true;
        let _ = h.command_tx.push(EngineCommand::Play);
        let mut out = vec![0.0_f32; 512 * 2];
        e.process(&mut out, 512, 2);
        let peak = out.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(peak > 0.01, "peak={peak}");
    }

    #[test]
    fn seek_and_stop() {
        let (mut e, mut h) = eng();
        let _ = h.command_tx.push(EngineCommand::SeekSamples(1000));
        let _ = h.command_tx.push(EngineCommand::Play);
        e.process(&mut vec![0.0; 128], 64, 2);
        assert!(e.sample_position >= 1000);
        let _ = h.command_tx.push(EngineCommand::Stop);
        e.process(&mut vec![0.0; 128], 64, 2);
        assert_eq!(e.state, TransportState::Stopped);
    }

    #[test]
    fn launch_clip_produces_audio() {
        let (mut e, mut h) = eng();
        e.test_tone = false;
        let _ = h.command_tx.push(EngineCommand::Play);
        let _ = h.command_tx.push(EngineCommand::LaunchClip {
            track_index: 0,
            slot_index: 0,
            quantize: QuantizeGrid::None,
        });
        let mut out = vec![0.0_f32; 1024 * 2];
        e.process(&mut out, 1024, 2);
        let peak = out.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(peak > 0.01, "clip peak={peak}");
    }

    #[test]
    fn mute_silences_track() {
        let (mut e, mut h) = eng();
        e.test_tone = false;
        let _ = h.command_tx.push(EngineCommand::Play);
        let _ = h.command_tx.push(EngineCommand::LaunchClip {
            track_index: 0,
            slot_index: 0,
            quantize: QuantizeGrid::None,
        });
        let _ = h.command_tx.push(EngineCommand::SetMute {
            track_index: 0,
            mute: true,
        });
        let mut out = vec![0.0_f32; 512 * 2];
        e.process(&mut out, 512, 2);
        let peak = out.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(peak < 0.001, "muted peak={peak}");
    }
}
