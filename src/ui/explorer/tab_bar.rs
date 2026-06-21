use eframe::egui::{self, Id, ScrollArea, Sense, Ui, vec2};

use crate::explorer::ExplorerState;
use crate::ui::{theme, text};

pub const TAB_MIN_WIDTH: f32 = 168.0;
pub const TAB_HEIGHT: f32 = 32.0;
const TAB_CHIP_HEIGHT: f32 = 28.0;
const SCROLL_STEP: f32 = 96.0;
const SCROLL_BTN: f32 = 28.0;
const CLOSE_BTN: f32 = 22.0;
const CONTROL_GAP: f32 = 4.0;

enum TabAction {
    Close(u64),
    Activate(u64),
}

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    let bar_height = ui.available_height();
    let tab_snapshots: Vec<(u64, String)> = state
        .tabs
        .iter()
        .map(|tab| (tab.id, tab.label()))
        .collect();
    let active_tab_id = state.tabs[state.active_tab].id;

    egui::Frame::new()
        .fill(theme::title_bar_fill())
        .inner_margin(egui::Margin::symmetric(4, 2))
        .show(ui, |ui| {
            let row_width = ui.available_width();
            ui.set_width(row_width);
            ui.set_min_width(row_width);
            ui.set_max_width(row_width);
            ui.set_min_height(bar_height);
            ui.set_max_height(bar_height);

            // Outer row: scroll-left | expanding tab strip | scroll-right
            ui.horizontal(|ui| {
                ui.set_width(row_width);
                ui.set_min_width(row_width);
                ui.set_max_width(row_width);
                ui.spacing_mut().item_spacing.x = CONTROL_GAP;

                if ui
                    .add_enabled(
                        state.tab_scroll > 0.0,
                        egui::Button::new("◀")
                            .small()
                            .min_size(vec2(SCROLL_BTN, TAB_CHIP_HEIGHT)),
                    )
                    .on_hover_text("Scroll tabs left")
                    .clicked()
                {
                    state.tab_scroll = (state.tab_scroll - SCROLL_STEP).max(0.0);
                }

                let strip_width =
                    (ui.available_width() - SCROLL_BTN - CONTROL_GAP).max(0.0);
                let mut tab_action = None;

                let scroll_output = ui
                    .allocate_ui(vec2(strip_width, TAB_CHIP_HEIGHT), |ui| {
                        ui.set_width(strip_width);
                        ui.set_min_width(strip_width);
                        ui.set_max_width(strip_width);

                        ScrollArea::horizontal()
                            .id_salt("explorer_tab_scroll")
                            .scroll_offset(vec2(state.tab_scroll, 0.0))
                            .auto_shrink([false, false])
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .show(ui, |ui| {
                                // Inner group: all tabs + new-tab button travel together
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    for (tab_id, label) in &tab_snapshots {
                                        if let Some(action) = tab_chip(
                                            ui,
                                            *tab_id,
                                            label,
                                            *tab_id == active_tab_id,
                                        ) {
                                            tab_action = Some(action);
                                        }
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new("+")
                                                .small()
                                                .min_size(vec2(
                                                    SCROLL_BTN,
                                                    TAB_CHIP_HEIGHT,
                                                )),
                                        )
                                        .on_hover_text("New tab")
                                        .clicked()
                                    {
                                        state.new_tab();
                                    }
                                });
                            })
                    })
                    .inner;

                state.tab_scroll = scroll_output.state.offset.x;

                let can_scroll_right =
                    scroll_output.content_size.x > scroll_output.inner_rect.width() + 1.0;

                if ui
                    .add_enabled(
                        can_scroll_right,
                        egui::Button::new("▶")
                            .small()
                            .min_size(vec2(SCROLL_BTN, TAB_CHIP_HEIGHT)),
                    )
                    .on_hover_text("Scroll tabs right")
                    .clicked()
                {
                    state.tab_scroll += SCROLL_STEP;
                }

                if let Some(action) = tab_action {
                    match action {
                        TabAction::Close(tab_id) => state.close_tab_by_id(tab_id),
                        TabAction::Activate(tab_id) => state.set_active_tab_by_id(tab_id),
                    }
                }
            });
        });
}

fn tab_chip(
    ui: &mut Ui,
    tab_id: u64,
    label: &str,
    active: bool,
) -> Option<TabAction> {
    let (rect, response) = ui.allocate_exact_size(
        vec2(TAB_MIN_WIDTH, TAB_CHIP_HEIGHT),
        Sense::click(),
    );

    let fill = if active {
        theme::glass_fill()
    } else if response.hovered() {
        theme::maximize_hover()
    } else {
        egui::Color32::TRANSPARENT
    };

    if fill != egui::Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 4.0, fill);
    }

    if active {
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, theme::glass_stroke()),
            egui::StrokeKind::Inside,
        );
    }

    let close_rect = egui::Rect::from_min_size(
        rect.right_top() + vec2(-CLOSE_BTN - 4.0, (rect.height() - CLOSE_BTN) * 0.5),
        vec2(CLOSE_BTN, CLOSE_BTN),
    );
    let label_rect = egui::Rect::from_min_max(
        rect.left_top() + vec2(8.0, 0.0),
        egui::pos2(close_rect.left() - 4.0, rect.bottom()),
    );

    let font_id = egui::FontId::proportional(12.0);
    let label_color = if active {
        theme::text_primary()
    } else {
        theme::text_muted()
    };
    text::paint_truncated(
        ui,
        label_rect,
        label,
        font_id,
        label_color,
        egui::Align2::LEFT_CENTER,
    );

    let close_response = ui.interact(close_rect, Id::new(("tab_close", tab_id)), Sense::click());
    let close_color = if close_response.hovered() {
        theme::text_primary()
    } else {
        theme::text_muted()
    };
    ui.painter().text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        egui::FontId::proportional(14.0),
        close_color,
    );

    if close_response.clicked() {
        Some(TabAction::Close(tab_id))
    } else if response.clicked() {
        Some(TabAction::Activate(tab_id))
    } else {
        None
    }
}
