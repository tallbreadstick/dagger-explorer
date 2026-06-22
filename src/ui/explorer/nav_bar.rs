use std::path::PathBuf;

use eframe::egui::{self, EventFilter, Key, Popup, PopupKind, ScrollArea, Sense, Ui, vec2};

use crate::explorer::{
    ExplorerState, apply_path_completion, list_directory_completions, path_completion_context,
    path_components,
};
use crate::ui::{theme, text};

pub const NAV_HEIGHT: f32 = 36.0;
const BTN_SIZE: f32 = 28.0;
const BREADCRUMB_MENU_WIDTH: f32 = 240.0;
const MENU_ITEM_HEIGHT: f32 = 24.0;
const MAX_MENU_ITEMS: usize = 10;
const PATH_AUTOCOMPLETE_WIDTH: f32 = 360.0;

enum PathBarEditAction {
    None,
    Cancel,
    Commit,
}

fn menu_viewport_height(ui: &Ui, visible_items: usize) -> f32 {
    let visible_items = visible_items.min(MAX_MENU_ITEMS);
    let spacing = ui.spacing().item_spacing.y;
    MENU_ITEM_HEIGHT * visible_items as f32 + spacing * visible_items.saturating_sub(1) as f32
}

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

                            if state.path_bar_edit.is_some() {
                                match path_bar_editor(ui, state, frame_width) {
                                    PathBarEditAction::Cancel => state.cancel_path_bar_edit(),
                                    PathBarEditAction::Commit => state.commit_path_bar_edit(),
                                    PathBarEditAction::None => {}
                                }
                            } else {
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

                                            let remaining = ui.available_width();
                                            if remaining > 1.0 {
                                                let (_, response) = ui.allocate_exact_size(
                                                    vec2(remaining, ui.available_height()),
                                                    Sense::click(),
                                                );
                                                if response.double_clicked() {
                                                    state.start_path_bar_edit();
                                                }
                                            }
                                        });
                                    });
                            }
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

fn move_text_edit_cursor_to_end(ctx: &egui::Context, id: egui::Id, text: &str) {
    use egui::text::{CCursor, CCursorRange};

    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
        let end = CCursor::new(text.chars().count());
        state.cursor.set_char_range(Some(CCursorRange::one(end)));
        egui::TextEdit::store_state(ctx, id, state);
    }
}

