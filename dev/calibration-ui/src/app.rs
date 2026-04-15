use eframe::egui;
use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Vec2};
use remodian_vision::preprocess::{deserialize_crops, serialize_crops};

use crate::camera::list_cameras;
use crate::http_source::HttpSource;
use crate::region::{CropRegion, PointSelectionSession};
use crate::view::ViewState;

#[derive(PartialEq)]
pub enum SourceMode {
    File,
    Cam,
    Http,
}

pub struct CropRenderApp {
    pub image_path: String,
    pub original_texture: Option<TextureHandle>,
    pub original_rgb: Option<image::RgbImage>,
    pub original_size: (u32, u32),

    pub regions: Vec<CropRegion>,
    pub dragging: Option<(usize, usize)>, // (region_index, point_index)
    pub point_selection: Option<PointSelectionSession>,

    pub available_cameras: Vec<(u32, String)>,
    pub camera_index: u32,

    pub view: ViewState,
    pub needs_reload: bool,

    pub source_mode: SourceMode,
    pub http: HttpSource,
}

impl CropRenderApp {
    pub fn new(initial_path: String, ctx: egui::Context) -> Self {
        let available_cameras = list_cameras();
        let camera_index = available_cameras.first().map(|(i, _)| *i).unwrap_or(0);
        let mut app = Self {
            image_path: initial_path,
            original_texture: None,
            original_rgb: None,
            original_size: (0, 0),
            regions: Vec::new(),
            dragging: None,
            point_selection: None,
            view: ViewState::default(),
            needs_reload: false,
            available_cameras,
            camera_index,
            source_mode: SourceMode::File,
            http: HttpSource::default(),
        };
        for i in 0..2 {
            let off = i as f32 * 50.0;
            app.regions.push(CropRegion::new(
                i,
                ctx.clone(),
                [
                    (off, off),
                    (off + 100.0, off),
                    (off + 100.0, off + 100.0),
                    (off, off + 100.0),
                ],
            ));
        }
        if !app.image_path.is_empty() {
            app.needs_reload = true;
        }
        app
    }

    pub fn default_points_for(index: usize, w: u32, h: u32) -> [(f32, f32); 4] {
        let n = 2usize;
        let pad = 20.0;
        let strip_w = (w as f32 - pad * 2.0) / n as f32;
        let x0 = pad + index as f32 * strip_w;
        let x1 = x0 + strip_w - pad;
        let y0 = pad;
        let y1 = h as f32 - 1.0 - pad;
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    }

    pub fn apply_rgb(&mut self, ctx: &egui::Context, rgb: image::RgbImage, reset_crops: bool) {
        let (w, h) = (rgb.width(), rgb.height());
        let pixels: Vec<Color32> = rgb
            .pixels()
            .map(|p| Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        let color_img = ColorImage {
            size: [w as usize, h as usize],
            source_size: Vec2::new(w as f32, h as f32),
            pixels,
        };
        self.original_texture =
            Some(ctx.load_texture("original", color_img, TextureOptions::LINEAR));
        self.original_size = (w, h);
        self.original_rgb = Some(rgb);
        if reset_crops {
            for (i, region) in self.regions.iter_mut().enumerate() {
                region.control_points = Self::default_points_for(i, w, h);
            }
        }
        self.view.reset = true;
        self.enqueue_all();
    }

    pub fn load_image(&mut self, ctx: &egui::Context) {
        match image::open(&self.image_path.clone()) {
            Ok(dyn_img) => self.apply_rgb(ctx, dyn_img.to_rgb8(), true),
            Err(e) => eprintln!("Failed to load '{}': {}", self.image_path, e),
        }
    }

    pub fn enqueue_all(&self) {
        if let Some(rgb) = &self.original_rgb {
            for region in &self.regions {
                region.enqueue(rgb, self.original_size);
            }
        }
    }

    pub fn serialized_crops(&self) -> String {
        let pts: Vec<[(u32, u32); 4]> = self
            .regions
            .iter()
            .map(|r| r.points_u32(self.original_size))
            .collect();
        serialize_crops(&pts)
    }

    #[allow(dead_code)]
    pub fn apply_crops_from_text(&mut self, text: &str) {
        match deserialize_crops(text) {
            Ok(crops) => {
                for (region, pts) in self.regions.iter_mut().zip(crops.iter()) {
                    region.control_points = pts.map(|(x, y)| (x as f32, y as f32));
                }
                self.enqueue_all();
            }
            Err(e) => eprintln!("Failed to deserialize crops: {}", e),
        }
    }
}
