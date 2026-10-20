use crate::theme::ArcLiveTheme;
use arclive_core::{AudioClipData, ClipSlot, Song, TrackKind, WarpMode};
use egui::{Color32, Pos2, RichText, Sense, Shape, Stroke, Ui, Vec2};

#[derive(Clone, Debug)]
pub struct ClipSlotRef {
    pub track: usize,
    pub slot: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SessionViewFlags {
    pub show_sends: bool,
    pub show_returns: bool,
    pub show_io: bool,
    pub show_xfade: bool,
    pub show_mixer: bool,
}

impl Default for SessionViewFlags {
    fn default() -> Self {
        Self {
            show_sends: true,
            show_returns: true,
            show_io: false,
            show_xfade: false,
            show_mixer: true,
        }
    }
}

pub fn meter_bar(ui: &mut Ui, peak: f32, width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, ArcLiveTheme::GRID);
    let level = peak.clamp(0.0, 1.0);
    let fill = egui::Rect::from_min_size(
        Pos2::new(rect.min.x, rect.max.y - height * level),
        Vec2::new(width, height * level),
    );
    let color = if level > 0.9 {
        ArcLiveTheme::RECORD
    } else if level > 0.6 {
        ArcLiveTheme::ACCENT
    } else {
        ArcLiveTheme::ACTIVATOR_ON
    };
    painter.rect_filled(fill, 0.0, color);
}

pub fn clip_launch_button(ui: &mut Ui, playing: bool, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let painter = ui.painter();
    let fill = if playing {
        ArcLiveTheme::ACCENT
    } else if resp.hovered() {
        Color32::from_rgb(0x70, 0x70, 0x70)
    } else {
        ArcLiveTheme::SLOT
    };
    painter.rect_filled(rect, 0.0, fill);
    let pad = size * 0.28;
    let tri = vec![
        Pos2::new(rect.min.x + pad, rect.min.y + pad),
        Pos2::new(rect.max.x - pad * 0.6, rect.center().y),
        Pos2::new(rect.min.x + pad, rect.max.y - pad),
    ];
    painter.add(Shape::convex_polygon(
        tri,
        if playing { Color32::BLACK } else { ArcLiveTheme::TEXT },
        Stroke::NONE,
    ));
    resp
}

pub fn clip_stop_button(ui: &mut Ui, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, ArcLiveTheme::STOP);
    let inset = size * 0.28;
    painter.rect_filled(rect.shrink(inset), 0.0, ArcLiveTheme::TEXT.linear_multiply(0.85));
    resp
}

pub fn track_status_field(
    ui: &mut Ui,
    width: f32,
    height: f32,
    looping: bool,
    playing: bool,
    loop_beats: f64,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, ArcLiveTheme::PANEL_ALT);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, ArcLiveTheme::BORDER),
        egui::StrokeKind::Inside,
    );
    if !playing {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "-",
            egui::FontId::proportional(11.0),
            ArcLiveTheme::MUTED,
        );
        return;
    }
    if looping {
        let c = rect.center();
        let r = height * 0.32;
        painter.circle_stroke(c, r, Stroke::new(2.0_f32, ArcLiveTheme::ACCENT));
        let mut pts = vec![c];
        for i in 0..=8 {
            let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / 8.0) * std::f32::consts::TAU * 0.65;
            pts.push(Pos2::new(c.x + r * a.cos(), c.y + r * a.sin()));
        }
        painter.add(Shape::convex_polygon(
            pts,
            ArcLiveTheme::ACCENT.linear_multiply(0.7),
            Stroke::NONE,
        ));
        painter.text(
            Pos2::new(rect.max.x - 4.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("{loop_beats:.0}"),
            egui::FontId::proportional(10.0),
            ArcLiveTheme::TEXT,
        );
    } else {
        let bar = egui::Rect::from_min_size(
            Pos2::new(rect.min.x + 6.0, rect.center().y - 4.0),
            Vec2::new(rect.width() - 12.0, 8.0),
        );
        painter.rect_filled(bar, 0.0, ArcLiveTheme::GRID);
        painter.rect_filled(
            egui::Rect::from_min_size(bar.min, Vec2::new(bar.width() * 0.45, bar.height())),
            0.0,
            ArcLiveTheme::ACCENT,
        );
    }
}

