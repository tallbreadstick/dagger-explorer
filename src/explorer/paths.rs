use std::path::{Path, PathBuf};

use sysinfo::Disks;

#[derive(Clone, Debug)]
pub struct QuickAccessEntry {
    pub label: String,
    pub path: PathBuf,
    pub icon: &'static str,
}

pub fn home_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"))
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"))
    }
}

/// Application data directory: `~/.local/share/dagger` (Linux/macOS) or `%LOCALAPPDATA%\dagger` (Windows).
pub fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA")
            .map(|root| PathBuf::from(root).join("dagger"))
            .unwrap_or_else(|_| home_dir().join("AppData\\Local\\dagger"))
    } else if cfg!(target_os = "macos") {
        home_dir().join("Library/Application Support/dagger")
    } else {
        home_dir().join(".local/share/dagger")
    }
}

pub fn preferences_path() -> PathBuf {
    data_dir().join("preferences.json")
}

pub fn thumbs_db_path() -> PathBuf {
    data_dir().join("thumbs.db")
}

fn user_subdir(name: &str) -> PathBuf {
    home_dir().join(name)
}

pub fn quick_access_entries() -> Vec<QuickAccessEntry> {
    let mut entries = vec![QuickAccessEntry {
        label: "Home".to_string(),
        path: home_dir(),
        icon: "🏠",
    }];

    let standard = [
        ("Documents", "📄"),
        ("Downloads", "⬇"),
        ("Desktop", "🖥"),
        ("Pictures", "🖼"),
        ("Music", "🎵"),
        ("Videos", "🎬"),
    ];

    for (name, icon) in standard {
        let path = user_subdir(name);
        if path.is_dir() {
            entries.push(QuickAccessEntry {
                label: name.to_string(),
                path,
                icon,
            });
        }
    }

    if cfg!(not(target_os = "windows")) {
        let projects = user_subdir("Projects");
        if projects.is_dir() {
            entries.push(QuickAccessEntry {
                label: "Projects".to_string(),
                path: projects,
                icon: "📁",
            });
        }
    }

    entries
}

pub fn list_drives() -> Vec<PathBuf> {
    let disks = Disks::new_with_refreshed_list();
    let mut drives: Vec<PathBuf> = disks
        .list()
        .iter()
        .map(|disk| disk.mount_point().to_path_buf())
        .filter(|path| path.exists())
        .collect();

    if cfg!(not(target_os = "windows")) && !drives.iter().any(|drive| drive == Path::new("/")) {
        drives.push(PathBuf::from("/"));
    }

    drives.sort();
    drives.dedup();
    drives
}

pub fn tab_display_name(path: &Path) -> String {
    if path == home_dir().as_path() {
        return "Home".to_string();
    }

    if cfg!(target_os = "windows") {
        if path.components().count() <= 1 {
            return path
                .to_str()
                .unwrap_or("Drive")
                .trim_end_matches('\\')
                .to_string();
        }
    } else if path == Path::new("/") {
        return "Root".to_string();
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn segment_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

pub fn path_components(path: &Path) -> Vec<(String, PathBuf)> {
    let mut components = Vec::new();
    let mut accumulated = PathBuf::new();

    if cfg!(target_os = "windows") {
        if let Some(prefix) = path.components().next() {
            use std::path::Component;
            if let Component::Prefix(prefix) = prefix {
                accumulated = PathBuf::from(format!("{}:\\", prefix.as_os_str().to_string_lossy()));
                components.push((
                    accumulated.to_string_lossy().trim_end_matches('\\').to_string(),
                    accumulated.clone(),
                ));
            }
        }
    } else if path.has_root() {
        accumulated = PathBuf::from("/");
        components.push(("Root".to_string(), accumulated.clone()));
    }

    for component in path.components().skip(if cfg!(target_os = "windows") { 1 } else { 1 }) {
        use std::path::Component;
        if let Component::Normal(name) = component {
            accumulated.push(name);
            let label = segment_display_name(&accumulated);
            components.push((label, accumulated.clone()));
        }
    }

    components
}

pub fn parent_path(path: &Path) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        if path.components().count() <= 1 {
            return None;
        }
    } else if path == Path::new("/") {
        return None;
    }
    path.parent().map(Path::to_path_buf)
}

pub fn is_filesystem_root(path: &Path) -> bool {
    if cfg!(target_os = "windows") {
        path.components().count() <= 1
    } else {
        path == Path::new("/")
    }
}

pub fn expand_tilde(text: &str) -> String {
    if text == "~" {
        return home_dir().to_string_lossy().into_owned();
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return format!("{}/{}", home_dir().display(), rest);
    }
    if let Some(rest) = text.strip_prefix("~\\") {
        return format!("{}\\{}", home_dir().display(), rest);
    }
    text.to_string()
}

/// Parent directory and partial final segment for tab completion.
pub fn path_completion_context(input: &str) -> (PathBuf, String) {
    let expanded = expand_tilde(input);
    let path = PathBuf::from(&expanded);

    if input.ends_with('/') || input.ends_with('\\') {
        return (path, String::new());
    }

    if let Some(prefix) = path.file_name().and_then(|name| name.to_str()).filter(|s| !s.is_empty())
    {
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
        if let Some(parent) = parent {
            return (parent.to_path_buf(), prefix.to_string());
        }
        if expanded.starts_with('/') {
            return (PathBuf::from("/"), prefix.to_string());
        }
        if expanded.contains(':') && cfg!(target_os = "windows") {
            return (path, String::new());
        }
    }

    if path.as_os_str().is_empty() {
        (PathBuf::new(), expanded)
    } else {
        (path, String::new())
    }
}

pub fn list_directory_completions(parent: &Path, prefix: &str) -> Vec<PathBuf> {
    if !parent.is_dir() {
        return Vec::new();
    }

    let Ok(read_dir) = std::fs::read_dir(parent) else {
        return Vec::new();
    };

    let prefix_lower = prefix.to_ascii_lowercase();
    let mut matches: Vec<PathBuf> = read_dir
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                if prefix.is_empty() {
                    true
                } else {
                    name.to_ascii_lowercase().starts_with(&prefix_lower)
                }
            })
        })
        .map(|entry| entry.path())
        .collect();

    matches.sort_by_key(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
    });
    matches
}

pub fn apply_path_completion(input: &mut String, completion: &Path) {
    let (_, prefix) = path_completion_context(input);
    let name = completion
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    if prefix.is_empty() && (input.ends_with('/') || input.ends_with('\\')) {
        input.push_str(name);
    } else if !prefix.is_empty() {
        let new_len = input.len().saturating_sub(prefix.len());
        input.truncate(new_len);
        input.push_str(name);
    } else {
        *input = completion.display().to_string();
        return;
    }

    let sep = if input.contains('\\') { '\\' } else { '/' };
    if !input.ends_with(sep) {
        input.push(sep);
    }
}

pub fn resolve_directory_path(text: &str, cwd: &Path) -> Option<PathBuf> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let expanded = expand_tilde(trimmed);
    let path = PathBuf::from(&expanded);
    let resolved = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };

    resolved.is_dir().then_some(resolved)
}
