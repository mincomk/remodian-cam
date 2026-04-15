/// Absolute value for f64 without libm dependency.
#[inline]
fn abs_f64(x: f64) -> f64 {
    if x >= 0.0 { x } else { -x }
}

/// Compute a 3×3 homography matrix mapping source points to destination points.
/// Uses Gaussian elimination to solve the 8×8 linear system.
///
/// # Arguments
/// * `src` - 4 source points (x, y) in the original image
/// * `dst` - 4 destination points (x, y) in the target 128×128 image
///
/// # Returns
/// A 3×3 homography matrix H such that:
///   [x']   [h00 h01 h02] [x]
///   [y'] = [h10 h11 h12] [y]
///   [w']   [h20 h21 h22] [1]
/// where (x', y') = (x'/w', y'/w')
pub fn compute_homography(src: [(f64, f64); 4], dst: [(f64, f64); 4]) -> [[f64; 3]; 3] {
    // Build the 8×8 system: A * h = b
    // where h = [h00, h01, h02, h10, h11, h12, h20, h21]
    // (h22 = 1 by normalization)

    let mut a = [[0.0f64; 8]; 8];
    let mut b = [0.0f64; 8];

    for i in 0..4 {
        let (sx, sy) = src[i];
        let (dx, dy) = dst[i];

        // Equation 1: h00*sx + h01*sy + h02 - h20*sx*dx - h21*sy*dx = dx
        a[2 * i][0] = sx;
        a[2 * i][1] = sy;
        a[2 * i][2] = 1.0;
        a[2 * i][3] = 0.0;
        a[2 * i][4] = 0.0;
        a[2 * i][5] = 0.0;
        a[2 * i][6] = -sx * dx;
        a[2 * i][7] = -sy * dx;
        b[2 * i] = dx;

        // Equation 2: h10*sx + h11*sy + h12 - h20*sx*dy - h21*sy*dy = dy
        a[2 * i + 1][0] = 0.0;
        a[2 * i + 1][1] = 0.0;
        a[2 * i + 1][2] = 0.0;
        a[2 * i + 1][3] = sx;
        a[2 * i + 1][4] = sy;
        a[2 * i + 1][5] = 1.0;
        a[2 * i + 1][6] = -sx * dy;
        a[2 * i + 1][7] = -sy * dy;
        b[2 * i + 1] = dy;
    }

    // Gaussian elimination with partial pivoting
    gaussian_eliminate(&mut a, &mut b);

    // Extract solution and build 3×3 matrix
    let mut h = [[0.0f64; 3]; 3];
    h[0][0] = b[0];
    h[0][1] = b[1];
    h[0][2] = b[2];
    h[1][0] = b[3];
    h[1][1] = b[4];
    h[1][2] = b[5];
    h[2][0] = b[6];
    h[2][1] = b[7];
    h[2][2] = 1.0;

    h
}

/// Gaussian elimination with partial pivoting to solve Ax = b.
/// Modifies A and b in-place.
fn gaussian_eliminate(a: &mut [[f64; 8]; 8], b: &mut [f64; 8]) {
    // Forward elimination
    for col in 0..8 {
        // Find pivot
        let mut max_row = col;
        for row in col + 1..8 {
            if abs_f64(a[row][col]) > abs_f64(a[max_row][col]) {
                max_row = row;
            }
        }

        // Swap rows (manual swap to avoid borrow checker issues)
        if col != max_row {
            for j in col..8 {
                let temp = a[col][j];
                a[col][j] = a[max_row][j];
                a[max_row][j] = temp;
            }
            let temp_b = b[col];
            b[col] = b[max_row];
            b[max_row] = temp_b;
        }

        // Check for singular matrix
        if abs_f64(a[col][col]) < 1e-10 {
            continue;
        }

        // Eliminate column
        for row in col + 1..8 {
            let factor = a[row][col] / a[col][col];
            for j in col..8 {
                a[row][j] -= factor * a[col][j];
            }
            b[row] -= factor * b[col];
        }
    }

    // Back substitution
    for i in (0..8).rev() {
        if abs_f64(a[i][i]) < 1e-10 {
            b[i] = 0.0;
            continue;
        }
        let mut sum = b[i];
        for j in i + 1..8 {
            sum -= a[i][j] * b[j];
        }
        b[i] = sum / a[i][i];
    }
}

/// Apply homography transformation to a point.
/// Returns (x', y') in normalized coordinates.
#[inline]
pub fn transform_point(h: &[[f64; 3]; 3], x: f64, y: f64) -> (f64, f64) {
    let x_proj = h[0][0] * x + h[0][1] * y + h[0][2];
    let y_proj = h[1][0] * x + h[1][1] * y + h[1][2];
    let w = h[2][0] * x + h[2][1] * y + h[2][2];

    (x_proj / w, y_proj / w)
}
