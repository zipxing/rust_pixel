// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Euclidean Distance Transform (EDT) implementation
// Based on Felzenszwalb-Huttenlocher O(n) algorithm

use image::RgbaImage;

/// 1D EDT using Felzenszwalb-Huttenlocher algorithm
/// Computes squared distances
fn edt_1d(f: &[f32], d: &mut [f32]) {
    let n = f.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        d[0] = f[0];
        return;
    }

    let mut v = vec![0usize; n]; // Envelope vertices
    let mut z = vec![f32::NEG_INFINITY; n + 1]; // Intersection points
    z[1] = f32::INFINITY;

    let mut k = 0;

    for q in 1..n {
        let mut s;
        loop {
            let vk = v[k];
            // s = intersection of parabolas at v[k] and q
            s = ((f[q] + (q * q) as f32) - (f[vk] + (vk * vk) as f32)) / (2.0 * (q - vk) as f32);
            if s > z[k] {
                break;
            }
            if k == 0 {
                break;
            }
            k -= 1;
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f32::INFINITY;
    }

    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let vk = v[k];
        let diff = q as i32 - vk as i32;
        d[q] = (diff * diff) as f32 + f[vk];
    }
}

/// 2D EDT - separable, row then column
/// Input: binary image where true = inside shape
/// Output: distance values (not squared)
pub fn distance_transform(binary: &[bool], width: usize, height: usize) -> Vec<f32> {
    let inf = (width * width + height * height) as f32;

    // Initialize: 0 for inside, infinity for outside
    let mut result: Vec<f32> = binary.iter().map(|&b| if b { 0.0 } else { inf }).collect();

    // Row transform
    let mut row_buf = vec![0.0f32; width];
    let mut tmp = vec![0.0f32; width];
    for y in 0..height {
        let offset = y * width;
        row_buf.copy_from_slice(&result[offset..offset + width]);
        edt_1d(&row_buf, &mut tmp);
        result[offset..offset + width].copy_from_slice(&tmp);
    }

    // Column transform
    let mut col = vec![0.0f32; height];
    let mut tmp_col = vec![0.0f32; height];
    for x in 0..width {
        for y in 0..height {
            col[y] = result[y * width + x];
        }
        edt_1d(&col, &mut tmp_col);
        for y in 0..height {
            // Take square root to get actual distance
            result[y * width + x] = tmp_col[y].sqrt();
        }
    }

    result
}

/// Convert bitmap image to SDF (Signed Distance Field)
///
/// Uses alpha channel to determine shape, computes signed distance
/// where positive = inside, negative = outside.
/// Normalizes to [0, 1] range where 0.5 = edge.
pub fn bitmap_to_sdf(bitmap: &RgbaImage, spread: f32) -> RgbaImage {
    let width = bitmap.width() as usize;
    let height = bitmap.height() as usize;

    // Extract binary mask from alpha channel (threshold at 0.5)
    let inside: Vec<bool> = bitmap
        .pixels()
        .map(|p| p[3] > 127) // Alpha > 0.5
        .collect();

    // Compute distances:
    // - dist_to_inside: for outside pixels, distance to nearest inside pixel
    // - dist_to_outside: for inside pixels, distance to nearest outside pixel
    let outside: Vec<bool> = inside.iter().map(|&b| !b).collect();

    let dist_to_inside = distance_transform(&inside, width, height);
    let dist_to_outside = distance_transform(&outside, width, height);

    // Signed distance: positive inside, negative outside
    let mut sdf_data = vec![0u8; width * height];
    for i in 0..width * height {
        let signed_dist = if inside[i] {
            // Inside pixel: positive distance to edge (nearest outside)
            dist_to_outside[i]
        } else {
            // Outside pixel: negative distance to edge (nearest inside)
            -dist_to_inside[i]
        };

        // Normalize to [0, 1]: 0.5 = edge, 1.0 = spread pixels inside, 0.0 = spread pixels outside
        let normalized = signed_dist / spread * 0.5 + 0.5;
        let clamped = normalized.clamp(0.0, 1.0);
        sdf_data[i] = (clamped * 255.0) as u8;
    }

    // Create RGBA image with SDF in RGB channels
    let mut result = RgbaImage::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let v = sdf_data[y * width + x];
            result.put_pixel(x as u32, y as u32, image::Rgba([v, v, v, 255]));
        }
    }

    result
}

/// Check if character is a graphic character (box drawing, blocks, etc.)
/// These need to fill the cell completely for proper tiling
pub fn is_graphic_char(ch: char) -> bool {
    let cp = ch as u32;
    (0x2500..=0x257F).contains(&cp)  // Box Drawing
        || (0x2580..=0x259F).contains(&cp)  // Block Elements
        || (0x2800..=0x28FF).contains(&cp)  // Braille Patterns
        || cp >= 0xE000  // Private Use / NerdFont / Powerline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edt_1d_simple() {
        let f = [0.0, 1e10, 1e10, 1e10, 0.0];
        let mut d = vec![0.0; 5];
        edt_1d(&f, &mut d);

        // d[0] = 0, d[1] = 1, d[2] = 4, d[3] = 1, d[4] = 0
        assert!((d[0] - 0.0).abs() < 0.001);
        assert!((d[1] - 1.0).abs() < 0.001);
        assert!((d[2] - 4.0).abs() < 0.001);
        assert!((d[3] - 1.0).abs() < 0.001);
        assert!((d[4] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_is_graphic_char() {
        assert!(is_graphic_char('─')); // Box drawing
        assert!(is_graphic_char('█')); // Block element
        assert!(is_graphic_char('⠀')); // Braille
        assert!(!is_graphic_char('A')); // Regular char
        assert!(!is_graphic_char('你')); // CJK
    }
}
