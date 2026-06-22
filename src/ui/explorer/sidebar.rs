use std::path::PathBuf;

use eframe::egui::{self, Rect, ScrollArea, Ui, UiBuilder, vec2};

use crate::explorer::{ExplorerState, QuickAccessEntry, list_drives, quick_access_entries};
use crate::ui::{theme, text};

const SIDEBAR_WIDTH: f32 = 220.0;
const SECTION_HEADER_HEIGHT: f32 = 14.0;
const SIDEBAR_ROW_HEIGHT: f32 = 24.0;
const ITEM_GAP: f32 = 4.0;
const BETWEEN_SECTIONS: f32 = 12.0;
const BOTTOM_PADDING: f32 = 8.0;

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    egui::Panel::left("explorer_sidebar")
        .resizable(true)
        .default_size(SIDEBAR_WIDTH)
        .size_range(160.0..=360.0)
        .frame(
            egui::Frame::new()
                .fill(theme::title_bar_fill())
                .inner_margin(8.0)
                .stroke(egui::Stroke::new(1.0, theme::glass_stroke())),
        )
        .show_inside(ui, |ui| {
            let quick_access = quick_access_entries();
            let drives = list_drives();
            let bottom_height = bottom_sections_height(quick_access.len(), drives.len());

            let panel = ui.available_rect_before_wrap();
            let bottom_rect = Rect::from_min_max(
                egui::pos2(panel.min.x, panel.max.y - bottom_height),
                panel.max,
            );
            let tree_rect = Rect::from_min_max(panel.min, egui::pos2(panel.max.x, bottom_rect.min.y));

            ui.scope_builder(UiBuilder::new().max_rect(tree_rect), |ui| {
                section_header(ui, "FILE TREE");
                ui.add_space(ITEM_GAP);

                ScrollArea::vertical()
                    .id_salt("sidebar_file_tree")
                    .auto_shrink([false, false])
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        let root = state.file_tree.path.clone();
                        show_tree_node(ui, state, &root, 0);
                    });
            });

            ui.scope_builder(UiBuilder::new().max_rect(bottom_rect), |ui| {
                show_bottom_sections(ui, state, &quick_access, &drives);
            });
        });
}

fn bottom_sections_height(quick_access_count: usize, drives_count: usize) -> f32 {
    let rows = |count: usize| {
        if count == 0 {
            0.0
        } else {
            count as f32 * SIDEBAR_ROW_HEIGHT + (count.saturating_sub(1) as f32 * ITEM_GAP)
        }
    };

    let mut height = BETWEEN_SECTIONS + 1.0 + 8.0; // separator block
    height += SECTION_HEADER_HEIGHT + ITEM_GAP + rows(quick_access_count) + BETWEEN_SECTIONS;
    height += SECTION_HEADER_HEIGHT + ITEM_GAP + rows(drives_count);
    height += BOTTOM_PADDING;
    height
}

fn show_bottom_sections(
    ui: &mut Ui,
    state: &mut ExplorerState,
    quick_access: &[QuickAccessEntry],
    drives: &[PathBuf],
) {
    ui.add_space(BETWEEN_SECTIONS);
    ui.separator();
    ui.add_space(8.0);

    section_header(ui, "QUICK ACCESS");
    ui.add_space(ITEM_GAP);
    for entry in quick_access {
        let is_active = state.active_path() == entry.path;
        if sidebar_link(ui, &format!("{}  {}", entry.icon, entry.label), is_active).clicked() {
            state.navigate_active(entry.path.clone());
        }
    }

    ui.add_space(BETWEEN_SECTIONS);
    section_header(ui, "DRIVES");
    ui.add_space(ITEM_GAP);
    for drive in drives {
        let label = drive_display(drive);
        let is_active = state.active_path() == *drive;
        if sidebar_link(ui, &format!("💾  {label}"), is_active).clicked() {
            state.navigate_active(drive.clone());
        }
    }
    ui.add_space(BOTTOM_PADDING);
}

fn section_header(ui: &mut Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(10.0)
            .color(theme::text_muted())
            .strong(),
    );
}

fn sidebar_link(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let max_width = ui.available_width();
    let text_color = if active {
        theme::text_primary()
    } else {
        theme::text_primary()
    };
    let display = text::ellipsize(
        ui,
        label,
        egui::FontId::proportional(12.0),
        text_color,
        max_width,
    );
    let response = ui.add(
        egui::Button::new(
            egui::RichText::new(display)
                .size(12.0)
                .color(text_color),
        )
        .frame(false)
        .min_size(vec2(max_width, SIDEBAR_ROW_HEIGHT)),
    );

    if active || response.hovered() {
        ui.painter().rect_filled(
            response.rect,
            4.0,
            if active {
                theme::glass_fill()
            } else {
                theme::maximize_hover()
            },
        );
    }

    response
}

