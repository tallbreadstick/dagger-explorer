use std::path::{Path, PathBuf};

use super::fs::{FsCache, open_path};
use super::paths::home_dir;
use super::tab::ExplorerTab;
use super::transfer::{TransferManager, TransferMode};
use super::view::FileViewOptions;
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
    clipboard_paste_available: bool,
    last_clipboard_check: f64,
}

impl ExplorerState {
    pub fn new() -> Self {
        let home = home_dir();
        let mut state = Self {
            tabs: vec![ExplorerTab::new(0, home.clone())],
            active_tab: 0,
            next_tab_id: 1,
            fs_cache: FsCache::new(),
            file_tree: TreeNode::root(),
            tab_scroll: 0.0,
            view_options: FileViewOptions::default(),
            selection_marquee: None,
            file_view_bounds: None,
            transfer: TransferManager::new(),
            clipboard_paste_available: false,
            last_clipboard_check: 0.0,
        };
        state.fs_cache.request_listing(home);
        state
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
        if self.fs_cache.poll() {
            ctx.request_repaint();
        }
        if self.transfer.poll() {
            self.apply_transfer_invalidation();
            ctx.request_repaint();
        }
        self.refresh_clipboard_state(ctx);
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
        if set_system_clipboard(paths.clone(), ClipboardOp::Move).is_ok() {
            self.view_options.clipboard = Some((ClipboardMode::Cut, paths));
            self.refresh_clipboard_state_now();
        }
    }

    pub fn copy_selection(&mut self) {
        if self.view_options.selected.is_empty() {
            return;
        }
        let paths = self.view_options.selected.clone();
        if set_system_clipboard(paths.clone(), ClipboardOp::Copy).is_ok() {
            self.view_options.clipboard = Some((ClipboardMode::Copy, paths));
            self.refresh_clipboard_state_now();
        }
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
        self.transfer.start(paths, dest, mode);
    }

    pub fn handle_clipboard_shortcuts(&mut self, ui: &egui::Ui) {
        let pressed = ui.input(|input| {
            let mods = input.modifiers;
            if !(mods.command || mods.ctrl) {
                return None;
            }
            if input.key_pressed(egui::Key::C) {
                Some("copy")
            } else if input.key_pressed(egui::Key::X) {
                Some("cut")
            } else if input.key_pressed(egui::Key::V) {
                Some("paste")
            } else {
                None
            }
        });

        match pressed {
            Some("copy") => self.copy_selection(),
            Some("cut") => self.cut_selection(),
            Some("paste") if self.can_paste() => self.paste_clipboard(),
            _ => {}
        }
    }

    pub fn ensure_listing(&mut self, path: PathBuf) {
        self.fs_cache.request_listing(path);
    }

    pub fn navigate_active(&mut self, path: PathBuf) {
        if !path.is_dir() {
            open_path(&path);
            return;
        }
        self.view_options.on_directory_changed();
        self.selection_marquee = None;
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
        if self.active_tab_mut().go_back() {
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
            let path = self.active_tab().current.clone();
            self.fs_cache.request_listing(path);
        }
    }

    pub fn go_forward(&mut self) {
        if self.active_tab_mut().go_forward() {
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
            let path = self.active_tab().current.clone();
            self.fs_cache.request_listing(path);
        }
    }

    pub fn go_up(&mut self) {
        if self.active_tab_mut().go_up() {
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
            let path = self.active_tab().current.clone();
            self.fs_cache.request_listing(path);
        }
    }

    pub fn refresh_active(&mut self) {
        let path = self.active_tab_mut().refresh_same();
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
            self.active_tab = index;
            self.view_options.on_directory_changed();
            self.selection_marquee = None;
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
            if let Some(node) = find_tree_node_mut(&mut self.file_tree, path) {
                if !node.children_loaded {
                    node.children = listing
                        .iter()
                        .filter(|e| e.is_dir)
                        .map(|e| TreeNode::from_path(e.path.clone(), true))
                        .collect();
                    node.children_loaded = true;
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
