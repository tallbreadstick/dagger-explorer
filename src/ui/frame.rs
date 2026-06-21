use eframe::egui::{
    self, CornerRadius, Id, PointerButton, Rect, Sense, Ui, UiBuilder, ViewportCommand,
    ResizeDirection, vec2,
};

use super::{theme, title_bar};

const RESIZE_GRIP: f32 = 6.0;

pub fn show(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    let panel_frame = egui::Frame::new()
        .fill(theme::glass_fill())
        .corner_radius(CornerRadius::same(10))
        .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
        .outer_margin(0.0);

    panel_frame.show(ui, |ui| {
        let app_rect = ui.max_rect();
        ui.expand_to_include_rect(app_rect);

        // Register edge grips before the title bar so window controls stay clickable.
        resize_edges(ui, app_rect);

        let title_bar_rect = title_bar::show(ui, title);

        let content_rect = {
            let mut rect = app_rect;
            rect.min.y = title_bar_rect.max.y;
            rect
        };

        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });
}

fn resize_edges(ui: &mut Ui, bounds: Rect) {
    let grip = RESIZE_GRIP;

    let edges = [
        (
            "resize_n",
            Rect::from_min_max(
                bounds.left_top() + vec2(grip, 0.0),
                bounds.right_top() + vec2(-grip, grip),
            ),
            ResizeDirection::North,
        ),
        (
            "resize_s",
            Rect::from_min_max(
                bounds.left_bottom() + vec2(grip, -grip),
                bounds.right_bottom() + vec2(-grip, 0.0),
            ),
            ResizeDirection::South,
        ),
        (
            "resize_w",
            Rect::from_min_max(
                bounds.left_top() + vec2(0.0, grip),
                bounds.left_bottom() + vec2(grip, -grip),
            ),
            ResizeDirection::West,
        ),
        (
            "resize_e",
            Rect::from_min_max(
                bounds.right_top() + vec2(-grip, grip),
                bounds.right_bottom(),
            ),
            ResizeDirection::East,
        ),
        (
            "resize_nw",
            Rect::from_min_size(bounds.left_top(), vec2(grip, grip)),
            ResizeDirection::NorthWest,
        ),
        (
            "resize_ne",
            Rect::from_min_size(bounds.right_top() + vec2(-grip, 0.0), vec2(grip, grip)),
            ResizeDirection::NorthEast,
        ),
        (
            "resize_sw",
            Rect::from_min_size(bounds.left_bottom() + vec2(0.0, -grip), vec2(grip, grip)),
            ResizeDirection::SouthWest,
        ),
        (
            "resize_se",
            Rect::from_min_size(bounds.right_bottom() + vec2(-grip, -grip), vec2(grip, grip)),
            ResizeDirection::SouthEast,
        ),
    ];

    for (id, rect, direction) in edges {
        let response = ui.interact(rect, Id::new(id), Sense::click_and_drag());
        if response.drag_started_by(PointerButton::Primary) {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
    }
}
