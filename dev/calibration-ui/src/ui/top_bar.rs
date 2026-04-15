use eframe::egui;
use egui::Color32;

use crate::app::{CropRenderApp, SourceMode};
use crate::camera::{capture_camera_frame, list_cameras};

impl CropRenderApp {
    pub fn draw_top_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // ── Poll HTTP channels before drawing ────────────────────────────────
        if let Some(result) = self.http.poll() {
            match result {
                Ok(rgb) => self.apply_rgb(ctx, rgb, false),
                Err(e) => self.http.status = format!("Error: {}", e),
            }
        }

        // ── Row 1: source mode selector ──────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Source:");
            ui.selectable_value(&mut self.source_mode, SourceMode::File, "File");
            ui.selectable_value(&mut self.source_mode, SourceMode::Cam, "Cam");
            ui.selectable_value(&mut self.source_mode, SourceMode::Http, "HTTP");
        });

        // ── Row 2: mode-specific controls ────────────────────────────────────
        ui.horizontal(|ui| {
            match self.source_mode {
                SourceMode::File => {
                    ui.label("Path:");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.image_path).desired_width(500.0),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.load_image(ctx);
                    }
                    if ui.button("Load").clicked() {
                        self.load_image(ctx);
                    }
                }

                SourceMode::Cam => {
                    if ui.button("Take a photo").clicked() {
                        let rgb = capture_camera_frame(self.camera_index).unwrap_or_else(|e| {
                            eprintln!("Camera capture failed: {}", e);
                            image::RgbImage::new(640, 480)
                        });
                        self.apply_rgb(ctx, rgb, false);
                    }
                    let selected_label = self
                        .available_cameras
                        .iter()
                        .find(|(i, _)| *i == self.camera_index)
                        .map(|(_, n)| n.as_str())
                        .unwrap_or("—");
                    egui::ComboBox::from_id_salt("cam_select")
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            for (idx, name) in &self.available_cameras {
                                ui.selectable_value(&mut self.camera_index, *idx, name);
                            }
                        });
                    if ui.small_button("↺").on_hover_text("Refresh camera list").clicked() {
                        self.available_cameras = list_cameras();
                        if !self.available_cameras.iter().any(|(i, _)| *i == self.camera_index) {
                            self.camera_index =
                                self.available_cameras.first().map(|(i, _)| *i).unwrap_or(0);
                        }
                    }
                }

                SourceMode::Http => {
                    ui.label("URL:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.http.base_url).desired_width(300.0),
                    );
                    if ui.small_button("↺ Cameras").on_hover_text("Fetch camera list").clicked() {
                        self.http.fetch_cameras();
                    }
                    let selected_label = self
                        .http
                        .cameras
                        .iter()
                        .find(|(i, _)| *i == self.http.camera_index)
                        .map(|(_, n)| n.as_str())
                        .unwrap_or("—");
                    egui::ComboBox::from_id_salt("http_cam_select")
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            for (idx, name) in &self.http.cameras {
                                ui.selectable_value(&mut self.http.camera_index, *idx, name);
                            }
                        });
                    if ui.button("Capture").clicked() {
                        self.http.fetch_capture();
                    }
                    if !self.http.status.is_empty() {
                        ui.label(
                            egui::RichText::new(&self.http.status).color(Color32::from_rgb(200, 200, 80)),
                        );
                    }
                }
            }
        });
    }
}
