mod homography;
mod pipeline;

#[cfg(not(feature = "std"))]
use alloc::{string::String, string::ToString, vec::Vec};

use crate::detect::sample::DigitSample;
use pipeline::{crop_to_bbox, extract_green, perspective_warp, threshold, to_digit_sample};

/// Abstract trait for image sources. Allows any input (file, camera, etc.)
/// to provide pixel data as normalized f32 RGB values [0.0, 1.0].
pub trait ImageSource {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn pixel_rgb(&self, x: u32, y: u32) -> (f32, f32, f32);
}

/// File-based image source using the `image` crate.
#[cfg(feature = "std")]
pub struct FileImageSource {
    img: image::RgbImage,
}

#[cfg(feature = "std")]
impl FileImageSource {
    /// Load a PNG or JPG image from a file path.
    pub fn new(path: &str) -> Result<Self, image::ImageError> {
        let img = image::open(path)?.to_rgb8();
        Ok(FileImageSource { img })
    }
}

#[cfg(feature = "std")]
impl ImageSource for FileImageSource {
    fn width(&self) -> u32 {
        self.img.width()
    }

    fn height(&self) -> u32 {
        self.img.height()
    }

    fn pixel_rgb(&self, x: u32, y: u32) -> (f32, f32, f32) {
        if x >= self.img.width() || y >= self.img.height() {
            return (0.0, 0.0, 0.0);
        }
        let pixel = self.img.get_pixel(x, y);
        (
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        )
    }
}

/// In-memory image source backed by an `image::RgbImage`.
#[cfg(feature = "std")]
pub struct RgbImageSource(pub image::RgbImage);

#[cfg(feature = "std")]
impl ImageSource for RgbImageSource {
    fn width(&self) -> u32 {
        self.0.width()
    }
    fn height(&self) -> u32 {
        self.0.height()
    }
    fn pixel_rgb(&self, x: u32, y: u32) -> (f32, f32, f32) {
        if x >= self.0.width() || y >= self.0.height() {
            return (0.0, 0.0, 0.0);
        }
        let p = self.0.get_pixel(x, y);
        (
            p[0] as f32 / 255.0,
            p[1] as f32 / 255.0,
            p[2] as f32 / 255.0,
        )
    }
}

/// In-memory image source backed by raw grayscale pixel bytes (one byte per pixel).
/// Each byte maps to equal R, G, B values in [0.0, 1.0].
pub struct GrayscalePixelSource<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
}

impl ImageSource for GrayscalePixelSource<'_> {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn pixel_rgb(&self, x: u32, y: u32) -> (f32, f32, f32) {
        if x >= self.width || y >= self.height {
            return (0.0, 0.0, 0.0);
        }
        let v = self.data[(y * self.width + x) as usize] as f32 / 255.0;
        (v, v, v)
    }
}

