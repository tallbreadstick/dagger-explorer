use std::fs;
use std::path::{Path, PathBuf};

pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|error| format!("Failed to move {} to trash: {error}", path.display()))
}

pub fn delete_permanently(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else if path.is_file() {
        fs::remove_file(path)
    } else if path.exists() {
        fs::remove_file(path)
    } else {
        return Ok(());
    }
    .map_err(|error| format!("Failed to delete {}: {error}", path.display()))
}

pub fn move_paths_to_trash(paths: &[PathBuf]) -> Result<(), String> {
    let mut error = None;
    for path in paths {
        if let Err(message) = move_to_trash(path) {
            error = Some(message);
        }
    }
    error.map_or(Ok(()), Err)
}

pub fn delete_paths_permanently(paths: &[PathBuf]) -> Result<(), String> {
    let mut error = None;
    for path in paths {
        if let Err(message) = delete_permanently(path) {
            error = Some(message);
        }
    }
    error.map_or(Ok(()), Err)
}
