/// Braille dot-matrix canvas for high-resolution character plotting.
///
/// Each character cell maps to a 2×4 dot grid (Unicode Braille U+2800-U+28FF).
/// This gives 2× horizontal and 4× vertical sub-pixel resolution.
///
/// Dot positions within a cell (bit index):
/// ```text
///   col0 col1
///   [0]  [3]   row 0
///   [1]  [4]   row 1
///   [2]  [5]   row 2
///   [6]  [7]   row 3
/// ```

/// Braille dot offset table: dot_bit[row][col]
const DOT_BIT: [[u8; 2]; 4] = [
    [0, 3], // row 0
    [1, 4], // row 1
    [2, 5], // row 2
    [6, 7], // row 3
];

/// A 2D canvas that renders to Braille characters.
///
/// Coordinates are in sub-pixel space:
///   x range: 0..width*2
///   y range: 0..height*4
pub struct BrailleCanvas {
    /// Width in character cells
    width: usize,
    /// Height in character cells
    height: usize,
    /// Dot buffer: one u8 per cell, bits correspond to Braille dots
    cells: Vec<u8>,
}

impl BrailleCanvas {
    /// Create a new canvas with given character cell dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![0u8; width * height],
        }
    }

    /// Sub-pixel width (2× character width)
    pub fn dot_width(&self) -> usize {
        self.width * 2
    }

    /// Sub-pixel height (4× character height)
    pub fn dot_height(&self) -> usize {
        self.height * 4
    }

    /// Set a single dot at sub-pixel coordinates.
    pub fn set(&mut self, x: usize, y: usize) {
        if x >= self.dot_width() || y >= self.dot_height() {
            return;
        }
        let cx = x / 2;
        let cy = y / 4;
        let dx = x % 2;
        let dy = y % 4;
        let idx = cy * self.width + cx;
        self.cells[idx] |= 1 << DOT_BIT[dy][dx];
    }

    /// Draw a line between two sub-pixel points (Bresenham).
    pub fn line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        let (mut x0, mut y0) = (x0 as isize, y0 as isize);
        let (x1, y1) = (x1 as isize, y1 as isize);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx: isize = if x0 < x1 { 1 } else { -1 };
        let sy: isize = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set(x0 as usize, y0 as usize);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Fill a circle sector (for pie charts).
    /// Center at (cx, cy) in sub-pixel coords, radius r,
    /// from angle_start to angle_end (radians, 0 = right, CCW).
    pub fn fill_sector(
        &mut self,
        cx: usize,
        cy: usize,
        r: usize,
        angle_start: f64,
        angle_end: f64,
    ) {
        let cx = cx as f64;
        let cy = cy as f64;
        let r = r as f64;
        let r2 = r * r;

        let min_x = ((cx - r).floor().max(0.0)) as usize;
        let max_x = ((cx + r).ceil() as usize).min(self.dot_width().saturating_sub(1));
        let min_y = ((cy - r).floor().max(0.0)) as usize;
        let max_y = ((cy + r).ceil() as usize).min(self.dot_height().saturating_sub(1));

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f64 - cx;
                let dy = py as f64 - cy;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mut angle = (-dy).atan2(dx); // y-up convention
                if angle < 0.0 {
                    angle += std::f64::consts::TAU;
                }
                if angle_in_range(angle, angle_start, angle_end) {
                    self.set(px, py);
                }
            }
        }
    }

    /// Get the Braille character for a cell.
    pub fn char_at(&self, cx: usize, cy: usize) -> char {
        if cx >= self.width || cy >= self.height {
            return ' ';
        }
        let bits = self.cells[cy * self.width + cx];
        char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
    }

    /// Get all characters as a 2D grid of strings (one string per row).
    pub fn rows(&self) -> Vec<String> {
        (0..self.height)
            .map(|cy| (0..self.width).map(|cx| self.char_at(cx, cy)).collect())
            .collect()
    }

    /// Clear the canvas.
    pub fn clear(&mut self) {
        self.cells.fill(0);
    }
}

/// Check if angle is within [start, end) range, handling wrap-around.
fn angle_in_range(angle: f64, start: f64, end: f64) -> bool {
    if start <= end {
        angle >= start && angle < end
    } else {
        // Wraps around 2π
        angle >= start || angle < end
    }
}
