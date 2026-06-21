use std::path::PathBuf;

use super::paths::{is_filesystem_root, tab_display_name};

#[derive(Clone, Debug)]
pub struct ExplorerTab {
    pub id: u64,
    pub current: PathBuf,
    back_stack: Vec<PathBuf>,
    forward_stack: Vec<PathBuf>,
}

impl ExplorerTab {
    pub fn new(id: u64, initial: PathBuf) -> Self {
        Self {
            id,
            current: initial,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
        }
    }

    pub fn label(&self) -> String {
        tab_display_name(&self.current)
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    pub fn can_go_up(&self) -> bool {
        !is_filesystem_root(&self.current)
    }

    pub fn navigate(&mut self, path: PathBuf) {
        if self.current == path {
            return;
        }
        self.back_stack.push(self.current.clone());
        self.forward_stack.clear();
        self.current = path;
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(previous) = self.back_stack.pop() {
            self.forward_stack.push(self.current.clone());
            self.current = previous;
            true
        } else {
            false
        }
    }

    pub fn go_forward(&mut self) -> bool {
        if let Some(next) = self.forward_stack.pop() {
            self.back_stack.push(self.current.clone());
            self.current = next;
            true
        } else {
            false
        }
    }

    pub fn go_up(&mut self) -> bool {
        if let Some(parent) = super::paths::parent_path(&self.current) {
            self.navigate(parent);
            true
        } else {
            false
        }
    }

    pub fn refresh_same(&mut self) -> PathBuf {
        self.current.clone()
    }
}
