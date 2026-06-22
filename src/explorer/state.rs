use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::delete::{delete_paths_permanently, move_paths_to_trash};
use super::directory_loading::DirectoryLoadingBar;
use super::fs::{FsCache, open_path};
use super::paths::home_dir;
use super::preferences::Preferences;
use super::tab::ExplorerTab;
use super::thumbs::ThumbnailRuntime;
use super::transfer::{TransferManager, TransferMode};
use super::view::{
    FileViewOptions, LinearDirection, ViewMode, linear_extend_direction, prepare_entries,
    selection_neighbor_index,
};
use super::{get_system_clipboard, has_file_clipboard, set_system_clipboard, ClipboardOp, ClipboardMode};

#[derive(Debug)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub children_loaded: bool,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn root() -> Self {
        let home = home_dir();
        Self {
            name: home
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Home")
                .to_string(),
            path: home,
            is_dir: true,
            expanded: true,
            children_loaded: false,
            children: Vec::new(),
        }
    }

    pub fn from_path(path: PathBuf, is_dir: bool) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        Self {
            path,
            name,
            is_dir,
            expanded: false,
            children_loaded: false,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SelectionMarquee {
    pub start: egui::Pos2,
    pub current: egui::Pos2,
}

#[derive(Clone, Debug)]
struct PendingTreeClick {
    path: PathBuf,
    is_dir: bool,
    at: f64,
}

pub struct ExplorerState {
    pub tabs: Vec<ExplorerTab>,
    pub active_tab: usize,
    next_tab_id: u64,
    pub fs_cache: FsCache,
    pub file_tree: TreeNode,
    pub tab_scroll: f32,
    pub view_options: FileViewOptions,
    pub selection_marquee: Option<SelectionMarquee>,
    pub file_view_bounds: Option<egui::Rect>,
    pub transfer: TransferManager,
    pub thumbnails: ThumbnailRuntime,
    pub     directory_loading_bar: DirectoryLoadingBar,
    tree_click_pending: Option<PendingTreeClick>,
    clipboard_paste_available: bool,
    last_clipboard_check: f64,
    pub path_bar_edit: Option<PathBarEditState>,
    pub quick_toast: Option<QuickToast>,
}

#[derive(Clone, Debug)]
pub struct PathBarEditState {
    pub text: String,
    revert_path: PathBuf,
    pub completion_index: usize,
}

#[derive(Clone, Debug)]
pub struct QuickToast {
    pub message: String,
    pub expires_at: f64,
}

impl ExplorerState {
    pub fn new() -> Self {
        let home = home_dir();
        let preferences = Preferences::load();
        let mut view_options = FileViewOptions::default();
        preferences.apply_to(&mut view_options);

        let mut state = Self {
            tabs: vec![ExplorerTab::new(0, home.clone())],
            active_tab: 0,
            next_tab_id: 1,
            fs_cache: FsCache::new(),
            file_tree: TreeNode::root(),
            tab_scroll: 0.0,
            view_options,
            selection_marquee: None,
            file_view_bounds: None,
            transfer: TransferManager::new(),
            thumbnails: ThumbnailRuntime::new(),
            directory_loading_bar: DirectoryLoadingBar::default(),
            tree_click_pending: None,
            clipboard_paste_available: false,
            last_clipboard_check: 0.0,
            path_bar_edit: None,
            quick_toast: None,
        };
        state.fs_cache.request_listing(home);
        state
    }

    pub fn save_preferences(&self) {
        let preferences = Preferences::from_view_options(&self.view_options);
        if let Err(error) = preferences.save() {
            eprintln!("[dagger-explorer] failed to save preferences: {error}");
        }
    }

    pub fn active_tab(&self) -> &ExplorerTab {
        &self.tabs[self.active_tab]
    }

    pub fn active_tab_mut(&mut self) -> &mut ExplorerTab {
        &mut self.tabs[self.active_tab]
    }

    pub fn active_path(&self) -> PathBuf {
        self.active_tab().current.clone()
    }

    pub fn poll_fs(&mut self, ctx: &egui::Context) {
        let active_path = self.active_path();

        if self.fs_cache.poll() {
            ctx.request_repaint();
        }

        self.schedule_directory_thumbnails(&active_path);

        if self.thumbnails.poll(ctx) {
            ctx.request_repaint();
        }

        let loading = self.is_directory_loading(&active_path);
        let progress = self.directory_load_progress(&active_path);
        let dt = ctx.input(|input| input.stable_dt);
        if self.directory_loading_bar.update(loading, progress, dt) {
            ctx.request_repaint();
        }

        if self.poll_tree_click(ctx) {
            ctx.request_repaint();
        }

        if self.transfer.poll() {
            self.apply_transfer_invalidation();
            ctx.request_repaint();
        }
        self.refresh_clipboard_state(ctx);
    }

    fn is_directory_loading(&self, path: &Path) -> bool {
        self.fs_cache.is_listing_loading(path) || self.thumbnails.is_loading(path)
    }

    fn directory_load_progress(&self, path: &Path) -> Option<f32> {
        if self.fs_cache.is_listing_loading(path) {
            return self
                .fs_cache
                .listing_progress(path)
                .map(|listing_fraction| listing_fraction * 0.45);
        }

        if self.thumbnails.is_loading(path) {
            return Some(0.45 + self.thumbnails.progress(path) * 0.50);
        }

        None
    }

    fn schedule_directory_thumbnails(&mut self, path: &Path) {
        if self.thumbnails.directory().is_some_and(|dir| dir == path) {
            return;
        }

        let Some(listing) = self.fs_cache.listing(path) else {
            return;
        };

        let entries = {
            let Ok(guard) = listing.lock() else {
                return;
            };
            if !guard.complete {
                return;
            }
            guard.entries.clone()
        };

        self.thumbnails
            .begin_directory_load(path.to_path_buf(), &entries);
    }

    pub fn schedule_tree_click(&mut self, path: PathBuf, is_dir: bool, at: f64) {
        self.tree_click_pending = Some(PendingTreeClick { path, is_dir, at });
    }

    pub fn cancel_tree_click(&mut self) {
        self.tree_click_pending = None;
    }

    /// Fire a deferred tree single-click once the double-click window has passed.
    pub fn poll_tree_click(&mut self, ctx: &egui::Context) -> bool {
        let Some(pending) = self.tree_click_pending.clone() else {
            return false;
        };

        let now = ctx.input(|input| input.time);
        let delay = ctx.options(|options| options.input_options.max_double_click_delay);
        let elapsed = now - pending.at;
        if elapsed < delay {
            ctx.request_repaint_after(Duration::from_secs_f64(delay - elapsed));
            return false;
        }

        self.tree_click_pending = None;
        if pending.is_dir {
            self.navigate_active(pending.path);
        } else {
            open_path(&pending.path);
        }
        true
    }

    pub fn refresh_clipboard_state(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|input| input.time);
        if now - self.last_clipboard_check < 0.5 {
            return;
        }
        self.last_clipboard_check = now;
        self.clipboard_paste_available =
            self.view_options.clipboard.is_some() || has_file_clipboard();
    }

    fn refresh_clipboard_state_now(&mut self) {
        self.clipboard_paste_available =
            self.view_options.clipboard.is_some() || has_file_clipboard();
        self.last_clipboard_check = f64::MAX;
    }

    fn apply_transfer_invalidation(&mut self) {
        for path in self.transfer.take_invalidation() {
            self.fs_cache.invalidate(&path);
            self.fs_cache.request_listing(path);
        }

        if !self.transfer.is_active()
            && self.transfer.progress.error.is_none()
            && self.transfer.progress.label == "Done"
        {
            self.view_options.clear_selection();
            self.view_options.clipboard = None;
        }
    }

    pub fn can_paste(&self) -> bool {
        self.clipboard_paste_available
    }

    pub fn cut_selection(&mut self) {
        if self.view_options.selected.is_empty() {
            return;
        }
        let paths = self.view_options.selected.clone();
        let item_count = paths.len();
        self.view_options.clipboard = Some((ClipboardMode::Cut, paths.clone()));
        self.refresh_clipboard_state_now();
        let _ = set_system_clipboard(paths, ClipboardOp::Move);
        self.show_quick_toast(format!("Moving {item_count} item(s)"));
    }

    pub fn copy_selection(&mut self) {
        if self.view_options.selected.is_empty() {
            return;
        }
        let paths = self.view_options.selected.clone();
        let item_count = paths.len();
        self.view_options.clipboard = Some((ClipboardMode::Copy, paths.clone()));
        self.refresh_clipboard_state_now();
        let _ = set_system_clipboard(paths, ClipboardOp::Copy);
        self.show_quick_toast(format!("Copied {item_count} item(s)"));
    }

    pub fn paste_clipboard(&mut self) {
        if self.transfer.is_active() {
            return;
        }

        let (paths, mode) = match get_system_clipboard() {
            Ok((paths, op)) if !paths.is_empty() => (
                paths,
                match op {
                    ClipboardOp::Copy => TransferMode::Copy,
                    ClipboardOp::Move => TransferMode::Move,
                },
            ),
            _ => {
                let Some((clip_mode, paths)) = self.view_options.clipboard.clone() else {
                    return;
                };
                if paths.is_empty() {
                    return;
                }
                (
                    paths,
                    match clip_mode {
                        ClipboardMode::Copy => TransferMode::Copy,
                        ClipboardMode::Cut => TransferMode::Move,
                    },
                )
            }
        };

        let dest = self.active_path();
        self.show_quick_toast(format!("Pasting {} item(s)", paths.len()));
        self.transfer.start(paths, dest, mode);
    }

    fn show_quick_toast(&mut self, message: String) {
        self.quick_toast = Some(QuickToast {
            message,
            expires_at: f64::INFINITY,
        });
    }

    pub fn start_rename_from_selection(&mut self) {
        if self.view_options.selected.len() != 1 {
            return;
        }

        let path = self.view_options.selected[0].clone();
        let parent = self.active_path();
        let text = self
            .fs_cache
            .listing(&parent)
            .map(|listing| {
                listing
                    .lock()
                    .ok()
                    .and_then(|guard| {
                        guard
                            .entries
                            .iter()
                            .find(|entry| entry.path == path)
                            .map(|entry| entry.name.clone())
                    })
            })
            .flatten()
            .unwrap_or_else(|| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
                    .to_string()
            });

        self.view_options.start_rename(path, text);
    }

    pub fn trash_selection(&mut self) {
        if !self.view_options.has_selection() {
            return;
        }

        let paths = self.view_options.selected.clone();
        let _ = move_paths_to_trash(&paths);
        self.invalidate_after_delete(&paths);
        self.view_options.clear_selection();
    }

    pub fn delete_selection_permanently(&mut self) {
        if !self.view_options.has_selection() {
            return;
        }

        let paths = self.view_options.selected.clone();
        let _ = delete_paths_permanently(&paths);
        self.invalidate_after_delete(&paths);
        self.view_options.clear_selection();
    }

    fn invalidate_after_delete(&mut self, paths: &[PathBuf]) {
        let mut parents = HashSet::new();
        for path in paths {
            if let Some(parent) = path.parent() {
                parents.insert(parent.to_path_buf());
            }
        }
        for parent in parents {
            self.fs_cache.invalidate(&parent);
            self.fs_cache.request_listing(parent);
        }
    }

    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if self.view_options.renaming.is_some() || self.path_bar_edit.is_some() {
            return;
        }

        use egui::{Event, Key, KeyboardShortcut, Modifiers};

        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            if self.view_options.has_selection() {
                self.view_options.clear_selection();
                ctx.request_repaint();
                return;
            }
        }

        let view_mode = self.view_options.view_mode;
        let shift_extend = ctx.input(|input| {
            if !input.modifiers.shift {
                return None;
            }
            for key in [
                Key::Space,
                Key::ArrowUp,
                Key::ArrowDown,
                Key::ArrowLeft,
                Key::ArrowRight,
            ] {
                if input.key_pressed(key) {
                    return linear_extend_direction(view_mode, key);
                }
            }
            None
        });
        if let Some(direction) = shift_extend {
            if self.view_options.has_selection() && self.extend_selection_linear(direction) {
                ctx.request_repaint();
                return;
            }
        }

        let linear_move = ctx.input(|input| {
            if input.modifiers.shift {
                return None;
            }
            for key in [
                Key::Space,
                Key::ArrowUp,
                Key::ArrowDown,
                Key::ArrowLeft,
                Key::ArrowRight,
            ] {
                if input.key_pressed(key) {
                    return linear_extend_direction(view_mode, key);
                }
            }
            None
        });
        if let Some(direction) = linear_move {
            if self.move_selection_linear(direction) {
                ctx.request_repaint();
                return;
            }
        }

        if ctx.input(|input| input.key_pressed(Key::Space) && !input.modifiers.shift) {
            if self.advance_selection_with_space() {
                ctx.request_repaint();
                return;
            }
        }

        if self.view_options.selected.len() == 1 {
            let arrow = ctx.input(|input| {
                if input.modifiers.shift {
                    return None;
                }
                match view_mode {
                    ViewMode::SmallIcons | ViewMode::LargeIcons => {
                        if input.key_pressed(Key::ArrowUp) {
                            Some(Key::ArrowUp)
                        } else if input.key_pressed(Key::ArrowDown) {
                            Some(Key::ArrowDown)
                        } else {
                            None
                        }
                    }
                    ViewMode::SmallList | ViewMode::LargeList => None,
                }
            });
            if let Some(key) = arrow {
                if self.move_selection_with_arrow(key) {
                    ctx.request_repaint();
                    return;
                }
            }

            if ctx.input(|input| input.key_pressed(Key::Enter)) {
                let path = self.view_options.selected[0].clone();
                self.navigate_active(path);
                ctx.request_repaint();
                return;
            }
        }

        enum Action {
            Copy,
            Cut,
            Paste,
            Rename,
            Trash,
            DeletePermanent,
        }

        let mut action = None;
        let mut saw_paste_event = false;

        // egui-winit turns Ctrl+C/X/V into Event::Copy/Cut/Paste, not Event::Key.
        // Paste is only emitted when the *text* clipboard is non-empty, so file-only
        // clipboards produce no event on press — we fall back to Key::V release below.
        ctx.input_mut(|input| {
            if action.is_none() {
                let copy = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
                let cut = KeyboardShortcut::new(Modifiers::COMMAND, Key::X);
                let paste = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);

                if input.consume_shortcut(&copy) {
                    action = Some(Action::Copy);
                } else if input.consume_shortcut(&cut) {
                    action = Some(Action::Cut);
                } else if input.consume_shortcut(&paste) {
                    action = Some(Action::Paste);
                    saw_paste_event = true;
                }
            }

            input.events.retain(|event| {
                match event {
                    Event::Copy => {
                        if action.is_none() {
                            action = Some(Action::Copy);
                        }
                        false
                    }
                    Event::Cut => {
                        if action.is_none() {
                            action = Some(Action::Cut);
                        }
                        false
                    }
                    Event::Paste(_) => {
                        saw_paste_event = true;
                        if action.is_none() {
                            action = Some(Action::Paste);
                        }
                        false
                    }
                    _ => true,
                }
            });

            if action.is_none() && !saw_paste_event {
                input.events.retain(|event| {
                    if let Event::Key {
                        key: Key::V,
                        pressed: false,
                        modifiers,
                        ..
                    } = event
                    {
                        if modifiers.command {
                            action = Some(Action::Paste);
                            return false;
                        }
                    }
                    true
                });
            }

            if action.is_none() {
                let rename = KeyboardShortcut::new(Modifiers::COMMAND, Key::R);
                let trash = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
                let delete_permanent = KeyboardShortcut::new(Modifiers::SHIFT, Key::Delete);

                if input.consume_shortcut(&delete_permanent) {
                    action = Some(Action::DeletePermanent);
                } else if input.consume_shortcut(&trash) {
                    action = Some(Action::Trash);
                } else if input.consume_shortcut(&rename) {
                    action = Some(Action::Rename);
                }
            }
        });

        match action {
            Some(Action::Copy) => self.copy_selection(),
            Some(Action::Cut) => self.cut_selection(),
            Some(Action::Paste) => self.paste_clipboard(),
            Some(Action::Rename) if self.view_options.selected.len() == 1 => {
                self.start_rename_from_selection();
            }
            Some(Action::Trash) if self.view_options.has_selection() => self.trash_selection(),
            Some(Action::DeletePermanent) if self.view_options.has_selection() => {
                self.delete_selection_permanently();
            }
            _ => {}
        }
    }

    /// Move the sole selected item with arrow keys. Returns true if selection changed.
    fn move_selection_with_arrow(&mut self, key: egui::Key) -> bool {
        if self.view_options.selected.len() != 1 {
            return false;
        }

        let selected_path = self.view_options.selected[0].clone();
        let Some(entries) = self.visible_directory_entries() else {
            return false;
        };

        let Some(current_index) = entries.iter().position(|entry| entry.path == selected_path)
        else {
            return false;
        };

        const FILE_VIEW_INNER_MARGIN: f32 = 16.0;
        let panel_width = self
            .file_view_bounds
            .map(|rect| (rect.width() - FILE_VIEW_INNER_MARGIN).max(1.0))
            .unwrap_or(600.0);

        let Some(next_index) = selection_neighbor_index(
            current_index,
            entries.len(),
            self.view_options.view_mode,
            panel_width,
            key,
        ) else {
            return false;
        };

        self.view_options
            .select_only(entries[next_index].path.clone());
        true
    }

    fn visible_directory_entries(&self) -> Option<Vec<super::fs::FileEntry>> {
        let directory = self.active_path();
        let listing = self.fs_cache.listing(&directory)?;
        let guard = listing.lock().ok()?;
        if !guard.complete {
            return None;
        }
        Some(prepare_entries(&guard.entries, &self.view_options))
    }

    /// Indices `(min, max)` when the current selection is a contiguous run in entry order.
    fn contiguous_selection_range(&self, entries: &[super::fs::FileEntry]) -> Option<(usize, usize)> {
        let mut indices: Vec<usize> = self
            .view_options
            .selected
            .iter()
            .filter_map(|path| entries.iter().position(|entry| entry.path == *path))
            .collect();
        if indices.is_empty() {
            return None;
        }
        indices.sort_unstable();
        if indices.windows(2).any(|pair| pair[1] != pair[0] + 1) {
            return None;
        }
        Some((indices[0], *indices.last()?))
    }

    /// Linear next/prev without shift: collapse a contiguous selection onto the adjacent item.
    fn move_selection_linear(&mut self, direction: LinearDirection) -> bool {
        let Some(entries) = self.visible_directory_entries() else {
            return false;
        };
        let Some((min, max)) = self.contiguous_selection_range(&entries) else {
            return false;
        };

        let target = match direction {
            LinearDirection::Next if max + 1 < entries.len() => max + 1,
            LinearDirection::Prev if min > 0 => min - 1,
            _ => return false,
        };

        self.view_options
            .select_only(entries[target].path.clone());
        true
    }

    /// Shift + linear navigation key: extend a contiguous selection by one entry.
    fn extend_selection_linear(&mut self, direction: LinearDirection) -> bool {
        let Some(entries) = self.visible_directory_entries() else {
            return false;
        };
        if entries.is_empty() {
            return false;
        }

        let Some((min, max)) = self.contiguous_selection_range(&entries) else {
            return false;
        };

        let add_index = match direction {
            LinearDirection::Next if max + 1 < entries.len() => max + 1,
            LinearDirection::Prev if min > 0 => min - 1,
            _ => return false,
        };

        self.view_options
            .add_to_selection(entries[add_index].path.clone());
        self.view_options.selected.sort_by_key(|path| {
            entries
                .iter()
                .position(|entry| entry.path == *path)
                .unwrap_or(usize::MAX)
        });
        true
    }

    /// Space with no selection picks the first entry.
    fn advance_selection_with_space(&mut self) -> bool {
        let Some(entries) = self.visible_directory_entries() else {
            return false;
        };
        if entries.is_empty() || !self.view_options.selected.is_empty() {
            return false;
        }

        self.view_options.select_only(entries[0].path.clone());
        true
    }

    pub fn ensure_listing(&mut self, path: PathBuf) {
        self.fs_cache.request_listing(path);
    }

    pub fn start_path_bar_edit(&mut self) {
        let path = self.active_path();
        self.view_options.cancel_rename();
        self.path_bar_edit = Some(PathBarEditState {
            text: path.display().to_string(),
            revert_path: path,
            completion_index: 0,
        });
    }

    pub fn cancel_path_bar_edit(&mut self) {
        self.path_bar_edit = None;
    }

    pub fn commit_path_bar_edit(&mut self) {
        let Some(edit) = self.path_bar_edit.take() else {
            return;
        };

        let cwd = edit.revert_path.clone();
        if let Some(path) = super::paths::resolve_directory_path(&edit.text, &cwd) {
            if path != self.active_path() {
                self.navigate_active(path);
            }
        }
    }

    pub fn navigate_active(&mut self, path: PathBuf) {
        self.path_bar_edit = None;
        self.cancel_tree_click();
        if !path.is_dir() {
            open_path(&path);
            return;
        }
        self.view_options.on_directory_changed();
        self.selection_marquee = None;
        self.thumbnails.on_directory_changing();
        self.active_tab_mut().navigate(path.clone());
        self.fs_cache.request_listing(path);
    }

    pub fn commit_rename(&mut self) -> bool {
        let Some(rename) = self.view_options.renaming.take() else {
            return false;
        };

        let trimmed = rename.text.trim();
        if trimmed.is_empty() {
            return false;
        }

        let Some(parent) = rename.path.parent() else {
            return false;
        };

        let new_path = parent.join(trimmed);
        if new_path == rename.path {
            return false;
        }

        if std::fs::rename(&rename.path, &new_path).is_err() {
            self.view_options.renaming = Some(rename);
            return false;
        }

        self.fs_cache.invalidate(parent);
        self.fs_cache.request_listing(parent.to_path_buf());

        for selected in &mut self.view_options.selected {
            if *selected == rename.path {
                *selected = new_path.clone();
            }
        }

        true
    }

    pub fn go_back(&mut self) {
        self.path_bar_edit = None;
        if self.active_tab_mut().go_back() {
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
            self.thumbnails.on_directory_changing();
            let path = self.active_tab().current.clone();
            self.fs_cache.request_listing(path);
        }
    }

    pub fn go_forward(&mut self) {
        self.path_bar_edit = None;
        if self.active_tab_mut().go_forward() {
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
            self.thumbnails.on_directory_changing();
            let path = self.active_tab().current.clone();
            self.fs_cache.request_listing(path);
        }
    }

    pub fn go_up(&mut self) {
        self.path_bar_edit = None;
        if self.active_tab_mut().go_up() {
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
            self.thumbnails.on_directory_changing();
            let path = self.active_tab().current.clone();
            self.fs_cache.request_listing(path);
        }
    }

    pub fn refresh_active(&mut self) {
        let path = self.active_tab_mut().refresh_same();
        self.thumbnails.on_directory_changing();
        self.fs_cache.invalidate(&path);
        self.fs_cache.request_listing(path);
    }

    pub fn new_tab(&mut self) {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let path = home_dir();
        self.tabs.push(ExplorerTab::new(id, path.clone()));
        self.active_tab = self.tabs.len() - 1;
        self.fs_cache.request_listing(path);
    }

    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() == 1 {
            return;
        }
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        self.path_bar_edit = None;
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        } else if index < self.active_tab {
            self.active_tab -= 1;
        }
    }

    pub fn close_tab_by_id(&mut self, tab_id: u64) {
        if self.tabs.len() == 1 {
            return;
        }
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == tab_id) {
            self.close_tab(index);
        }
    }

    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.path_bar_edit = None;
            self.active_tab = index;
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
            self.thumbnails.on_directory_changing();
            let path = self.active_tab().current.clone();
            self.fs_cache.request_listing(path);
        }
    }

    pub fn set_active_tab_by_id(&mut self, tab_id: u64) {
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == tab_id) {
            self.set_active_tab(index);
        }
    }

    pub fn populate_tree_children(&mut self, path: &PathBuf) {
        if let Some(listing) = self.fs_cache.listing(path) {
            let entries = listing
                .lock()
                .ok()
                .filter(|guard| guard.complete)
                .map(|guard| guard.entries.clone());
            if let Some(entries) = entries {
                if let Some(node) = find_tree_node_mut(&mut self.file_tree, path) {
                    if !node.children_loaded {
                        node.children = entries
                            .iter()
                            .filter(|e| e.is_dir)
                            .map(|e| TreeNode::from_path(e.path.clone(), true))
                            .collect();
                        node.children_loaded = true;
                    }
                }
            }
        } else {
            self.fs_cache.request_listing(path.clone());
        }
    }

    pub fn toggle_tree_expand(&mut self, path: &PathBuf) {
        let Some(expanded) = self.tree_node(path).map(|node| node.expanded) else {
            return;
        };

        if expanded {
            if let Some(node) = find_tree_node_mut(&mut self.file_tree, path) {
                collapse_node_and_descendants(node);
            }
            return;
        }

        self.load_tree_path(path);
        enforce_single_branch(&mut self.file_tree, path);

        if let Some(node) = find_tree_node_mut(&mut self.file_tree, path) {
            node.expanded = true;
            if !node.children_loaded {
                self.populate_tree_children(path);
            }
        }
    }

    fn load_tree_path(&mut self, target: &Path) {
        use std::path::Component;

        let root = self.file_tree.path.clone();
        if !target.starts_with(&root) {
            return;
        }

        self.populate_tree_children(&root);

        let mut current = root;
        if target == current.as_path() {
            return;
        }

        let Ok(relative) = target.strip_prefix(&current) else {
            return;
        };

        for component in relative.components() {
            if let Component::Normal(name) = component {
                current.push(name);
                self.populate_tree_children(&current);
                if current.as_path() == target {
                    break;
                }
            }
        }
    }

    pub fn tree_node(&self, path: &PathBuf) -> Option<&TreeNode> {
        find_tree_node(&self.file_tree, path)
    }
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self::new()
    }
}

