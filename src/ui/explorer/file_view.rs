use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align2, Color32, Frame, Key, LayerId, Order, Rect, Response, ScrollArea, Sense, Ui,
    UiBuilder, vec2,
};
use eframe::egui::containers::scroll_area::ScrollSource;

use crate::explorer::{
    ExplorerState, FileEntry, SelectionMarquee, ViewMode, multi_select_modifiers, open_path,
    prepare_entries,
};
use crate::ui::{theme, text};

const MARQUEE_MIN: f32 = 3.0;
const MARQUEE_PREVIEW_MAX: usize = 150;

struct ViewMetrics {
    tile_width: f32,
    tile_height: f32,
    icon_size: f32,
    label_size: f32,
    list_mode: bool,
    list_row_height: f32,
    detail_size: f32,
}

struct ListColumns {
    icon: f32,
    name: f32,
    modified: f32,
    kind: f32,
    size: f32,
}

struct ItemRect {
    path: PathBuf,
    rect: Rect,
}

impl ViewMetrics {
    fn for_mode(mode: ViewMode) -> Self {
        match mode {
            ViewMode::SmallIcons => Self {
                tile_width: 72.0,
                tile_height: 72.0,
                icon_size: 22.0,
                label_size: 10.0,
                list_mode: false,
                list_row_height: 0.0,
                detail_size: 0.0,
            },
            ViewMode::LargeIcons => Self {
                tile_width: 96.0,
                tile_height: 96.0,
                icon_size: 28.0,
                label_size: 11.0,
                list_mode: false,
                list_row_height: 0.0,
                detail_size: 0.0,
            },
            ViewMode::SmallList => Self {
                tile_width: 0.0,
                tile_height: 0.0,
                icon_size: 16.0,
                label_size: 11.0,
                list_mode: true,
                list_row_height: 24.0,
                detail_size: 10.0,
            },
            ViewMode::LargeList => Self {
                tile_width: 0.0,
                tile_height: 0.0,
                icon_size: 22.0,
                label_size: 12.0,
                list_mode: true,
                list_row_height: 36.0,
                detail_size: 11.0,
            },
        }
    }
}

impl ListColumns {
    fn layout(row_width: f32, icon_width: f32) -> Self {
        let size = 72.0;
        let modified = 148.0;
        let kind = 96.0;
        let fixed = icon_width + modified + kind + size + 12.0;
        let name = (row_width - fixed).max(120.0);

        Self {
            icon: icon_width,
            name,
            modified,
            kind,
            size,
        }
    }

    fn name_start(&self, origin: f32) -> f32 {
        origin + self.icon
    }

    fn name_end(&self, origin: f32) -> f32 {
        self.name_start(origin) + self.name
    }

    fn modified_start(&self, origin: f32) -> f32 {
        self.name_end(origin)
    }

    fn modified_end(&self, origin: f32) -> f32 {
        self.modified_start(origin) + self.modified
    }

    fn kind_start(&self, origin: f32) -> f32 {
        self.modified_end(origin)
    }

    fn kind_end(&self, origin: f32) -> f32 {
        self.kind_start(origin) + self.kind
    }

    fn size_start(&self, origin: f32) -> f32 {
        self.kind_end(origin)
    }

    fn size_end(&self, origin: f32) -> f32 {
        self.size_start(origin) + self.size
    }
}

