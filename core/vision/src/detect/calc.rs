use crate::detect::sample::DigitSample;

/// Penalty weight for a bright cell where the template expects it dark.
const EMPTY_WEIGHT: f32 = 3.5;
/// Penalty weight for a dark cell where the template expects it lit.
/// Must be close to EMPTY_WEIGHT so the effective "is this segment on?" threshold
/// sits near 0.5, preventing bleed from adjacent segments causing subset-digit
/// confusion (e.g. 3→9 or 3→8).
/// Threshold = MISSING_WEIGHT / (1 + MISSING_WEIGHT + EMPTY_WEIGHT).
const MISSING_WEIGHT: f32 = 3.5;

fn cells_digit_onehot(templates: &[[bool; 140]; 10], cells: &[f32; 140]) -> [f32; 10] {
    let mut onehot = [0.0f32; 10];
    for (digit, template) in templates.iter().enumerate() {
        let mut score = 0.0f32;
        for i in 0..140 {
            if template[i] {
                // On pixel: reward brightness, penalize darkness
                score += cells[i];
                score -= (1.0 - cells[i]) * MISSING_WEIGHT;
            } else {
                // Off pixel: penalize brightness
                score -= cells[i] * EMPTY_WEIGHT;
            }
        }
        onehot[digit] = score;
    }
    onehot
}

/// Detect whether the display is empty (all segments off).
/// Returns true if mean cell brightness is below the empty threshold.
fn is_empty(cells: &[f32; 140]) -> bool {
    const EMPTY_THRESHOLD: f32 = 0.05;
    let mean = cells.iter().sum::<f32>() / 140.0;
    mean < EMPTY_THRESHOLD
}

/// Combine multiple digit readings into a single number (positional decimal).
/// Returns `None` if any digit reading is `None` (empty display).
/// Example: `[Some(3), Some(7)]` → `Some(37)`
pub fn combine_digits(digits: &[Option<u8>]) -> Option<u32> {
    digits
        .iter()
        .try_fold(0u32, |acc, d| Some(acc * 10 + (*d)? as u32))
}

/// Detect the digit in the given cells against templates.
/// Returns `None` if the display appears empty, or `Some(digit)` otherwise.
pub fn get_digit(
    templates: &[[bool; 140]; 10],
    cells: &[f32; 140],
    _sample: &DigitSample,
) -> Option<u8> {
    if is_empty(cells) {
        return None;
    }

    let onehot = cells_digit_onehot(templates, cells);
    onehot
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(d, _)| d as u8)
}
