use eframe::egui::{self, Ui, vec2};

use crate::explorer::ExplorerState;

use super::{file_view, status_bar, toolbar};

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    let panel = ui.available_rect_before_wrap();

    ui.allocate_ui(vec2(panel.width(), toolbar::TOOLBAR_HEIGHT), |ui| {
        ui.set_width(panel.width());
        ui.set_max_width(panel.width());
        toolbar::show(ui, state);
    });

    let content = ui.available_rect_before_wrap();
    let file_view_height = (content.height() - status_bar::STATUS_BAR_HEIGHT).max(0.0);
    state.file_view_bounds = Some(egui::Rect::from_min_size(
        content.min,
        vec2(content.width(), file_view_height),
    ));

    ui.allocate_ui(vec2(content.width(), file_view_height), |ui| {
        ui.set_width(content.width());
        ui.set_max_width(content.width());
        ui.set_min_height(file_view_height);
        file_view::show(ui, state);
    });

    ui.allocate_ui(vec2(content.width(), status_bar::STATUS_BAR_HEIGHT), |ui| {
        ui.set_width(content.width());
        ui.set_max_width(content.width());
        status_bar::show(ui, state);
    });
}
