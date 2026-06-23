use eframe::egui::{self, Area, Frame, Id, Order, vec2};

use crate::explorer::{
    ExplorerState, Keybind, KeybindAction, SettingsTab, ShortcutKey, ThemePreset,
};

use super::theme;

const DROPDOWN_WIDTH: f32 = 660.0;
const DROPDOWN_Y: f32 = 42.0;

pub fn show(ctx: &egui::Context, state: &mut ExplorerState) {
    if !state.settings_dialog_open {
        return;
    }

    let dropdown = Area::new(Id::new("settings_dropdown_area"))
        .order(Order::Foreground)
        .fixed_pos(egui::pos2(
            (ctx.content_rect().center().x - DROPDOWN_WIDTH * 0.5).max(10.0),
            DROPDOWN_Y,
        ))
        .show(ctx, |ui| {
            Frame::new()
                .fill(theme::title_bar_fill())
                .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_min_width(DROPDOWN_WIDTH);
                    ui.set_max_width(DROPDOWN_WIDTH);

                    ui.horizontal(|ui| {
                        tab_button(ui, state, SettingsTab::Keybinds, "Keybinds");
                        tab_button(ui, state, SettingsTab::Themes, "Themes");
                    });
                    ui.separator();
                    ui.add_space(4.0);

                    match state.settings_tab {
                        SettingsTab::Keybinds => show_keybind_settings(ui, state),
                        SettingsTab::Themes => show_theme_settings(ui, state),
                    }
                });
        });

    let outside_click = ctx.input(|input| {
        input.pointer.primary_clicked()
            && input
                .pointer
                .latest_pos()
                .is_some_and(|pos| !dropdown.response.rect.contains(pos))
    });
    if outside_click {
        if state.settings_ignore_next_outside_click {
            state.settings_ignore_next_outside_click = false;
        } else {
            state.close_settings_dialog();
        }
    }
}

fn tab_button(ui: &mut egui::Ui, state: &mut ExplorerState, tab: SettingsTab, label: &str) {
    let selected = state.settings_tab == tab;
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new(label).size(12.0).color(if selected {
                    theme::selection_text()
                } else {
                    theme::text_primary()
                }),
            )
            .fill(if selected {
                theme::selection_fill()
            } else {
                egui::Color32::TRANSPARENT
            })
            .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
            .min_size(vec2(96.0, 26.0)),
        )
        .clicked()
    {
        state.settings_tab = tab;
    }
}

fn show_keybind_settings(ui: &mut egui::Ui, state: &mut ExplorerState) {
    ui.label(
        egui::RichText::new("Click a keybind to remap it.")
            .size(11.0)
            .color(theme::text_muted()),
    );
    ui.add_space(8.0);

    let mut captured: Option<(KeybindAction, Keybind)> = None;
    if let Some(action) = state.capturing_keybind_action {
        captured = capture_keybind(ui).map(|binding| (action, binding));
    }

    egui::Grid::new("settings_keybind_grid")
        .num_columns(4)
        .spacing(vec2(10.0, 8.0))
        .striped(true)
        .show(ui, |ui| {
            for action in KeybindAction::ALL {
                ui.label(
                    egui::RichText::new(action.label())
                        .size(12.0)
                        .color(theme::text_primary()),
                );
                let active_capture = state.capturing_keybind_action == Some(action);
                let bind_text = if active_capture {
                    "Press keys..."
                } else {
                    &state.keybind_label(action)
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(bind_text)
                                .size(11.0)
                                .color(theme::text_primary()),
                        )
                        .min_size(vec2(170.0, 24.0)),
                    )
                    .clicked()
                {
                    state.capturing_keybind_action = Some(action);
                }

                if ui
                    .add_enabled(
                        state.keybind_for(action).is_some(),
                        egui::Button::new("Clear").min_size(vec2(52.0, 24.0)),
                    )
                    .clicked()
                {
                    state.set_keybind(action, None);
                    if active_capture {
                        state.capturing_keybind_action = None;
                    }
                }

                if ui
                    .add_enabled(
                        !state.keybind_is_default(action),
                        egui::Button::new("Reset").min_size(vec2(52.0, 24.0)),
                    )
                    .clicked()
                {
                    state.reset_keybind_to_default(action);
                    if active_capture {
                        state.capturing_keybind_action = None;
                    }
                }
                ui.end_row();
            }
        });

    if let Some((action, binding)) = captured {
        state.set_keybind(action, Some(binding));
        state.capturing_keybind_action = None;
    }

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new("Go Back / Go Forward / Up One Level are unassigned by default.")
            .size(11.0)
            .color(theme::text_muted()),
    );
}

