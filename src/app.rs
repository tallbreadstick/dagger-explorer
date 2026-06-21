use crate::{explorer::ExplorerState, fonts, ui};
use eframe::egui::{self, Rgba};

pub struct DaggerExplorerApp {
    blur_requested: bool,
    explorer: ExplorerState,
}

impl DaggerExplorerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        fonts::setup(&cc.egui_ctx);
        ui::theme::apply(&cc.egui_ctx);
        Self {
            blur_requested: false,
            explorer: ExplorerState::new(),
        }
    }

    fn request_native_blur(&mut self, frame: &eframe::Frame) {
        if self.blur_requested {
            return;
        }
        self.blur_requested = true;

        if let Some(window) = frame.winit_window() {
            window.set_blur(true);
        }
    }
}

impl eframe::App for DaggerExplorerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Rgba::TRANSPARENT.to_array()
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.request_native_blur(frame);
        self.explorer.poll_fs(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::show_frame(ui, "Dagger Explorer", |ui| {
            ui::show_explorer(ui, &mut self.explorer);
        });
    }
}