pub fn show(ui: &mut Ui, state: &mut ExplorerState) {
    let path = state.active_path();
    state.ensure_listing(path.clone());

    let area = ui.available_rect_before_wrap();
    let view_mode = state.view_options.view_mode;
    let show_extensions = state.view_options.show_file_extensions;
    let metrics = ViewMetrics::for_mode(view_mode);

    ui.allocate_ui(area.size(), |ui| {
        ui.set_width(area.width());
        ui.set_max_width(area.width());
        ui.set_min_height(area.height());

        Frame::new()
            .fill(theme::glass_fill())
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(area.height() - 24.0);

                if let Some(listing) = state.fs_cache.listing(&path) {
                    let entries = prepare_entries(listing.as_ref(), &state.view_options);

                    if entries.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("This folder is empty.")
                                    .color(theme::text_muted()),
                            );
                        });
                        handle_rename_input(ui, state);
                        return;
                    }

                    let mut item_rects = Vec::new();
                    let width = ui.available_width();
                    let viewport_h = ui.available_height();
                    let content_h =
                        content_height(entries.len(), &metrics, width, viewport_h);

                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .scroll_source(ScrollSource::MOUSE_WHEEL | ScrollSource::SCROLL_BAR)
                        .show(ui, |ui| {
                            let origin = ui.cursor().min;
                            let content_rect =
                                Rect::from_min_size(origin, vec2(width, content_h));
                            let bg_response = ui.interact(
                                content_rect,
                                ui.id().with("marquee_bg"),
                                Sense::click_and_drag(),
                            );

                            ui.scope_builder(UiBuilder::new().max_rect(content_rect), |ui| {
                                ui.set_min_size(content_rect.size());

                                if metrics.list_mode {
                                    list_header(ui, &metrics, width);
                                    ui.add_space(2.0);
                                    for entry in &entries {
                                        if let Some(item) = list_row(
                                            ui,
                                            state,
                                            entry,
                                            &metrics,
                                            show_extensions,
                                            width,
                                        ) {
                                            item_rects.push(item);
                                        }
                                    }
                                } else {
                                    let cols = ((width / metrics.tile_width).floor() as usize)
                                        .max(1);

                                    egui::Grid::new("file_grid")
                                        .num_columns(cols)
                                        .spacing(vec2(8.0, 8.0))
                                        .show(ui, |ui| {
                                            for (index, entry) in entries.iter().enumerate() {
                                                if let Some(item) = icon_tile(
                                                    ui,
                                                    state,
                                                    entry,
                                                    &metrics,
                                                    show_extensions,
                                                ) {
                                                    item_rects.push(item);
                                                }
                                                if (index + 1) % cols == 0 {
                                                    ui.end_row();
                                                }
                                            }
                                        });
                                }
                            });

                            let layout_rect = ui.min_rect();
                            handle_marquee(
                                ui,
                                state,
                                &bg_response,
                                &item_rects,
                                layout_rect,
                            );
                        });

                    let clip = state.file_view_bounds.unwrap_or(area);
                    paint_marquee_overlay(ui.ctx(), state, &item_rects, clip);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                        ui.label(
                            egui::RichText::new("Loading…").color(theme::text_muted()),
                        );
                    });
                }

                handle_rename_input(ui, state);
            });
    });
}

fn content_height(
    entry_count: usize,
    metrics: &ViewMetrics,
    width: f32,
    viewport_h: f32,
) -> f32 {
    let items_h = if metrics.list_mode {
        let header = metrics.list_row_height + 2.0;
        header + entry_count as f32 * metrics.list_row_height
    } else {
        let cols = ((width / metrics.tile_width).floor() as usize).max(1);
        let rows = entry_count.div_ceil(cols);
        rows as f32 * (metrics.tile_height + 8.0)
    };

    items_h.max(viewport_h)
}

