mod fonts;
mod explorer;
mod app;
mod ui;

use eframe::egui::ViewportBuilder;
use std::path::{Path, PathBuf};

fn main() -> eframe::Result {
    let initial_directory = parse_initial_directory_arg();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_app_id("dev.dagger.explorer")
            .with_title("Dagger Explorer")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([640.0, 480.0])
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "Dagger Explorer",
        options,
        Box::new(move |cc| Ok(Box::new(app::DaggerExplorerApp::new(cc, initial_directory.clone())))),
    )
}

fn parse_initial_directory_arg() -> Option<PathBuf> {
    let mut args = std::env::args();
    let bin = args.next().unwrap_or_else(|| "dagger".to_string());

    let first = args.next();
    let extra = args.next();
    if first.is_some() && extra.is_some() {
        eprintln!("Usage: {bin} [path]");
        std::process::exit(2);
    }

    let Some(raw) = first else {
        return None;
    };

    let provided = PathBuf::from(raw);
    let resolved = resolve_to_absolute_path(&provided);
    if resolved.is_dir() {
        return Some(resolved);
    }
    if resolved.is_file() {
        return resolved.parent().map(Path::to_path_buf);
    }

    eprintln!(
        "[dagger-explorer] Provided path is not accessible, opening default location: {}",
        resolved.display()
    );
    None
}

fn resolve_to_absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}
