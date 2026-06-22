use eframe::egui::{self, Ui};

use crate::explorer::{format_size_kb, item_count_label, prepare_entries, ExplorerState, FileEntry};
use crate::ui::theme;

pub const STATUS_BAR_HEIGHT: f32 = 24.0;

pub fn show(ui: &mut Ui, state: &ExplorerState) {
    let path = state.active_path();
    let selected_count = state.view_options.selected.len();

    let (total_count, selected_bytes) = if let Some(listing) = state.fs_cache.listing(&path) {
        let guard = listing.lock().ok();
        let entries = guard
            .as_ref()
            .map(|guard| prepare_entries(&guard.entries, &state.view_options))
            .unwrap_or_default();
        let raw_entries = guard
            .map(|guard| guard.entries.clone())
            .unwrap_or_default();
        let total = entries.len();
        let bytes = selected_bytes_from_listing(&raw_entries, &state.view_options.selected);
        (total, bytes)
    } else {
        (0, 0)
    };

    egui::Frame::new()
        .fill(theme::title_bar_fill())
        .inner_margin(egui::Margin::symmetric(10, 0))
        .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
        .show(ui, |ui| {
            ui.set_min_height(STATUS_BAR_HEIGHT);
            ui.set_max_height(STATUS_BAR_HEIGHT);

            ui.horizontal(|ui| {
                ui.set_height(STATUS_BAR_HEIGHT);

                let left = format!(
                    "{} selected / {}",
                    item_count_label(selected_count),
                    item_count_label(total_count)
                );

                ui.label(
                    egui::RichText::new(left)
                        .size(11.0)
                        .color(theme::text_muted()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new(format_size_kb(selected_bytes))
                            .size(11.0)
                            .color(theme::text_muted()),
                    );
                });
            });
        });
}

fn selected_bytes_from_listing(listing: &[FileEntry], selected: &[std::path::PathBuf]) -> u64 {
    selected
        .iter()
        .filter_map(|path| listing.iter().find(|entry| entry.path == *path))
        .map(|entry| entry.size)
        .sum()
}
