use crate::{explorer::ExplorerState, fonts, ui};
use eframe::egui::{self, Rgba, vec2};
use std::path::PathBuf;

pub struct DaggerExplorerApp {
    blur_requested: bool,
    explorer: ExplorerState,
}

impl DaggerExplorerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_directory: Option<PathBuf>) -> Self {
        fonts::setup(&cc.egui_ctx);
        let explorer = initial_directory
            .map(ExplorerState::with_initial_path)
            .unwrap_or_else(ExplorerState::new);
        ui::theme::apply_with_preset(&cc.egui_ctx, explorer.theme_preset());
        Self {
            blur_requested: false,
            explorer,
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
        ui::theme::apply_with_preset(ctx, self.explorer.theme_preset());
        self.explorer.handle_keyboard_shortcuts(ctx);
        self.explorer.poll_fs(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut toggle_settings = false;
        let settings_open = self.explorer.settings_dialog_open;
        ui::show_with_title_center(ui, "Dagger Explorer", |ui| {
            let label = if settings_open {
                "⌘ Settings ▴"
            } else {
                "⌘ Settings ▾"
            };
            let button = egui::Button::new(
                egui::RichText::new(label)
                    .size(12.0)
                    .color(ui::theme::text_primary()),
            )
            .min_size(vec2(192.0, 26.0))
            .fill(ui::theme::glass_fill())
            .stroke(egui::Stroke::new(1.0, ui::theme::glass_stroke()))
            .corner_radius(6.0);
            if ui.add(button).clicked() {
                toggle_settings = true;
            }
        }, |ui| {
            ui::show_explorer(ui, &mut self.explorer);
        });
        if toggle_settings {
            self.explorer.toggle_settings_dialog();
        }
        ui::show_settings_dropdown(ui.ctx(), &mut self.explorer);
    }
}
