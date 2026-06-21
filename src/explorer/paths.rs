use std::path::{Path, PathBuf};

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
    if cfg!(target_os = "windows") {
        (b'A'..=b'Z')
            .filter_map(|letter| {
                let path = PathBuf::from(format!("{}:\\", letter as char));
                path.exists().then_some(path)
            })
            .collect()
    } else {
        let mut drives = vec![PathBuf::from("/")];

        for base in ["/media", "/mnt", "/run/media"] {
            let base_path = PathBuf::from(base);
            if let Ok(read_dir) = std::fs::read_dir(&base_path) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if base == "/run/media" {
                            if let Ok(user_dirs) = std::fs::read_dir(&path) {
                                for user_entry in user_dirs.flatten() {
                                    let mount = user_entry.path();
                                    if mount.is_dir() {
                                        drives.push(mount);
                                    }
                                }
                            }
                        } else {
                            drives.push(path);
                        }
                    }
                }
            }
        }

        drives.sort();
        drives.dedup();
        drives
    }
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
    if path == home_dir().as_path() {
        "Home".to_string()
    } else {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| path.display().to_string())
    }
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
