use crate::theme::ArcLiveTheme;
use crate::widgets::{
    arrangement_view, meter_bar, piano_roll, sample_browser, session_columns, warp_mode_combo,
    waveform_view,
};
use arclive_core::{
    load_wav, ClipSlot, QuantizeGrid, Song, TransportState, WorkspaceView, WarpMode,
};
use egui::{RichText, Ui};
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct UiSnapshot {
    pub device_name: String,
    pub sample_rate: String,
    pub buffer_frames: String,
    pub latency_ms: String,
    pub xruns: u64,
    pub cpu_hint: String,
    pub master_l: f32,
    pub master_r: f32,
    pub track_peaks: Vec<f32>,
    pub engine_sample_pos: u64,
    pub status_message: String,
    pub midi_inputs: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum UiAction {
    Play,
    Stop,
    TogglePlayStop,
    SeekBeats(f64),
    SetBpm(f64),
    SetMetronome(bool),
    ArmRecord(bool),
    LaunchClip { track: usize, slot: usize },
    LaunchScene { scene: usize },
    StopAllClips,
    SetGain { track: usize, gain_db: f32 },
    SetPan { track: usize, pan: f32 },
    SetMute { track: usize, mute: bool },
    SetSolo { track: usize, solo: bool },
    SetArrangementMode(bool),
    SetLoop { enabled: bool, start: f64, end: f64 },
    AddAudioTrack,
    RemoveTrack(usize),
    SetWorkspace(WorkspaceView),
    SetQuantize(QuantizeGrid),
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    SelectBottomTab(BottomTab),
    MoveArrangementClip { index: usize, start_beats: f64 },
    Republish,
    ImportWavDialog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BottomTab {
    #[default]
    Clip,
    Sample,
    Devices,
    PianoRoll,
}

pub struct UiApp {
    pub song: Song,
    pub snapshot: UiSnapshot,
    pub actions: Vec<UiAction>,
    bottom_tab: BottomTab,
    browser_tab: usize,
    selected_track: usize,
    selected_slot: Option<(usize, usize)>,
    selected_marker: usize,
    selected_arr_clip: Option<usize>,
    arr_zoom: f32,
    arr_scroll: f32,
    show_browser: bool,
    show_side_mixer: bool,
    sample_files: Vec<String>,
    samples_dir: PathBuf,
}

impl Default for UiApp {
    fn default() -> Self {
        let samples_dir = PathBuf::from("samples");
        let mut app = Self {
            song: Song::new_demo(),
            snapshot: UiSnapshot::default(),
            actions: Vec::new(),
            bottom_tab: BottomTab::Sample,
            browser_tab: 0,
            selected_track: 0,
            selected_slot: Some((0, 0)),
            selected_marker: 1,
            selected_arr_clip: None,
            arr_zoom: 40.0,
            arr_scroll: 0.0,
            show_browser: true,
            show_side_mixer: false,
            sample_files: Vec::new(),
            samples_dir,
        };
        app.refresh_sample_list();
        app
    }
}

impl UiApp {
    pub fn new(song: Song) -> Self {
        Self {
            song,
            ..Self::default()
        }
    }

    pub fn selected_slot_or_default(&self) -> (usize, usize) {
        self.selected_slot.unwrap_or((self.selected_track, 0))
    }

    pub fn take_actions(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.actions)
    }

    pub fn sync_transport_from_engine(&mut self, sample_pos: u64, sample_rate: f64) {
        self.song.transport.sample_position = sample_pos;
        let _ = sample_rate;
    }

    pub fn refresh_sample_list(&mut self) {
        self.sample_files.clear();
        if let Ok(rd) = std::fs::read_dir(&self.samples_dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.eq_ignore_ascii_case("wav"))
                    .unwrap_or(false)
                {
                    self.sample_files.push(p.to_string_lossy().into_owned());
                }
            }
        }
        self.sample_files.sort();
    }

    pub fn import_wav_to_slot(&mut self, path: &str, track: usize, slot: usize) {
        match load_wav(std::path::Path::new(path), self.song.transport.bpm) {
            Ok(clip) => {
                if let Some(t) = self.song.tracks.get_mut(track) {
                    while t.slots.len() <= slot {
                        t.slots.push(ClipSlot::Empty);
                    }
                    t.slots[slot] = ClipSlot::Audio(clip);
                    self.selected_track = track;
                    self.selected_slot = Some((track, slot));
                    self.bottom_tab = BottomTab::Sample;
                    self.actions.push(UiAction::Republish);
                    self.snapshot.status_message = format!("Loaded {path}");
                }
            }
            Err(e) => {
                self.snapshot.status_message = format!("WAV error: {e}");
            }
        }
    }

    pub fn move_clip_slot(&mut self, from_t: usize, from_s: usize, to_t: usize, to_s: usize) {
        let clip = match self.song.tracks.get_mut(from_t).and_then(|t| t.slots.get_mut(from_s)) {
            Some(slot) => std::mem::replace(slot, ClipSlot::Empty),
            None => return,
        };
        if matches!(clip, ClipSlot::Empty) {
            return;
        }
        if let Some(t) = self.song.tracks.get_mut(to_t) {
            while t.slots.len() <= to_s {
                t.slots.push(ClipSlot::Empty);
            }
            t.slots[to_s] = clip;
        }
        self.actions.push(UiAction::Republish);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ArcLiveTheme::apply(&ctx);

        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && !ctx.egui_wants_keyboard_input() {
            self.actions.push(UiAction::TogglePlayStop);
            self.song.transport.toggle_play_stop();
        }

        // External file drop onto window
        let dropped: Vec<String> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.as_ref().map(|p| p.to_string_lossy().into_owned()))
                .collect()
        });
        for path in dropped {
            if path.to_ascii_lowercase().ends_with(".wav") {
                let (t, s) = self.selected_slot.unwrap_or((self.selected_track, 0));
                self.import_wav_to_slot(&path, t, s);
            }
        }

        self.menu_bar(ui);
        self.transport_bar(ui);

        egui::Panel::bottom("status")
            .exact_size(26.0)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}  |  {} Hz  |  {}  |  CPU {}",
                            self.snapshot.device_name,
                            self.snapshot.sample_rate,
                            self.snapshot.buffer_frames,
                            self.snapshot.cpu_hint
                        ))
                        .small()
                        .color(ArcLiveTheme::MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(&self.snapshot.status_message)
                                .small()
                                .color(ArcLiveTheme::MUTED),
                        );
                        meter_bar(ui, self.snapshot.master_r, 8.0, 14.0);
                        meter_bar(ui, self.snapshot.master_l, 8.0, 14.0);
                    });
                });
            });

        egui::Panel::bottom("clip_view")
            .resizable(true)
            .default_size(220.0)
            .show_inside(ui, |ui| {
                self.clip_view(ui);
            });

        if self.show_browser {
            egui::Panel::left("browser")
                .resizable(true)
                .default_size(180.0)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.browser_tab, 0, "Samples");
                        ui.selectable_value(&mut self.browser_tab, 1, "Devices");
                        ui.selectable_value(&mut self.browser_tab, 2, "Projects");
                    });
                    ui.separator();
                    match self.browser_tab {
                        0 => {
                            let mut import = false;
                            sample_browser(ui, &self.sample_files, &mut || import = true);
                            if import {
                                self.actions.push(UiAction::ImportWavDialog);
                            }
                        }
                        1 => {
                            ui.label(RichText::new("Device browser").color(ArcLiveTheme::MUTED));
                            ui.small("Gain / Filter / FX on tracks");
                        }
                        _ => {
                            ui.label(RichText::new("Projects").color(ArcLiveTheme::MUTED));
                            ui.small("Use File → Open / Save");
                        }
                    }
                });
        }

        if self.show_side_mixer {
            egui::Panel::right("side_mixer")
                .resizable(true)
                .default_size(120.0)
                .show_inside(ui, |ui| {
                    ui.label(RichText::new("Mixer").strong());
                    ui.small("(under-track mixer is primary)");
                    for (ti, track) in self.song.tracks.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.label(&track.name);
                            ui.add(egui::Slider::new(&mut track.mixer.gain_db, -48.0..=6.0));
                            let peak = self.snapshot.track_peaks.get(ti).copied().unwrap_or(0.0);
                            meter_bar(ui, peak, 16.0, 48.0);
                        });
                    }
                });
        }

        // Central workspace
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.song.workspace,
                WorkspaceView::Session,
                "Session",
            );
            ui.selectable_value(
                &mut self.song.workspace,
                WorkspaceView::Arrangement,
                "Arrangement",
            );
            if ui.button("+ Audio Track").clicked() {
                self.song.add_audio_track();
                self.actions.push(UiAction::AddAudioTrack);
                self.actions.push(UiAction::Republish);
            }
        });
        ui.separator();

        match self.song.workspace {
            WorkspaceView::Session => {
                let mut launches: Vec<(usize, usize)> = Vec::new();
                let mut scenes: Vec<usize> = Vec::new();
                let mut drops: Vec<(usize, usize, String)> = Vec::new();
                let mut moves: Vec<(usize, usize, usize, usize)> = Vec::new();
                let mut mixer_dirty = false;
                let peaks = self.snapshot.track_peaks.clone();
                session_columns(
                    ui,
                    &mut self.song,
                    &peaks,
                    &mut self.selected_track,
                    &mut self.selected_slot,
                    &mut |t, s| launches.push((t, s)),
                    &mut |s| scenes.push(s),
                    &mut |t, s, p| drops.push((t, s, p)),
                    &mut |a, b, c, d| moves.push((a, b, c, d)),
                    &mut mixer_dirty,
                );
                for (t, s) in launches {
                    self.song.tracks[t].playing_slot = Some(s);
                    self.actions.push(UiAction::LaunchClip { track: t, slot: s });
                }
                for s in scenes {
                    self.actions.push(UiAction::LaunchScene { scene: s });
                }
                for (t, s, p) in drops {
                    self.import_wav_to_slot(&p, t, s);
                }
                for (a, b, c, d) in moves {
                    self.move_clip_slot(a, b, c, d);
                }
                if mixer_dirty {
                    self.actions.push(UiAction::Republish);
                    for (ti, track) in self.song.tracks.iter().enumerate() {
                        self.actions.push(UiAction::SetGain { track: ti, gain_db: track.mixer.gain_db });
                        self.actions.push(UiAction::SetPan { track: ti, pan: track.mixer.pan });
                        self.actions.push(UiAction::SetMute { track: ti, mute: track.mixer.mute });
                        self.actions.push(UiAction::SetSolo { track: ti, solo: track.mixer.solo });
                    }
                }
            }
            WorkspaceView::Arrangement => {
                let mut sel = None;
                arrangement_view(
                    ui,
                    &self.song,
                    self.arr_zoom,
                    self.arr_scroll,
                    &mut |i| sel = Some(i),
                );
                if let Some(i) = sel {
                    self.selected_arr_clip = Some(i);
                }
                ui.horizontal(|ui| {
                    ui.label("Zoom");
                    ui.add(egui::Slider::new(&mut self.arr_zoom, 10.0..=120.0));
                    ui.label("Scroll");
                    ui.add(egui::Slider::new(&mut self.arr_scroll, 0.0..=64.0));
                    if let Some(idx) = self.selected_arr_clip {
                        if let Some(c) = self.song.arrangement.get_mut(idx) {
                            let mut start = c.start_beats as f32;
                            if ui
                                .add(egui::Slider::new(&mut start, 0.0..=64.0).text("Start"))
                                .changed()
                            {
                                c.start_beats = f64::from(start);
                                self.actions.push(UiAction::MoveArrangementClip {
                                    index: idx,
                                    start_beats: c.start_beats,
                                });
                            }
                        }
                    }
                });
            }
        }
    }

    fn clip_view(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Clip, "Clip");
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Sample, "Sample");
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Devices, "Devices");
            ui.selectable_value(&mut self.bottom_tab, BottomTab::PianoRoll, "Piano Roll");
        });
        ui.separator();

        match self.bottom_tab {
            BottomTab::Clip => {
                if let Some((ti, si)) = self.selected_slot {
                    if let Some(ClipSlot::Audio(c)) = self.song.tracks.get(ti).and_then(|t| t.slots.get(si))
                    {
                        ui.label(RichText::new(&c.name).strong());
                        ui.label(format!("Length: {:.2} beats", c.length_beats));
                        ui.label(RichText::new("Groove: — (stub)").color(ArcLiveTheme::MUTED));
                    } else {
                        ui.label("Select an audio clip slot");
                    }
                }
            }
            BottomTab::Sample => self.sample_tab(ui),
            BottomTab::Devices => self.device_chain(ui),
            BottomTab::PianoRoll => piano_roll(ui, &self.song, self.selected_track),
        }
    }

    fn sample_tab(&mut self, ui: &mut Ui) {
        let Some((ti, si)) = self.selected_slot else {
            ui.label("Select a clip slot in Session");
            return;
        };
        let Some(slot) = self.song.tracks.get_mut(ti).and_then(|t| t.slots.get_mut(si)) else {
            return;
        };
        let ClipSlot::Audio(clip) = slot else {
            ui.label("Audio clip required for Sample / Warp");
            return;
        };

        let mut changed = false;
        ui.horizontal(|ui| {
            let mut warp = clip.warp_enabled;
            if ui.checkbox(&mut warp, "Warp").changed() {
                clip.warp_enabled = warp;
                changed = true;
            }
            ui.label("Mode");
            let mut mode = clip.warp_mode;
            warp_mode_combo(ui, &mut mode);
            if mode != clip.warp_mode {
                clip.warp_mode = mode;
                changed = true;
                if mode == WarpMode::Complex {
                    self.snapshot.status_message =
                        "Complex is experimental (falls back to Beats)".into();
                }
            }
            let mut seg = clip.segment_bpm_at(self.selected_marker.max(1)) as f32;
            ui.label("Seg. BPM");
            if ui
                .add(egui::DragValue::new(&mut seg).speed(0.1).range(20.0..=999.0))
                .changed()
            {
                // Adjust selected marker sample to match seg bpm vs previous marker
                let idx = self.selected_marker.max(1).min(clip.warp_markers.len().saturating_sub(1));
                if idx > 0 && clip.warp_markers.len() > idx {
                    let prev = clip.warp_markers[idx - 1];
                    let beat_delta = (clip.warp_markers[idx].beat - prev.beat).abs().max(1e-9);
                    let seconds = beat_delta * 60.0 / f64::from(seg).max(1.0);
                    let frames = (seconds * f64::from(clip.sample_rate)).round() as u64;
                    clip.warp_markers[idx].sample_frame = prev.sample_frame.saturating_add(frames);
                    changed = true;
                }
            }
            ui.label("Transpose");
            let mut tr = i32::from(clip.transpose_semitones);
            if ui.add(egui::DragValue::new(&mut tr).range(-48..=48)).changed() {
                clip.transpose_semitones = tr as i8;
                changed = true;
            }
            ui.label("Detune");
            let mut det = i32::from(clip.detune_cents);
            if ui
                .add(egui::DragValue::new(&mut det).range(-50..=50))
                .changed()
            {
                clip.detune_cents = det as i16;
                changed = true;
            }
            let mut loop_on = clip.loop_on;
            if ui.checkbox(&mut loop_on, "Loop").changed() {
                clip.loop_on = loop_on;
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!("Start {:.2}", clip.clip_start_beats));
            ui.label(format!("End {:.2}", clip.clip_end_beats));
            ui.label(format!("Loop {:.2}–{:.2}", clip.loop_start_beats, clip.loop_end_beats));
            ui.label(
                RichText::new(format!("Clip tempo {:.1}", clip.clip_tempo_bpm()))
                    .color(ArcLiveTheme::MUTED),
            );
        });

        let before = clip.warp_markers.clone();
        waveform_view(ui, clip, &mut self.selected_marker, 120.0);
        if before != clip.warp_markers {
            changed = true;
        }

        if changed {
            self.actions.push(UiAction::Republish);
        }
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::Panel::top("menu").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.actions.push(UiAction::NewProject);
                        ui.close();
                    }
                    if ui.button("Open...").clicked() {
                        self.actions.push(UiAction::OpenProject);
                        ui.close();
                    }
                    if ui.button("Save").clicked() {
                        self.actions.push(UiAction::SaveProject);
                        ui.close();
                    }
                    if ui.button("Save As...").clicked() {
                        self.actions.push(UiAction::SaveProjectAs);
                        ui.close();
                    }
                    if ui.button("Import WAV...").clicked() {
                        self.actions.push(UiAction::ImportWavDialog);
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_browser, "Browser");
                    ui.checkbox(&mut self.show_side_mixer, "Side Mixer");
                });
                ui.menu_button("Help", |ui| {
                    ui.label("ArcLive — open-source DAW");
                    ui.label("Live 7–inspired layout; original warp DSP.");
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("ArcLive")
                            .strong()
                            .color(ArcLiveTheme::ACCENT),
                    );
                });
            });
        });
    }

    fn transport_bar(&mut self, ui: &mut Ui) {
        egui::Panel::top("transport")
            .exact_size(36.0)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let playing = self.song.transport.is_rolling();
                    if ui
                        .add(egui::Button::new(if playing { "Pause" } else { "Play" }).fill(
                            if playing {
                                ArcLiveTheme::ACCENT.linear_multiply(0.4)
                            } else {
                                ArcLiveTheme::PANEL_ALT
                            },
                        ))
                        .clicked()
                    {
                        self.actions.push(UiAction::TogglePlayStop);
                        self.song.transport.toggle_play_stop();
                    }
                    if ui.button("Stop").clicked() {
                        self.song.transport.stop();
                        self.actions.push(UiAction::Stop);
                        self.actions.push(UiAction::StopAllClips);
                        for t in &mut self.song.tracks {
                            t.playing_slot = None;
                        }
                    }
                    let rec_color = if self.song.transport.record_armed {
                        ArcLiveTheme::RECORD
                    } else {
                        ArcLiveTheme::PANEL_ALT
                    };
                    if ui.add(egui::Button::new("Rec").fill(rec_color)).clicked() {
                        let armed = !self.song.transport.record_armed;
                        self.song.transport.arm_record(armed);
                        self.actions.push(UiAction::ArmRecord(armed));
                    }

                    ui.separator();
                    ui.label(RichText::new("BPM").strong());
                    let mut bpm = self.song.transport.bpm as f32;
                    if ui
                        .add(egui::DragValue::new(&mut bpm).speed(0.1).range(20.0..=999.0))
                        .changed()
                    {
                        self.song.transport.bpm = f64::from(bpm);
                        self.actions.push(UiAction::SetBpm(self.song.transport.bpm));
                    }
                    let ts = self.song.time_signature;
                    ui.label(
                        RichText::new(format!("{}/{}", ts.numerator, ts.denominator)).strong(),
                    );

                    ui.separator();
                    let mut metro = self.song.transport.metronome;
                    if ui.checkbox(&mut metro, "Metro").changed() {
                        self.song.transport.metronome = metro;
                        self.actions.push(UiAction::SetMetronome(metro));
                    }
                    let mut arr = self.song.arrangement_mode;
                    if ui.checkbox(&mut arr, "Arr").changed() {
                        self.song.arrangement_mode = arr;
                        self.actions.push(UiAction::SetArrangementMode(arr));
                    }

                    ui.separator();
                    egui::ComboBox::from_id_salt("quantize")
                        .selected_text(format!("{:?}", self.song.launch_quantize))
                        .show_ui(ui, |ui| {
                            for q in [
                                QuantizeGrid::None,
                                QuantizeGrid::Bar,
                                QuantizeGrid::Beat,
                                QuantizeGrid::Eighth,
                                QuantizeGrid::Sixteenth,
                            ] {
                                if ui
                                    .selectable_label(self.song.launch_quantize == q, format!("{q:?}"))
                                    .clicked()
                                {
                                    self.song.launch_quantize = q;
                                    self.actions.push(UiAction::SetQuantize(q));
                                }
                            }
                        });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("CPU {}", self.snapshot.cpu_hint))
                                .small()
                                .color(ArcLiveTheme::MUTED),
                        );
                        let state = match self.song.transport.state {
                            TransportState::Playing => "Playing",
                            TransportState::Recording => "Recording",
                            TransportState::Stopped => "Stopped",
                        };
                        ui.label(RichText::new(state).small());
                    });
                });
            });
    }

    fn device_chain(&mut self, ui: &mut Ui) {
        let Some(track) = self.song.tracks.get_mut(self.selected_track) else {
            return;
        };
        ui.label(RichText::new(format!("Devices — {}", track.name)).strong());
        let mut dirty = false;
        for (di, dev) in track.devices.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{:?}", dev.kind));
                    if ui.checkbox(&mut dev.bypass, "Bypass").changed() {
                        dirty = true;
                    }
                });
                for (pi, p) in dev.params.iter_mut().enumerate() {
                    if ui
                        .add(egui::Slider::new(p, 0.0..=1.0).text(format!("P{pi}")))
                        .changed()
                    {
                        dirty = true;
                    }
                }
                let _ = di;
            });
        }
        if dirty {
            self.actions.push(UiAction::Republish);
        }
    }
}