use eframe::egui::{
    self, Area, Frame, Id, Order, ViewportBuilder, ViewportCommand, ViewportId, vec2,
};

use crate::explorer::{ConflictChoice, ExplorerState, TransferManager};
use crate::ui::theme;

pub fn show(ctx: &egui::Context, state: &mut ExplorerState) {
    show_quick_toast(ctx, state);

    if state.transfer.has_conflict() {
        show_conflict_window(ctx, state);
    } else if state.transfer.is_active() || state.transfer.progress.error.is_some() {
        show_transfer_toast(ctx, &mut state.transfer);
    }

    if state.properties_dialog.is_some() {
        show_properties_window(ctx, state);
    }
}

fn show_quick_toast(ctx: &egui::Context, state: &mut ExplorerState) {
    let now = ctx.input(|input| input.time);
    let mut message = None;
    if let Some(toast) = state.quick_toast.as_mut() {
        if !toast.expires_at.is_finite() {
            toast.expires_at = now + 2.0;
        }
        if now >= toast.expires_at {
            state.quick_toast = None;
        } else {
            message = Some(toast.message.clone());
        }
    }

    let Some(message) = message else {
        return;
    };

    let screen = ctx.content_rect();
    let width = 280.0;
    let height = 44.0;
    let margin = 16.0;
    let offset = 90.0;

    Area::new(Id::new("quick_clipboard_toast"))
        .order(Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.right() - width - margin,
            screen.bottom() - height - margin - offset,
        ))
        .show(ctx, |ui| {
            Frame::new()
                .fill(theme::title_bar_fill())
                .stroke(egui::Stroke::new(1.0, theme::glass_stroke()))
                .inner_margin(10.0)
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_width(width - 20.0);
                    ui.label(
                        egui::RichText::new(message)
                            .size(12.0)
                            .color(theme::text_primary()),
                    );
                });
        });
}

fn show_transfer_toast(ctx: &egui::Context, transfer: &mut TransferManager) {
    let has_error = transfer.progress.error.is_some();
    let screen = ctx.content_rect();
    let width = 320.0;
    let height = if has_error { 72.0 } else { 80.0 };
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

                    let action = if transfer.progress.counting {
                        "Calculating Size".to_string()
                    } else if transfer.progress.operation.is_empty() {
                        "Transferring".to_string()
                    } else {
                        transfer.progress.operation.clone()
                    };
                    ui.label(
                        egui::RichText::new(action)
                            .size(12.0)
                            .color(theme::text_primary()),
                    );
                    if !transfer.progress.label.is_empty() {
                        ui.label(
                            egui::RichText::new(transfer.progress.label.clone())
                                .size(10.0)
                                .color(theme::text_muted()),
                        );
                    }

                    let progress = &transfer.progress;
                    let denominator = progress.total_bytes.max(progress.done_bytes).max(1);
                    let byte_fraction = if denominator > 0 {
                        progress.done_bytes as f32 / denominator as f32
                    } else {
                        0.0
                    };
                    let fraction = byte_fraction.clamp(0.0, 1.0);

                    let bar_width = width - 24.0;
                    let bar_height = 4.0;
                    let (bar_rect, _) =
                        ui.allocate_exact_size(vec2(bar_width, bar_height), egui::Sense::hover());
                    ui.painter().rect_filled(bar_rect, 2.0, theme::glass_stroke());
                    let show_indeterminate = transfer.progress.active && transfer.progress.counting;
                    if show_indeterminate {
                        paint_indeterminate_bar(ui, bar_rect);
                    } else {
                        let fill_rect = egui::Rect::from_min_size(
                            bar_rect.min,
                            vec2(bar_rect.width() * fraction, bar_rect.height()),
                        );
                        ui.painter()
                            .rect_filled(fill_rect, 2.0, theme::selection_fill());
                    }
                    ui.add_space(2.0);

                    if show_indeterminate {
                        ui.label(
                            egui::RichText::new("Working…")
                                .size(10.0)
                                .color(theme::text_muted()),
                        );
                    } else {
                        let done_kb = format_kb_grouped(progress.done_bytes);
                        let total_kb = format_kb_grouped(progress.total_bytes);
                        ui.label(
                            egui::RichText::new(format!("{done_kb} / {total_kb} KB"))
                            .size(10.0)
                            .color(theme::text_muted()),
                        );
                    }
                });
        });
}

