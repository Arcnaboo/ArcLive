use crate::theme::ArcLiveTheme;
use crate::widgets::{
    arrangement_view, meter_bar, piano_roll, sample_browser, session_columns, warp_mode_combo,
    waveform_view, SessionViewFlags,
};
use arclive_core::{
    load_wav, ClipSlot, MusicalTime, QuantizeGrid, Song, TransportState, WorkspaceView,
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
    StopClip { track: usize },
    LaunchScene { scene: usize },
    StopAllClips,
    BackToArrangement,
    SetGain { track: usize, gain_db: f32 },
    SetPan { track: usize, pan: f32 },
    SetMute { track: usize, mute: bool },
    SetSolo { track: usize, solo: bool },
    SetArmed { track: usize, armed: bool },
    SetActivator { track: usize, activator: bool },
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
    MoveArrangementClip { index: usize, start_beats: f64 },
    Republish,
    ImportWavDialog,
    TempoNudge(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ClipViewBox {
    Clip,
    Launch,
    Envelopes,
    Sample,
    Notes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LaunchMode {
    #[default]
    Trigger,
    Gate,
    Toggle,
    Repeat,
}

pub struct UiApp {
    pub song: Song,
    pub snapshot: UiSnapshot,
    pub actions: Vec<UiAction>,
    show_box_clip: bool,
    show_box_launch: bool,
    show_box_envelopes: bool,
    show_box_sample: bool,
    browser_tab: usize,
    selected_track: usize,
    selected_slot: Option<(usize, usize)>,
    selected_marker: usize,
    selected_arr_clip: Option<usize>,
    arr_zoom: f32,
    arr_scroll: f32,
    show_browser: bool,
    view_flags: SessionViewFlags,
    sample_files: Vec<String>,
    samples_dir: PathBuf,
    browser_preview: bool,
    key_map_mode: bool,
    midi_map_mode: bool,
    stop_click_armed: bool,
    tap_times: Vec<f64>,
    launch_mode: LaunchMode,
    clip_quantize: QuantizeGrid,
}

impl Default for UiApp {
    fn default() -> Self {
        let samples_dir = PathBuf::from("samples");
        let mut app = Self {
            song: Song::new_demo(),
            snapshot: UiSnapshot::default(),
            actions: Vec::new(),
            show_box_clip: true,
            show_box_launch: true,
            show_box_envelopes: false,
            show_box_sample: true,
            browser_tab: 0,
            selected_track: 0,
            selected_slot: Some((0, 0)),
            selected_marker: 1,
            selected_arr_clip: None,
            arr_zoom: 40.0,
            arr_scroll: 0.0,
            show_browser: true,
            view_flags: SessionViewFlags::default(),
            sample_files: Vec::new(),
            samples_dir,
            browser_preview: false,
            key_map_mode: false,
            midi_map_mode: false,
            stop_click_armed: false,
            tap_times: Vec::new(),
            launch_mode: LaunchMode::Trigger,
            clip_quantize: QuantizeGrid::Bar,
        };
        app.refresh_sample_list();
        app
    }
}

impl UiApp {
    pub fn new(song: Song) -> Self {
        Self { song, ..Self::default() }
    }

    pub fn selected_slot_or_default(&self) -> (usize, usize) {
        self.selected_slot.unwrap_or((self.selected_track, 0))
    }

    pub fn take_actions(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.actions)
    }

    pub fn sync_transport_from_engine(&mut self, sample_pos: u64, _sample_rate: f64) {
        self.song.transport.sample_position = sample_pos;
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
                    self.show_box_sample = true;
                    self.actions.push(UiAction::Republish);
                    self.snapshot.status_message = format!("Loaded {path}");
                }
            }
            Err(e) => self.snapshot.status_message = format!("WAV error: {e}"),
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
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) && !ctx.egui_wants_keyboard_input() {
            self.song.workspace = match self.song.workspace {
                WorkspaceView::Session => WorkspaceView::Arrangement,
                WorkspaceView::Arrangement => WorkspaceView::Session,
            };
            self.actions
                .push(UiAction::SetWorkspace(self.song.workspace));
        }

        let dropped: Vec<String> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.as_ref().map(|p| p.to_string_lossy().into_owned()))
                .collect()
        });
        for path in dropped {
            if path.to_ascii_lowercase().ends_with(".wav") {
                let (t, s) = self.selected_slot_or_default();
                self.import_wav_to_slot(&path, t, s);
            }
        }

        self.menu_bar(ui);
        self.control_bar(ui);

        egui::Panel::bottom("status").exact_size(24.0).show_inside(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} | {} | buf {} | {}",
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
                    meter_bar(ui, self.snapshot.master_r, 8.0, 12.0);
                    meter_bar(ui, self.snapshot.master_l, 8.0, 12.0);
                });
            });
        });

        egui::Panel::bottom("clip_view")
            .resizable(true)
            .default_size(240.0)
            .show_inside(ui, |ui| {
                self.clip_view(ui);
            });

        if self.show_browser {
            egui::Panel::left("browser")
                .resizable(true)
                .default_size(170.0)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.browser_tab, 0, "Samples");
                        ui.selectable_value(&mut self.browser_tab, 1, "Devices");
                        ui.selectable_value(&mut self.browser_tab, 2, "Places");
                    });
                    ui.separator();
                    match self.browser_tab {
                        0 => {
                            let mut import = false;
                            sample_browser(
                                ui,
                                &self.sample_files,
                                &mut self.browser_preview,
                                &mut || import = true,
                            );
                            if import {
                                self.actions.push(UiAction::ImportWavDialog);
                            }
                        }
                        1 => {
                            ui.label(RichText::new("Categories").small().color(ArcLiveTheme::MUTED));
                            for name in ["Audio Effects", "MIDI Effects", "Instruments"] {
                                let _ = ui.selectable_label(false, name);
                            }
                        }
                        _ => {
                            ui.label(RichText::new("Places").small().color(ArcLiveTheme::MUTED));
                            ui.small("Current Project");
                            ui.small("User Library");
                        }
                    }
                });
        }

        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.song.workspace == WorkspaceView::Session, "Session")
                .clicked()
            {
                self.song.workspace = WorkspaceView::Session;
            }
            if ui
                .selectable_label(self.song.workspace == WorkspaceView::Arrangement, "Arrangement")
                .clicked()
            {
                self.song.workspace = WorkspaceView::Arrangement;
            }
            if ui.small_button("+ Audio").clicked() {
                self.song.add_audio_track();
                self.actions.push(UiAction::AddAudioTrack);
                self.actions.push(UiAction::Republish);
            }
            if self.key_map_mode {
                ui.colored_label(ArcLiveTheme::ACCENT, "KEY MAP");
            }
            if self.midi_map_mode {
                ui.colored_label(ArcLiveTheme::ACCENT, "MIDI MAP");
            }
        });
        ui.separator();

        match self.song.workspace {
            WorkspaceView::Session => self.session_view(ui),
            WorkspaceView::Arrangement => self.arrangement_panel(ui),
        }
    }

    fn session_view(&mut self, ui: &mut Ui) {
        let mut launches = Vec::new();
        let mut stops = Vec::new();
        let mut scenes = Vec::new();
        let mut drops = Vec::new();
        let mut moves = Vec::new();
        let mut mixer_dirty = false;
        let peaks = self.snapshot.track_peaks.clone();
        let flags = self.view_flags;
        session_columns(
            ui,
            &mut self.song,
            &peaks,
            &mut self.selected_track,
            &mut self.selected_slot,
            &flags,
            &mut |t, s| launches.push((t, s)),
            &mut |t| stops.push(t),
            &mut |s| scenes.push(s),
            &mut |t, s, p| drops.push((t, s, p)),
            &mut |a, b, c, d| moves.push((a, b, c, d)),
            &mut mixer_dirty,
        );
        for (t, s) in launches {
            self.song.tracks[t].playing_slot = Some(s);
            self.actions.push(UiAction::LaunchClip { track: t, slot: s });
        }
        for t in stops {
            if let Some(tr) = self.song.tracks.get_mut(t) {
                tr.playing_slot = None;
            }
            self.actions.push(UiAction::StopClip { track: t });
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
            for (ti, track) in self.song.tracks.iter().enumerate() {
                self.actions.push(UiAction::SetGain {
                    track: ti,
                    gain_db: track.mixer.gain_db,
                });
                self.actions.push(UiAction::SetPan {
                    track: ti,
                    pan: track.mixer.pan,
                });
                self.actions.push(UiAction::SetMute {
                    track: ti,
                    mute: !track.mixer.activator,
                });
                self.actions.push(UiAction::SetSolo {
                    track: ti,
                    solo: track.mixer.solo,
                });
                self.actions.push(UiAction::SetArmed {
                    track: ti,
                    armed: track.mixer.armed,
                });
                self.actions.push(UiAction::SetActivator {
                    track: ti,
                    activator: track.mixer.activator,
                });
            }
            self.actions.push(UiAction::Republish);
        }
    }

    fn arrangement_panel(&mut self, ui: &mut Ui) {
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
            ui.add(egui::Slider::new(&mut self.arr_zoom, 10.0..=120.0).text("Zoom"));
            ui.add(egui::Slider::new(&mut self.arr_scroll, 0.0..=64.0).text("Scroll"));
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
                    ui.checkbox(&mut self.view_flags.show_io, "In/Out");
                    ui.checkbox(&mut self.view_flags.show_sends, "Sends");
                    ui.checkbox(&mut self.view_flags.show_returns, "Returns");
                    ui.checkbox(&mut self.view_flags.show_mixer, "Mixer");
                    ui.checkbox(&mut self.view_flags.show_xfade, "Crossfader");
                    ui.separator();
                    ui.label(RichText::new("Clip View Boxes").small().color(ArcLiveTheme::MUTED));
                    ui.checkbox(&mut self.show_box_clip, "Clip");
                    ui.checkbox(&mut self.show_box_launch, "Launch");
                    ui.checkbox(&mut self.show_box_envelopes, "Envelopes");
                    ui.checkbox(&mut self.show_box_sample, "Sample / Notes");
                });
                ui.menu_button("Help", |ui| {
                    ui.label("ArcLive — Live 7–faithful layout");
                    ui.label("Not affiliated with Ableton AG.");
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("ArcLive").strong().color(ArcLiveTheme::ACCENT));
                });
            });
        });
    }

    fn control_bar(&mut self, ui: &mut Ui) {
        egui::Panel::top("control_bar")
            .exact_size(40.0)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let playing = self.song.transport.is_rolling();
                    if ui
                        .add(
                            egui::Button::new("Play").fill(if playing {
                                ArcLiveTheme::ACCENT.linear_multiply(0.5)
                            } else {
                                ArcLiveTheme::PANEL_ALT
                            }),
                        )
                        .clicked()
                    {
                        if !playing {
                            self.song.transport.play();
                            self.actions.push(UiAction::Play);
                        }
                        self.stop_click_armed = false;
                    }
                    if ui.button("Stop").clicked() {
                        if self.stop_click_armed {
                            // Double-stop: reset to 1.1.1
                            self.song.transport.stop();
                            self.song.transport.sample_position = 0;
                            self.actions.push(UiAction::Stop);
                            self.actions.push(UiAction::StopAllClips);
                            self.actions.push(UiAction::SeekBeats(0.0));
                            for t in &mut self.song.tracks {
                                t.playing_slot = None;
                            }
                            self.stop_click_armed = false;
                        } else {
                            self.song.transport.stop();
                            self.actions.push(UiAction::Stop);
                            self.actions.push(UiAction::StopAllClips);
                            for t in &mut self.song.tracks {
                                t.playing_slot = None;
                            }
                            self.stop_click_armed = true;
                        }
                    }
                    let rec = if self.song.transport.record_armed {
                        ArcLiveTheme::RECORD
                    } else {
                        ArcLiveTheme::PANEL_ALT
                    };
                    if ui.add(egui::Button::new("Rec").fill(rec)).clicked() {
                        let armed = !self.song.transport.record_armed;
                        self.song.transport.arm_record(armed);
                        self.actions.push(UiAction::ArmRecord(armed));
                    }

                    ui.separator();
                    let bta = if self.song.arrangement_mode {
                        ArcLiveTheme::PANEL_ALT
                    } else {
                        ArcLiveTheme::ACCENT.linear_multiply(0.4)
                    };
                    if ui
                        .add(egui::Button::new("Arr").fill(bta))
                        .on_hover_text("Back to Arrangement")
                        .clicked()
                    {
                        self.song.arrangement_mode = true;
                        for t in &mut self.song.tracks {
                            t.playing_slot = None;
                        }
                        self.actions.push(UiAction::BackToArrangement);
                        self.actions.push(UiAction::SetArrangementMode(true));
                    }

                    ui.separator();
                    // Arrangement Position
                    let sr = 48_000.0_f64;
                    let beats = self.song.transport.sample_position as f64
                        / (sr * 60.0 / self.song.transport.bpm.max(1.0));
                    let pos = MusicalTime::new(beats)
                        .format_display(self.song.time_signature);
                    ui.label(RichText::new(pos).strong().monospace());

                    ui.separator();
                    ui.label(RichText::new("Tempo").small());
                    let mut bpm = self.song.transport.bpm as f32;
                    if ui
                        .add(egui::DragValue::new(&mut bpm).speed(0.1).range(20.0..=999.0))
                        .changed()
                    {
                        self.song.transport.bpm = f64::from(bpm);
                        self.actions.push(UiAction::SetBpm(self.song.transport.bpm));
                    }
                    let ts = self.song.time_signature;
                    ui.label(format!("{}/{}", ts.numerator, ts.denominator));

                    if ui.button("Tap").on_hover_text("Tap Tempo").clicked() {
                        let now = ui.input(|i| i.time);
                        self.tap_times.push(now);
                        self.tap_times.retain(|t| now - t < 3.0);
                        if self.tap_times.len() >= 2 {
                            let mut intervals = Vec::new();
                            for w in self.tap_times.windows(2) {
                                intervals.push(w[1] - w[0]);
                            }
                            let avg = intervals.iter().sum::<f64>() / intervals.len() as f64;
                            if avg > 0.05 {
                                let bpm = (60.0 / avg).clamp(20.0, 999.0);
                                self.song.transport.bpm = bpm;
                                self.actions.push(UiAction::SetBpm(bpm));
                            }
                        }
                        // 4 taps count-in start in 4/4
                        if self.tap_times.len() >= 4 && !self.song.transport.is_rolling() {
                            self.song.transport.play();
                            self.actions.push(UiAction::Play);
                        }
                    }
                    if ui.small_button("-").on_hover_text("Tempo Nudge down").clicked() {
                        self.actions.push(UiAction::TempoNudge(-0.5));
                        self.song.transport.bpm = (self.song.transport.bpm - 0.5).max(20.0);
                        self.actions.push(UiAction::SetBpm(self.song.transport.bpm));
                    }
                    if ui.small_button("+").on_hover_text("Tempo Nudge up").clicked() {
                        self.actions.push(UiAction::TempoNudge(0.5));
                        self.song.transport.bpm = (self.song.transport.bpm + 0.5).min(999.0);
                        self.actions.push(UiAction::SetBpm(self.song.transport.bpm));
                    }

                    ui.separator();
                    ui.label(RichText::new("Groove").small());
                    let mut g = self.song.global_groove as f32;
                    if ui
                        .add(egui::Slider::new(&mut g, 0.0..=1.0).show_value(false))
                        .changed()
                    {
                        self.song.global_groove = f64::from(g);
                    }

                    ui.separator();
                    egui::ComboBox::from_id_salt("gquant")
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
                                    .selectable_label(
                                        self.song.launch_quantize == q,
                                        format!("{q:?}"),
                                    )
                                    .clicked()
                                {
                                    self.song.launch_quantize = q;
                                    self.actions.push(UiAction::SetQuantize(q));
                                }
                            }
                        });

                    let mut metro = self.song.transport.metronome;
                    if ui.checkbox(&mut metro, "Metro").changed() {
                        self.song.transport.metronome = metro;
                        self.actions.push(UiAction::SetMetronome(metro));
                    }

                    ui.separator();
                    if ui
                        .selectable_label(self.key_map_mode, "KEY")
                        .on_hover_text("Key Map Mode")
                        .clicked()
                    {
                        self.key_map_mode = !self.key_map_mode;
                        if self.key_map_mode {
                            self.midi_map_mode = false;
                        }
                    }
                    if ui
                        .selectable_label(self.midi_map_mode, "MIDI")
                        .on_hover_text("MIDI Map Mode")
                        .clicked()
                    {
                        self.midi_map_mode = !self.midi_map_mode;
                        if self.midi_map_mode {
                            self.key_map_mode = false;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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

    fn clip_view(&mut self, ui: &mut Ui) {
        // Clip View Box selector (Live 7)
        ui.horizontal(|ui| {
            ui.label(RichText::new("Boxes").small().color(ArcLiveTheme::MUTED));
            ui.checkbox(&mut self.show_box_clip, "Clip");
            ui.checkbox(&mut self.show_box_launch, "Launch");
            ui.checkbox(&mut self.show_box_envelopes, "Envelopes");
            ui.checkbox(&mut self.show_box_sample, "Sample/Notes");
        });
        ui.separator();

        let Some((ti, si)) = self.selected_slot else {
            ui.label("Select a clip in Session");
            return;
        };

        ui.horizontal(|ui| {
            if self.show_box_clip {
                ui.group(|ui| {
                    ui.set_min_width(160.0);
                    ui.label(RichText::new("Clip").strong().small());
                    self.clip_box(ui, ti, si);
                });
            }
            if self.show_box_launch {
                ui.group(|ui| {
                    ui.set_min_width(140.0);
                    ui.label(RichText::new("Launch").strong().small());
                    egui::ComboBox::from_id_salt("launch_mode")
                        .selected_text(format!("{:?}", self.launch_mode))
                        .show_ui(ui, |ui| {
                            for m in [
                                LaunchMode::Trigger,
                                LaunchMode::Gate,
                                LaunchMode::Toggle,
                                LaunchMode::Repeat,
                            ] {
                                ui.selectable_value(&mut self.launch_mode, m, format!("{m:?}"));
                            }
                        });
                    egui::ComboBox::from_id_salt("clip_q")
                        .selected_text(format!("{:?}", self.clip_quantize))
                        .show_ui(ui, |ui| {
                            for q in [
                                QuantizeGrid::None,
                                QuantizeGrid::Bar,
                                QuantizeGrid::Beat,
                                QuantizeGrid::Eighth,
                                QuantizeGrid::Sixteenth,
                            ] {
                                ui.selectable_value(&mut self.clip_quantize, q, format!("{q:?}"));
                            }
                        });
                    ui.label(RichText::new("Legato — stub").small().color(ArcLiveTheme::MUTED));
                    ui.label(RichText::new("Follow Action — stub").small().color(ArcLiveTheme::MUTED));
                });
            }
            if self.show_box_envelopes {
                ui.group(|ui| {
                    ui.set_min_width(120.0);
                    ui.label(RichText::new("Envelopes").strong().small());
                    ui.label(
                        RichText::new("Envelope editor stub")
                            .small()
                            .color(ArcLiveTheme::MUTED),
                    );
                });
            }
            if self.show_box_sample {
                ui.group(|ui| {
                    ui.set_min_width(280.0);
                    let is_midi = matches!(
                        self.song.tracks.get(ti).and_then(|t| t.slots.get(si)),
                        Some(ClipSlot::Midi(_))
                    );
                    if is_midi {
                        ui.label(RichText::new("Notes").strong().small());
                        piano_roll(ui, &self.song, ti);
                    } else {
                        ui.label(RichText::new("Sample").strong().small());
                        self.sample_box(ui, ti, si);
                    }
                });
            }
        });

        // Sample Display / MIDI
        if self.show_box_sample {
            if let Some(ClipSlot::Audio(clip)) =
                self.song.tracks.get_mut(ti).and_then(|t| t.slots.get_mut(si))
            {
                let before = clip.warp_markers.clone();
                waveform_view(ui, clip, &mut self.selected_marker, 110.0);
                if before != clip.warp_markers {
                    self.actions.push(UiAction::Republish);
                }
            }
        }
    }

    fn clip_box(&mut self, ui: &mut Ui, ti: usize, si: usize) {
        let Some(slot) = self.song.tracks.get_mut(ti).and_then(|t| t.slots.get_mut(si)) else {
            return;
        };
        match slot {
            ClipSlot::Audio(c) => {
                ui.horizontal(|ui| {
                    let mut on = true;
                    ui.checkbox(&mut on, "").on_hover_text("Clip Activator");
                    ui.label(RichText::new(&c.name).strong());
                });
                ui.label(format!("Color track tint"));
                ui.label(format!(
                    "Sig {}/{}",
                    self.song.time_signature.numerator, self.song.time_signature.denominator
                ));
                ui.label(RichText::new("Groove: Swing 8 (stub)").small().color(ArcLiveTheme::MUTED));
                ui.horizontal(|ui| {
                    if ui.small_button("<").on_hover_text("Nudge").clicked() {
                        self.snapshot.status_message = "Clip nudge -".into();
                    }
                    if ui.small_button(">").on_hover_text("Nudge").clicked() {
                        self.snapshot.status_message = "Clip nudge +".into();
                    }
                });
                ui.label(format!("Len {:.2} beats", c.length_beats));
            }
            ClipSlot::Midi(c) => {
                ui.label(RichText::new(&c.name).strong());
                ui.label(format!("{} notes", c.notes.len()));
            }
            ClipSlot::Empty => {
                ui.label("Empty slot");
            }
        }
    }

    fn sample_box(&mut self, ui: &mut Ui, ti: usize, si: usize) {
        let Some(ClipSlot::Audio(clip)) =
            self.song.tracks.get_mut(ti).and_then(|t| t.slots.get_mut(si))
        else {
            return;
        };
        let mut changed = false;
        ui.horizontal(|ui| {
            let mut warp = clip.warp_enabled;
            if ui.checkbox(&mut warp, "Warp").changed() {
                clip.warp_enabled = warp;
                changed = true;
            }
            let mut mode = clip.warp_mode;
            warp_mode_combo(ui, &mut mode);
            if mode != clip.warp_mode {
                clip.warp_mode = mode;
                changed = true;
            }
            let mut seg = clip.segment_bpm_at(self.selected_marker.max(1)) as f32;
            ui.label("Seg.BPM");
            if ui
                .add(egui::DragValue::new(&mut seg).speed(0.1).range(20.0..=999.0))
                .changed()
            {
                let idx = self
                    .selected_marker
                    .max(1)
                    .min(clip.warp_markers.len().saturating_sub(1));
                if idx > 0 && clip.warp_markers.len() > idx {
                    let prev = clip.warp_markers[idx - 1];
                    let beat_delta = (clip.warp_markers[idx].beat - prev.beat).abs().max(1e-9);
                    let seconds = beat_delta * 60.0 / f64::from(seg).max(1.0);
                    let frames = (seconds * f64::from(clip.sample_rate)).round() as u64;
                    clip.warp_markers[idx].sample_frame = prev.sample_frame.saturating_add(frames);
                    changed = true;
                }
            }
        });
        ui.horizontal(|ui| {
            let mut tr = i32::from(clip.transpose_semitones);
            ui.label("Transp");
            if ui.add(egui::DragValue::new(&mut tr).range(-48..=48)).changed() {
                clip.transpose_semitones = tr as i8;
                changed = true;
            }
            let mut det = i32::from(clip.detune_cents);
            ui.label("Detune");
            if ui.add(egui::DragValue::new(&mut det).range(-50..=50)).changed() {
                clip.detune_cents = det as i16;
                changed = true;
            }
            let mut loop_on = clip.loop_on;
            if ui.checkbox(&mut loop_on, "Loop").changed() {
                clip.loop_on = loop_on;
                changed = true;
            }
        });
        ui.label(
            RichText::new(format!(
                "Start {:.2}  End {:.2}  Tempo {:.1}",
                clip.clip_start_beats,
                clip.clip_end_beats,
                clip.clip_tempo_bpm()
            ))
            .small()
            .color(ArcLiveTheme::MUTED),
        );
        if changed {
            self.actions.push(UiAction::Republish);
        }
    }
}
