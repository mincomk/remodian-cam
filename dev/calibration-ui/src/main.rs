mod app;
mod camera;
mod http_source;
mod region;
mod ui;
mod view;
mod worker;

use eframe::egui;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let initial_path = args.get(1).cloned().unwrap_or_default();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Crop Render",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(app::CropRenderApp::new(
                initial_path.clone(),
                cc.egui_ctx.clone(),
            )))
        }),
    )
    .unwrap();
}
