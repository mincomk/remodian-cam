mod bottom_bar;
mod image_panel;
mod region_panel;
mod top_bar;

use eframe::egui;

use crate::app::CropRenderApp;

impl eframe::App for CropRenderApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.needs_reload {
            self.needs_reload = false;
            self.load_image(&ctx);
        }
        for region in &mut self.regions {
            region.poll_result(&ctx);
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_top_bar(ui, &ctx);
            ui.separator();

            let available = ui.available_size();
            let bottom_height = 40.0;
            let main_height = (available.y - bottom_height - 8.0).max(1.0);

            ui.horizontal(|ui| {
                let col_w = (ui.available_width() / 2.0).max(1.0);
                self.draw_image_panel(ui, &ctx, col_w, main_height);
                ui.separator();
                self.draw_region_panels(ui, &ctx);
            });

            ui.separator();
            self.draw_bottom_bar(ui, &ctx);
        });
    }
}
