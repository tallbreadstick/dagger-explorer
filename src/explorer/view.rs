use std::path::{Path, PathBuf};

use super::fs::FileEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
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
        self.renaming = Some(RenameState { path, text });
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
