use std::cell::Cell;

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

use crate::explorer::ThemePreset;

#[derive(Clone, Copy)]
struct ThemePalette {
    base_visual: BaseVisual,
    glass_fill: Color32,
    glass_stroke: Color32,
    title_bar_fill: Color32,
    title_bar_stroke: Color32,
    maximize_hover: Color32,
    minimize_hover: Color32,
    text_primary: Color32,
    text_muted: Color32,
    default_icon: Color32,
    selection_fill: Color32,
    selection_stroke: Color32,
    selection_text: Color32,
    marquee_fill: Color32,
    marquee_stroke: Color32,
    selection_preview_fill: Color32,
    extreme_bg: Color32,
    faint_bg: Color32,
    hyperlink: Color32,
}

#[derive(Clone, Copy)]
enum BaseVisual {
    Dark,
    Light,
}

thread_local! {
    static ACTIVE_THEME: Cell<ThemePreset> = const { Cell::new(ThemePreset::GlassSquid) };
}

fn active_theme() -> ThemePreset {
    ACTIVE_THEME.with(|cell| cell.get())
}

fn set_active_theme(preset: ThemePreset) {
    ACTIVE_THEME.with(|cell| cell.set(preset));
}

fn palette_for(preset: ThemePreset) -> ThemePalette {
    match preset {
        ThemePreset::GlassSquid => ThemePalette {
            base_visual: BaseVisual::Dark,
            glass_fill: Color32::from_rgba_unmultiplied(22, 24, 32, 175),
            glass_stroke: Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            title_bar_fill: Color32::from_rgba_unmultiplied(16, 18, 26, 195),
            title_bar_stroke: Color32::from_rgba_unmultiplied(255, 255, 255, 18),
            maximize_hover: Color32::from_rgba_unmultiplied(255, 255, 255, 24),
            minimize_hover: Color32::from_rgba_unmultiplied(255, 255, 255, 24),
            text_primary: Color32::from_rgba_unmultiplied(235, 238, 245, 240),
            text_muted: Color32::from_rgba_unmultiplied(160, 168, 185, 210),
            default_icon: Color32::from_rgba_unmultiplied(235, 238, 245, 240),
            selection_fill: Color32::from_rgba_unmultiplied(0, 120, 215, 200),
            selection_stroke: Color32::from_rgba_unmultiplied(255, 255, 255, 90),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(0, 120, 215, 90),
            marquee_stroke: Color32::from_rgb(0, 120, 215),
            selection_preview_fill: Color32::from_rgba_unmultiplied(0, 120, 215, 140),
            extreme_bg: Color32::from_rgba_unmultiplied(12, 14, 20, 200),
            faint_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 8),
            hyperlink: Color32::from_rgb(120, 170, 255),
        },
        ThemePreset::Glacier => ThemePalette {
            base_visual: BaseVisual::Light,
            glass_fill: Color32::from_rgba_unmultiplied(244, 248, 255, 132),
            glass_stroke: Color32::from_rgba_unmultiplied(20, 28, 40, 25),
            title_bar_fill: Color32::from_rgba_unmultiplied(236, 242, 252, 136),
            title_bar_stroke: Color32::from_rgba_unmultiplied(25, 33, 46, 22),
            maximize_hover: Color32::from_rgba_unmultiplied(15, 25, 35, 12),
            minimize_hover: Color32::from_rgba_unmultiplied(15, 25, 35, 12),
            text_primary: Color32::from_rgba_unmultiplied(16, 24, 36, 220),
            text_muted: Color32::from_rgba_unmultiplied(58, 70, 88, 180),
            default_icon: Color32::from_rgba_unmultiplied(10, 10, 10, 220),
            selection_fill: Color32::from_rgba_unmultiplied(71, 140, 230, 160),
            selection_stroke: Color32::from_rgba_unmultiplied(255, 255, 255, 128),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(71, 140, 230, 55),
            marquee_stroke: Color32::from_rgb(52, 121, 214),
            selection_preview_fill: Color32::from_rgba_unmultiplied(71, 140, 230, 90),
            extreme_bg: Color32::from_rgba_unmultiplied(222, 230, 244, 180),
            faint_bg: Color32::from_rgba_unmultiplied(0, 0, 0, 5),
            hyperlink: Color32::from_rgb(45, 112, 215),
        },
        ThemePreset::Smoky => ThemePalette {
            base_visual: BaseVisual::Light,
            // All background colors are much darker; text/icons are white.
            glass_fill: Color32::from_rgba_unmultiplied(36, 36, 38, 182),
            glass_stroke: Color32::from_rgba_unmultiplied(20, 20, 22, 30),
            title_bar_fill: Color32::from_rgba_unmultiplied(28, 28, 32, 190),
            title_bar_stroke: Color32::from_rgba_unmultiplied(18, 18, 20, 26),
            maximize_hover: Color32::from_rgba_unmultiplied(22, 22, 24, 24),
            minimize_hover: Color32::from_rgba_unmultiplied(22, 22, 24, 24),
            text_primary: Color32::from_rgba_unmultiplied(255, 255, 255, 245),
            text_muted: Color32::from_rgba_unmultiplied(210, 210, 210, 180),
            default_icon: Color32::from_rgba_unmultiplied(255, 255, 255, 245),
            selection_fill: Color32::from_rgba_unmultiplied(68, 78, 98, 176),
            selection_stroke: Color32::from_rgba_unmultiplied(248, 248, 252, 138),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(68, 78, 98, 74),
            marquee_stroke: Color32::from_rgb(130, 138, 164),
            selection_preview_fill: Color32::from_rgba_unmultiplied(68, 78, 98, 112),
            extreme_bg: Color32::from_rgba_unmultiplied(24, 24, 28, 194),
            faint_bg: Color32::from_rgba_unmultiplied(13, 13, 15, 12),
            hyperlink: Color32::from_rgb(152, 181, 255),
        },

        ThemePreset::Baltic => ThemePalette {
            base_visual: BaseVisual::Light,
            glass_fill: Color32::from_rgba_unmultiplied(241, 238, 232, 156),
            glass_stroke: Color32::from_rgba_unmultiplied(70, 58, 46, 30),
            title_bar_fill: Color32::from_rgba_unmultiplied(233, 228, 220, 164),
            title_bar_stroke: Color32::from_rgba_unmultiplied(72, 60, 48, 28),
            maximize_hover: Color32::from_rgba_unmultiplied(70, 58, 46, 16),
            minimize_hover: Color32::from_rgba_unmultiplied(70, 58, 46, 16),
            text_primary: Color32::from_rgba_unmultiplied(52, 40, 32, 245),
            text_muted: Color32::from_rgba_unmultiplied(92, 78, 66, 220),
            default_icon: Color32::from_rgba_unmultiplied(58, 44, 34, 245),
            selection_fill: Color32::from_rgba_unmultiplied(178, 132, 94, 192),
            selection_stroke: Color32::from_rgba_unmultiplied(250, 239, 226, 150),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(178, 132, 94, 84),
            marquee_stroke: Color32::from_rgb(162, 117, 82),
            selection_preview_fill: Color32::from_rgba_unmultiplied(178, 132, 94, 126),
            extreme_bg: Color32::from_rgba_unmultiplied(220, 212, 201, 214),
            faint_bg: Color32::from_rgba_unmultiplied(38, 26, 18, 6),
            hyperlink: Color32::from_rgb(132, 93, 62),
        },
        ThemePreset::Nebula => ThemePalette {
            base_visual: BaseVisual::Dark,
            glass_fill: Color32::from_rgba_unmultiplied(36, 26, 56, 192),
            glass_stroke: Color32::from_rgba_unmultiplied(205, 180, 255, 46),
            title_bar_fill: Color32::from_rgba_unmultiplied(30, 21, 48, 210),
            title_bar_stroke: Color32::from_rgba_unmultiplied(225, 195, 255, 34),
            maximize_hover: Color32::from_rgba_unmultiplied(210, 160, 255, 36),
            minimize_hover: Color32::from_rgba_unmultiplied(210, 160, 255, 36),
            text_primary: Color32::from_rgba_unmultiplied(66, 96, 156, 245),
            text_muted: Color32::from_rgba_unmultiplied(136, 145, 183, 225),
            default_icon: Color32::from_rgba_unmultiplied(66, 96, 156, 245),
            selection_fill: Color32::from_rgba_unmultiplied(176, 106, 245, 190),
            selection_stroke: Color32::from_rgba_unmultiplied(246, 223, 255, 140),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(176, 106, 245, 84),
            marquee_stroke: Color32::from_rgb(176, 106, 245),
            selection_preview_fill: Color32::from_rgba_unmultiplied(176, 106, 245, 134),
            extreme_bg: Color32::from_rgba_unmultiplied(18, 14, 28, 220),
            faint_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 9),
            hyperlink: Color32::from_rgb(186, 146, 255),
        },
        ThemePreset::Prismarine => ThemePalette {
            base_visual: BaseVisual::Dark,
            glass_fill: Color32::from_rgba_unmultiplied(18, 54, 72, 186),
            glass_stroke: Color32::from_rgba_unmultiplied(182, 240, 255, 42),
            title_bar_fill: Color32::from_rgba_unmultiplied(16, 45, 62, 208),
            title_bar_stroke: Color32::from_rgba_unmultiplied(207, 246, 255, 34),
            maximize_hover: Color32::from_rgba_unmultiplied(175, 241, 255, 30),
            minimize_hover: Color32::from_rgba_unmultiplied(175, 241, 255, 30),
            text_primary: Color32::from_rgba_unmultiplied(245, 252, 255, 245),
            text_muted: Color32::from_rgba_unmultiplied(172, 210, 220, 224),
            default_icon: Color32::from_rgba_unmultiplied(245, 252, 255, 245),
            selection_fill: Color32::from_rgba_unmultiplied(74, 178, 198, 204),
            selection_stroke: Color32::from_rgba_unmultiplied(245, 255, 255, 145),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(74, 178, 198, 92),
            marquee_stroke: Color32::from_rgb(74, 178, 198),
            selection_preview_fill: Color32::from_rgba_unmultiplied(74, 178, 198, 138),
            extreme_bg: Color32::from_rgba_unmultiplied(9, 28, 40, 220),
            faint_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 8),
            hyperlink: Color32::from_rgb(120, 219, 240),
        },
        ThemePreset::Aurora => ThemePalette {
            base_visual: BaseVisual::Dark,
            glass_fill: Color32::from_rgba_unmultiplied(22, 44, 52, 188),
            glass_stroke: Color32::from_rgba_unmultiplied(168, 240, 228, 40),
            title_bar_fill: Color32::from_rgba_unmultiplied(18, 37, 44, 210),
            title_bar_stroke: Color32::from_rgba_unmultiplied(188, 240, 234, 34),
            maximize_hover: Color32::from_rgba_unmultiplied(165, 240, 230, 28),
            minimize_hover: Color32::from_rgba_unmultiplied(165, 240, 230, 28),
            text_primary: Color32::from_rgba_unmultiplied(210, 245, 238, 242),
            text_muted: Color32::from_rgba_unmultiplied(145, 196, 188, 222),
            default_icon: Color32::from_rgba_unmultiplied(184, 236, 227, 245),
            selection_fill: Color32::from_rgba_unmultiplied(64, 186, 161, 196),
            selection_stroke: Color32::from_rgba_unmultiplied(224, 255, 247, 140),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(64, 186, 161, 86),
            marquee_stroke: Color32::from_rgb(64, 186, 161),
            selection_preview_fill: Color32::from_rgba_unmultiplied(64, 186, 161, 132),
            extreme_bg: Color32::from_rgba_unmultiplied(10, 24, 30, 218),
            faint_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 8),
            hyperlink: Color32::from_rgb(110, 229, 212),
        },
        ThemePreset::EmberMist => ThemePalette {
            base_visual: BaseVisual::Dark,
            glass_fill: Color32::from_rgba_unmultiplied(46, 30, 28, 188),
            glass_stroke: Color32::from_rgba_unmultiplied(255, 201, 165, 42),
            title_bar_fill: Color32::from_rgba_unmultiplied(38, 24, 22, 210),
            title_bar_stroke: Color32::from_rgba_unmultiplied(255, 205, 170, 34),
            maximize_hover: Color32::from_rgba_unmultiplied(255, 184, 143, 30),
            minimize_hover: Color32::from_rgba_unmultiplied(255, 184, 143, 30),
            text_primary: Color32::from_rgba_unmultiplied(250, 232, 220, 242),
            text_muted: Color32::from_rgba_unmultiplied(214, 172, 150, 225),
            default_icon: Color32::from_rgba_unmultiplied(248, 219, 194, 245),
            selection_fill: Color32::from_rgba_unmultiplied(221, 118, 66, 202),
            selection_stroke: Color32::from_rgba_unmultiplied(255, 232, 214, 140),
            selection_text: Color32::from_rgb(255, 255, 255),
            marquee_fill: Color32::from_rgba_unmultiplied(221, 118, 66, 88),
            marquee_stroke: Color32::from_rgb(221, 118, 66),
            selection_preview_fill: Color32::from_rgba_unmultiplied(221, 118, 66, 138),
            extreme_bg: Color32::from_rgba_unmultiplied(24, 15, 14, 220),
            faint_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 7),
            hyperlink: Color32::from_rgb(255, 182, 128),
        },
    }
}

