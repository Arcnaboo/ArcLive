use crate::theme::ArcLiveTheme;
use arclive_core::{AudioClipData, ClipSlot, Song, TrackKind, WarpMode};
use egui::{Color32, DragAndDrop, Id, RichText, Sense, Ui};


#[derive(Clone, Debug)]
pub struct ClipSlotRef {
    pub track: usize,
    pub slot: usize,
}

pub fn meter_bar(ui: &mut Ui, peak: f32, width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, ArcLiveTheme::GRID);
    let level = peak.clamp(0.0, 1.0);
    let fill = egui::Rect::from_min_size(
        egui::pos2(rect.min.x, rect.max.y - height * level),
        egui::vec2(width, height * level),
    );
    let color = if level > 0.9 {
        ArcLiveTheme::RECORD
    } else if level > 0.6 {
        ArcLiveTheme::ACCENT
    } else {
        ArcLiveTheme::CLIP_A
    };
    painter.rect_filled(fill, 0.0, color);
}

/// Live 7–style vertical track columns with scene rows and under-track mixer.
pub fn session_columns(
    ui: &mut Ui,
    song: &mut Song,
    track_peaks: &[f32],
    selected_track: &mut usize,
    selected_slot: &mut Option<(usize, usize)>,
    on_launch: &mut dyn FnMut(usize, usize),
    on_scene: &mut dyn FnMut(usize),
    on_drop_sample: &mut dyn FnMut(usize, usize, String),
    on_move_clip: &mut dyn FnMut(usize, usize, usize, usize),
    mixer_dirty: &mut bool,
) {
    let scene_count = song.scenes.len().max(1);
    let track_w = 92.0_f32;
    let slot_h = 28.0_f32;
    let mixer_h = 160.0_f32;

    egui::ScrollArea::both().show(ui, |ui| {
        ui.horizontal(|ui| {
            // Scene launch column
            ui.vertical(|ui| {
                ui.add_sized(
                    [70.0, 22.0],
                    egui::Label::new(RichText::new("Scenes").small().color(ArcLiveTheme::MUTED)),
                );
                for (si, scene) in song.scenes.iter().enumerate() {
                    if ui
                        .add_sized(
                            [70.0, slot_h],
                            egui::Button::new(RichText::new(&scene.name).small()),
                        )
                        .clicked()
                    {
                        on_scene(si);
                    }
                }
                ui.add_space(mixer_h);
            });

            for ti in 0..song.tracks.len() {
                let track_color = ArcLiveTheme::track_clip_color(ti);
                ui.vertical(|ui| {
                    let selected = *selected_track == ti;
                    let header = egui::Button::new(RichText::new(&song.tracks[ti].name).strong().small())
                        .fill(if selected {
                            ArcLiveTheme::ACCENT.linear_multiply(0.35)
                        } else {
                            ArcLiveTheme::PANEL_ALT
                        });
                    if ui.add_sized([track_w, 22.0], header).clicked() {
                        *selected_track = ti;
                    }

                    for si in 0..scene_count {
                        let slot = song.tracks[ti].slots.get(si).cloned().unwrap_or(ClipSlot::Empty);
                        let filled = !slot.is_empty();
                        let playing = song.tracks[ti].playing_slot == Some(si);
                        let label = if filled {
                            slot.name().to_string()
                        } else {
                            "■".into()
                        };
                        let fill = if playing {
                            ArcLiveTheme::ACCENT.linear_multiply(0.55)
                        } else if filled {
                            track_color.linear_multiply(0.75)
                        } else {
                            ArcLiveTheme::STOP
                        };
                        let resp = ui.add_sized(
                            [track_w, slot_h],
                            egui::Button::new(RichText::new(label).small()).fill(fill),
                        );

                        // Drag filled clip
                        if filled {
                            let id = Id::new(("clip_dnd", ti, si));
                            let dnd = resp.dnd_set_drag_payload(ClipSlotRef {
                                track: ti,
                                slot: si,
                            });
                            let _ = (id, dnd);
                        }

                        // Drop sample path or clip
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
                    }

                    // Under-track mixer
                    under_track_mixer(ui, song, ti, track_peaks.get(ti).copied().unwrap_or(0.0), track_w, mixer_h, mixer_dirty);
                });
            }

            // Master stub column
            ui.vertical(|ui| {
                ui.add_sized(
                    [track_w, 22.0],
                    egui::Label::new(RichText::new("Master").strong().small()),
                );
                for _ in 0..scene_count {
                    ui.add_sized([track_w, slot_h], egui::Label::new(""));
                }
                ui.group(|ui| {
                    ui.set_min_width(track_w);
                    ui.set_min_height(mixer_h);
                    ui.label(RichText::new("M").small());
                    let peak = track_peaks.iter().cloned().fold(0.0f32, f32::max);
                    meter_bar(ui, peak, 14.0, mixer_h - 40.0);
                });
            });
        });
    });
}

