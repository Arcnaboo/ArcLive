use egui::{Color32, CornerRadius, Stroke, Visuals};

/// Live-era charcoal density; original ArcLive palette (not Ableton branding).
pub struct ArcLiveTheme;

impl ArcLiveTheme {
    pub const BG: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
    pub const PANEL: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
    pub const PANEL_ALT: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
    pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xe8);
    pub const MUTED: Color32 = Color32::from_rgb(0x9a, 0x9a, 0x9a);
    pub const ACCENT: Color32 = Color32::from_rgb(0xe8, 0xc4, 0x40); // amber warp accent
    pub const RECORD: Color32 = Color32::from_rgb(0xc4, 0x3c, 0x32);
    pub const CLIP_A: Color32 = Color32::from_rgb(0x4a, 0x9a, 0x4a);
    pub const CLIP_B: Color32 = Color32::from_rgb(0xb0, 0x4a, 0x4a);
    pub const CLIP_C: Color32 = Color32::from_rgb(0x4a, 0x7a, 0xa8);
    pub const WAVE_BG: Color32 = Color32::from_rgb(0xf2, 0xf0, 0xe8);
    pub const WAVE_FG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
    pub const GRID: Color32 = Color32::from_rgb(0x48, 0x48, 0x48);
    pub const STOP: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);

    pub fn track_clip_color(track_index: usize) -> Color32 {
        match track_index % 3 {
            0 => Self::CLIP_A,
            1 => Self::CLIP_B,
            _ => Self::CLIP_C,
        }
    }

    pub fn apply(ctx: &egui::Context) {
        let mut visuals = Visuals::dark();
        visuals.panel_fill = Self::PANEL;
        visuals.window_fill = Self::BG;
        visuals.extreme_bg_color = Self::BG;
        visuals.faint_bg_color = Self::PANEL_ALT;
        visuals.widgets.noninteractive.bg_fill = Self::PANEL;
        visuals.widgets.inactive.bg_fill = Self::PANEL_ALT;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x45, 0x45, 0x45);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0x50, 0x50, 0x50);
        visuals.selection.bg_fill = Self::ACCENT.linear_multiply(0.35);
        visuals.override_text_color = Some(Self::TEXT);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Self::MUTED);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Self::TEXT);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Self::TEXT);
        visuals.window_corner_radius = CornerRadius::same(0);
        visuals.menu_corner_radius = CornerRadius::same(0);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(0);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(0);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(0);
        visuals.widgets.active.corner_radius = CornerRadius::same(0);
        ctx.set_visuals(visuals);
    }
}