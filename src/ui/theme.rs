use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

pub fn glass_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(22, 24, 32, 175)
}

pub fn glass_stroke() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 28)
}

pub fn title_bar_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(16, 18, 26, 195)
}

pub fn title_bar_stroke() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 18)
}

pub fn close_hover() -> Color32 {
    Color32::from_rgb(232, 17, 35)
}

pub fn maximize_hover() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 24)
}

pub fn minimize_hover() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 24)
}

pub fn text_primary() -> Color32 {
    Color32::from_rgba_unmultiplied(235, 238, 245, 240)
}

pub fn text_muted() -> Color32 {
    Color32::from_rgba_unmultiplied(160, 168, 185, 210)
}

pub fn selection_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 120, 215, 200)
}

pub fn selection_stroke() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 90)
}

pub fn selection_text() -> Color32 {
    Color32::from_rgb(255, 255, 255)
}

pub fn marquee_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 120, 215, 90)
}

pub fn marquee_stroke() -> Color32 {
    Color32::from_rgb(0, 120, 215)
}

pub fn selection_preview_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 120, 215, 140)
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.window_fill = glass_fill();
    visuals.panel_fill = glass_fill();
    visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(12, 14, 20, 200);
    visuals.faint_bg_color = Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    visuals.window_stroke = Stroke::new(1.0, glass_stroke());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_muted());
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text_primary());
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text_primary());
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, text_primary());
    visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
    visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
    visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 26);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 6);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 12);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(90, 130, 220, 120);
    visuals.hyperlink_color = Color32::from_rgb(120, 170, 255);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(6);
    ctx.set_visuals(visuals);
    // File lists swap widget trees on navigation; disable egui's dev-only red outline.
    ctx.global_style_mut(|style| {
        style.debug.warn_if_rect_changes_id = false;
    });
}