/// Live 7 Session: track columns; Master + Scene Launch on the RIGHT.
pub fn session_columns(
    ui: &mut Ui,
    song: &mut Song,
    track_peaks: &[f32],
    selected_track: &mut usize,
    selected_slot: &mut Option<(usize, usize)>,
    flags: &SessionViewFlags,
    on_launch: &mut dyn FnMut(usize, usize),
    on_stop_clip: &mut dyn FnMut(usize),
    on_scene: &mut dyn FnMut(usize),
    on_drop_sample: &mut dyn FnMut(usize, usize, String),
    on_move_clip: &mut dyn FnMut(usize, usize, usize, usize),
    mixer_dirty: &mut bool,
) {
    let scene_count = song.scenes.len().max(1);
    let track_w = 88.0_f32;
    let slot_h = 26.0_f32;
    let status_h = 22.0_f32;
    let mixer_h = if flags.show_mixer { 150.0 } else { 0.0 };

    egui::ScrollArea::both().show(ui, |ui| {
        ui.horizontal(|ui| {
            for ti in 0..song.tracks.len() {
                let track_color = ArcLiveTheme::track_clip_color(ti);
                ui.vertical(|ui| {
                    let selected = *selected_track == ti;
                    let header = egui::Button::new(
                        RichText::new(&song.tracks[ti].name).strong().small(),
                    )
                    .fill(if selected {
                        ArcLiveTheme::ACCENT.linear_multiply(0.35)
                    } else {
                        ArcLiveTheme::PANEL_ALT
                    });
                    if ui.add_sized([track_w, 20.0], header).clicked() {
                        *selected_track = ti;
                    }

                    for si in 0..scene_count {
                        let slot = song
                            .tracks
                            .get(ti)
                            .and_then(|t| t.slots.get(si))
                            .cloned()
                            .unwrap_or(ClipSlot::Empty);
                        let filled = !slot.is_empty();
                        let playing =
                            song.tracks.get(ti).and_then(|t| t.playing_slot) == Some(si);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 1.0;
                            let launch = clip_launch_button(ui, playing, slot_h - 2.0);
                            if launch.clicked() && filled {
                                *selected_track = ti;
                                *selected_slot = Some((ti, si));
                                on_launch(ti, si);
                            }
                            let label = if filled {
                                slot.name().to_string()
                            } else {
                                String::new()
                            };
                            let fill = if playing {
                                ArcLiveTheme::ACCENT.linear_multiply(0.45)
                            } else if filled {
                                track_color.linear_multiply(0.8)
                            } else {
                                ArcLiveTheme::SLOT
                            };
                            let name_w = track_w - slot_h - 4.0;
                            let resp = ui.add_sized(
                                [name_w.max(20.0), slot_h],
                                egui::Button::new(RichText::new(label).small()).fill(fill),
                            );
                            if filled {
                                resp.dnd_set_drag_payload(ClipSlotRef {
                                    track: ti,
                                    slot: si,
                                });
                            }
                            if let Some(path) = resp.dnd_release_payload::<String>() {
                                on_drop_sample(ti, si, (*path).clone());
                            }
                            if let Some(src) = resp.dnd_release_payload::<ClipSlotRef>() {
                                if src.track != ti || src.slot != si {
                                    on_move_clip(src.track, src.slot, ti, si);
                                }
                            }
                            if resp.clicked() {
                                *selected_track = ti;
                                *selected_slot = Some((ti, si));
                                if filled {
                                    on_launch(ti, si);
                                }
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        if clip_stop_button(ui, status_h - 2.0).clicked() {
                            on_stop_clip(ti);
                        }
                        let playing = song.tracks.get(ti).and_then(|t| t.playing_slot).is_some();
                        let looping = song
                            .tracks
                            .get(ti)
                            .and_then(|t| t.playing_slot)
                            .and_then(|si| slot_looping(song, ti, si))
                            .unwrap_or(true);
                        let beats = song
                            .tracks
                            .get(ti)
                            .and_then(|t| t.playing_slot)
                            .and_then(|si| slot_length(song, ti, si))
                            .unwrap_or(4.0);
                        track_status_field(
                            ui,
                            track_w - status_h,
                            status_h,
                            looping,
                            playing,
                            beats,
                        );
                    });

                    if flags.show_mixer {
                        under_track_mixer(
                            ui,
                            song,
                            ti,
                            track_peaks.get(ti).copied().unwrap_or(0.0),
                            track_w,
                            mixer_h,
                            flags,
                            mixer_dirty,
                        );
                    }
                });
            }

            if flags.show_returns {
                for ri in 0..song.return_track_count {
                    ui.vertical(|ui| {
                        let letter = (b'A' + ri as u8) as char;
                        ui.add_sized(
                            [track_w, 20.0],
                            egui::Label::new(
                                RichText::new(format!("Return {letter}")).small().strong(),
                            ),
                        );
                        for _ in 0..scene_count {
                            ui.add_sized([track_w, slot_h], egui::Label::new(""));
                        }
                        ui.add_sized([track_w, status_h], egui::Label::new(""));
                        if flags.show_mixer {
                            ui.group(|ui| {
                                ui.set_min_size(Vec2::new(track_w, mixer_h));
                                ui.label(RichText::new("Ret").small().color(ArcLiveTheme::MUTED));
                                meter_bar(ui, 0.0, 12.0, (mixer_h - 40.0).max(20.0));
                            });
                        }
                    });
                }
            }

            // Master + Scene Launch (rightmost) — Live 7 Ch.7
            ui.vertical(|ui| {
                ui.add_sized(
                    [track_w, 20.0],
                    egui::Label::new(RichText::new("Master").strong().small()),
                );
                for (si, scene) in song.scenes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if clip_launch_button(ui, false, slot_h - 2.0).clicked() {
                            on_scene(si);
                        }
                        if ui
                            .add_sized(
                                [track_w - slot_h, slot_h],
                                egui::Button::new(RichText::new(&scene.name).small())
                                    .fill(ArcLiveTheme::PANEL_ALT),
                            )
                            .clicked()
                        {
                            on_scene(si);
                        }
                    });
                }
                ui.add_sized([track_w, status_h], egui::Label::new(""));
                if flags.show_mixer {
                    ui.group(|ui| {
                        ui.set_min_size(Vec2::new(track_w, mixer_h));
                        ui.label(RichText::new("M").small());
                        let peak = track_peaks.iter().cloned().fold(0.0f32, f32::max);
                        meter_bar(ui, peak, 14.0, (mixer_h - 36.0).max(20.0));
                    });
                }
            });
        });
    });
}

