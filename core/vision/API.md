# LCD Pixel Digit Recognition API

This document describes the API for the lcd-pixel digit recognition system.

## Overview

The lcd-pixel system recognizes single digits (0-9) from LCD display images through a processing pipeline that:
1. Loads an image from disk or camera
2. Applies perspective correction to straighten the display
3. Extracts and analyzes the 5×7 segment grid
4. Matches against trained digit templates
5. Returns the detected digit or 0 if the display is empty

## Image Input

### Supported Formats

- **PNG** - Portable Network Graphics (any bit depth)
- **JPEG** - Joint Photographic Experts Group
- **Grayscale** - Single-channel images (typically pre-thresholded)
- **Color** - RGB images (green channel extracted during preprocessing)

### Image Requirements

- **Size**: Arbitrary (will be cropped and resized to 128×128)
- **Quality**: Should contain visible LCD segments
- **Color Space**: RGB or Grayscale (8-bit per channel)

### Input Interface

#### Rust API

```rust
pub trait ImageSource {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn pixel_rgb(&self, x: u32, y: u32) -> (f32, f32, f32);
}
```

**Example: File-based Input**

```rust
use lcd_pixel::preprocess::FileImageSource;

let source = FileImageSource::new("path/to/image.jpg")?;
```

**Future: Camera Input**

The `ImageSource` trait enables camera integration without modifying core code:

```rust
pub struct Ov2640Source {
    buffer: Vec<u8>,
    width: u32,
    height: u32,
}

impl ImageSource for Ov2640Source {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn pixel_rgb(&self, x: u32, y: u32) -> (f32, f32, f32) {
        // Decode from raw camera buffer
        // Return normalized (r, g, b) in [0.0, 1.0]
    }
}
```

## Crop Points Input

### Purpose

Four perspective corner points define the LCD display region within the image and correct for skew/tilt.

### Format

```
[(u32, u32); 4]
```

Four `(x, y)` coordinate pairs in the **source image coordinate system**.

### Point Order

Points must be provided in a specific order:
1. **Top-left** — upper-left corner of display
2. **Top-right** — upper-right corner of display
3. **Bottom-right** — lower-right corner of display
4. **Bottom-left** — lower-left corner of display

```
(0,0)              (127,0)
  +------------------+
  |  L C D D I S P   |
  |                  |
  +------------------+
(0,127)          (127,127)
```

### Coordinate System

- **Origin**: Top-left of the source image (0, 0)
- **X-axis**: Increases left to right
- **Y-axis**: Increases top to bottom
- **Range**: [0, image_width) × [0, image_height)

### Identity Transform (No Correction)

For an already-straight 128×128 image:

```rust
let points = [(0, 0), (127, 0), (127, 127), (0, 127)];
```

### Perspective Correction

For a tilted or skewed display, provide the actual corner positions:

```rust
// Display tilted right, bottom-right corner shifted out
let points = [(10, 5), (120, 2), (125, 135), (5, 130)];
```

## Processing Pipeline

### Steps

1. **Extract Green Channel** — RGB images are reduced to green channel for better LCD contrast
2. **Crop to Bounding Box** — Axis-aligned bbox of 4 points is cropped for efficiency
3. **Perspective Warp** — Homography transform maps points to 128×128 output
4. **Threshold** — Binary thresholding at 195/255 ≈ 0.765 (Otsu's method available)
5. **Cell Extraction** — 128×128 image divided into 5-wide × 7-tall grid, cells averaged
6. **Template Matching** — Cells scored against 10 digit templates (0-9)
7. **Empty Detection** — If mean brightness < 0.05, display is considered off/empty

## Output Format

### Return Type

```rust
pub fn get_digit(templates: &[[bool; 35]; 10], cells: &[f32; 35]) -> Option<u8>
```

**Returns:**
- `Some(digit)` — Detected digit (0-9)
- `None` — Display is empty/off (output as 0)

### Output Values

| Value | Meaning |
|-------|---------|
| 0 | Empty display or digit 0 |
| 1-9 | Detected digit |

### Command-Line Binary

```
$ lcd-pixel <image> <x1> <y1> <x2> <y2> <x3> <y3> <x4> <y4>
Digit: N
```

**Exit Codes:**
- `0` — Successful detection
- `1` — Error (missing arguments, invalid file, invalid coordinates)

### Detection Details

The `get_digit()` function performs **off-pixel scoring**:

```
score = Σ(cells[i] if template[i] is ON)
      - Σ(cells[i] if template[i] is OFF)
```

This rewards matching ON segments and penalizes unwanted brightness in OFF segments, improving accuracy.

## Usage Examples

### Rust Library

```rust
use lcd_pixel::preprocess::{preprocess, FileImageSource};
use lcd_pixel::detect::{calc::get_digit, template::load_templates};

// Load image
let source = FileImageSource::new("display.jpg")?;

// Define display region (e.g., known corners)
let points = [(20, 15), (480, 10), (490, 220), (15, 225)];

// Run pipeline
let sample = preprocess(&source, points);
let cells = sample.extract_cells();
let templates = load_templates();

// Detect digit
match get_digit(&templates, &cells) {
    Some(digit) => println!("Detected: {}", digit),
    None => println!("Display is off or empty"),
}
```

### CLI Binary

```bash
# Simple centered display (already 128×128)
./lcd-pixel display.jpg 0 0 127 0 127 127 0 127

# Tilted display in a larger photo
./lcd-pixel photo.jpg 150 100 450 95 460 310 145 315

# Grayscale pre-thresholded image
./lcd-pixel thresholded.png 0 0 127 0 127 127 0 127
```

## Error Handling

### File Loading

```
Failed to load image 'path.jpg': Format error decoding Jpeg: ...
```

**Causes:**
- File doesn't exist
- Unsupported image format
- Corrupted image data

### Invalid Coordinates

```
Invalid coordinate: 999999999999
```

**Causes:**
- Non-integer input
- Exceeds u32 range

## Performance Characteristics

| Operation | Time | Notes |
|-----------|------|-------|
| Image loading | ~10-50ms | Depends on file size and format |
| Green channel extraction | ~5-20ms | O(width × height) |
| Perspective warp | ~50-100ms | 128×128 output with homography |
| Cell extraction | <1ms | Fixed 5×7 grid on 128×128 |
| Template matching | <1ms | 10 templates, 35 cells each |
| **Total** | ~100-200ms | Per image |

## Future Extensions

### Camera Support

Implement `ImageSource` for OV2640 or similar:
```rust
pub struct Ov2640Source { ... }
impl ImageSource for Ov2640Source { ... }
```

No pipeline changes needed—fully backward compatible.

### Confidence Scoring

Return top-N matches with scores:
```rust
pub fn get_digit_ranked(
    templates: &[[bool; 35]; 10], 
    cells: &[f32; 35]
) -> Vec<(u8, f32)>
```

### Multi-Digit Recognition

Process multiple displays by providing multiple point sets:
```rust
let displays = vec![points1, points2, points3];
for points in displays {
    let sample = preprocess(&source, points);
    let digit = get_digit(&templates, &sample.extract_cells());
}
```

### Adaptive Thresholding

Already implemented—use `compute_otsu_threshold()` instead of fixed 195:
```rust
let threshold = compute_otsu_threshold(&warped);
let thresholded = threshold(&warped, threshold);
```