fn find_tree_node_mut<'a>(node: &'a mut TreeNode, path: &PathBuf) -> Option<&'a mut TreeNode> {
    if &node.path == path {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = find_tree_node_mut(child, path) {
            return Some(found);
        }
    }
    None
}

fn find_tree_node<'a>(node: &'a TreeNode, path: &PathBuf) -> Option<&'a TreeNode> {
    if &node.path == path {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_tree_node(child, path) {
            return Some(found);
        }
    }
    None
}

/// Keep only the ancestor chain leading to `target` expanded; collapse all siblings and cousins.
fn enforce_single_branch(node: &mut TreeNode, target: &Path) {
    if target.starts_with(node.path.as_path()) {
        node.expanded = true;
        if node.path.as_path() == target {
            for child in &mut node.children {
                collapse_node_and_descendants(child);
            }
        } else {
            for child in &mut node.children {
                if target.starts_with(child.path.as_path()) {
                    enforce_single_branch(child, target);
                } else {
                    collapse_node_and_descendants(child);
                }
            }
        }
    } else {
        collapse_node_and_descendants(node);
    }
}

fn collapse_node_and_descendants(node: &mut TreeNode) {
    node.expanded = false;
    for child in &mut node.children {
        collapse_node_and_descendants(child);
    }
}
