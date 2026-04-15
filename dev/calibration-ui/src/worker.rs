use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;

use eframe::egui;
use egui::Color32;
use remodian_vision::detect::{calc::get_digit, template::load_templates};
use remodian_vision::preprocess::{RgbImageSource, preprocess_visual};

pub struct WorkRequest {
    pub rgb: image::RgbImage,
    pub points: [(u32, u32); 4],
}

pub struct WorkResult {
    pub pixels: Vec<Color32>, // 128×128 thresholded
    pub digit: Option<Option<u8>>,
}

pub fn spawn_worker(
    ctx: egui::Context,
) -> (
    Arc<(Mutex<Option<WorkRequest>>, Condvar)>,
    mpsc::Receiver<WorkResult>,
) {
    let slot: Arc<(Mutex<Option<WorkRequest>>, Condvar)> =
        Arc::new((Mutex::new(None), Condvar::new()));
    let slot_worker = Arc::clone(&slot);
    let (tx, rx) = mpsc::channel::<WorkResult>();

    thread::spawn(move || {
        let templates = load_templates();
        loop {
            let req: WorkRequest = {
                let (lock, cvar) = &*slot_worker;
                let mut guard = lock.lock().unwrap();
                while guard.is_none() {
                    guard = cvar.wait(guard).unwrap();
                }
                guard.take().unwrap()
            };

            let source = RgbImageSource(req.rgb);
            let result = preprocess_visual(&source, req.points);
            let cells = result.sample.extract_cells();
            let digit = get_digit(&templates, &cells, &result.sample);

            let pixels: Vec<Color32> = result
                .thresholded
                .iter()
                .map(|&v| {
                    let c = (v * 255.0) as u8;
                    Color32::from_rgb(c, c, c)
                })
                .collect();

            if tx.send(WorkResult { pixels, digit: Some(digit) }).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });

    (slot, rx)
}
