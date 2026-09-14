use eframe::egui::{self, Color32, Stroke};

pub const BG: Color32 = Color32::from_rgb(4, 12, 27);
pub const SIDEBAR: Color32 = Color32::from_rgb(5, 17, 38);
pub const CARD: Color32 = Color32::from_rgb(8, 23, 48);
pub const CARD_HOVER: Color32 = Color32::from_rgb(10, 32, 67);
pub const BORDER: Color32 = Color32::from_rgb(16, 82, 145);
pub const ACCENT: Color32 = Color32::from_rgb(0, 176, 255);
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(43, 223, 255);
pub const TEXT: Color32 = Color32::from_rgb(234, 245, 255);
pub const MUTED: Color32 = Color32::from_rgb(137, 166, 195);
pub const GREEN: Color32 = Color32::from_rgb(48, 214, 123);
pub const RED: Color32 = Color32::from_rgb(255, 83, 101);
pub const PURPLE: Color32 = Color32::from_rgb(132, 97, 255);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = Color32::from_rgb(3, 10, 23);
    visuals.faint_bg_color = Color32::from_rgb(7, 21, 43);
    visuals.code_bg_color = Color32::from_rgb(4, 18, 39);
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.inactive.weak_bg_fill = CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER.gamma_multiply(0.55));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_fill = CARD_HOVER;
    visuals.widgets.hovered.weak_bg_fill = CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.9));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_fill = Color32::from_rgb(8, 64, 116);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(8, 64, 116);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.hyperlink_color = ACCENT_BRIGHT;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size.y = 34.0;
    style.visuals.override_text_color = Some(TEXT);
    ctx.set_style(style);
}
