use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use jwalk::WalkDir;

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

fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

enum ListingEvent {
    Entry(FileEntry),
    Complete,
}

fn stream_directory(path: &Path, tx: mpsc::Sender<ListingEvent>) {
    for entry in WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .sort(true)
        .into_iter()
        .flatten()
    {
        let entry_path = entry.path();
        let file_type = entry.file_type();
        let name = match entry_path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let modified = match metadata.modified() {
            Ok(modified) => modified,
            Err(_) => continue,
        };
        let size = if file_type.is_dir() { 0 } else { metadata.len() };
        let is_hidden = is_hidden_entry(&name, &metadata);

        let _ = tx.send(ListingEvent::Entry(FileEntry {
            name,
            path: entry_path.to_path_buf(),
            is_dir: file_type.is_dir(),
            size,
            modified,
            is_hidden,
        }));
    }
    let _ = tx.send(ListingEvent::Complete);
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

#[derive(Debug)]
pub struct DirectoryListing {
    pub entries: Vec<FileEntry>,
    pub complete: bool,
}

impl DirectoryListing {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            complete: false,
        }
    }
}

pub type SharedListing = Arc<Mutex<DirectoryListing>>;

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
    listings: std::collections::HashMap<PathBuf, SharedListing>,
    pending: Vec<(PathBuf, Receiver<ListingEvent>)>,
}

impl FsCache {
    pub fn new() -> Self {
        Self {
            listings: std::collections::HashMap::new(),
            pending: Vec::new(),
        }
    }

    pub fn listing(&self, path: &Path) -> Option<SharedListing> {
        self.listings.get(path).cloned()
    }

    pub fn is_listing_loading(&self, path: &Path) -> bool {
        if self.pending.iter().any(|(pending, _)| pending == path) {
            return true;
        }

        self.listings
            .get(path)
            .and_then(|listing| listing.lock().ok())
            .is_some_and(|guard| !guard.complete)
    }

    /// Soft progress while entries stream in; `1.0` once complete.
    pub fn listing_progress(&self, path: &Path) -> Option<f32> {
        let listing = self.listings.get(path)?;
        let guard = listing.lock().ok()?;
        if guard.complete {
            return Some(1.0);
        }
        if guard.entries.is_empty() {
            return Some(0.05);
        }
        Some(1.0 - (-(guard.entries.len() as f32) / 80.0).exp())
    }

    pub fn request_listing(&mut self, path: PathBuf) {
        if let Some(listing) = self.listings.get(&path) {
            if let Ok(guard) = listing.lock() {
                if guard.complete {
                    return;
                }
            }
        }
        if self.pending.iter().any(|(p, _)| p == &path) {
            return;
        }

        let shared = Arc::new(Mutex::new(DirectoryListing::new()));
        self.listings.insert(path.clone(), Arc::clone(&shared));

        let (tx, rx) = mpsc::channel();
        let path_for_thread = path.clone();
        std::thread::spawn(move || stream_directory(&path_for_thread, tx));
        self.pending.push((path, rx));
    }

    pub fn invalidate(&mut self, path: &Path) {
        self.listings.remove(path);
        self.pending.retain(|(pending, _)| pending != path);
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        self.pending.retain_mut(|(path, rx)| {
            loop {
                match rx.try_recv() {
                    Ok(ListingEvent::Entry(entry)) => {
                        if let Some(listing) = self.listings.get(path) {
                            if let Ok(mut guard) = listing.lock() {
                                guard.entries.push(entry);
                            }
                        }
                        changed = true;
                    }
                    Ok(ListingEvent::Complete) => {
                        if let Some(listing) = self.listings.get(path) {
                            if let Ok(mut guard) = listing.lock() {
                                sort_entries(&mut guard.entries);
                                guard.complete = true;
                            }
                        }
                        changed = true;
                        return false;
                    }
                    Err(mpsc::TryRecvError::Empty) => return true,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if let Some(listing) = self.listings.get(path) {
                            if let Ok(mut guard) = listing.lock() {
                                sort_entries(&mut guard.entries);
                                guard.complete = true;
                            }
                        }
                        changed = true;
                        return false;
                    }
                }
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