fn slot_looping(song: &Song, ti: usize, si: usize) -> Option<bool> {
    match song.tracks.get(ti)?.slots.get(si)? {
        ClipSlot::Audio(c) => Some(c.loop_on),
        ClipSlot::Midi(_) => Some(true),
        ClipSlot::Empty => None,
    }
}

fn slot_length(song: &Song, ti: usize, si: usize) -> Option<f64> {
    match song.tracks.get(ti)?.slots.get(si)? {
        ClipSlot::Audio(c) => Some(c.length_beats),
        ClipSlot::Midi(c) => Some(c.length_beats),
        ClipSlot::Empty => None,
    }
}

fn under_track_mixer(
    ui: &mut Ui,
    song: &mut Song,
    ti: usize,
    peak: f32,
    width: f32,
    height: f32,
    flags: &SessionViewFlags,
    dirty: &mut bool,
) {
    ui.group(|ui| {
        ui.set_min_size(Vec2::new(width, height));
        ui.spacing_mut().item_spacing.y = 2.0;
        let Some(track) = song.tracks.get_mut(ti) else {
            return;
        };
        if flags.show_io {
            ui.label(RichText::new("In/Out").small().color(ArcLiveTheme::MUTED));
        }
        if flags.show_sends {
            ui.horizontal(|ui| {
                ui.label(RichText::new("A").small());
                if ui
                    .add(egui::Slider::new(&mut track.mixer.send_a, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    *dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("B").small());
                if ui
                    .add(egui::Slider::new(&mut track.mixer.send_b, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    *dirty = true;
                }
            });
        }
        if flags.show_xfade {
            ui.horizontal(|ui| {
                for (v, lab) in [(0u8, "-"), (1, "A"), (2, "B")] {
                    if ui
                        .selectable_label(track.mixer.xfade_assign == v, RichText::new(lab).small())
                        .clicked()
                    {
                        track.mixer.xfade_assign = v;
                        *dirty = true;
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            let on = track.mixer.activator;
            let act = egui::Button::new(RichText::new(if on { "I" } else { "O" }).small()).fill(
                if on {
                    ArcLiveTheme::ACTIVATOR_ON
                } else {
                    ArcLiveTheme::STOP
                },
            );
            if ui.add_sized([18.0, 18.0], act).on_hover_text("Track Activator").clicked() {
                track.mixer.activator = !track.mixer.activator;
                *dirty = true;
            }
            if ui
                .selectable_label(track.mixer.solo, RichText::new("S").small())
                .clicked()
            {
                track.mixer.solo = !track.mixer.solo;
                *dirty = true;
            }
            let arm = egui::Button::new(RichText::new("R").small()).fill(if track.mixer.armed {
                ArcLiveTheme::RECORD
            } else {
                ArcLiveTheme::PANEL_ALT
            });
            if ui.add_sized([18.0, 18.0], arm).on_hover_text("Arm").clicked() {
                track.mixer.armed = !track.mixer.armed;
                *dirty = true;
            }
        });
        if ui
            .add(
                egui::Slider::new(&mut track.mixer.pan, -1.0..=1.0)
                    .show_value(false)
                    .text("Pan"),
            )
            .changed()
        {
            *dirty = true;
        }
        if ui.small_button("^").on_hover_text("Reset pan").clicked() {
            track.mixer.pan = 0.0;
            *dirty = true;
        }
        ui.horizontal(|ui| {
            meter_bar(ui, peak, 10.0, 64.0);
            if ui
                .add(
                    egui::Slider::new(&mut track.mixer.gain_db, -48.0..=6.0)
                        .vertical()
                        .show_value(false),
                )
                .changed()
            {
                *dirty = true;
            }
        });
    });
}

pub fn arrangement_view(
    ui: &mut Ui,
    song: &Song,
    zoom: f32,
    scroll_beats: f32,
    on_select: &mut dyn FnMut(usize),
) {
    let beats_visible = (ui.available_width() / zoom).max(4.0);
    let height = 28.0 * song.tracks.len() as f32 + 24.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height.max(80.0)),
        Sense::click(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, ArcLiveTheme::PANEL);
    for beat in 0..(beats_visible as i32 + 2) {
        let b = scroll_beats + beat as f32;
        let x = rect.min.x + (b - scroll_beats) * zoom;
        let color = if (b as i32) % 4 == 0 {
            ArcLiveTheme::MUTED
        } else {
            ArcLiveTheme::GRID
        };
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(1.0_f32, color),
        );
    }
    for (ti, track) in song.tracks.iter().enumerate() {
        let y = rect.min.y + 20.0 + ti as f32 * 28.0;
        painter.text(
            Pos2::new(rect.min.x + 4.0, y),
            egui::Align2::LEFT_TOP,
            &track.name,
            egui::FontId::proportional(11.0),
            ArcLiveTheme::MUTED,
        );
    }
    for (ci, clip) in song.arrangement.iter().enumerate() {
        let Some(ti) = song.tracks.iter().position(|t| t.id == clip.track_id) else {
            continue;
        };
        let x = rect.min.x + ((clip.start_beats as f32) - scroll_beats) * zoom;
        let w = (clip.length_beats as f32) * zoom;
        let y = rect.min.y + 20.0 + ti as f32 * 28.0;
        let r = egui::Rect::from_min_size(Pos2::new(x, y), Vec2::new(w.max(4.0), 24.0));
        painter.rect_filled(r, 0.0, ArcLiveTheme::track_clip_color(ti));
        painter.text(
            r.min + Vec2::new(4.0, 4.0),
            egui::Align2::LEFT_TOP,
            &clip.name,
            egui::FontId::proportional(10.0),
            ArcLiveTheme::TEXT,
        );
        let id = ui.id().with(("arr", ci));
        if ui.interact(r, id, Sense::click()).clicked() {
            on_select(ci);
        }
    }
}

pub fn piano_roll(ui: &mut Ui, song: &Song, track_index: usize) {
    ui.label(RichText::new("Notes").color(ArcLiveTheme::MUTED));
    let Some(track) = song.tracks.get(track_index) else {
        return;
    };
    if track.kind != TrackKind::Midi {
        ui.label("Select a MIDI track");
        return;
    }
    for slot in &track.slots {
        if let ClipSlot::Midi(c) = slot {
            ui.label(format!("{} — {} notes", c.name, c.notes.len()));
            for n in &c.notes {
                ui.small(format!(
                    "p{} v{} @ {:.2} len {:.2}",
                    n.pitch, n.velocity, n.start_beats, n.length_beats
                ));
            }
        }
    }
}

pub fn waveform_view(
    ui: &mut Ui,
    clip: &mut AudioClipData,
    selected_marker: &mut usize,
    height: f32,
) {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, ArcLiveTheme::WAVE_BG);
    let ch = clip.channels.max(1) as usize;
    let frames = clip.samples.len() / ch;
    if frames > 1 {
        let peaks = 512.min(frames);
        let step = frames / peaks;
        for i in 0..peaks {
            let mut mn = 0.0f32;
            let mut mx = 0.0f32;
            let base = i * step;
            for f in base..(base + step).min(frames) {
                let s = clip.samples[f * ch];
                mn = mn.min(s);
                mx = mx.max(s);
            }
            let x = rect.min.x + (i as f32 / peaks as f32) * rect.width();
            let mid = rect.center().y;
            painter.line_segment(
                [
                    Pos2::new(x, mid - mx * rect.height() * 0.45),
                    Pos2::new(x, mid - mn * rect.height() * 0.45),
                ],
                Stroke::new(1.0_f32, ArcLiveTheme::WAVE_FG),
            );
        }
    }
    let end_beat = clip
        .warp_markers
        .last()
        .map(|m| m.beat)
        .unwrap_or(clip.length_beats)
        .max(0.001);
    for (mi, m) in clip.warp_markers.iter().enumerate() {
        let x = rect.min.x + (m.beat / end_beat) as f32 * rect.width();
        let color = if mi == *selected_marker {
            ArcLiveTheme::ACCENT
        } else {
            Color32::from_rgb(0xc8, 0xa0, 0x20)
        };
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(2.0_f32, color),
        );
    }
    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let t = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0) as f64;
            let beat = t * end_beat;
            let frame = if frames > 0 {
                ((beat / end_beat) * frames as f64) as u64
            } else {
                0
            };
            if pos.y < rect.center().y {
                clip.warp_markers
                    .push(arclive_core::WarpMarker::new(frame, beat));
                clip.sort_markers();
                *selected_marker = clip
                    .warp_markers
                    .iter()
                    .position(|m| (m.beat - beat).abs() < 1e-6)
                    .unwrap_or(0);
            } else if !clip.warp_markers.is_empty() {
                let mut best = 0usize;
                let mut best_d = f64::MAX;
                for (i, m) in clip.warp_markers.iter().enumerate() {
                    let d = (m.beat - beat).abs();
                    if d < best_d {
                        best_d = d;
                        best = i;
                    }
                }
                *selected_marker = best;
            }
        }
    }
    if resp.dragged() && !clip.warp_markers.is_empty() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let t = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0) as f64;
            let beat = t * end_beat;
            let idx = (*selected_marker).min(clip.warp_markers.len() - 1);
            if ui.input(|i| i.modifiers.shift) {
                let frame = ((beat / end_beat) * frames as f64).max(0.0) as u64;
                clip.warp_markers[idx].sample_frame = frame;
            } else {
                clip.warp_markers[idx].beat = beat;
            }
            clip.sort_markers();
        }
    }
}

pub fn sample_browser(ui: &mut Ui, files: &[String], preview: &mut bool, on_import: &mut dyn FnMut()) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Samples").strong());
        ui.checkbox(preview, "").on_hover_text("Preview (stub)");
        if ui.small_button("Browse...").clicked() {
            on_import();
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if files.is_empty() {
            ui.label(
                RichText::new("Drop WAVs or browse ./samples")
                    .small()
                    .color(ArcLiveTheme::MUTED),
            );
        }
        for path in files {
            let name = std::path::Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(path);
            let item = ui.add(
                egui::Button::new(RichText::new(name).small())
                    .fill(ArcLiveTheme::PANEL_ALT)
                    .sense(Sense::click_and_drag()),
            );
            item.dnd_set_drag_payload(path.clone());
        }
    });
}

pub fn warp_mode_combo(ui: &mut Ui, mode: &mut WarpMode) {
    egui::ComboBox::from_id_salt("warp_mode")
        .selected_text(mode.label())
        .show_ui(ui, |ui| {
            for m in WarpMode::ALL {
                ui.selectable_value(mode, m, m.label());
            }
        });
}
