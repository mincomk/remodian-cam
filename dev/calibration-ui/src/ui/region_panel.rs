use eframe::egui;
use egui::{Color32, Pos2, Rect, Vec2};

use crate::app::CropRenderApp;
use crate::region::{PointSelectionSession, REGION_STYLES};

impl CropRenderApp {
    pub fn draw_region_panels(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.vertical(|ui| {
            let region_w = (ui.available_width() / self.regions.len() as f32).max(1.0);

            enum PsAction {
                Start(usize),
                Cancel,
            }
            let mut ps_action: Option<PsAction> = None;

            ui.horizontal(|ui| {
                for (ri, region) in self.regions.iter().enumerate() {
                    let (outline_col, _) = REGION_STYLES
                        .get(ri)
                        .copied()
                        .unwrap_or((Color32::WHITE, Color32::GRAY));

                    ui.vertical(|ui| {
                        ui.set_width(region_w);
                        ui.label(
                            egui::RichText::new(format!("Region {}", ri)).color(outline_col),
                        );

                        let side = (region_w - 8.0).clamp(1.0, 300.0);
                        let (proj_resp, proj_painter) =
                            ui.allocate_painter(Vec2::splat(side), egui::Sense::hover());

                        if let Some(tex) = &region.projected_texture {
                            proj_painter.image(
                                tex.id(),
                                proj_resp.rect,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            proj_painter.rect_filled(proj_resp.rect, 4.0, Color32::from_gray(40));
                        }

                        // 5×7 gridlines overlay
                        let r = proj_resp.rect;
                        let grid_stroke =
                            egui::Stroke::new(0.5, Color32::from_white_alpha(80));
                        for col in 1..5 {
                            let x = r.left() + (col as f32 / 5.0) * r.width();
                            proj_painter.line_segment(
                                [Pos2::new(x, r.top()), Pos2::new(x, r.bottom())],
                                grid_stroke,
                            );
                        }
                        for row in 1..7 {
                            let y = r.top() + (row as f32 / 7.0) * r.height();
                            proj_painter.line_segment(
                                [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
                                grid_stroke,
                            );
                        }

                        let digit_str = match region.digit {
                            None => "—".to_string(),
                            Some(None) => "∅".to_string(),
                            Some(Some(d)) => d.to_string(),
                        };
                        ui.label(
                            egui::RichText::new(format!("Digit: {}", digit_str))
                                .size(18.0)
                                .color(outline_col),
                        );

                        let is_selecting = self
                            .point_selection
                            .as_ref()
                            .map_or(false, |s| s.region_index == ri);
                        if is_selecting {
                            if ui.button("Cancel").clicked() {
                                ps_action = Some(PsAction::Cancel);
                            }
                        } else {
                            let label = format!("Select Points for crop {}", ri + 1);
                            if ui.button(label).clicked() {
                                ps_action = Some(PsAction::Start(ri));
                            }
                        }
                    });
                }
            });

            match ps_action {
                Some(PsAction::Start(ri)) => {
                    self.point_selection = Some(PointSelectionSession {
                        region_index: ri,
                        collected: Vec::new(),
                    });
                }
                Some(PsAction::Cancel) => {
                    self.point_selection = None;
                }
                None => {}
            }
        });
    }
}
