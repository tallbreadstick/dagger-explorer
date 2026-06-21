use eframe::egui::{self, Align2, Color32, FontId, Ui};

const ELLIPSIS: &str = "…";

/// Shorten `text` so it fits within `max_width` pixels, appending an ellipsis if needed.
pub fn ellipsize(ui: &Ui, text: &str, font_id: FontId, color: Color32, max_width: f32) -> String {
    if max_width <= 4.0 {
        return ELLIPSIS.to_string();
    }

    let measure = |s: &str| -> f32 {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font_id.clone(), color)
            .size()
            .x
    };

    if measure(text) <= max_width {
        return text.to_string();
    }

    let ellipsis_width = measure(ELLIPSIS);
    let budget = (max_width - ellipsis_width).max(0.0);

    let mut end = text.len();
    while end > 0 {
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == 0 {
            break;
        }
        if measure(&text[..end]) <= budget {
            return format!("{}{ELLIPSIS}", &text[..end]);
        }
        end -= 1;
    }

    ELLIPSIS.to_string()
}

fn anchor_for_rect(rect: egui::Rect, align: Align2) -> egui::Pos2 {
    match align {
        Align2::LEFT_TOP => rect.left_top(),
        Align2::LEFT_CENTER => rect.left_center(),
        Align2::LEFT_BOTTOM => rect.left_bottom(),
        Align2::CENTER_TOP => rect.center_top(),
        Align2::CENTER_CENTER => rect.center(),
        Align2::CENTER_BOTTOM => rect.center_bottom(),
        Align2::RIGHT_TOP => rect.right_top(),
        Align2::RIGHT_CENTER => rect.right_center(),
        Align2::RIGHT_BOTTOM => rect.right_bottom(),
    }
}

/// Draw single-line truncated text within `rect`.
pub fn paint_truncated(
    ui: &Ui,
    rect: egui::Rect,
    text: &str,
    font_id: FontId,
    color: Color32,
    align: Align2,
) {
    let display = ellipsize(ui, text, font_id.clone(), color, rect.width());
    ui.painter().text(
        anchor_for_rect(rect, align),
        align,
        display,
        font_id,
        color,
    );
}
