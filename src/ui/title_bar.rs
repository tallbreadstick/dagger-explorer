use eframe::egui::{
    self, Align2, Color32, FontId, Id, PointerButton, Rect, Sense, Ui, UiBuilder, ViewportCommand,
    vec2,
};

use super::theme;

const TITLE_BAR_HEIGHT: f32 = 36.0;
const CONTROL_WIDTH: f32 = 46.0;
const CONTROL_COUNT: f32 = 3.0;

pub fn show(ui: &mut Ui, title: &str, add_title_center: impl FnOnce(&mut Ui)) -> Rect {
    let title_bar_rect = {
        let mut rect = ui.max_rect();
        rect.max.y = rect.min.y + TITLE_BAR_HEIGHT;
        rect
    };

    ui.painter().rect_filled(title_bar_rect, 0.0, theme::title_bar_fill());
    ui.painter().hline(
        title_bar_rect.x_range(),
        title_bar_rect.bottom(),
        egui::Stroke::new(1.0, theme::title_bar_stroke()),
    );

    let controls_width = CONTROL_WIDTH * CONTROL_COUNT;
    let controls_rect = Rect::from_min_max(
        title_bar_rect.right_top() - vec2(controls_width, 0.0),
        title_bar_rect.right_bottom(),
    );
    let drag_rect = title_bar_rect.with_max_x(controls_rect.left());

    // Drag region first; controls are registered after so they win hit-testing on the right.
    let drag_response = ui.interact(
        drag_rect,
        Id::new("title_bar_drag"),
        Sense::click_and_drag(),
    );

    ui.painter().text(
        drag_rect.left_center() + vec2(14.0, 0.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(13.0),
        theme::text_primary(),
    );

    if drag_response.double_clicked() {
        let is_maximized = viewport_maximized(ui);
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if drag_response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let center_size = vec2(360.0, (TITLE_BAR_HEIGHT - 8.0).max(1.0));
    let center_rect = Rect::from_center_size(drag_rect.center(), center_size)
        .intersect(drag_rect.shrink2(vec2(12.0, 4.0)));
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(center_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::LeftToRight,
            )),
        |ui| add_title_center(ui),
    );

    window_controls(ui, controls_rect);

    title_bar_rect
}

fn viewport_maximized(ui: &Ui) -> bool {
    ui.input(|i| i.viewport().maximized.unwrap_or(false))
}

fn window_controls(ui: &mut Ui, area: Rect) {
    let mut x = area.right() - CONTROL_WIDTH;

    close_button(ui, button_rect(area, x));
    x -= CONTROL_WIDTH;
    maximize_button(ui, button_rect(area, x));
    x -= CONTROL_WIDTH;
    minimize_button(ui, button_rect(area, x));
}

fn button_rect(area: Rect, left: f32) -> Rect {
    Rect::from_min_size(
        egui::pos2(left, area.top()),
        vec2(CONTROL_WIDTH, area.height()),
    )
}

fn close_button(ui: &mut Ui, rect: Rect) {
    let response = ui.interact(rect, Id::new("title_bar_close"), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::close_hover());
        draw_close_icon(ui, rect, Color32::WHITE);
    } else {
        draw_close_icon(ui, rect, theme::text_muted());
    }
    if response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
}

fn maximize_button(ui: &mut Ui, rect: Rect) {
    let is_maximized = viewport_maximized(ui);
    let response = ui.interact(rect, Id::new("title_bar_maximize"), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::maximize_hover());
    }
    let icon_color = if response.hovered() {
        theme::text_primary()
    } else {
        theme::text_muted()
    };
    if is_maximized {
        draw_restore_icon(ui, rect, icon_color);
    } else {
        draw_maximize_icon(ui, rect, icon_color);
    }
    if response.clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }
}

fn minimize_button(ui: &mut Ui, rect: Rect) {
    let response = ui.interact(rect, Id::new("title_bar_minimize"), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::minimize_hover());
        draw_minimize_icon(ui, rect, theme::text_primary());
    } else {
        draw_minimize_icon(ui, rect, theme::text_muted());
    }
    if response.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

fn draw_close_icon(ui: &Ui, rect: Rect, color: Color32) {
    let center = rect.center();
    let size = 5.0;
    let stroke = egui::Stroke::new(1.5, color);
    ui.painter().line_segment([center + vec2(-size, -size), center + vec2(size, size)], stroke);
    ui.painter().line_segment([center + vec2(-size, size), center + vec2(size, -size)], stroke);
}

fn draw_maximize_icon(ui: &Ui, rect: Rect, color: Color32) {
    let center = rect.center();
    let size = 4.5;
    let icon_rect = Rect::from_center_size(center, vec2(size * 2.0, size * 2.0));
    ui.painter()
        .rect_stroke(icon_rect, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Inside);
}

fn draw_restore_icon(ui: &Ui, rect: Rect, color: Color32) {
    let center = rect.center();
    let size = 4.0;
    let back = Rect::from_center_size(center + vec2(-1.5, -1.5), vec2(size * 2.0, size * 2.0));
    let front = Rect::from_center_size(center + vec2(1.5, 1.5), vec2(size * 2.0, size * 2.0));
    ui.painter()
        .rect_stroke(back, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Inside);
    ui.painter().rect_filled(front, 0.0, theme::title_bar_fill());
    ui.painter()
        .rect_stroke(front, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Inside);
}

fn draw_minimize_icon(ui: &Ui, rect: Rect, color: Color32) {
    let center = rect.center();
    ui.painter().hline(
        (center.x - 4.5)..=(center.x + 4.5),
        center.y,
        egui::Stroke::new(1.5, color),
    );
}
