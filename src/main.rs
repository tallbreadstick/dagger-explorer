mod fonts;
mod explorer;
mod app;
mod ui;

use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result {
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
        Box::new(|cc| Ok(Box::new(app::DaggerExplorerApp::new(cc)))),
    )
}
