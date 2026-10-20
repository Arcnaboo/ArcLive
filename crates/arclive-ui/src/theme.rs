use egui::{Color32, CornerRadius, Stroke, Visuals};

/// Live 7–era charcoal density. ArcLive palette — not Ableton branding.
pub struct ArcLiveTheme;

impl ArcLiveTheme {
    pub const BG: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
    pub const PANEL: Color32 = Color32::from_rgb(0x32, 0x32, 0x32);
    pub const PANEL_ALT: Color32 = Color32::from_rgb(0x3c, 0x3c, 0x3c);
    pub const SLOT: Color32 = Color32::from_rgb(0x48, 0x48, 0x48);
    pub const TEXT: Color32 = Color32::from_rgb(0xee, 0xee, 0xee);
    pub const MUTED: Color32 = Color32::from_rgb(0x9a, 0x9a, 0x9a);
    pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0xcc, 0x00); // launch / playing yellow
    pub const RECORD: Color32 = Color32::from_rgb(0xd0, 0x32, 0x28);
    pub const ACTIVATOR_ON: Color32 = Color32::from_rgb(0x6a, 0xc0, 0x4a);
    pub const CLIP_A: Color32 = Color32::from_rgb(0x58, 0xa0, 0x58);
    pub const CLIP_B: Color32 = Color32::from_rgb(0xb8, 0x50, 0x50);
    pub const CLIP_C: Color32 = Color32::from_rgb(0x50, 0x78, 0xb0);
    pub const CLIP_D: Color32 = Color32::from_rgb(0xa0, 0x80, 0x40);
    pub const WAVE_BG: Color32 = Color32::from_rgb(0xf0, 0xee, 0xe6);
    pub const WAVE_FG: Color32 = Color32::from_rgb(0x20, 0x20, 0x20);
    pub const GRID: Color32 = Color32::from_rgb(0x50, 0x50, 0x50);
    pub const STOP: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x5a);
    pub const BORDER: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);

    pub fn track_clip_color(track_index: usize) -> Color32 {
        match track_index % 4 {
            0 => Self::CLIP_A,
            1 => Self::CLIP_B,
            2 => Self::CLIP_C,
            _ => Self::CLIP_D,
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
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x4a, 0x4a, 0x4a);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0x55, 0x55, 0x55);
        visuals.selection.bg_fill = Self::ACCENT.linear_multiply(0.35);
        visuals.override_text_color = Some(Self::TEXT);
        let stroke = Stroke::new(1.0_f32, Self::BORDER);
        visuals.widgets.noninteractive.bg_stroke = stroke;
        visuals.widgets.inactive.bg_stroke = stroke;
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