use eframe::egui;
use egui::{Pos2, Rect, Vec2};

pub struct ViewState {
    pub scale: f32,  // screen pixels per image pixel
    pub offset: Vec2, // top-left of the visible area in image coordinates
    pub reset: bool,  // fit-to-panel on next frame
}

impl Default for ViewState {
    fn default() -> Self {
        Self { scale: 1.0, offset: Vec2::ZERO, reset: false }
    }
}

impl ViewState {
    /// Fit the image into `panel`, resetting the offset to the origin.
    pub fn fit_to(&mut self, panel: Rect, img_w: u32, img_h: u32) {
        self.scale = (panel.width() / img_w as f32).min(panel.height() / img_h as f32);
        self.offset = Vec2::ZERO;
    }

    /// Apply scroll-wheel zoom centred on `cursor`.
    pub fn apply_scroll_zoom(&mut self, panel: Rect, cursor: Option<Pos2>, delta: f32) {
        let factor = 1.15f32.powf(delta);
        if let Some(cursor) = cursor {
            let cx = self.offset.x + (cursor.x - panel.left()) / self.scale;
            let cy = self.offset.y + (cursor.y - panel.top()) / self.scale;
            self.scale = (self.scale * factor).clamp(0.05, 100.0);
            self.offset.x = cx - (cursor.x - panel.left()) / self.scale;
            self.offset.y = cy - (cursor.y - panel.top()) / self.scale;
        }
    }

    /// Translate the view by a screen-space drag delta.
    pub fn apply_pan(&mut self, delta: Vec2) {
        self.offset.x -= delta.x / self.scale;
        self.offset.y -= delta.y / self.scale;
    }

    /// Prevent scrolling completely off the image.
    pub fn clamp_pan(&mut self, panel: Rect, img_w: u32, img_h: u32) {
        let vis_w = panel.width() / self.scale;
        let vis_h = panel.height() / self.scale;
        self.offset.x = self.offset.x.clamp(-(vis_w * 0.5), img_w as f32 - vis_w * 0.5);
        self.offset.y = self.offset.y.clamp(-(vis_h * 0.5), img_h as f32 - vis_h * 0.5);
    }

    /// Convert an image-space point to a screen position inside `panel`.
    pub fn to_screen(&self, panel: Rect, pt: (f32, f32)) -> Pos2 {
        Pos2::new(
            panel.left() + (pt.0 - self.offset.x) * self.scale,
            panel.top() + (pt.1 - self.offset.y) * self.scale,
        )
    }

    /// Convert a screen position to image-space coordinates.
    pub fn to_image(&self, panel: Rect, p: Pos2) -> (f32, f32) {
        (
            self.offset.x + (p.x - panel.left()) / self.scale,
            self.offset.y + (p.y - panel.top()) / self.scale,
        )
    }

    /// Returns `(uv_rect, screen_img_rect)` — the UV window and the clamped
    /// screen sub-rectangle that corresponds to the visible image pixels.
    pub fn view_rects(&self, panel: Rect, img_w: u32, img_h: u32) -> (Rect, Rect) {
        let vis_w = panel.width() / self.scale;
        let vis_h = panel.height() / self.scale;

        let uv_x0 = (self.offset.x / img_w as f32).clamp(0.0, 1.0);
        let uv_y0 = (self.offset.y / img_h as f32).clamp(0.0, 1.0);
        let uv_x1 = ((self.offset.x + vis_w) / img_w as f32).clamp(0.0, 1.0);
        let uv_y1 = ((self.offset.y + vis_h) / img_h as f32).clamp(0.0, 1.0);

        let sx0 = panel.left() + (uv_x0 * img_w as f32 - self.offset.x) * self.scale;
        let sy0 = panel.top() + (uv_y0 * img_h as f32 - self.offset.y) * self.scale;
        let sx1 = panel.left() + (uv_x1 * img_w as f32 - self.offset.x) * self.scale;
        let sy1 = panel.top() + (uv_y1 * img_h as f32 - self.offset.y) * self.scale;

        let uv = Rect::from_min_max(Pos2::new(uv_x0, uv_y0), Pos2::new(uv_x1, uv_y1));
        let screen = Rect::from_min_max(
            Pos2::new(
                sx0.clamp(panel.left(), panel.right()),
                sy0.clamp(panel.top(), panel.bottom()),
            ),
            Pos2::new(
                sx1.clamp(panel.left(), panel.right()),
                sy1.clamp(panel.top(), panel.bottom()),
            ),
        );
        (uv, screen)
    }
}