fn under_track_mixer(ui: &mut Ui, song: &mut Song, ti: usize, peak: f32, width: f32, height: f32, dirty: &mut bool) {
    ui.group(|ui| {
        ui.set_min_size(egui::vec2(width, height));
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("A  B").small().color(ArcLiveTheme::MUTED));
            // send stubs
            let mut send_a = 0.0f32;
            let mut send_b = 0.0f32;
            ui.add(egui::Slider::new(&mut send_a, 0.0..=1.0).show_value(false).text("A"));
            ui.add(egui::Slider::new(&mut send_b, 0.0..=1.0).show_value(false).text("B"));

            if let Some(track) = song.tracks.get_mut(ti) {
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
                ui.horizontal(|ui| {
                    meter_bar(ui, peak, 10.0, 70.0);
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
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(track.mixer.mute, RichText::new("M").small())
                        .clicked()
                    {
                        track.mixer.mute = !track.mixer.mute;
                        *dirty = true;
                    }
                    if ui
                        .selectable_label(track.mixer.solo, RichText::new("S").small())
                        .clicked()
                    {
                        track.mixer.solo = !track.mixer.solo;
                        *dirty = true;
                    }
                });
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
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height.max(80.0)),
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
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            egui::Stroke::new(1.0_f32, color),
        );
    }

    for (ti, track) in song.tracks.iter().enumerate() {
        let y = rect.min.y + 20.0 + ti as f32 * 28.0;
        painter.text(
            egui::pos2(rect.min.x + 4.0, y),
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
        let r = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w.max(4.0), 24.0));
        painter.rect_filled(r, 0.0, ArcLiveTheme::track_clip_color(ti));
        painter.text(
            r.min + egui::vec2(4.0, 4.0),
            egui::Align2::LEFT_TOP,
            &clip.name,
            egui::FontId::proportional(10.0),
            ArcLiveTheme::TEXT,
        );
        let id = ui.id().with(("arr", ci));
        let resp = ui.interact(r, id, Sense::click());
        if resp.clicked() {
            on_select(ci);
        }
    }
}

pub fn piano_roll(ui: &mut Ui, song: &Song, track_index: usize) {
    ui.label(RichText::new("Piano Roll").color(ArcLiveTheme::MUTED));
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
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click_and_drag());
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
            let y0 = mid - mx * rect.height() * 0.45;
            let y1 = mid - mn * rect.height() * 0.45;
            painter.line_segment(
                [egui::pos2(x, y0), egui::pos2(x, y1)],
                egui::Stroke::new(1.0_f32, ArcLiveTheme::WAVE_FG),
            );
        }
    }

    // Markers
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
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            egui::Stroke::new(2.0_f32, color),
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
            // upper half adds marker
            if pos.y < rect.center().y {
                clip.warp_markers
                    .push(arclive_core::WarpMarker::new(frame, beat));
                clip.sort_markers();
                *selected_marker = clip
                    .warp_markers
                    .iter()
                    .position(|m| (m.beat - beat).abs() < 1e-6)
                    .unwrap_or(0);
            } else {
                // select nearest
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
                // shift-drag: move sample under marker
                let frame = ((beat / end_beat) * frames as f64).max(0.0) as u64;
                clip.warp_markers[idx].sample_frame = frame;
            } else {
                clip.warp_markers[idx].beat = beat;
            }
            clip.sort_markers();
        }
    }
}

pub fn sample_browser(ui: &mut Ui, files: &[String], on_import_dialog: &mut dyn FnMut()) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Samples").strong());
        if ui.small_button("Browse…").clicked() {
            on_import_dialog();
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if files.is_empty() {
            ui.label(
                RichText::new("Drop WAVs here or browse.\nScan: ./samples")
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

// silence unused import warning helpers
#[allow(dead_code)]
fn _dnd_touch() {
    let _ = DragAndDrop::has_payload_of_type::<String>;
}