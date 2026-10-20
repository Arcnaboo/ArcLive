use arclive_core::Song;
use arclive_engine::{AudioHost, EngineCommand, MidiInputHub, PublishedGraph};
use arclive_project::{open_project, save_autosave, save_project};
use arclive_ui::{UiAction, UiApp, UiSnapshot};
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tracing_subscriber::EnvFilter;

struct App {
    ui: UiApp,
    host: Option<AudioHost>,
    project_dir: Option<PathBuf>,
    last_autosave: Instant,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        arclive_ui::ArcLiveTheme::apply(&cc.egui_ctx);
        let song = Song::new_demo();
        let graph = PublishedGraph::from_song(&song);
        let host = match AudioHost::try_open(graph) {
            Ok(h) => {
                tracing::info!("audio device: {:?}", h.status.device_name);
                Some(h)
            }
            Err(e) => {
                tracing::warn!("audio open failed: {e}");
                None
            }
        };

        let midi_names = MidiInputHub::list_inputs()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.name)
            .collect();

        let mut ui = UiApp::new(song);
        ui.snapshot = UiSnapshot {
            device_name: host
                .as_ref()
                .and_then(|h| h.status.device_name.clone())
                .unwrap_or_else(|| "not open".into()),
            sample_rate: host
                .as_ref()
                .and_then(|h| h.status.sample_rate.map(|s| format!("{s:.0}")))
                .unwrap_or_else(|| "—".into()),
            buffer_frames: host
                .as_ref()
                .and_then(|h| h.status.buffer_frames.map(|b| b.to_string()))
                .unwrap_or_else(|| "—".into()),
            latency_ms: host
                .as_ref()
                .and_then(|h| h.status.latency_ms.map(|l| format!("{l:.1} ms")))
                .unwrap_or_else(|| "—".into()),
            xruns: 0,
            cpu_hint: "n/a".into(),
            master_l: 0.0,
            master_r: 0.0,
            track_peaks: vec![0.0; ui.song.tracks.len()],
            engine_sample_pos: 0,
            status_message: "Ready".into(),
            midi_inputs: midi_names,
        };

        Self {
            ui,
            host,
            project_dir: None,
            last_autosave: Instant::now(),
        }
    }

    fn publish(&mut self) {
        let graph = PublishedGraph::from_song(&self.ui.song);
        if let Some(host) = self.host.as_mut() {
            host.publish_graph(graph);
        }
    }

    fn push(&mut self, cmd: EngineCommand) {
        if let Some(host) = self.host.as_mut() {
            let _ = host.push(cmd);
        }
    }

    fn apply_actions(&mut self) {
        let actions = self.ui.take_actions();
        for action in actions {
            match action {
                UiAction::Play => self.push(EngineCommand::Play),
                UiAction::Stop => self.push(EngineCommand::Stop),
                UiAction::TogglePlayStop => {
                    if self.ui.song.transport.is_rolling() {
                        self.push(EngineCommand::Play);
                    } else {
                        self.push(EngineCommand::Stop);
                    }
                }
                UiAction::SeekBeats(b) => {
                    let sr = self
                        .host
                        .as_ref()
                        .and_then(|h| h.status.sample_rate)
                        .unwrap_or(48_000.0);
                    self.ui.song.transport.seek_beats(b, sr);
                    self.push(EngineCommand::SeekSamples(
                        self.ui.song.transport.sample_position,
                    ));
                }
                UiAction::SetBpm(b) => self.push(EngineCommand::SetBpm(b)),
                UiAction::SetMetronome(v) => self.push(EngineCommand::SetMetronome(v)),
                UiAction::ArmRecord(_) => {}
                UiAction::LaunchClip { track, slot } => {
                    self.push(EngineCommand::LaunchClip {
                        track_index: track as u16,
                        slot_index: slot as u16,
                        quantize: self.ui.song.launch_quantize,
                    });
                }
                UiAction::LaunchScene { scene } => {
                    self.push(EngineCommand::LaunchScene {
                        scene_index: scene as u16,
                        quantize: self.ui.song.launch_quantize,
                    });
                }
                UiAction::StopAllClips => self.push(EngineCommand::StopAllClips),
                UiAction::SetGain { track, gain_db } => {
                    self.push(EngineCommand::SetGain {
                        track_index: track as u16,
                        gain_db,
                    });
                    self.publish();
                }
                UiAction::SetPan { track, pan } => {
                    self.push(EngineCommand::SetPan {
                        track_index: track as u16,
                        pan,
                    });
                }
                UiAction::SetMute { track, mute } => {
                    self.push(EngineCommand::SetMute {
                        track_index: track as u16,
                        mute,
                    });
                }
                UiAction::SetSolo { track, solo } => {
                    self.push(EngineCommand::SetSolo {
                        track_index: track as u16,
                        solo,
                    });
                }
                UiAction::SetArrangementMode(v) => {
                    self.ui.song.arrangement_mode = v;
                    self.push(EngineCommand::SetArrangementMode(v));
                }
                UiAction::SetLoop {
                    enabled,
                    start,
                    end,
                } => {
                    self.push(EngineCommand::SetLoop {
                        enabled,
                        start_beats: start,
                        end_beats: end,
                    });
                }
                UiAction::AddAudioTrack => self.publish(),
                UiAction::RemoveTrack(i) => {
                    if let Some(t) = self.ui.song.tracks.get(i).map(|t| t.id) {
                        self.ui.song.remove_track(t);
                        self.publish();
                    }
                }
                UiAction::SetWorkspace(w) => self.ui.song.workspace = w,
                UiAction::SetQuantize(q) => self.ui.song.launch_quantize = q,
                UiAction::NewProject => {
                    self.ui.song = Song::new_demo();
                    self.project_dir = None;
                    self.publish();
                    self.ui.snapshot.status_message = "New project".into();
                }
                UiAction::OpenProject => self.open_project(),
                UiAction::SaveProject => self.save_project(false),
                UiAction::SaveProjectAs => self.save_project(true),
                UiAction::MoveArrangementClip { index, start_beats } => {
                    if let Some(c) = self.ui.song.arrangement.get_mut(index) {
                        c.start_beats = start_beats;
                    }
                    self.publish();
                }
                UiAction::Republish => self.publish(),
                UiAction::ImportWavDialog => self.import_wav_dialog(),
                UiAction::StopClip { track } => {
                    self.push(EngineCommand::StopClip {
                        track_index: track as u16,
                    });
                }
                UiAction::BackToArrangement => {
                    self.push(EngineCommand::BackToArrangement);
                }
                UiAction::SetArmed { track, armed } => {
                    self.push(EngineCommand::SetArmed {
                        track_index: track as u16,
                        armed,
                    });
                    self.publish();
                }
                UiAction::SetActivator { track, activator } => {
                    self.push(EngineCommand::SetActivator {
                        track_index: track as u16,
                        activator,
                    });
                }
                UiAction::TempoNudge(_) => {}
            }
        }
    }

    fn import_wav_dialog(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("WAV", &["wav"])
            .pick_file();
        if let Some(path) = path {
            let (t, s) = self.ui.selected_slot_or_default();
            self.ui.import_wav_to_slot(&path.to_string_lossy(), t, s);
            self.ui.refresh_sample_list();
            self.publish();
        }
    }

    fn open_project(&mut self) {
        let path = rfd::FileDialog::new()
            .set_title("Open ArcLive Project")
            .pick_folder();
        if let Some(dir) = path {
            match open_project(&dir) {
                Ok(song) => {
                    self.ui.song = song;
                    self.project_dir = Some(dir);
                    self.publish();
                    self.ui.snapshot.status_message = "Project opened".into();
                }
                Err(e) => {
                    self.ui.snapshot.status_message = format!("Open failed: {e}");
                }
            }
        }
    }

    fn save_project(&mut self, save_as: bool) {
        let dir = if save_as || self.project_dir.is_none() {
            rfd::FileDialog::new()
                .set_title("Save ArcLive Project")
                .pick_folder()
        } else {
            self.project_dir.clone()
        };
        let Some(dir) = dir else {
            return;
        };
        let target = if dir.extension().and_then(|e| e.to_str()) == Some("arclive") {
            dir
        } else {
            dir.join(format!("{}.arclive", sanitize(&self.ui.song.name)))
        };
        match save_project(&target, &self.ui.song) {
            Ok(_) => {
                self.project_dir = Some(target);
                self.ui.snapshot.status_message = "Project saved".into();
            }
            Err(e) => {
                self.ui.snapshot.status_message = format!("Save failed: {e}");
            }
        }
    }

    fn refresh_snapshot(&mut self) {
        if let Some(host) = self.host.as_ref() {
            let m = &host.handle.meters;
            self.ui.snapshot.xruns = m.xruns.load(Ordering::Relaxed);
            let (l, r) = m.master_peaks();
            self.ui.snapshot.master_l = l;
            self.ui.snapshot.master_r = r;
            let n = self.ui.song.tracks.len();
            self.ui.snapshot.track_peaks.resize(n, 0.0);
            for i in 0..n {
                self.ui.snapshot.track_peaks[i] = m.track_peak(i);
            }
            let pos = m.sample_position.load(Ordering::Relaxed);
            self.ui.snapshot.engine_sample_pos = pos;
            self.ui.song.transport.sample_position = pos;
            if m.fault.load(Ordering::Relaxed) != 0 {
                self.ui.snapshot.status_message = "Audio device fault — restart playback".into();
            }
        }
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui.ui(ui);
        self.apply_actions();
        self.refresh_snapshot();

        if self.last_autosave.elapsed().as_secs() >= 60 {
            if let Some(dir) = &self.project_dir {
                let _ = save_autosave(dir, &self.ui.song);
            }
            self.last_autosave = Instant::now();
        }

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));
    }
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("arclive=info".parse().unwrap()),
        )
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("ArcLive"),
        ..Default::default()
    };

    eframe::run_native(
        "ArcLive",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
