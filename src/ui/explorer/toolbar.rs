use eframe::egui::containers::menu::{MenuButton, MenuConfig};
use eframe::egui::{self, PopupCloseBehavior, Ui, vec2};

use crate::explorer::{
    ExplorerState, SortField, SortOrder, ViewMode,
};
use crate::ui::theme;

pub const TOOLBAR_HEIGHT: f32 = 36.0;
const BTN_SIZE: f32 = 28.0;
const MENU_BTN_MIN_WIDTH: f32 = 76.0;
const GAP: f32 = 4.0;

const ICON_CUT: &str = "✂";
const ICON_COPY: &str = "📋";
const ICON_PASTE: &str = "📃";
const ICON_RENAME: &str = "✏";
const ICON_DELETE: &str = "🗑";
const ICON_SORT: &str = "↕";
const ICON_VIEW: &str = "👁";

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    let bar_height = ui.available_height();

    egui::Frame::new()
        .fill(theme::title_bar_fill())
        .inner_margin(egui::Margin::symmetric(8, 4))
        .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
        .show(ui, |ui| {
            let row_width = ui.available_width();
            let row_height = ui.available_height();
            ui.set_width(row_width);
            ui.set_min_width(row_width);
            ui.set_max_width(row_width);
            ui.set_min_height(bar_height);
            ui.set_max_height(bar_height);

            ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(row_width);
                    ui.set_min_width(row_width);
                    ui.set_max_width(row_width);
                    ui.set_min_height(row_height);
                    ui.set_max_height(row_height);
                    ui.spacing_mut().item_spacing.x = GAP;

                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = GAP;

                            if text_button(ui, "+ New File", "Create a new file", true).clicked() {
                                state.new_file_in_active();
                            }

                            toolbar_divider(ui);

                            let has_selection = state.view_options.has_selection();
                            if icon_button(ui, ICON_CUT, "Cut", has_selection).clicked() {
                                state.cut_selection();
                            }
                            if icon_button(ui, ICON_COPY, "Copy", has_selection).clicked() {
                                state.copy_selection();
                            }
                            if icon_button(ui, ICON_PASTE, "Paste", state.can_paste()).clicked() {
                                state.paste_clipboard();
                            }
                            if icon_button(ui, ICON_RENAME, "Rename", has_selection).clicked() {
                                state.start_rename_from_selection();
                            }
                            if icon_button(ui, ICON_DELETE, "Delete", has_selection).clicked() {
                                state.trash_selection();
                            }

                            toolbar_divider(ui);

                            sort_menu(ui, state);
                            view_menu(ui, state);
                        },
                    );

                    let flex_width = ui.available_width().max(0.0);
                    ui.allocate_ui(vec2(flex_width, row_height), |ui| {
                        ui.set_width(flex_width);
                        ui.set_min_width(flex_width);
                        ui.set_max_width(flex_width);
                        ui.set_min_height(row_height);
                        ui.set_max_height(row_height);

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.set_width(flex_width);
                                ui.set_min_width(flex_width);
                                ui.set_max_width(flex_width);
                                ui.spacing_mut().item_spacing.x = GAP;

                                view_mode_toggle(
                                    ui,
                                    state,
                                    ViewMode::LargeList,
                                    "☰",
                                    "Large list",
                                );
                                view_mode_toggle(
                                    ui,
                                    state,
                                    ViewMode::SmallList,
                                    "≡",
                                    "Small list",
                                );
                                view_mode_toggle(
                                    ui,
                                    state,
                                    ViewMode::LargeIcons,
                                    "▣",
                                    "Large icons",
                                );
                                view_mode_toggle(
                                    ui,
                                    state,
                                    ViewMode::SmallIcons,
                                    "▦",
                                    "Small icons",
                                );
                            },
                        );
                    });
                },
            );
        });
}

fn sort_menu(ui: &mut Ui, state: &mut ExplorerState) {
    let label = sort_button_label(state.view_options.sort_field, state.view_options.sort_order);
    MenuButton::new(menu_label(&format!("{ICON_SORT} {label}")))
        .ui(ui, |ui| {
            ui.set_min_width(180.0);
            sort_option(ui, state, SortField::Name, SortOrder::Ascending, "Name (A → Z)");
            sort_option(ui, state, SortField::Name, SortOrder::Descending, "Name (Z → A)");
            ui.separator();
            sort_option(
                ui,
                state,
                SortField::Date,
                SortOrder::Ascending,
                "Date (Oldest first)",
            );
            sort_option(
                ui,
                state,
                SortField::Date,
                SortOrder::Descending,
                "Date (Newest first)",
            );
            ui.separator();
            sort_option(
                ui,
                state,
                SortField::FileSize,
                SortOrder::Ascending,
                "Size (Smallest first)",
            );
            sort_option(
                ui,
                state,
                SortField::FileSize,
                SortOrder::Descending,
                "Size (Largest first)",
            );
            ui.separator();
            sort_option(ui, state, SortField::Type, SortOrder::Ascending, "Type (A → Z)");
            sort_option(ui, state, SortField::Type, SortOrder::Descending, "Type (Z → A)");
        })
        .0
        .on_hover_text("Sort");
}