fn handle_marquee(
    ui: &Ui,
    state: &mut ExplorerState,
    bg: &Response,
    item_rects: &[ItemRect],
    content_rect: Rect,
) {
    let clip = state.file_view_bounds.unwrap_or(content_rect);
    let modifiers = ui.input(|input| input.modifiers);
    let additive = multi_select_modifiers(&modifiers);

    let on_item = |point: egui::Pos2| item_rects.iter().any(|item| item.rect.contains(point));

    if bg.drag_started() {
        if let Some(origin) = bg.interact_pointer_pos() {
            let origin = clamp_pos(origin, clip);
            if clip.contains(origin) && !on_item(origin) {
                state.selection_marquee = Some(SelectionMarquee {
                    start: origin,
                    current: origin,
                });
            }
        }
    }

    if let Some(marquee) = state.selection_marquee.as_mut() {
        marquee.start = clamp_pos(marquee.start, clip);

        if bg.dragged() {
            if let Some(point) = ui.input(|input| input.pointer.latest_pos()) {
                let clamped = clamp_pos(point, clip);
                if clamped.distance_sq(marquee.current) > 0.25 {
                    marquee.current = clamped;
                }
            }
        } else {
            marquee.current = clamp_pos(marquee.current, clip);
        }

        if bg.drag_stopped() {
            let marquee_rect = clamped_marquee_rect(*marquee, clip);
            if marquee_rect.width() > MARQUEE_MIN || marquee_rect.height() > MARQUEE_MIN {
                let paths = item_rects
                    .iter()
                    .filter(|item| item.rect.intersects(marquee_rect))
                    .map(|item| item.path.clone())
                    .collect::<Vec<_>>();

                if additive {
                    for path in paths {
                        if !state.view_options.is_selected(&path) {
                            state.view_options.selected.push(path);
                        }
                    }
                } else {
                    state.view_options.set_selection(paths);
                }
            }

            state.selection_marquee = None;
        }
    } else if bg.clicked() {
        if let Some(origin) = bg.interact_pointer_pos() {
            let origin = clamp_pos(origin, clip);
            if clip.contains(origin) && !on_item(origin) && !additive {
                state.view_options.clear_selection();
            }
        }
    }
}

fn clamp_pos(point: egui::Pos2, bounds: Rect) -> egui::Pos2 {
    egui::pos2(
        point.x.clamp(bounds.left(), bounds.right()),
        point.y.clamp(bounds.top(), bounds.bottom()),
    )
}

fn clamped_marquee_rect(marquee: SelectionMarquee, bounds: Rect) -> Rect {
    marquee_rect(SelectionMarquee {
        start: clamp_pos(marquee.start, bounds),
        current: clamp_pos(marquee.current, bounds),
    })
}

fn paint_marquee_overlay(
    ctx: &egui::Context,
    state: &ExplorerState,
    item_rects: &[ItemRect],
    clip: Rect,
) {
    let Some(marquee) = state.selection_marquee else {
        return;
    };

    let marquee_rect = clamped_marquee_rect(marquee, clip);
    if marquee_rect.width() <= MARQUEE_MIN && marquee_rect.height() <= MARQUEE_MIN {
        return;
    }

    let mut painter = ctx.layer_painter(LayerId::new(
        Order::Foreground,
        egui::Id::new("file_view_marquee"),
    ));
    painter.set_clip_rect(clip);

    if item_rects.len() <= MARQUEE_PREVIEW_MAX {
        for item in item_rects {
            if item.rect.intersects(marquee_rect) && !state.view_options.is_selected(&item.path) {
                let radius = if item.rect.height() > 30.0 { 4.0 } else { 6.0 };
                painter.rect_filled(item.rect, radius, theme::selection_preview_fill());
            }
        }
    }

    painter.rect_filled(marquee_rect, 0.0, theme::marquee_fill());
    painter.rect_stroke(
        marquee_rect,
        0.0,
        egui::Stroke::new(1.5, theme::marquee_stroke()),
        egui::StrokeKind::Inside,
    );
}

fn marquee_rect(marquee: SelectionMarquee) -> Rect {
    Rect::from_two_pos(marquee.start, marquee.current)
}

fn handle_rename_input(ui: &mut Ui, state: &mut ExplorerState) {
    if state.view_options.renaming.is_none() {
        return;
    }

    if ui.input(|input| input.key_pressed(Key::Escape)) {
        state.view_options.cancel_rename();
        return;
    }

    if ui.input(|input| input.key_pressed(Key::Enter)) {
        state.commit_rename();
    }
}

