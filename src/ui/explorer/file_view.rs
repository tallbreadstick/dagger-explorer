use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, Frame, Key, LayerId, Order, Rect, Response, ScrollArea, Sense, Shape,
    Stroke, Ui, UiBuilder, pos2, vec2,
};
use eframe::egui::containers::scroll_area::ScrollSource;

use crate::explorer::{
    ExplorerState, FileEntry, SelectionMarquee, ViewMode, multi_select_modifiers, open_path,
};
use crate::ui::{theme, text};

const MARQUEE_MIN: f32 = 3.0;
const MARQUEE_PREVIEW_MAX: usize = 150;
const MAX_ICON_FILL: f32 = 0.75;
const TILE_LABEL_HEIGHT: f32 = 32.0;
const LIST_ICON_COLUMN_PADDING: f32 = 16.0;
const CONTEXT_MENU_WIDTH: f32 = 220.0;
const CONTEXT_MENU_ROW_HEIGHT: f32 = 24.0;

struct ViewMetrics {
    tile_width: f32,
    tile_height: f32,
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
                label_size: 10.0,
                list_mode: false,
                list_row_height: 0.0,
                detail_size: 0.0,
            },
            ViewMode::LargeIcons => Self {
                tile_width: 96.0,
                tile_height: 96.0,
                label_size: 11.0,
                list_mode: false,
                list_row_height: 0.0,
                detail_size: 0.0,
            },
            ViewMode::SmallList => Self {
                tile_width: 0.0,
                tile_height: 0.0,
                label_size: 11.0,
                list_mode: true,
                list_row_height: 24.0,
                detail_size: 10.0,
            },
            ViewMode::LargeList => Self {
                tile_width: 0.0,
                tile_height: 0.0,
                label_size: 12.0,
                list_mode: true,
                list_row_height: 36.0,
                detail_size: 11.0,
            },
        }
    }

    /// Longest side of icons/thumbnails, up to 75% of the available cell space.
    fn icon_max_side(&self) -> f32 {
        if self.list_mode {
            self.list_row_height * MAX_ICON_FILL
        } else {
            let icon_width = self.tile_width;
            let icon_height = (self.tile_height - TILE_LABEL_HEIGHT).max(1.0);
            icon_width.min(icon_height) * MAX_ICON_FILL
        }
    }

    fn list_icon_column_width(&self) -> f32 {
        self.icon_max_side() + LIST_ICON_COLUMN_PADDING
    }

    fn tile_icon_center(&self, rect: Rect) -> egui::Pos2 {
        let icon_area = Rect {
            min: rect.min,
            max: egui::pos2(rect.max.x, rect.max.y - TILE_LABEL_HEIGHT),
        };
        icon_area.center()
    }

    fn tile_label_center(&self, rect: Rect) -> egui::Pos2 {
        rect.center() + vec2(0.0, (self.tile_height - TILE_LABEL_HEIGHT) * 0.5)
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
            .fill(egui::Color32::TRANSPARENT)
            .inner_margin(egui::Margin::symmetric(8, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_max_width(ui.available_width());
                ui.set_min_height(area.height() - 16.0);

                if let Some(clip) = state.file_view_bounds {
                    ui.set_clip_rect(clip);
                }

                let width = ui.available_width();
                let viewport_h = ui.available_height();

                enum FileViewBody {
                    Loading,
                    Empty,
                    Entries(Arc<Vec<FileEntry>>),
                }

                let body = if let Some(listing) = state.fs_cache.listing(&path) {
                    let (entries, listing_incomplete) = {
                        let listing_guard = listing.lock().expect("directory listing lock");
                        let listing_incomplete = !listing_guard.complete;
                        let entries = state.prepared_entries_for(&path, &listing_guard.entries);
                        (entries, listing_incomplete)
                    };

                    if entries.is_empty() && listing_incomplete {
                        FileViewBody::Loading
                    } else if entries.is_empty() {
                        FileViewBody::Empty
                    } else {
                        FileViewBody::Entries(entries)
                    }
                } else {
                    FileViewBody::Loading
                };

                let content_h = match &body {
                    FileViewBody::Loading | FileViewBody::Empty => viewport_h,
                    FileViewBody::Entries(entries) => {
                        content_height(entries.len(), &metrics, width, viewport_h)
                    }
                };

                let mut item_rects = Vec::new();

                ScrollArea::vertical()
                    .id_salt("file_view_scroll")
                    .auto_shrink([false, false])
                    .scroll_source(ScrollSource::MOUSE_WHEEL | ScrollSource::SCROLL_BAR)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.set_max_width(width);
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

                            match &body {
                                FileViewBody::Loading => {
                                    ui.with_layout(
                                        egui::Layout::centered_and_justified(
                                            egui::Direction::TopDown,
                                        ),
                                        |ui| {
                                            ui.spinner();
                                            ui.label(
                                                egui::RichText::new("Loading…")
                                                    .color(theme::text_muted()),
                                            );
                                        },
                                    );
                                }
                                FileViewBody::Empty => {
                                    ui.with_layout(
                                        egui::Layout::centered_and_justified(
                                            egui::Direction::TopDown,
                                        ),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new("This folder is empty.")
                                                    .color(theme::text_muted()),
                                            );
                                        },
                                    );
                                }
                                FileViewBody::Entries(entries) => {
                                    if metrics.list_mode {
                                        list_header(ui, &metrics, width);
                                        ui.add_space(2.0);
                                        let row_height = metrics.list_row_height.max(1.0);
                                        let start_y = ui.cursor().top();
                                        let (min_row, max_row) = visible_row_range(
                                            ui.clip_rect(),
                                            start_y,
                                            row_height,
                                            entries.len(),
                                        );
                                        ui.add_space(min_row as f32 * row_height);
                                        for row in min_row..max_row {
                                            if let Some(item) = list_row(
                                                ui,
                                                state,
                                                &entries[row],
                                                &metrics,
                                                show_extensions,
                                                width,
                                            ) {
                                                item_rects.push(item);
                                            }
                                        }
                                        ui.add_space((entries.len().saturating_sub(max_row) as f32) * row_height);
                                    } else {
                                        let tile_step = metrics.tile_width + 8.0;
                                        let cols =
                                            ((width / tile_step).floor() as usize).max(1);
                                        let row_height = metrics.tile_height + 8.0;
                                        let total_rows = entries.len().div_ceil(cols);
                                        let start_y = ui.cursor().top();
                                        let (min_row, max_row) = visible_row_range(
                                            ui.clip_rect(),
                                            start_y,
                                            row_height,
                                            total_rows,
                                        );
                                        ui.add_space(min_row as f32 * row_height);
                                        for row in min_row..max_row {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 8.0;
                                                for col in 0..cols {
                                                    let index = row * cols + col;
                                                    if index >= entries.len() {
                                                        break;
                                                    }
                                                    if let Some(item) = icon_tile(
                                                        ui,
                                                        state,
                                                        &entries[index],
                                                        &metrics,
                                                        show_extensions,
                                                    ) {
                                                        item_rects.push(item);
                                                }
                                                }
                                            });
                                            ui.add_space(8.0);
                                        }
                                        ui.add_space((total_rows.saturating_sub(max_row) as f32) * row_height);
                                    }
                                }
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
                        bg_response.context_menu(|ui| {
                            let pointer_on_item = ui
                                .ctx()
                                .input(|input| input.pointer.latest_pos())
                                .is_some_and(|pos| item_rects.iter().any(|item| item.rect.contains(pos)));
                            if pointer_on_item {
                                return;
                            }
                            show_background_context_menu(ui, state);
                        });
                    });

                let clip = state.file_view_bounds.unwrap_or(area);
                paint_marquee_overlay(ui.ctx(), state, &item_rects, clip);

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
        let tile_step = metrics.tile_width + 8.0;
        let cols = ((width / tile_step).floor() as usize).max(1);
        let rows = entry_count.div_ceil(cols);
        rows as f32 * (metrics.tile_height + 8.0)
    };

    items_h.max(viewport_h)
}

