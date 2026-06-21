use std::path::PathBuf;

use eframe::egui::{self, ScrollArea, Ui, vec2};

use crate::explorer::{ExplorerState, path_components};
use crate::ui::{theme, text};

pub const NAV_HEIGHT: f32 = 36.0;
const BTN_SIZE: f32 = 28.0;
const BREADCRUMB_MENU_WIDTH: f32 = 240.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MAX_MENU_ITEMS: usize = 10;

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    let can_back = state.active_tab().can_go_back();
    let can_forward = state.active_tab().can_go_forward();
    let can_up = state.active_tab().can_go_up();
    let current_path = state.active_path();
    let bar_height = ui.available_height();

    egui::Frame::new()
        .fill(theme::title_bar_fill())
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            let row_width = ui.available_width();
            ui.set_width(row_width);
            ui.set_min_width(row_width);
            ui.set_max_width(row_width);
            ui.set_min_height(bar_height);
            ui.set_max_height(bar_height);

            // Outer row: nav buttons | expanding breadcrumb | search
            ui.horizontal(|ui| {
                ui.set_width(row_width);
                ui.set_min_width(row_width);
                ui.set_max_width(row_width);
                ui.spacing_mut().item_spacing.x = 4.0;

                if nav_button(ui, "◀", "Back", can_back).clicked() {
                    state.go_back();
                }
                if nav_button(ui, "▶", "Forward", can_forward).clicked() {
                    state.go_forward();
                }
                if nav_button(ui, "↑", "Up one level", can_up).clicked() {
                    state.go_up();
                }
                if nav_button(ui, "↻", "Refresh", true).clicked() {
                    state.refresh_active();
                }

                ui.add_space(8.0);

                let gap = ui.spacing().item_spacing.x;
                let search_block = BTN_SIZE + gap;
                let breadcrumb_width = (ui.available_width() - search_block).max(0.0);
                let breadcrumb_height = ui.available_height();

                ui.allocate_ui(vec2(breadcrumb_width, breadcrumb_height), |ui| {
                    ui.set_width(breadcrumb_width);
                    ui.set_min_width(breadcrumb_width);
                    ui.set_max_width(breadcrumb_width);

                    egui::Frame::new()
                        .fill(theme::glass_fill())
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
                        .show(ui, |ui| {
                            let frame_width = ui.available_width();
                            ui.set_width(frame_width);
                            ui.set_min_width(frame_width);
                            ui.set_max_width(frame_width);

                            ScrollArea::horizontal()
                                .id_salt("breadcrumb_scroll")
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                                )
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        breadcrumb(
                                            ui,
                                            state,
                                            &current_path,
                                            frame_width,
                                        );
                                    });
                                });
                        });
                });

                ui.add(
                    egui::Button::new("🔍")
                        .min_size(vec2(BTN_SIZE, BTN_SIZE))
                        .fill(egui::Color32::TRANSPARENT),
                )
                .on_hover_text("Search (coming soon)");
            });
        });
}

fn nav_button(ui: &mut Ui, label: &str, tooltip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(
            egui::RichText::new(label)
                .size(14.0)
                .color(if enabled {
                    theme::text_primary()
                } else {
                    theme::text_muted()
                }),
        )
        .min_size(vec2(BTN_SIZE, BTN_SIZE)),
    )
    .on_hover_text(tooltip)
}

fn breadcrumb(
    ui: &mut Ui,
    state: &mut ExplorerState,
    current: &PathBuf,
    available_width: f32,
) {
    let components = path_components(current);
    let segment_budget = if components.is_empty() {
        120.0
    } else {
        (available_width / components.len() as f32).clamp(48.0, 120.0)
    };

    for (index, (label, path)) in components.iter().enumerate() {
        if index > 0 {
            let parent_path = components[index - 1].1.clone();
            chevron_dropdown(ui, state, &parent_path);
        }

        let is_last = index + 1 == components.len();
        let max_w = if is_last {
            segment_budget.max(80.0)
        } else {
            segment_budget
        };

        let display = text::ellipsize(
            ui,
            label,
            egui::FontId::proportional(12.0),
            if is_last {
                theme::text_primary()
            } else {
                theme::text_muted()
            },
            max_w,
        );

        if is_last {
            ui.label(
                egui::RichText::new(display)
                    .size(12.0)
                    .color(theme::text_primary()),
            );
        } else if ui
            .add(
                egui::Button::new(
                    egui::RichText::new(display)
                        .size(12.0)
                        .color(theme::text_muted()),
                )
                .frame(false),
            )
            .clicked()
        {
            state.navigate_active(path.clone());
        }
    }
}

fn chevron_dropdown(ui: &mut Ui, state: &mut ExplorerState, parent_path: &PathBuf) {
    let path = parent_path.clone();
    ui.menu_button(
        egui::RichText::new("›")
            .size(12.0)
            .color(theme::text_muted()),
        |ui| {
            ui.set_min_width(BREADCRUMB_MENU_WIDTH);
            ui.set_max_width(BREADCRUMB_MENU_WIDTH);
            folder_submenu(ui, state, &path);
        },
    );
}

fn folder_submenu(ui: &mut Ui, state: &mut ExplorerState, parent_path: &PathBuf) {
    ui.label(
        egui::RichText::new("Folders")
            .small()
            .color(theme::text_muted()),
    );

    let label_width = BREADCRUMB_MENU_WIDTH - 16.0;
    let font_id = egui::FontId::proportional(12.0);

    if let Some(listing) = state.fs_cache.listing(parent_path) {
        let subdirs: Vec<_> = listing.iter().filter(|e| e.is_dir).collect();
        if subdirs.is_empty() {
            ui.label(
                egui::RichText::new("No subfolders")
                    .small()
                    .color(theme::text_muted()),
            );
        } else if subdirs.len() <= MAX_MENU_ITEMS {
            for entry in subdirs {
                folder_menu_item(ui, state, entry, label_width, &font_id);
            }
        } else {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(MENU_ITEM_HEIGHT * MAX_MENU_ITEMS as f32)
                .show(ui, |ui| {
                    for entry in subdirs {
                        folder_menu_item(ui, state, entry, label_width, &font_id);
                    }
                });
        }
    } else {
        ui.label(
            egui::RichText::new("Loading…")
                .small()
                .color(theme::text_muted()),
        );
        state.ensure_listing(parent_path.clone());
    }
}

fn folder_menu_item(
    ui: &mut Ui,
    state: &mut ExplorerState,
    entry: &crate::explorer::FileEntry,
    label_width: f32,
    font_id: &egui::FontId,
) {
    let display = text::ellipsize(
        ui,
        &entry.name,
        font_id.clone(),
        theme::text_primary(),
        label_width,
    );
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new(display)
                    .size(12.0)
                    .color(theme::text_primary()),
            )
            .frame(false)
            .min_size(vec2(label_width, MENU_ITEM_HEIGHT)),
        )
        .clicked()
    {
        state.navigate_active(entry.path.clone());
        ui.close();
    }
}
