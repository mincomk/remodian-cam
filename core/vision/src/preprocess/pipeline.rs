use super::ImageSource;
use super::homography::{compute_homography, transform_point};
use crate::detect::sample::DigitSample;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub fn extract_green(source: &dyn ImageSource) -> Vec<f32> {
    let w = source.width() as usize;
    let h = source.height() as usize;
    let mut green = vec![0.0f32; w * h];

    for y in 0..h as u32 {
        for x in 0..w as u32 {
            let (_, g, _) = source.pixel_rgb(x, y);
            let index = (y as usize) * w + (x as usize);
            green[index] = g;
        }
    }

    green
}

pub fn compute_bbox(points: &[(u32, u32); 4]) -> (u32, u32, u32, u32) {
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = u32::MIN;
    let mut max_y = u32::MIN;

    for (x, y) in points {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }

    (min_x, min_y, max_x, max_y)
}

pub fn crop_to_bbox(
    green: &[f32],
    width: u32,
    height: u32,
    bbox: (u32, u32, u32, u32),
) -> (Vec<f32>, u32, u32, (u32, u32)) {
    let (min_x, min_y, max_x, max_y) = bbox;
    let crop_w = max_x - min_x + 1;
    let crop_h = max_y - min_y + 1;

    let mut cropped = vec![0.0f32; (crop_w * crop_h) as usize];

    for y in 0..crop_h {
        for x in 0..crop_w {
            let src_x = min_x + x;
            let src_y = min_y + y;
            if src_x < width && src_y < height {
                let src_idx = (src_y as usize) * (width as usize) + (src_x as usize);
                let dst_idx = (y as usize) * (crop_w as usize) + (x as usize);
                cropped[dst_idx] = green[src_idx];
            }
        }
    }

    (cropped, crop_w, crop_h, (min_x, min_y))
}

pub fn perspective_warp(
    cropped: &[f32],
    crop_w: u32,
    crop_h: u32,
    src_points: &[(u32, u32); 4],
) -> Vec<f32> {
    const OUTPUT_SIZE: u32 = 128;

    // Destination corners: [0,0], [127,0], [127,127], [0,127]
    let dst_points = [
        (0.0f64, 0.0f64),
        (127.0f64, 0.0f64),
        (127.0f64, 127.0f64),
        (0.0f64, 127.0f64),
    ];

    // Convert source points to f64 for homography computation
    let src_f64 = [
        (src_points[0].0 as f64, src_points[0].1 as f64),
        (src_points[1].0 as f64, src_points[1].1 as f64),
        (src_points[2].0 as f64, src_points[2].1 as f64),
        (src_points[3].0 as f64, src_points[3].1 as f64),
    ];

    // Compute inverse homography: from destination to source
    let h_inv = compute_homography(dst_points, src_f64);

    let mut output = vec![0.0f32; (OUTPUT_SIZE * OUTPUT_SIZE) as usize];

    for y in 0..OUTPUT_SIZE {
        for x in 0..OUTPUT_SIZE {
            // Map destination pixel to source
            let (src_x, src_y) = transform_point(&h_inv, x as f64, y as f64);

            // Nearest-neighbor sampling (manual round to avoid libm dependency)
            let src_xi = if src_x >= 0.0 { (src_x + 0.5) as i32 } else { (src_x - 0.5) as i32 };
            let src_yi = if src_y >= 0.0 { (src_y + 0.5) as i32 } else { (src_y - 0.5) as i32 };

            if src_xi >= 0 && src_xi < (crop_w as i32) && src_yi >= 0 && src_yi < (crop_h as i32) {
                let src_idx = (src_yi as u32 * crop_w + src_xi as u32) as usize;
                if src_idx < cropped.len() {
                    let dst_idx = (y * OUTPUT_SIZE + x) as usize;
                    output[dst_idx] = cropped[src_idx];
                }
            }
        }
    }

    output
}

pub fn compute_otsu_threshold(buffer: &[f32]) -> f32 {
    // Convert to u8 range [0, 255] for histogram computation
    let histogram = compute_histogram(buffer);

    let total_pixels = buffer.len() as f32;
    let mut sum_all = 0.0f32;
    for (i, &count) in histogram.iter().enumerate() {
        sum_all += (i as f32) * count;
    }

    let mut sum_bg = 0.0f32;
    let mut count_bg = 0.0f32;
    let mut max_variance = 0.0f32;
    let mut best_threshold = 0u8;

    for t in 0..256 {
        count_bg += histogram[t];
        if count_bg == 0.0 {
            continue;
        }

        let count_fg = total_pixels - count_bg;
        if count_fg == 0.0 {
            break;
        }

        sum_bg += (t as f32) * histogram[t];
        let mean_bg = sum_bg / count_bg;
        let mean_fg = (sum_all - sum_bg) / count_fg;

        let diff = mean_bg - mean_fg;
        let variance = count_bg * count_fg * diff * diff;
        if variance > max_variance {
            max_variance = variance;
            best_threshold = t as u8;
        }
    }

    best_threshold as f32 / 255.0
}

fn compute_histogram(buffer: &[f32]) -> [f32; 256] {
    let mut hist = [0.0f32; 256];
    for &val in buffer {
        let bin = ((val.clamp(0.0, 1.0) * 255.0) as usize).min(255);
        hist[bin] += 1.0;
    }
    hist
}

pub fn threshold(buffer: &[f32], threshold: f32) -> Vec<f32> {
    buffer
        .iter()
        .map(|&v| if v >= threshold { 1.0 } else { 0.0 })
        .collect()
}

pub fn to_digit_sample(buffer: &[f32]) -> DigitSample {
    let mut data = [0.0f32; 16384];
    for (i, &val) in buffer.iter().take(16384).enumerate() {
        data[i] = val;
    }
    DigitSample { data }
}