fn path_bar_editor(
    ui: &mut Ui,
    state: &mut ExplorerState,
    width: f32,
) -> PathBarEditAction {
    let mut action = PathBarEditAction::None;
    let edit = state.path_bar_edit.as_mut().expect("path_bar_edit");

    let text_before = edit.text.clone();
    let (parent, prefix) = path_completion_context(&edit.text);
    let completions = list_directory_completions(&parent, &prefix);
    if edit.completion_index >= completions.len() {
        edit.completion_index = 0;
    }

    let text_edit_id = ui.id().with("path_bar_edit");
    let output = egui::TextEdit::singleline(&mut edit.text)
        .id(text_edit_id)
        .font(egui::FontId::proportional(12.0))
        .desired_width(width)
        .margin(egui::Margin::ZERO)
        .lock_focus(false)
        .show(ui);
    let response = &output.response;
    response.request_focus();

    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            response.id,
            EventFilter {
                tab: true,
                vertical_arrows: !completions.is_empty(),
                ..Default::default()
            },
        );
    });

    if edit.text != text_before {
        edit.completion_index = 0;
    }

    let mut move_cursor_to_end = false;

    if response.has_focus() {
        if ui.input(|input| input.key_pressed(Key::Escape)) {
            return PathBarEditAction::Cancel;
        }
        if ui.input(|input| input.key_pressed(Key::Enter)) {
            return PathBarEditAction::Commit;
        }
        if ui.input(|input| input.key_pressed(Key::Tab)) {
            if !completions.is_empty() {
                let index = edit.completion_index % completions.len();
                apply_path_completion(&mut edit.text, &completions[index]);
                edit.completion_index = (index + 1) % completions.len();
                move_cursor_to_end = true;
            }
            ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Tab));
        }
        if !completions.is_empty() {
            if ui.input(|input| input.key_pressed(Key::ArrowDown)) {
                edit.completion_index = (edit.completion_index + 1) % completions.len();
                ui.input_mut(|input| {
                    input.consume_key(egui::Modifiers::NONE, Key::ArrowDown)
                });
            } else if ui.input(|input| input.key_pressed(Key::ArrowUp)) {
                edit.completion_index = edit.completion_index.checked_sub(1).unwrap_or(
                    completions.len().saturating_sub(1),
                );
                ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::ArrowUp));
            }
        }
    }

    let mut completion_clicked = false;
    if !completions.is_empty() {
        let selected = edit.completion_index;
        let popup_width = width.max(PATH_AUTOCOMPLETE_WIDTH);
        Popup::from_response(response)
            .kind(PopupKind::Menu)
            .width(popup_width)
            .show(|ui| {
                ui.set_min_width(popup_width);
                ui.set_max_width(popup_width);

                if completions.len() <= MAX_MENU_ITEMS {
                    for (index, path) in completions.iter().enumerate() {
                        if completion_menu_item(ui, path, index == selected).clicked() {
                            apply_path_completion(
                                &mut state.path_bar_edit.as_mut().unwrap().text,
                                path,
                            );
                            state.path_bar_edit.as_mut().unwrap().completion_index = index;
                            completion_clicked = true;
                        }
                    }
                } else {
                    let scroll_height = menu_viewport_height(ui, MAX_MENU_ITEMS);
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .min_scrolled_height(scroll_height)
                        .max_height(scroll_height)
                        .show(ui, |ui| {
                            for (index, path) in completions.iter().enumerate() {
                                if completion_menu_item(ui, path, index == selected).clicked() {
                                    apply_path_completion(
                                        &mut state.path_bar_edit.as_mut().unwrap().text,
                                        path,
                                    );
                                    state.path_bar_edit.as_mut().unwrap().completion_index = index;
                                    completion_clicked = true;
                                }
                            }
                        });
                }
            });
    }

    if move_cursor_to_end || completion_clicked {
        let text = state.path_bar_edit.as_ref().unwrap().text.clone();
        move_text_edit_cursor_to_end(ui.ctx(), text_edit_id, &text);
    }

    if completion_clicked {
        response.request_focus();
        return PathBarEditAction::None;
    }

    let clicked_elsewhere = response.has_focus()
        && ui.input(|input| input.pointer.primary_clicked())
        && !response.hovered()
        && !Popup::is_any_open(ui.ctx());
    if clicked_elsewhere || (response.lost_focus() && !Popup::is_any_open(ui.ctx())) {
        action = PathBarEditAction::Commit;
    }

    action
}

fn completion_menu_item(ui: &mut Ui, path: &PathBuf, selected: bool) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(path.display().to_string())
                .size(12.0)
                .color(if selected {
                    theme::text_primary()
                } else {
                    theme::text_muted()
                }),
        )
        .frame(false)
        .min_size(vec2(ui.available_width(), MENU_ITEM_HEIGHT)),
    )
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
        let subdirs: Vec<_> = listing
            .lock()
            .ok()
            .map(|guard| guard.entries.iter().filter(|e| e.is_dir).cloned().collect())
            .unwrap_or_default();
        if subdirs.is_empty() {
            ui.label(
                egui::RichText::new("No subfolders")
                    .small()
                    .color(theme::text_muted()),
            );
        } else if subdirs.len() <= MAX_MENU_ITEMS {
            for entry in subdirs.iter() {
                folder_menu_item(ui, state, entry, label_width, &font_id);
            }
        } else {
            let scroll_height = menu_viewport_height(ui, MAX_MENU_ITEMS);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .min_scrolled_height(scroll_height)
                .max_height(scroll_height)
                .show(ui, |ui| {
                    for entry in subdirs.iter() {
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
