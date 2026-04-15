use eframe::egui;
use egui::Color32;
use remodian_vision::detect::combine_digits;

use crate::app::CropRenderApp;

impl CropRenderApp {
    pub fn draw_bottom_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let digits: Vec<Option<u8>> =
            self.regions.iter().map(|r| r.digit.flatten()).collect();
        let combined = combine_digits(&digits);

        ui.horizontal(|ui| {
            ui.label("Number:");
            match combined {
                None => {
                    ui.label(egui::RichText::new("—").color(Color32::GRAY));
                }
                Some(n) => {
                    ui.label(
                        egui::RichText::new(n.to_string())
                            .size(28.0)
                            .strong()
                            .color(Color32::from_rgb(100, 220, 100)),
                    );
                }
            }
            ui.separator();
            ui.label("Crops:");
            let serialized = self.serialized_crops();
            ui.monospace(&serialized);
            if ui.small_button("Copy").clicked() {
                ctx.copy_text(serialized);
            }
        });
    }
}
