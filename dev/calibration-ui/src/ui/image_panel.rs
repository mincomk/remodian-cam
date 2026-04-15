use eframe::egui;
use egui::{Color32, Pos2, Vec2};

use crate::app::CropRenderApp;
use crate::region::REGION_STYLES;

impl CropRenderApp {
    pub fn draw_image_panel(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        col_w: f32,
        main_height: f32,
    ) {
        let (resp, painter) = ui.allocate_painter(
            Vec2::new(col_w, main_height),
            egui::Sense::click_and_drag(),
        );
        let rect = resp.rect;

        let Some(tex) = self.original_texture.clone() else {
            painter.rect_filled(rect, 4.0, Color32::from_gray(40));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Load an image to begin",
                egui::FontId::proportional(16.0),
                Color32::GRAY,
            );
            return;
        };

        let (iw, ih) = self.original_size;

        // ── Fit-to-panel on first frame after load ───────────────────────────
        if self.view.reset {
            self.view.reset = false;
            self.view.fit_to(rect, iw, ih);
        }

        // ── Scroll-wheel zoom ────────────────────────────────────────────────
        let scroll_delta: f32 = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                    _ => None,
                })
                .sum()
        });
        if rect.contains(ctx.pointer_hover_pos().unwrap_or(Pos2::ZERO)) && scroll_delta != 0.0 {
            self.view.apply_scroll_zoom(rect, ctx.pointer_hover_pos(), scroll_delta);
        }

        // ── Middle / right-drag pans ─────────────────────────────────────────
        if resp.dragged_by(egui::PointerButton::Middle)
            || resp.dragged_by(egui::PointerButton::Secondary)
        {
            self.view.apply_pan(resp.drag_delta());
        }
        self.view.clamp_pan(rect, iw, ih);

        // ── Draw image ───────────────────────────────────────────────────────
        let (uv, img_rect) = self.view.view_rects(rect, iw, ih);
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        if img_rect.is_positive() {
            painter.image(tex.id(), img_rect, uv, Color32::WHITE);
        }

        // ── Draw region outlines + handles ───────────────────────────────────
        for (ri, region) in self.regions.iter().enumerate() {
            let selecting_this =
                self.point_selection.as_ref().map_or(false, |s| s.region_index == ri);
            if selecting_this {
                continue;
            }
            let (outline_col, handle_col) =
                REGION_STYLES.get(ri).copied().unwrap_or((Color32::WHITE, Color32::GRAY));
            let order = [0usize, 1, 2, 3, 0];
            for i in 0..4 {
                painter.line_segment(
                    [
                        self.view.to_screen(rect, region.control_points[order[i]]),
                        self.view.to_screen(rect, region.control_points[order[i + 1]]),
                    ],
                    egui::Stroke::new(1.5, outline_col),
                );
            }
            for (pi, &pt) in region.control_points.iter().enumerate() {
                let sp = self.view.to_screen(rect, pt);
                let active = self.dragging == Some((ri, pi));
                painter.circle_filled(sp, 8.0, if active { Color32::WHITE } else { handle_col });
                painter.circle_stroke(sp, 8.0, egui::Stroke::new(1.5, outline_col));
            }
        }

        // ── In-progress point-selection overlay ──────────────────────────────
        if let Some(session) = &self.point_selection {
            let col = Color32::from_rgb(255, 165, 0);
            for (i, &pt) in session.collected.iter().enumerate() {
                let sp = self.view.to_screen(rect, pt);
                if i > 0 {
                    painter.line_segment(
                        [self.view.to_screen(rect, session.collected[i - 1]), sp],
                        egui::Stroke::new(1.5, col),
                    );
                }
                painter.circle_filled(sp, 7.0, col);
                painter.circle_stroke(sp, 7.0, egui::Stroke::new(1.5, Color32::WHITE));
            }
            let remaining = 4 - session.collected.len();
            painter.text(
                rect.left_bottom() + Vec2::new(6.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                format!("Click {} more point(s) clockwise", remaining),
                egui::FontId::proportional(13.0),
                col,
            );
        }

        // ── Input: point-selection clicks or handle drag ──────────────────────
        let mut completed: Option<(usize, [(f32, f32); 4])> = None;
        if self.point_selection.is_some() {
            if resp.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = ctx.pointer_interact_pos() {
                    if rect.contains(pos) {
                        let (ix, iy) = self.view.to_image(rect, pos);
                        let pt = (ix.clamp(0.0, iw as f32 - 1.0), iy.clamp(0.0, ih as f32 - 1.0));
                        let session = self.point_selection.as_mut().unwrap();
                        session.collected.push(pt);
                        if session.collected.len() == 4 {
                            completed = Some((
                                session.region_index,
                                [
                                    session.collected[0],
                                    session.collected[1],
                                    session.collected[2],
                                    session.collected[3],
                                ],
                            ));
                        }
                    }
                }
            }
        } else {
            const HIT_RADIUS: f32 = 24.0;
            if resp.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pos) = ctx.pointer_interact_pos() {
                    'outer: for (ri, region) in self.regions.iter().enumerate() {
                        for (pi, &pt) in region.control_points.iter().enumerate() {
                            if self.view.to_screen(rect, pt).distance(pos) < HIT_RADIUS {
                                self.dragging = Some((ri, pi));
                                break 'outer;
                            }
                        }
                    }
                }
            }
            if resp.drag_stopped() {
                self.dragging = None;
            }
            if let Some((ri, pi)) = self.dragging {
                if let Some(cursor) = ctx.pointer_interact_pos() {
                    let (ix, iy) = self.view.to_image(rect, cursor);
                    let pt = &mut self.regions[ri].control_points[pi];
                    pt.0 = ix.clamp(0.0, iw as f32 - 1.0);
                    pt.1 = iy.clamp(0.0, ih as f32 - 1.0);
                    if let Some(rgb) = &self.original_rgb.clone() {
                        self.regions[ri].enqueue(rgb, self.original_size);
                    }
                }
            }
        }

        if let Some((ri, pts)) = completed {
            self.regions[ri].control_points = pts;
            self.point_selection = None;
            if let Some(rgb) = &self.original_rgb.clone() {
                self.regions[ri].enqueue(rgb, self.original_size);
            }
        }

        // ── Zoom hint ────────────────────────────────────────────────────────
        painter.text(
            rect.right_bottom() - Vec2::new(4.0, 4.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("{:.0}%", self.view.scale * 100.0),
            egui::FontId::monospace(11.0),
            Color32::from_white_alpha(160),
        );
    }
}