fn drive_display(path: &PathBuf) -> String {
    if cfg!(not(target_os = "windows")) && path == &PathBuf::from("/") {
        "Root (/)".to_string()
    } else {
        path.display().to_string()
    }
}

fn show_tree_node(ui: &mut Ui, state: &mut ExplorerState, node_path: &PathBuf, depth: u32) {
    let Some(node) = state.tree_node(node_path) else {
        return;
    };

    let name = node.name.clone();
    let is_dir = node.is_dir;
    let expanded = node.expanded;
    let child_paths: Vec<PathBuf> = node.children.iter().map(|c| c.path.clone()).collect();

    let indent = depth as f32 * 14.0;
    let row_height = 22.0;

    ui.horizontal(|ui| {
        ui.add_space(indent);

        let row_width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(
            vec2(row_width, row_height),
            egui::Sense::click(),
        );

        let is_active = state.active_path() == *node_path;
        if is_active || response.hovered() {
            ui.painter().rect_filled(
                rect,
                3.0,
                if is_active {
                    theme::glass_fill()
                } else {
                    theme::maximize_hover()
                },
            );
        }

        let arrow = if is_dir {
            if expanded { "▾" } else { "▸" }
        } else {
            " "
        };
        let icon = if is_dir { "📁" } else { "📄" };
        let prefix = format!("{arrow} {icon} ");

        let font_id = egui::FontId::proportional(12.0);
        let prefix_width = ui
            .painter()
            .layout_no_wrap(
                prefix.clone(),
                font_id.clone(),
                theme::text_primary(),
            )
            .size()
            .x;
        let name_max_width = (rect.width() - prefix_width - 8.0).max(0.0);
        let display_name = text::ellipsize(
            ui,
            &name,
            font_id.clone(),
            theme::text_primary(),
            name_max_width,
        );

        ui.painter().text(
            rect.left_center() + vec2(4.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("{prefix}{display_name}"),
            font_id,
            theme::text_primary(),
        );

        let clicked_path = node_path.clone();
        if response.double_clicked() && is_dir {
            state.cancel_tree_click();
            state.toggle_tree_expand(&clicked_path);
        } else if response.clicked() {
            let at = ui.input(|input| input.time);
            state.schedule_tree_click(clicked_path, is_dir, at);
        }
    });

    if expanded {
        state.populate_tree_children(node_path);
        for child_path in child_paths {
            show_tree_node(ui, state, &child_path, depth + 1);
        }

        if is_dir {
            let file_count = state
                .fs_cache
                .listing(node_path)
                .and_then(|listing| {
                    listing
                        .lock()
                        .ok()
                        .map(|guard| guard.entries.iter().filter(|entry| !entry.is_dir).count())
                })
                .unwrap_or(0);

            if file_count > 0 {
                show_tree_files_entry(ui, state, node_path, depth + 1, file_count);
            }
        }
    }
}

fn show_tree_files_entry(
    ui: &mut Ui,
    state: &mut ExplorerState,
    dir_path: &PathBuf,
    depth: u32,
    file_count: usize,
) {
    let indent = depth as f32 * 14.0;
    let row_height = 22.0;
    let label = format!("[{file_count}] files...");

    ui.horizontal(|ui| {
        ui.add_space(indent);

        let row_width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(
            vec2(row_width, row_height),
            egui::Sense::click(),
        );

        let is_active = state.active_path() == *dir_path;
        if is_active || response.hovered() {
            ui.painter().rect_filled(
                rect,
                3.0,
                if is_active {
                    theme::glass_fill()
                } else {
                    theme::maximize_hover()
                },
            );
        }

        let prefix = "  📄 ";
        let font_id = egui::FontId::proportional(12.0);
        let prefix_width = ui
            .painter()
            .layout_no_wrap(
                prefix.to_string(),
                font_id.clone(),
                theme::text_muted(),
            )
            .size()
            .x;
        let label_max_width = (rect.width() - prefix_width - 8.0).max(0.0);
        let display_label = text::ellipsize(
            ui,
            &label,
            font_id.clone(),
            theme::text_muted(),
            label_max_width,
        );

        ui.painter().text(
            rect.left_center() + vec2(4.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("{prefix}{display_label}"),
            font_id,
            theme::text_muted(),
        );

        if response.clicked() {
            state.navigate_active(dir_path.clone());
        }
    });
}