fn capture_keybind(ui: &egui::Ui) -> Option<Keybind> {
    use egui::Event;

    ui.input(|input| {
        input.events.iter().rev().find_map(|event| {
            let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return None;
            };
            let shortcut_key = ShortcutKey::from_egui(*key)?;
            Some(Keybind {
                command: modifiers.command,
                shift: modifiers.shift,
                alt: modifiers.alt,
                key: shortcut_key,
            })
        })
    })
}

fn show_theme_settings(ui: &mut egui::Ui, state: &mut ExplorerState) {
    ui.label(
        egui::RichText::new("User icon colors always override theme default icon colors.")
            .size(11.0)
            .color(theme::text_muted()),
    );
    ui.add_space(8.0);

    egui::Grid::new("theme_cards_grid")
        .num_columns(2)
        .spacing(vec2(10.0, 10.0))
        .show(ui, |ui| {
            for (index, preset) in ThemePreset::ALL.iter().copied().enumerate() {
                theme_card(ui, state, preset);
                if index % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

fn theme_card(ui: &mut egui::Ui, state: &mut ExplorerState, preset: ThemePreset) {
    let selected = state.theme_preset() == preset;
    egui::Frame::new()
        .fill(if selected {
            theme::selection_preview_fill()
        } else {
            theme::glass_fill()
        })
        .stroke(egui::Stroke::new(
            if selected { 2.0 } else { 1.0 },
            if selected {
                theme::selection_stroke()
            } else {
                theme::glass_stroke()
            },
        ))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(306.0, 74.0));
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(preset.label())
                            .size(13.0)
                            .color(theme::text_primary()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for swatch in theme_swatches(preset).iter().rev() {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(16.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, *swatch);
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, theme::glass_stroke()),
                                egui::StrokeKind::Inside,
                            );
                            ui.add_space(4.0);
                        }
                    });
                });
                ui.add_space(8.0);
                let button_label = if selected { "Active" } else { "Use Theme" };
                if ui
                    .add_sized(
                        vec2(96.0, 24.0),
                        egui::Button::new(button_label),
                    )
                    .clicked()
                {
                    state.set_theme_preset(preset);
                }
            });
        });
}

fn theme_swatches(preset: ThemePreset) -> [egui::Color32; 4] {
    match preset {
        ThemePreset::GlassSquid => [
            egui::Color32::from_rgb(22, 24, 32),
            egui::Color32::from_rgb(16, 18, 26),
            egui::Color32::from_rgb(90, 130, 220),
            egui::Color32::from_rgb(235, 238, 245),
        ],
        ThemePreset::Glacier => [
            egui::Color32::from_rgb(244, 248, 255),
            egui::Color32::from_rgb(236, 242, 252),
            egui::Color32::from_rgb(71, 140, 230),
            egui::Color32::from_rgb(10, 10, 10),
        ],
        ThemePreset::Smoky => [
            egui::Color32::from_rgb(36, 36, 38),
            egui::Color32::from_rgb(28, 28, 32),
            egui::Color32::from_rgb(68, 78, 98),
            egui::Color32::from_rgb(255, 255, 255),
        ],
        ThemePreset::Baltic => [
            egui::Color32::from_rgb(241, 238, 232),
            egui::Color32::from_rgb(233, 228, 220),
            egui::Color32::from_rgb(178, 132, 94),
            egui::Color32::from_rgb(58, 44, 34),
        ],
        ThemePreset::Nebula => [
            egui::Color32::from_rgb(36, 26, 56),
            egui::Color32::from_rgb(30, 21, 48),
            egui::Color32::from_rgb(176, 106, 245),
            egui::Color32::from_rgb(66, 96, 156),
        ],
        ThemePreset::Prismarine => [
            egui::Color32::from_rgb(18, 54, 72),
            egui::Color32::from_rgb(16, 45, 62),
            egui::Color32::from_rgb(74, 178, 198),
            egui::Color32::from_rgb(245, 252, 255),
        ],
        ThemePreset::Aurora => [
            egui::Color32::from_rgb(22, 44, 52),
            egui::Color32::from_rgb(18, 37, 44),
            egui::Color32::from_rgb(64, 186, 161),
            egui::Color32::from_rgb(184, 236, 227),
        ],
        ThemePreset::EmberMist => [
            egui::Color32::from_rgb(46, 30, 28),
            egui::Color32::from_rgb(38, 24, 22),
            egui::Color32::from_rgb(221, 118, 66),
            egui::Color32::from_rgb(248, 219, 194),
        ],
    }
}
