use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::fs::FileEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    #[default]
    SmallIcons,
    LargeIcons,
    SmallList,
    LargeList,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SortField {
    #[default]
    Name,
    Date,
    FileSize,
    Type,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone, Debug)]
pub struct RenameState {
    pub path: PathBuf,
    pub text: String,
    pub select_all_on_focus: bool,
    pub cancel_removes_target: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FileViewOptions {
    pub view_mode: ViewMode,
    pub sort_field: SortField,
    pub sort_order: SortOrder,
    pub show_hidden_files: bool,
    pub show_file_extensions: bool,
    pub selected: Vec<PathBuf>,
    pub renaming: Option<RenameState>,
    pub clipboard: Option<(ClipboardMode, Vec<PathBuf>)>,
}

impl FileViewOptions {
    pub fn set_sort(&mut self, field: SortField, order: SortOrder) {
        self.sort_field = field;
        self.sort_order = order;
    }

    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    pub fn is_selected(&self, path: &Path) -> bool {
        self.selected.iter().any(|selected| selected == path)
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.renaming = None;
    }

    pub fn cancel_rename(&mut self) {
        self.renaming = None;
    }

    pub fn select_only(&mut self, path: PathBuf) {
        self.selected = vec![path];
        self.renaming = None;
    }

    pub fn toggle_in_selection(&mut self, path: PathBuf) {
        if let Some(index) = self.selected.iter().position(|selected| selected == &path) {
            self.selected.remove(index);
            if self.renaming.as_ref().is_some_and(|rename| rename.path == path) {
                self.renaming = None;
            }
        } else {
            self.selected.push(path);
        }
    }

    pub fn set_selection(&mut self, paths: Vec<PathBuf>) {
        self.selected = paths;
        if let Some(rename) = &self.renaming {
            if !self.is_selected(&rename.path) {
                self.renaming = None;
            }
        }
    }

    pub fn start_rename(&mut self, path: PathBuf, text: String) {
        self.renaming = Some(RenameState {
            path,
            text,
            select_all_on_focus: true,
            cancel_removes_target: false,
        });
    }

    pub fn start_rename_select_all(&mut self, path: PathBuf, text: String) {
        self.renaming = Some(RenameState {
            path,
            text,
            select_all_on_focus: true,
            cancel_removes_target: true,
        });
    }

    pub fn add_to_selection(&mut self, path: PathBuf) {
        if self.is_selected(&path) {
            return;
        }
        self.selected.push(path);
    }

    pub fn on_directory_changed(&mut self) {
        self.clear_selection();
    }
}

pub fn prepare_entries(listing: &[FileEntry], options: &FileViewOptions) -> Vec<FileEntry> {
    let mut entries: Vec<FileEntry> = listing
        .iter()
        .filter(|entry| options.show_hidden_files || !entry.is_hidden)
        .cloned()
        .collect();
    sort_entries(&mut entries, options.sort_field, options.sort_order);
    entries
}

fn sort_entries(entries: &mut [FileEntry], field: SortField, order: SortOrder) {
    use std::cmp::Ordering;

    entries.sort_by(|a, b| {
        let folder_cmp = b.is_dir.cmp(&a.is_dir);
        if folder_cmp != Ordering::Equal {
            return folder_cmp;
        }

        let cmp = match field {
            SortField::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortField::Date => timestamp(a.modified).cmp(&timestamp(b.modified)),
            SortField::FileSize => a.size.cmp(&b.size),
            SortField::Type => type_key(a).cmp(&type_key(b)),
        };

        match order {
            SortOrder::Ascending => cmp,
            SortOrder::Descending => cmp.reverse(),
        }
    });
}

fn timestamp(time: std::time::SystemTime) -> u64 {
    use std::time::UNIX_EPOCH;

    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn type_key(entry: &FileEntry) -> String {
    if entry.is_dir {
        String::new()
    } else {
        entry
            .extension()
            .unwrap_or("")
            .to_ascii_lowercase()
    }
}

pub fn multi_select_modifiers(modifiers: &eframe::egui::Modifiers) -> bool {
    modifiers.ctrl || modifiers.shift || modifiers.command || modifiers.mac_cmd
}

const GRID_TILE_GAP: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinearDirection {
    Next,
    Prev,
}

/// Spacebar / linear traversal direction for shift-extend in each view mode.
pub fn linear_extend_direction(mode: ViewMode, key: eframe::egui::Key) -> Option<LinearDirection> {
    use eframe::egui::Key;

    match mode {
        ViewMode::SmallIcons | ViewMode::LargeIcons => match key {
            Key::ArrowRight | Key::Space => Some(LinearDirection::Next),
            Key::ArrowLeft => Some(LinearDirection::Prev),
            _ => None,
        },
        ViewMode::SmallList | ViewMode::LargeList => match key {
            Key::ArrowDown | Key::Space => Some(LinearDirection::Next),
            Key::ArrowUp => Some(LinearDirection::Prev),
            _ => None,
        },
    }
}

fn grid_tile_width(mode: ViewMode) -> f32 {
    match mode {
        ViewMode::SmallIcons => 72.0,
        ViewMode::LargeIcons => 96.0,
        ViewMode::SmallList | ViewMode::LargeList => 0.0,
    }
}

pub fn grid_columns(panel_width: f32, mode: ViewMode) -> usize {
    let tile_width = grid_tile_width(mode);
    if tile_width <= 0.0 {
        return 1;
    }
    let tile_step = tile_width + GRID_TILE_GAP;
    ((panel_width / tile_step).floor() as usize).max(1)
}

/// Next entry index when navigating with arrow keys; `None` if the selection stays put.
pub fn selection_neighbor_index(
    current: usize,
    count: usize,
    mode: ViewMode,
    panel_width: f32,
    key: eframe::egui::Key,
) -> Option<usize> {
    if count == 0 || current >= count {
        return None;
    }

    match mode {
        ViewMode::SmallList | ViewMode::LargeList => match key {
            eframe::egui::Key::ArrowUp => current.checked_sub(1),
            eframe::egui::Key::ArrowDown if current + 1 < count => Some(current + 1),
            _ => None,
        },
        ViewMode::SmallIcons | ViewMode::LargeIcons => {
            let cols = grid_columns(panel_width, mode);
            let row = current / cols;
            let col = current % cols;

            match key {
                eframe::egui::Key::ArrowLeft if col > 0 => Some(current - 1),
                eframe::egui::Key::ArrowRight if col + 1 < cols && current + 1 < count => {
                    Some(current + 1)
                }
                eframe::egui::Key::ArrowUp if row > 0 => Some(current.saturating_sub(cols)),
                eframe::egui::Key::ArrowDown if current + cols < count => Some(current + cols),
                _ => None,
            }
        }
    }
}