fn paint_indeterminate_bar(ui: &mut egui::Ui, rect: egui::Rect) {
    let t = ui.input(|input| input.time) as f32;
    let segment_width = rect.width() * 0.30;
    let travel = rect.width() + segment_width;
    let left = rect.left() + (t * 180.0).rem_euclid(travel) - segment_width;
    let right = (left + segment_width).min(rect.right());
    if right > rect.left() {
        let segment = egui::Rect::from_min_max(
            egui::pos2(left.max(rect.left()), rect.top()),
            egui::pos2(right, rect.bottom()),
        );
        ui.painter().rect_filled(segment, 2.0, theme::selection_fill());
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
}

fn format_kb_grouped(bytes: u64) -> String {
    let kb = if bytes == 0 { 0 } else { bytes.div_ceil(1024) };
    let digits = kb.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn show_conflict_window(ctx: &egui::Context, state: &mut ExplorerState) {
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

    let viewport_id = ViewportId::from_hash_of("transfer_conflict_window");
    let builder = ViewportBuilder::default()
        .with_title("Resolve File Conflict")
        .with_inner_size(vec2(460.0, 210.0))
        .with_transparent(false)
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false);
    ctx.show_viewport_immediate(viewport_id, builder, |ui, _class| {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Transparent(false));
        if ui.input(|input| input.viewport().close_requested()) {
            state.transfer.resolve_conflict(ConflictChoice::Cancel);
            return;
        }

        let viewport_rect = ui.max_rect();
        ui.painter()
            .rect_filled(viewport_rect, 0.0, theme::title_bar_fill());
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(viewport_rect.shrink2(vec2(14.0, 12.0))),
            |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new("File already exists")
                        .size(14.0)
                        .color(theme::text_primary()),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!("“{file_name}” already exists in this location."))
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

fn show_properties_window(ctx: &egui::Context, state: &mut ExplorerState) {
    let Some(properties) = state.properties_dialog.clone() else {
        return;
    };
    let viewport_id = ViewportId::from_hash_of("properties_window");
    let builder = ViewportBuilder::default()
        .with_title("Properties")
        .with_inner_size(vec2(500.0, 208.0))
        .with_transparent(false)
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false);
    ctx.show_viewport_immediate(viewport_id, builder, |ui, _class| {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Transparent(false));
        if ui.input(|input| input.viewport().close_requested())
            || ui.input(|input| input.key_pressed(egui::Key::Escape))
        {
            state.close_properties_dialog();
            return;
        }

        let viewport_rect = ui.max_rect();
        ui.painter()
            .rect_filled(viewport_rect, 0.0, theme::title_bar_fill());
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(viewport_rect.shrink2(vec2(14.0, 12.0))),
            |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new("Properties")
                        .size(14.0)
                        .color(theme::text_primary()),
                );
                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new(format!("Name: {}", properties.title))
                        .size(12.0)
                        .color(theme::text_primary()),
                );
                ui.label(
                    egui::RichText::new(format!("Location: {}", properties.location))
                        .size(11.0)
                        .color(theme::text_muted()),
                );
                ui.separator();
                if properties.loading {
                    ui.label(
                        egui::RichText::new("Calculating size and item details…")
                            .size(11.0)
                            .color(theme::text_muted()),
                    );
                    ui.add_space(6.0);
                    ui.spinner();
                } else {
                    ui.label(
                        egui::RichText::new(format!("Selected items: {}", properties.item_count))
                            .size(11.0)
                            .color(theme::text_muted()),
                    );
                    ui.label(
                        egui::RichText::new(format!("Files: {}", properties.file_count))
                            .size(11.0)
                            .color(theme::text_muted()),
                    );
                    ui.label(
                        egui::RichText::new(format!("Folders: {}", properties.folder_count))
                            .size(11.0)
                            .color(theme::text_muted()),
                    );
                    ui.label(
                        egui::RichText::new(format!("Total size: {}", properties.size_label))
                            .size(11.0)
                            .color(theme::text_muted()),
                    );
                }

                ui.add_space(14.0);
                if ui.button("Close").clicked() {
                    state.close_properties_dialog();
                }
            });
    });
}