fn palette() -> ThemePalette {
    palette_for(active_theme())
}

pub fn apply_with_preset(ctx: &egui::Context, preset: ThemePreset) {
    set_active_theme(preset);
    let palette = palette_for(preset);
    let mut visuals = match palette.base_visual {
        BaseVisual::Dark => Visuals::dark(),
        BaseVisual::Light => Visuals::light(),
    };
    visuals.window_fill = palette.glass_fill;
    visuals.panel_fill = palette.glass_fill;
    visuals.extreme_bg_color = palette.extreme_bg;
    visuals.faint_bg_color = palette.faint_bg;
    visuals.window_stroke = Stroke::new(1.0, palette.glass_stroke);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_muted);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text_primary);

    if matches!(palette.base_visual, BaseVisual::Light) {
        visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(0, 0, 0, 6);
        visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(0, 0, 0, 10);
        visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(0, 0, 0, 14);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(0, 0, 0, 5);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(0, 0, 0, 8);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(0, 0, 0, 12);
    } else {
        visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
        visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 26);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 6);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 12);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
    }

    visuals.selection.bg_fill = palette.selection_preview_fill;
    visuals.hyperlink_color = palette.hyperlink;
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(6);
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.debug.warn_if_rect_changes_id = false;
    });
}

