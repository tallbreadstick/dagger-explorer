use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use jwalk::WalkDir;
use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
    pub is_hidden: bool,
}

impl FileEntry {
    pub fn extension(&self) -> Option<&str> {
        if self.is_dir {
            None
        } else {
            self.path.extension()?.to_str()
        }
    }

    pub fn display_name(&self, show_extensions: bool) -> String {
        if self.is_dir || show_extensions {
            return self.name.clone();
        }

        self.path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&self.name)
            .to_string()
    }

    pub fn type_label(&self) -> String {
        if self.is_dir {
            "Folder".to_string()
        } else if let Some(ext) = self.extension() {
            if ext.is_empty() {
                "File".to_string()
            } else {
                format!("{} file", ext.to_ascii_uppercase())
            }
        } else {
            "File".to_string()
        }
    }

    pub fn formatted_size(&self) -> String {
        if self.is_dir {
            return String::new();
        }
        format_byte_size(self.size)
    }

    pub fn formatted_modified(&self) -> String {
        DateTime::<Local>::from(self.modified)
            .format("%b %d, %Y  %H:%M")
            .to_string()
    }
}

fn format_byte_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{size} B")
    }
}

pub fn list_directory(path: &Path) -> Vec<FileEntry> {
    let mut entries: Vec<FileEntry> = WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .sort(true)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let file_type = entry.file_type();
            let name = path.file_name()?.to_str()?.to_string();
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            let size = if file_type.is_dir() { 0 } else { metadata.len() };
            let is_hidden = is_hidden_entry(&name, &metadata);

            Some(FileEntry {
                name,
                path: path.to_path_buf(),
                is_dir: file_type.is_dir(),
                size,
                modified,
                is_hidden,
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

fn is_hidden_entry(name: &str, metadata: &std::fs::Metadata) -> bool {
    if name.starts_with('.') {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return metadata.file_attributes() & 0x2 != 0;
    }

    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

pub fn open_path(path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy();
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &path_str])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = path;
    }
}

pub struct FsCache {
    listings: std::collections::HashMap<PathBuf, Arc<Vec<FileEntry>>>,
    pending: Vec<(PathBuf, Receiver<Vec<FileEntry>>)>,
}

impl FsCache {
    pub fn new() -> Self {
        Self {
            listings: std::collections::HashMap::new(),
            pending: Vec::new(),
        }
    }

    pub fn listing(&self, path: &Path) -> Option<Arc<Vec<FileEntry>>> {
        self.listings.get(path).cloned()
    }

    pub fn request_listing(&mut self, path: PathBuf) {
        if self.listings.contains_key(&path) {
            return;
        }
        if self.pending.iter().any(|(p, _)| p == &path) {
            return;
        }

        let (tx, rx) = mpsc::channel();
        let path_for_thread = path.clone();
        std::thread::spawn(move || {
            let entries = list_directory(&path_for_thread);
            let _ = tx.send(entries);
        });
        self.pending.push((path, rx));
    }

    pub fn invalidate(&mut self, path: &Path) {
        self.listings.remove(path);
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        self.pending.retain_mut(|(path, rx)| {
            match rx.try_recv() {
                Ok(entries) => {
                    self.listings.insert(path.clone(), Arc::new(entries));
                    changed = true;
                    false
                }
                Err(mpsc::TryRecvError::Empty) => true,
                Err(mpsc::TryRecvError::Disconnected) => false,
            }
        });
        changed
    }
}

impl Default for FsCache {
    fn default() -> Self {
        Self::new()
    }
}