fn visible_row_range(
    clip_rect: Rect,
    rows_start_y: f32,
    row_height: f32,
    total_rows: usize,
) -> (usize, usize) {
    if total_rows == 0 {
        return (0, 0);
    }

    let min_row = ((clip_rect.top() - rows_start_y) / row_height).floor().max(0.0) as usize;
    let max_row = ((clip_rect.bottom() - rows_start_y) / row_height).ceil().max(0.0) as usize + 1;
    (min_row.min(total_rows), max_row.min(total_rows).max(min_row.min(total_rows)))
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
        state.cancel_active_rename();
        return;
    }

    if ui.input(|input| input.key_pressed(Key::Enter)) {
        state.commit_rename();
    }
}

fn list_header(ui: &mut Ui, metrics: &ViewMetrics, row_width: f32) {
    let icon_width = metrics.list_icon_column_width();
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
    let icon_default = if selected {
        theme::selection_text()
    } else {
        theme::default_icon_color()
    };
    let icon_color = state.icon_color_for(&path).unwrap_or(icon_default);

    let (rect, response) = ui.allocate_exact_size(
        vec2(metrics.tile_width, metrics.tile_height),
        Sense::click(),
    );

    paint_item_background(ui, rect, selected, response.hovered(), 6.0);

    paint_file_icon(
        ui,
        state,
        entry,
        metrics.tile_icon_center(rect),
        metrics.icon_max_side(),
        icon_color,
    );

    let label_rect = egui::Rect::from_center_size(
        metrics.tile_label_center(rect),
        vec2(metrics.tile_width - 8.0, TILE_LABEL_HEIGHT),
    );

    if renaming {
        show_rename_field(ui, state, &path, label_rect, Align2::CENTER_CENTER);
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
    response.context_menu(|ui| {
        show_selection_context_menu(ui, state);
    });

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
    let path = entry.path.clone();
    let is_dir = entry.is_dir;
    let label = entry.display_name(show_extensions);
    let selected = state.view_options.is_selected(&path);
    let renaming = is_renaming(&state.view_options.renaming, &path);
    let icon_width = metrics.list_icon_column_width();
    let cols = ListColumns::layout(row_width, icon_width);
    let name_font = egui::FontId::proportional(metrics.label_size);
    let detail_font = egui::FontId::proportional(metrics.detail_size);
    let primary = if selected {
        theme::selection_text()
    } else {
        theme::text_primary()
    };
    let icon_color = state
        .icon_color_for(&path)
        .unwrap_or(theme::default_icon_color());
    let muted = if selected {
        Color32::from_rgba_unmultiplied(230, 235, 245, 200)
    } else {
        theme::text_muted()
    };

    let (rect, response) = ui.allocate_exact_size(
        vec2(row_width, metrics.list_row_height),
        Sense::click(),
    );

    paint_item_background(ui, rect, selected, response.hovered(), 4.0);

    let icon_side = metrics.icon_max_side();
    let icon_center = pos2(
        rect.min.x + LIST_ICON_COLUMN_PADDING * 0.5 + icon_side * 0.5,
        rect.center().y,
    );
    paint_file_icon(ui, state, entry, icon_center, icon_side, icon_color);

    let name_rect = cell_rect(rect, cols.name_start(rect.min.x), cols.name_end(rect.min.x));
    if renaming {
        show_rename_field(ui, state, &path, name_rect, Align2::LEFT_CENTER);
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
    response.context_menu(|ui| {
        show_selection_context_menu(ui, state);
    });

    Some(ItemRect { path, rect })
}

fn paint_file_icon(
    ui: &Ui,
    state: &mut ExplorerState,
    entry: &FileEntry,
    center: egui::Pos2,
    size: f32,
    fallback_color: Color32,
) {
    if entry.is_dir {
        ui.painter().text(
            center,
            Align2::CENTER_CENTER,
            "📁",
            egui::FontId::proportional(size),
            fallback_color,
        );
        return;
    }

    if let (Some(texture), Some(display)) = (
        state.thumbnails.texture(&entry.path),
        state.thumbnails.display_size(&entry.path, size),
    ) {
        let rect = Rect::from_center_size(center, display);
        ui.painter().image(
            texture,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        if state.thumbnails.is_video_thumbnail(&entry.path) {
            paint_video_play_overlay(ui.painter(), rect);
        }
    } else {
        ui.painter().text(
            center,
            Align2::CENTER_CENTER,
            "📄",
            egui::FontId::proportional(size),
            fallback_color,
        );
    }
}

fn paint_video_play_overlay(painter: &egui::Painter, thumb_rect: Rect) {
    let badge = thumb_rect.width().min(thumb_rect.height()) * 0.34;
    if badge < 6.0 {
        return;
    }

    let center = thumb_rect.center();
    let radius = badge * 0.52;
    painter.circle_filled(
        center,
        radius,
        Color32::from_rgba_unmultiplied(0, 0, 0, 150),
    );
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 180)),
    );

    let half_h = badge * 0.22;
    let half_w = badge * 0.20;
    let tip = vec2(half_w * 0.85, 0.0);
    let left = vec2(-half_w * 0.55, -half_h);
    let right = vec2(-half_w * 0.55, half_h);
    painter.add(Shape::convex_polygon(
        vec![
            center + left,
            center + right,
            center + tip,
        ],
        Color32::WHITE,
        Stroke::NONE,
    ));
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

fn show_rename_field(
    ui: &mut Ui,
    state: &mut ExplorerState,
    path: &Path,
    rect: Rect,
    align: Align2,
) {
    use egui::text::{CCursor, CCursorRange};

    let Some(rename) = state.view_options.renaming.as_mut() else {
        return;
    };
    if rename.path != *path {
        return;
    }

    let (h_align, v_align) = match align {
        Align2::LEFT_CENTER => (egui::Align::LEFT, egui::Align::Center),
        Align2::CENTER_CENTER => (egui::Align::Center, egui::Align::Center),
        _ => (egui::Align::LEFT, egui::Align::Center),
    };

    let response = ui.place(
        rect,
        egui::TextEdit::singleline(&mut rename.text)
            .id(ui.id().with("rename").with(path))
            .font(egui::FontId::proportional(12.0))
            .desired_width(rect.width())
            .horizontal_align(h_align)
            .vertical_align(v_align)
            .margin(egui::Margin::ZERO),
    );
    response.request_focus();

    if rename.select_all_on_focus {
        let id = ui.id().with("rename").with(path);
        if let Some(mut text_state) = egui::TextEdit::load_state(ui.ctx(), id) {
            let end = CCursor::new(rename.text.chars().count());
            text_state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(0),
                end,
            )));
            egui::TextEdit::store_state(ui.ctx(), id, text_state);
        }
        rename.select_all_on_focus = false;
    }
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

    if response.secondary_clicked() {
        state.selection_marquee = None;
        if !is_selected {
            state.view_options.select_only(path);
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

fn show_background_context_menu(ui: &mut Ui, state: &mut ExplorerState) {
    ui.set_min_width(CONTEXT_MENU_WIDTH);
    ui.set_max_width(CONTEXT_MENU_WIDTH);

    context_menu_action(ui, "New File", true, || state.new_file_in_active());
    context_menu_action(ui, "New Folder", true, || state.new_folder_in_active());
    ui.separator();
    context_menu_action(ui, "Open Terminal Here", true, || state.open_terminal_here());
    if state.can_paste() {
        context_menu_action(ui, "Paste item(s)", true, || state.paste_clipboard());
    }
    ui.separator();
    context_menu_action(ui, "Properties", true, || {
        let current = state.active_path();
        state.show_properties_for_paths(vec![current]);
    });
}

fn show_selection_context_menu(ui: &mut Ui, state: &mut ExplorerState) {
    ui.set_min_width(CONTEXT_MENU_WIDTH);
    ui.set_max_width(CONTEXT_MENU_WIDTH);

    let has_selection = state.view_options.has_selection();
    let single_selection = state.view_options.selected.len() == 1;
    let ctx = ui.ctx().clone();

    context_menu_action(ui, "Open", has_selection, || state.open_selection());
    context_menu_action(ui, "Open With", has_selection, || state.open_with_selection());
    ui.separator();
    context_menu_action(ui, "Cut", has_selection, || state.cut_selection());
    context_menu_action(ui, "Copy", has_selection, || state.copy_selection());
    context_menu_action(ui, "Copy as Path", has_selection, || {
        state.copy_selection_as_paths(&ctx);
    });
    context_menu_action(ui, "Rename", single_selection, || {
        state.start_rename_from_selection();
    });
    context_menu_action(ui, "Move to Trash", has_selection, || state.trash_selection());
    context_menu_action(ui, "Icon Color…", has_selection, || state.open_icon_color_dialog());
    ui.separator();
    context_menu_action(ui, "Properties", has_selection, || {
        state.show_properties_for_selection();
    });
}

fn context_menu_action<F: FnOnce()>(ui: &mut Ui, label: &str, enabled: bool, action: F) {
    let response = ui
        .add_enabled(
            enabled,
            egui::Button::new(
                egui::RichText::new(label)
                    .size(12.0)
                    .color(theme::text_primary()),
            )
            .frame(false)
            .min_size(vec2(CONTEXT_MENU_WIDTH - 12.0, CONTEXT_MENU_ROW_HEIGHT)),
        );

    if enabled && response.hovered() {
        ui.painter()
            .rect_filled(response.rect, 4.0, theme::maximize_hover());
    }

    if response.clicked() {
        action();
        ui.close();
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
