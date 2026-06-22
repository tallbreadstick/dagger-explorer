use eframe::egui::{self, Ui, vec2};

use crate::explorer::ExplorerState;

use super::{file_view, loading_bar, status_bar, toolbar};

pub const PANEL_INSET: f32 = 12.0;
const STATUS_GAP: f32 = 8.0;
const LOADING_BAR_SLOT: f32 = loading_bar::LOADING_BAR_HEIGHT;

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    let panel = ui.available_rect_before_wrap();
    let inset = PANEL_INSET;

    ui.allocate_ui(vec2(panel.width(), toolbar::TOOLBAR_HEIGHT), |ui| {
        ui.set_width(panel.width());
        ui.set_max_width(panel.width());
        toolbar::show(ui, state);
    });

    ui.allocate_ui(vec2(panel.width(), LOADING_BAR_SLOT), |ui| {
        ui.set_width(panel.width());
        ui.set_max_width(panel.width());
        loading_bar::show(ui, state);
    });

    let content = ui.available_rect_before_wrap();
    let status_height = status_bar::STATUS_BAR_HEIGHT;
    let inset_width = (content.width() - inset * 2.0).max(0.0);
    let file_view_height = (content.height()
        - status_height
        - STATUS_GAP
        - inset)
        .max(0.0);

    state.file_view_bounds = Some(egui::Rect::from_min_max(
        egui::pos2(content.min.x + inset, content.min.y),
        egui::pos2(content.max.x - inset, content.min.y + file_view_height),
    ));

    ui.horizontal(|ui| {
        ui.add_space(inset);

        ui.vertical(|ui| {
            ui.set_width(inset_width);
            ui.set_max_width(inset_width);

            ui.allocate_ui(vec2(inset_width, file_view_height), |ui| {
                ui.set_width(inset_width);
                ui.set_max_width(inset_width);
                ui.set_min_height(file_view_height);
                file_view::show(ui, state);
            });

            ui.add_space(STATUS_GAP);

            ui.allocate_ui(vec2(inset_width, status_height), |ui| {
                ui.set_width(inset_width);
                ui.set_max_width(inset_width);
                status_bar::show(ui, state);
            });

            ui.add_space(inset);
        });

        ui.add_space(inset);
    });
}
