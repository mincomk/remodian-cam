use std::sync::{Arc, Condvar, Mutex, mpsc};

use eframe::egui;
use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Vec2};

use crate::worker::{WorkRequest, WorkResult, spawn_worker};

pub const REGION_STYLES: &[(Color32, Color32)] = &[
    (Color32::YELLOW, Color32::RED),
    (Color32::from_rgb(0, 200, 255), Color32::BLUE),
];

/// Clamp an image-space float point into integer pixel coordinates.
pub fn clamp_pt(x: f32, y: f32, w: u32, h: u32) -> (u32, u32) {
    (
        x.clamp(0.0, w as f32 - 1.0) as u32,
        y.clamp(0.0, h as f32 - 1.0) as u32,
    )
}

pub struct PointSelectionSession {
    pub region_index: usize,
    pub collected: Vec<(f32, f32)>, // image-space points, up to 4
}

pub struct CropRegion {
    pub index: usize,
    pub control_points: [(f32, f32); 4],
    request_slot: Arc<(Mutex<Option<WorkRequest>>, Condvar)>,
    result_rx: mpsc::Receiver<WorkResult>,
    pub projected_texture: Option<TextureHandle>,
    pub digit: Option<Option<u8>>,
}

impl CropRegion {
    pub fn new(index: usize, ctx: egui::Context, initial_points: [(f32, f32); 4]) -> Self {
        let (request_slot, result_rx) = spawn_worker(ctx);
        Self {
            index,
            control_points: initial_points,
            request_slot,
            result_rx,
            projected_texture: None,
            digit: None,
        }
    }

    pub fn enqueue(&self, rgb: &image::RgbImage, img_size: (u32, u32)) {
        let (w, h) = img_size;
        let pts = self.control_points.map(|(x, y)| clamp_pt(x, y, w, h));
        let (lock, cvar) = &*self.request_slot;
        *lock.lock().unwrap() = Some(WorkRequest { rgb: rgb.clone(), points: pts });
        cvar.notify_one();
    }

    pub fn poll_result(&mut self, ctx: &egui::Context) {
        let mut latest: Option<WorkResult> = None;
        while let Ok(r) = self.result_rx.try_recv() {
            latest = Some(r);
        }
        if let Some(r) = latest {
            let color_img = ColorImage {
                size: [128, 128],
                source_size: Vec2::splat(128.0),
                pixels: r.pixels,
            };
            let name = format!("projected_{}", self.index);
            self.projected_texture =
                Some(ctx.load_texture(name, color_img, TextureOptions::NEAREST));
            self.digit = r.digit;
        }
    }

    pub fn points_u32(&self, img_size: (u32, u32)) -> [(u32, u32); 4] {
        let (w, h) = img_size;
        if w == 0 || h == 0 {
            return self.control_points.map(|(x, y)| (x as u32, y as u32));
        }
        self.control_points.map(|(x, y)| clamp_pt(x, y, w, h))
    }
}
