use eframe::egui::{self, Area, Frame, Id, LayerId, Order, vec2};

use crate::explorer::{ConflictChoice, ExplorerState, TransferManager};
use crate::ui::theme;

pub fn show(ctx: &egui::Context, state: &mut ExplorerState) {
    if state.transfer.has_conflict() {
        show_conflict_dialog(ctx, state);
    } else if state.transfer.is_active() || state.transfer.progress.error.is_some() {
        show_transfer_toast(ctx, &mut state.transfer);
    }
}

fn show_transfer_toast(ctx: &egui::Context, transfer: &mut TransferManager) {
    let has_error = transfer.progress.error.is_some();
    let screen = ctx.content_rect();
    let width = 320.0;
    let height = if has_error { 72.0 } else { 88.0 };
    let margin = 16.0;

    Area::new(Id::new("transfer_toast"))
        .order(Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.right() - width - margin,
            screen.bottom() - height - margin,
        ))
        .show(ctx, |ui| {
            Frame::new()
                .fill(theme::title_bar_fill())
                .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
                .inner_margin(12.0)
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_width(width - 24.0);

                    if let Some(error) = transfer.progress.error.clone() {
                        ui.label(
                            egui::RichText::new(error)
                                .size(12.0)
                                .color(theme::text_primary()),
                        );
                        ui.add_space(8.0);
                        if ui.button("Dismiss").clicked() {
                            *transfer = TransferManager::default();
                        }
                        return;
                    }

                    let action = if transfer.progress.label.is_empty() {
                        "Transferring".to_string()
                    } else {
                        transfer.progress.label.clone()
                    };
                    ui.label(
                        egui::RichText::new(action)
                            .size(12.0)
                            .color(theme::text_primary()),
                    );

                    let progress = &transfer.progress;
                    let file_fraction = if progress.total_files > 0 {
                        progress.done_files as f32 / progress.total_files as f32
                    } else {
                        0.0
                    };
                    let byte_fraction = if progress.total_bytes > 0 {
                        progress.done_bytes as f32 / progress.total_bytes as f32
                    } else {
                        file_fraction
                    };
                    let fraction = file_fraction.max(byte_fraction).clamp(0.0, 1.0);

                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(width - 24.0)
                            .fill(theme::selection_fill()),
                    );

                    ui.label(
                        egui::RichText::new(format!(
                            "{} / {} files",
                            progress.done_files, progress.total_files
                        ))
                        .size(10.0)
                        .color(theme::text_muted()),
                    );
                });
        });
}

fn show_conflict_dialog(ctx: &egui::Context, state: &mut ExplorerState) {
    let Some(conflict) = state.transfer.pending_conflict.as_ref() else {
        return;
    };

    let file_name = conflict
        .destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("item")
        .to_string();
    let source = conflict.source.display().to_string();
    let destination = conflict.destination.display().to_string();

    let screen = ctx.content_rect();
    let dim = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120);
    ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("conflict_dim")))
        .rect_filled(screen, 0.0, dim);

    let size = vec2(420.0, 220.0);
    let pos = screen.center() - size / 2.0;

    Area::new(Id::new("transfer_conflict_dialog"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            Frame::new()
                .fill(theme::title_bar_fill())
                .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
                .inner_margin(16.0)
                .corner_radius(10.0)
                .show(ui, |ui| {
                    ui.set_width(size.x - 32.0);

                    ui.label(
                        egui::RichText::new("File already exists")
                            .size(14.0)
                            .color(theme::text_primary()),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "“{file_name}” already exists in this location."
                        ))
                        .size(12.0)
                        .color(theme::text_muted()),
                    );
                    ui.label(
                        egui::RichText::new(format!("From: {source}"))
                            .size(11.0)
                            .color(theme::text_muted()),
                    );
                    ui.label(
                        egui::RichText::new(format!("To: {destination}"))
                            .size(11.0)
                            .color(theme::text_muted()),
                    );

                    ui.add_space(12.0);
                    ui.checkbox(
                        &mut state.transfer.apply_to_all,
                        egui::RichText::new("Apply to all current items")
                            .size(12.0)
                            .color(theme::text_primary()),
                    );

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Skip").clicked() {
                            state.transfer.resolve_conflict(ConflictChoice::Skip);
                        }
                        if ui.button("Rename").clicked() {
                            state.transfer.resolve_conflict(ConflictChoice::Rename);
                        }
                        if ui.button("Replace").clicked() {
                            state.transfer.resolve_conflict(ConflictChoice::Replace);
                        }
                        if ui.button("Cancel").clicked() {
                            state.transfer.resolve_conflict(ConflictChoice::Cancel);
                        }
                    });
                });
        });
}
