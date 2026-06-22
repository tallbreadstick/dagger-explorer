use eframe::egui::{Rect, Ui, vec2};

use crate::explorer::ExplorerState;
use crate::ui::theme;

pub const LOADING_BAR_HEIGHT: f32 = 3.0;

pub fn show(ui: &mut Ui, state: &ExplorerState) {
    if !state.directory_loading_bar.visible() {
        return;
    }

    let width = ui.available_width();
    ui.set_width(width);
    ui.set_min_height(LOADING_BAR_HEIGHT);
    ui.set_max_height(LOADING_BAR_HEIGHT);

    let rect = ui.max_rect();
    let fraction = state.directory_loading_bar.fraction();
    let track = theme::glass_stroke();
    let fill = theme::selection_fill();

    ui.painter().rect_filled(rect, 0.0, track);
    if fraction > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, vec2(rect.width() * fraction, rect.height()));
        ui.painter().rect_filled(fill_rect, 0.0, fill);
    }
}