fn sort_button_label(field: SortField, order: SortOrder) -> String {
    let field_label = match field {
        SortField::Name => "Name",
        SortField::Date => "Date",
        SortField::FileSize => "Size",
        SortField::Type => "Type",
    };
    let order_symbol = match order {
        SortOrder::Ascending => "↑",
        SortOrder::Descending => "↓",
    };
    format!("{field_label} {order_symbol}")
}

fn sort_option(
    ui: &mut Ui,
    state: &mut ExplorerState,
    field: SortField,
    order: SortOrder,
    label: &str,
) {
    let selected =
        state.view_options.sort_field == field && state.view_options.sort_order == order;
    if ui
        .selectable_label(
            selected,
            egui::RichText::new(label).size(12.0).color(theme::text_primary()),
        )
        .clicked()
    {
        state.view_options.set_sort(field, order);
        ui.close();
    }
}

fn view_menu(ui: &mut Ui, state: &mut ExplorerState) {
    MenuButton::new(menu_label(&format!("{ICON_VIEW} View")))
        .config(
            MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside),
        )
        .ui(ui, |ui| {
            ui.set_min_width(180.0);
            view_mode_option(ui, state, ViewMode::SmallIcons, "Small icons");
            view_mode_option(ui, state, ViewMode::LargeIcons, "Large icons");
            view_mode_option(ui, state, ViewMode::SmallList, "Small list");
            view_mode_option(ui, state, ViewMode::LargeList, "Large list");
            ui.separator();
            if preference_checkbox(
                ui,
                &mut state.view_options.show_hidden_files,
                "Hidden files",
            ) {
                state.save_preferences();
            }
            if preference_checkbox(
                ui,
                &mut state.view_options.show_file_extensions,
                "File extensions",
            ) {
                state.save_preferences();
            }
        })
        .0
        .on_hover_text("View options");
}

fn view_mode_option(ui: &mut Ui, state: &mut ExplorerState, mode: ViewMode, label: &str) {
    if ui
        .selectable_label(
            state.view_options.view_mode == mode,
            egui::RichText::new(label).size(12.0).color(theme::text_primary()),
        )
        .clicked()
    {
        state.view_options.view_mode = mode;
        state.save_preferences();
        ui.close();
    }
}

fn preference_checkbox(ui: &mut Ui, value: &mut bool, label: &str) -> bool {
    let mut enabled = *value;
    let changed = ui
        .checkbox(
            &mut enabled,
            egui::RichText::new(label).size(12.0).color(theme::text_primary()),
        )
        .changed();
    if changed {
        *value = enabled;
    }
    changed
}

fn view_mode_toggle(
    ui: &mut Ui,
    state: &mut ExplorerState,
    mode: ViewMode,
    icon: &str,
    tooltip: &str,
) {
    let active = state.view_options.view_mode == mode;
    let fill = if active {
        theme::glass_fill()
    } else {
        egui::Color32::TRANSPARENT
    };

    if ui
        .add(
            egui::Button::new(
                egui::RichText::new(icon)
                    .size(14.0)
                    .color(if active {
                        theme::text_primary()
                    } else {
                        theme::text_muted()
                    }),
            )
            .min_size(vec2(BTN_SIZE, BTN_SIZE))
            .fill(fill)
            .stroke(if active {
                egui::Stroke::new(1.0, theme::glass_stroke())
            } else {
                egui::Stroke::NONE
            }),
        )
        .on_hover_text(tooltip)
        .clicked()
    {
        state.view_options.view_mode = mode;
        state.save_preferences();
    }
}

fn menu_label(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(12.0)
        .color(theme::text_primary())
}

fn text_button(ui: &mut Ui, label: &str, tooltip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(
            egui::RichText::new(label)
                .size(12.0)
                .color(if enabled {
                    theme::text_primary()
                } else {
                    theme::text_muted()
                }),
        )
        .min_size(vec2(MENU_BTN_MIN_WIDTH, BTN_SIZE)),
    )
    .on_hover_text(tooltip)
}

fn icon_button(ui: &mut Ui, icon: &str, tooltip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(
            egui::RichText::new(icon)
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

fn toolbar_divider(ui: &mut Ui) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, BTN_SIZE - 8.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, theme::glass_stroke());
    ui.add_space(4.0);
}
