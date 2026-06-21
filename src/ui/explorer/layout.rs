use eframe::egui::{Ui, vec2};

use crate::explorer::ExplorerState;

use super::{file_panel, nav_bar, sidebar, tab_bar, toast};

const TAB_BAR_HEIGHT: f32 = tab_bar::TAB_HEIGHT;
const NAV_BAR_HEIGHT: f32 = nav_bar::NAV_HEIGHT;

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    let width = ui.available_width();

    ui.allocate_ui(vec2(width, TAB_BAR_HEIGHT), |ui| {
        ui.set_width(width);
        ui.set_max_width(width);
        tab_bar::show(ui, state);
    });

    ui.allocate_ui(vec2(width, NAV_BAR_HEIGHT), |ui| {
        ui.set_width(width);
        ui.set_max_width(width);
        nav_bar::show(ui, state);
    });

    let body = ui.available_rect_before_wrap();
    ui.allocate_ui(body.size(), |ui| {
        ui.set_width(body.width());
        ui.set_max_width(body.width());
        ui.set_min_height(body.height());
        sidebar::show(ui, state);
        file_panel::show(ui, state);
    });

    toast::show(ui.ctx(), state);
}
