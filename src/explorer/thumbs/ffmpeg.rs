use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

const FFMPEG_NAME: &str = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };

fn platform_dir() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("windows", "aarch64") => "windows-aarch64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "macos-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        _ => "unknown",
    }
}

fn bundled_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let rel = PathBuf::from("assets")
        .join("ffmpeg")
        .join(platform_dir())
        .join(FFMPEG_NAME);

    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        paths.push(PathBuf::from(manifest).join(&rel));
    }

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(&rel));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(&rel));
            paths.push(dir.join("..").join(&rel));
        }
    }

    paths
}

fn which_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

fn is_runnable(path: &PathBuf) -> bool {
    Command::new(path)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn discover_ffmpeg() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = which_in_path(FFMPEG_NAME) {
        candidates.push(path);
    }

    candidates.extend(bundled_candidates());

    candidates.into_iter().find(is_runnable)
}

/// Path to a working ffmpeg binary (system PATH preferred, bundled fallback).
pub fn resolve_ffmpeg() -> Option<PathBuf> {
    static FFMPEG: OnceLock<Option<PathBuf>> = OnceLock::new();
    FFMPEG.get_or_init(discover_ffmpeg).clone()
}