fn list_header(ui: &mut Ui, metrics: &ViewMetrics, row_width: f32) {
    let icon_width = metrics.icon_size + 16.0;
    let cols = ListColumns::layout(row_width, icon_width);
    let header_height = metrics.list_row_height - 4.0;
    let font = egui::FontId::proportional(metrics.detail_size);
    let color = theme::text_muted();

    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(
            vec2(row_width, header_height),
            Sense::hover(),
        );

        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, theme::glass_stroke()),
        );

        paint_list_cell(
            ui,
            cell_rect(rect, cols.name_start(rect.min.x), cols.name_end(rect.min.x)),
            "Name",
            &font,
            color,
            Align2::LEFT_CENTER,
        );
        paint_list_cell(
            ui,
            cell_rect(
                rect,
                cols.modified_start(rect.min.x),
                cols.modified_end(rect.min.x),
            ),
            "Date modified",
            &font,
            color,
            Align2::LEFT_CENTER,
        );
        paint_list_cell(
            ui,
            cell_rect(rect, cols.kind_start(rect.min.x), cols.kind_end(rect.min.x)),
            "Type",
            &font,
            color,
            Align2::LEFT_CENTER,
        );
        paint_list_cell(
            ui,
            cell_rect(rect, cols.size_start(rect.min.x), cols.size_end(rect.min.x)),
            "Size",
            &font,
            color,
            Align2::RIGHT_CENTER,
        );
    });
}

fn icon_tile(
    ui: &mut Ui,
    state: &mut ExplorerState,
    entry: &FileEntry,
    metrics: &ViewMetrics,
    show_extensions: bool,
) -> Option<ItemRect> {
    let icon = if entry.is_dir { "📁" } else { "📄" };
    let path = entry.path.clone();
    let is_dir = entry.is_dir;
    let label = entry.display_name(show_extensions);
    let selected = state.view_options.is_selected(&path);
    let renaming = is_renaming(&state.view_options.renaming, &path);
    let text_color = if selected {
        theme::selection_text()
    } else {
        theme::text_primary()
    };

    let (rect, response) = ui.allocate_exact_size(
        vec2(metrics.tile_width, metrics.tile_height),
        Sense::click(),
    );

    paint_item_background(ui, rect, selected, response.hovered(), 6.0);

    ui.painter().text(
        rect.center() + vec2(0.0, -12.0),
        Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(metrics.icon_size),
        text_color,
    );

    let label_rect = egui::Rect::from_center_size(
        rect.center() + vec2(0.0, metrics.tile_height * 0.28),
        vec2(metrics.tile_width - 8.0, 32.0),
    );

    if renaming {
        show_rename_field(ui, state, &path, label_rect);
    } else {
        text::paint_truncated(
            ui,
            label_rect,
            &label,
            egui::FontId::proportional(metrics.label_size),
            text_color,
            Align2::CENTER_CENTER,
        );
    }

    handle_item_click(ui, state, &response, path.clone(), is_dir, label);

    Some(ItemRect { path, rect })
}

