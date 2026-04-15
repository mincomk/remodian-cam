#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

pub mod detect;
pub mod preprocess;

use detect::calc::{combine_digits, get_digit};
use detect::template::load_templates;
use preprocess::{GrayscalePixelSource, deserialize_crops, preprocess};

/// Run the full detection pipeline on every crop region described by `crops_str`.
///
/// `crops_str` uses '+' as the separator between regions (same format as `serialize_crops`).
/// Each region is encoded as `x1,y1,x2,y2,x3,y3,x4,y4` (TL,TR,BR,BL).
///
/// Takes raw grayscale pixel bytes (one byte per pixel) with the given dimensions.
///
/// Returns `Ok(None)` when any region reads as empty display.
/// Returns `Err` when the crop string is malformed.
pub fn detect_number_raw(
    pixels: &[u8],
    width: u32,
    height: u32,
    crops_str: &str,
) -> Result<Option<u32>, String> {
    let crops = deserialize_crops(crops_str)?;
    let templates = load_templates();
    let source = GrayscalePixelSource { data: pixels, width, height };
    let digits: Vec<Option<u8>> = crops
        .iter()
        .map(|&pts| {
            let sample = preprocess(&source, pts);
            let cells = sample.extract_cells();
            get_digit(&templates, &cells, &sample)
        })
        .collect();
    Ok(combine_digits(&digits))
}

/// Run the full detection pipeline on every crop region described by `crops_str`.
///
/// `crops_str` uses '+' as the separator between regions (same format as `serialize_crops`).
/// Each region is encoded as `x1,y1,x2,y2,x3,y3,x4,y4` (TL,TR,BR,BL).
///
/// Returns `Ok(None)` when any region reads as empty display.
/// Returns `Err` when the crop string is malformed.
#[cfg(feature = "std")]
pub fn detect_number(image: &image::RgbImage, crops_str: &str) -> Result<Option<u32>, String> {
    use preprocess::RgbImageSource;
    let crops = deserialize_crops(crops_str)?;
    let templates = load_templates();
    let source = RgbImageSource(image.clone());
    let digits: Vec<Option<u8>> = crops
        .iter()
        .map(|&pts| {
            let sample = preprocess(&source, pts);
            let cells = sample.extract_cells();
            get_digit(&templates, &cells, &sample)
        })
        .collect();
    Ok(combine_digits(&digits))
}