/// Serialize multiple crop regions to a '+'-separated string.
/// Each region encodes one crop as `x1,y1,x2,y2,x3,y3,x4,y4` (TL,TR,BR,BL order).
pub fn serialize_crops(regions: &[[(u32, u32); 4]]) -> String {
    regions
        .iter()
        .map(|pts| {
            format!(
                "{},{},{},{},{},{},{},{}",
                pts[0].0, pts[0].1, pts[1].0, pts[1].1, pts[2].0, pts[2].1, pts[3].0, pts[3].1,
            )
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Deserialize crop regions from the format produced by `serialize_crops`.
/// Regions are separated by '+'. Returns an error string if any region is malformed.
pub fn deserialize_crops(s: &str) -> Result<Vec<[(u32, u32); 4]>, String> {
    s.split('+')
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let nums: Vec<u32> = line
                .split(',')
                .map(|n| n.trim().parse::<u32>().map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?;
            if nums.len() != 8 {
                return Err(format!("expected 8 values, got {}", nums.len()));
            }
            Ok([
                (nums[0], nums[1]),
                (nums[2], nums[3]),
                (nums[4], nums[5]),
                (nums[6], nums[7]),
            ])
        })
        .collect()
}

/// Result of the visual preprocessing pipeline, exposing intermediate images.
pub struct PreprocessResult {
    /// 128×128 warped green-channel image (pre-threshold), values in [0.0, 1.0]
    pub warped: Vec<f32>,
    /// 128×128 binary thresholded image (0.0 or 1.0)
    pub thresholded: Vec<f32>,
    pub sample: DigitSample,
}

/// Visual preprocessing pipeline that returns intermediate images.
/// Same logic as `preprocess` but also returns warped and thresholded buffers.
pub fn preprocess_visual(source: &dyn ImageSource, points: [(u32, u32); 4]) -> PreprocessResult {
    let green = extract_green(source);
    let bbox = pipeline::compute_bbox(&points);
    let (cropped, crop_w, crop_h, origin) =
        crop_to_bbox(&green, source.width(), source.height(), bbox);

    let mut cropped_points = points;
    for point in &mut cropped_points {
        point.0 -= origin.0;
        point.1 -= origin.1;
    }

    let warped = perspective_warp(&cropped, crop_w, crop_h, &cropped_points);
    let fixed_threshold = pipeline::compute_otsu_threshold(&warped);
    let thresholded = threshold(&warped, fixed_threshold);
    let sample = to_digit_sample(&thresholded);

    PreprocessResult {
        warped,
        thresholded,
        sample,
    }
}

/// Main preprocessing pipeline.
/// Takes an image source and 4 perspective corner points, returns a DigitSample.
///
/// # Arguments
/// * `source` - An implementation of ImageSource (file, camera, etc.)
/// * `points` - 4 corner points in the source image, typically the display corners
///   Conventionally: [(top-left), (top-right), (bottom-right), (bottom-left)]
///
/// # Returns
/// A DigitSample containing a normalized 128×128 f32 image ready for digit recognition.
pub fn preprocess(source: &dyn ImageSource, points: [(u32, u32); 4]) -> DigitSample {
    // Step 1: Extract green channel
    let green = extract_green(source);

    // Step 2: Crop to bounding box
    let bbox = pipeline::compute_bbox(&points);
    let (cropped, crop_w, crop_h, origin) =
        crop_to_bbox(&green, source.width(), source.height(), bbox);

    // Offset points relative to crop origin
    let mut cropped_points = points;
    for point in &mut cropped_points {
        point.0 -= origin.0;
        point.1 -= origin.1;
    }

    // Step 3: Perspective transform
    let warped = perspective_warp(&cropped, crop_w, crop_h, &cropped_points);

    // Step 4: Threshold to binary using Otsu's method
    let fixed_threshold = pipeline::compute_otsu_threshold(&warped);
    let thresholded = threshold(&warped, fixed_threshold);

    // Step 5: Convert to DigitSample
    to_digit_sample(&thresholded)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_load_digit_images() {
        // Test loading the 10 sample images from image-analysis
        let base_path = "../image-analysis/data/40_resized";

        for digit in 0..10 {
            let path = format!("{}/a{}.jpg", base_path, digit);

            // Try to load the image
            match FileImageSource::new(&path) {
                Ok(source) => {
                    let w = source.width();
                    let h = source.height();
                    println!("Loaded a{}.jpg: {}x{}", digit, w, h);

                    // Test preprocessing with corners as identity transform
                    let points = [(0, 0), (w - 1, 0), (w - 1, h - 1), (0, h - 1)];
                    let sample = preprocess(&source, points);

                    // Verify the result is a valid DigitSample
                    assert_eq!(sample.data.len(), 16384);

                    // Check that we have some non-zero data
                    let has_nonzero = sample.data.iter().any(|&v| v > 0.0);
                    assert!(
                        has_nonzero,
                        "Sample should have non-zero pixels for digit {}",
                        digit
                    );
                }
                Err(e) => {
                    eprintln!("Failed to load a{}.jpg: {}", digit, e);
                    eprintln!("Make sure image-analysis submodule is present at ../image-analysis");
                }
            }
        }
    }
}