fn list_row(
    ui: &mut Ui,
    state: &mut ExplorerState,
    entry: &FileEntry,
    metrics: &ViewMetrics,
    show_extensions: bool,
    row_width: f32,
) -> Option<ItemRect> {
    let icon = if entry.is_dir { "📁" } else { "📄" };
    let path = entry.path.clone();
    let is_dir = entry.is_dir;
    let label = entry.display_name(show_extensions);
    let selected = state.view_options.is_selected(&path);
    let renaming = is_renaming(&state.view_options.renaming, &path);
    let icon_width = metrics.icon_size + 16.0;
    let cols = ListColumns::layout(row_width, icon_width);
    let name_font = egui::FontId::proportional(metrics.label_size);
    let detail_font = egui::FontId::proportional(metrics.detail_size);
    let primary = if selected {
        theme::selection_text()
    } else {
        theme::text_primary()
    };
    let muted = if selected {
        Color32::from_rgba_unmultiplied(230, 235, 245, 200)
    } else {
        theme::text_muted()
    };

    let mut item_rect = None;

    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(
            vec2(row_width, metrics.list_row_height),
            Sense::click(),
        );

        item_rect = Some(ItemRect {
            path: path.clone(),
            rect,
        });

        paint_item_background(ui, rect, selected, response.hovered(), 4.0);

        ui.painter().text(
            rect.left_center() + vec2(8.0, 0.0),
            Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(metrics.icon_size),
            primary,
        );

        let name_rect = cell_rect(rect, cols.name_start(rect.min.x), cols.name_end(rect.min.x));
        if renaming {
            show_rename_field(ui, state, &path, name_rect);
        } else {
            paint_list_cell(ui, name_rect, &label, &name_font, primary, Align2::LEFT_CENTER);
        }

        paint_list_cell(
            ui,
            cell_rect(
                rect,
                cols.modified_start(rect.min.x),
                cols.modified_end(rect.min.x),
            ),
            &entry.formatted_modified(),
            &detail_font,
            muted,
            Align2::LEFT_CENTER,
        );
        paint_list_cell(
            ui,
            cell_rect(rect, cols.kind_start(rect.min.x), cols.kind_end(rect.min.x)),
            &entry.type_label(),
            &detail_font,
            muted,
            Align2::LEFT_CENTER,
        );
        paint_list_cell(
            ui,
            cell_rect(rect, cols.size_start(rect.min.x), cols.size_end(rect.min.x)),
            &entry.formatted_size(),
            &detail_font,
            muted,
            Align2::RIGHT_CENTER,
        );

        handle_item_click(ui, state, &response, path.clone(), is_dir, label);
    });

    item_rect
}

fn paint_item_background(ui: &Ui, rect: Rect, selected: bool, hovered: bool, radius: f32) {
    if selected {
        ui.painter().rect_filled(rect, radius, theme::selection_fill());
        ui.painter().rect_stroke(
            rect,
            radius,
            egui::Stroke::new(1.0, theme::selection_stroke()),
            egui::StrokeKind::Inside,
        );
    } else if hovered {
        ui.painter().rect_filled(rect, radius, theme::maximize_hover());
    }
}

fn is_renaming(renaming: &Option<crate::explorer::RenameState>, path: &Path) -> bool {
    renaming.as_ref().is_some_and(|rename| rename.path == path)
}

fn show_rename_field(ui: &mut Ui, state: &mut ExplorerState, path: &Path, rect: Rect) {
    let Some(rename) = state.view_options.renaming.as_mut() else {
        return;
    };
    if rename.path != *path {
        return;
    }

    let response = ui.put(
        rect,
        egui::TextEdit::singleline(&mut rename.text)
            .font(egui::FontId::proportional(12.0))
            .desired_width(rect.width()),
    );
    response.request_focus();
}

fn handle_item_click(
    ui: &Ui,
    state: &mut ExplorerState,
    response: &Response,
    path: PathBuf,
    is_dir: bool,
    label: String,
) {
    let multi = ui.input(|input| multi_select_modifiers(&input.modifiers));
    let is_selected = state.view_options.is_selected(&path);

    if response.double_clicked() {
        state.view_options.cancel_rename();
        state.selection_marquee = None;
        if is_dir {
            state.navigate_active(path);
        } else {
            open_path(&path);
        }
        return;
    }

    if response.clicked() && !response.dragged() {
        if state.selection_marquee.is_some() {
            return;
        }
        state.selection_marquee = None;

        if multi {
            state.view_options.toggle_in_selection(path);
        } else if is_selected {
            state.view_options.start_rename(path, label);
        } else {
            state.view_options.select_only(path);
        }
    }
}

fn cell_rect(row: Rect, min_x: f32, max_x: f32) -> Rect {
    Rect::from_min_max(egui::pos2(min_x, row.min.y), egui::pos2(max_x, row.max.y))
}

fn paint_list_cell(
    ui: &Ui,
    rect: Rect,
    value: &str,
    font: &egui::FontId,
    color: egui::Color32,
    align: Align2,
) {
    if value.is_empty() {
        return;
    }
    text::paint_truncated(ui, rect, value, font.clone(), color, align);
}