pub fn glass_fill() -> Color32 {
    palette().glass_fill
}

pub fn glass_stroke() -> Color32 {
    palette().glass_stroke
}

pub fn title_bar_fill() -> Color32 {
    palette().title_bar_fill
}

pub fn title_bar_stroke() -> Color32 {
    palette().title_bar_stroke
}

pub fn close_hover() -> Color32 {
    Color32::from_rgb(232, 17, 35)
}

pub fn maximize_hover() -> Color32 {
    palette().maximize_hover
}

pub fn minimize_hover() -> Color32 {
    palette().minimize_hover
}

pub fn text_primary() -> Color32 {
    palette().text_primary
}

pub fn text_muted() -> Color32 {
    palette().text_muted
}

pub fn default_icon_color() -> Color32 {
    palette().default_icon
}

pub fn selection_fill() -> Color32 {
    palette().selection_fill
}

pub fn selection_stroke() -> Color32 {
    palette().selection_stroke
}

pub fn selection_text() -> Color32 {
    palette().selection_text
}

pub fn marquee_fill() -> Color32 {
    palette().marquee_fill
}

pub fn marquee_stroke() -> Color32 {
    palette().marquee_stroke
}

pub fn selection_preview_fill() -> Color32 {
    palette().selection_preview_fill
}
