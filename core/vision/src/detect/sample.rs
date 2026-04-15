#[cfg(feature = "std")]
use std::collections::VecDeque;
#[cfg(not(feature = "std"))]
use alloc::collections::VecDeque;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy)]
pub struct DigitSample {
    pub data: [f32; 16384], // 128x128 image, normalized 0.0..=1.0
}

impl DigitSample {
    fn pull_cell(&self, cell_x: u8, cell_y: u8, cols: usize, rows: usize) -> f32 {
        let start_y = cell_y as usize * 128 / rows;
        let end_y = ((cell_y as usize + 1) * 128 / rows).min(128);
        let start_x = cell_x as usize * 128 / cols;
        let end_x = ((cell_x as usize + 1) * 128 / cols).min(128);

        let mut sum = 0.0f32;
        let mut count = 0usize;

        for pixel_y in start_y..end_y {
            for pixel_x in start_x..end_x {
                sum += self.data[pixel_y * 128 + pixel_x];
                count += 1;
            }
        }

        if count > 0 { sum / count as f32 } else { 0.0 }
    }

    pub fn extract_cells(&self) -> [f32; 140] {
        let mut cells = [0.0f32; 140];
        for cy in 0..14usize {
            for cx in 0..10usize {
                cells[cy * 10 + cx] = self.pull_cell(cx as u8, cy as u8, 10, 14);
            }
        }
        cells
    }

    /// Count enclosed background loops using BFS flood-fill from image border.
    /// Digit 8 has 2 loops, 0/6/9 have 1, all others have 0.
    pub fn count_enclosed_loops(&self) -> u8 {
        const W: usize = 128;
        const H: usize = 128;

        let is_bg: Vec<bool> = self.data.iter().map(|&v| v < 0.5).collect();
        let mut exterior = vec![false; W * H];
        let mut queue = VecDeque::new();

        // Seed exterior flood-fill from all 4 borders
        for x in 0..W {
            let idx_top = x;
            let idx_bot = (H - 1) * W + x;
            if is_bg[idx_top] && !exterior[idx_top] {
                exterior[idx_top] = true;
                queue.push_back(idx_top);
            }
            if is_bg[idx_bot] && !exterior[idx_bot] {
                exterior[idx_bot] = true;
                queue.push_back(idx_bot);
            }
        }
        for y in 0..H {
            let idx_left = y * W;
            let idx_right = y * W + (W - 1);
            if is_bg[idx_left] && !exterior[idx_left] {
                exterior[idx_left] = true;
                queue.push_back(idx_left);
            }
            if is_bg[idx_right] && !exterior[idx_right] {
                exterior[idx_right] = true;
                queue.push_back(idx_right);
            }
        }

        while let Some(idx) = queue.pop_front() {
            let (x, y) = (idx % W, idx / W);
            let neighbors = [
                if x > 0 { Some(idx - 1) } else { None },
                if x + 1 < W { Some(idx + 1) } else { None },
                if y > 0 { Some(idx - W) } else { None },
                if y + 1 < H { Some(idx + W) } else { None },
            ];
            for nb in neighbors.into_iter().flatten() {
                if is_bg[nb] && !exterior[nb] {
                    exterior[nb] = true;
                    queue.push_back(nb);
                }
            }
        }

        // Count enclosed (unreachable from border) background components
        let mut visited = exterior;
        let mut loops = 0u8;
        for start in 0..W * H {
            if is_bg[start] && !visited[start] {
                loops += 1;
                visited[start] = true;
                let mut q2 = VecDeque::new();
                q2.push_back(start);
                while let Some(idx) = q2.pop_front() {
                    let (x, y) = (idx % W, idx / W);
                    let neighbors = [
                        if x > 0 { Some(idx - 1) } else { None },
                        if x + 1 < W { Some(idx + 1) } else { None },
                        if y > 0 { Some(idx - W) } else { None },
                        if y + 1 < H { Some(idx + W) } else { None },
                    ];
                    for nb in neighbors.into_iter().flatten() {
                        if is_bg[nb] && !visited[nb] {
                            visited[nb] = true;
                            q2.push_back(nb);
                        }
                    }
                }
            }
        }
        loops
    }
}
