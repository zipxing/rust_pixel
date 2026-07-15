use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// Configuration shared by the legacy converter and the deterministic optimizer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversionConfig {
    pub width: u32,
    pub height: u32,
    /// 0=PETSCII color, 1=exact two-color PETSCII, 2=exclude letters/digits.
    pub mode: u8,
    /// Number of glyph alternatives retained for deterministic optimization.
    pub top_k: usize,
    /// Contrast adjustment applied before conversion. Zero preserves legacy behavior.
    pub contrast: f32,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            width: 40,
            height: 25,
            mode: 0,
            top_k: 1,
            contrast: 0.0,
        }
    }
}

impl ConversionConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("width and height must be non-zero".to_string());
        }
        if self.width > 320 || self.height > 200 {
            return Err("width/height exceed the bounded converter limit".to_string());
        }
        if self.mode > 2 {
            return Err("mode must be 0, 1, or 2".to_string());
        }
        if !(1..=16).contains(&self.top_k) {
            return Err("top_k must be between 1 and 16".to_string());
        }
        if !self.contrast.is_finite() || !(-80.0..=80.0).contains(&self.contrast) {
            return Err("contrast must be finite and between -80 and 80".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetsciiCell {
    pub glyph: u8,
    pub fg: u8,
    pub bg: u8,
    pub texture: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PetsciiGrid {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<PetsciiCell>,
}

impl PetsciiGrid {
    pub fn new(width: u32, height: u32, cells: Vec<PetsciiCell>) -> Result<Self, String> {
        let expected = width as usize * height as usize;
        if width == 0 || height == 0 || cells.len() != expected {
            return Err(format!(
                "invalid grid: {}x{} expects {} cells, got {}",
                width,
                height,
                expected,
                cells.len()
            ));
        }
        Ok(Self {
            width,
            height,
            cells,
        })
    }

    #[inline]
    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> PetsciiCell {
        self.cells[self.index(x, y)]
    }

    /// Serialize the four-field `.pix` representation used by exact PETSCII mode.
    pub fn to_pix_string(&self) -> String {
        let mut out = format!("width={},height={},texture=255\n", self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.get(x, y);
                let _ = write!(
                    out,
                    "{},{},{},{} ",
                    cell.glyph, cell.fg, cell.texture, cell.bg
                );
            }
            out.push('\n');
        }
        out
    }

    /// Preserve the historical three-field output for modes 0 and 2.
    pub fn to_legacy_string(&self, mode: u8) -> String {
        if mode == 1 {
            return self.to_pix_string();
        }
        let mut out = format!("width={},height={},texture=255\n", self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.get(x, y);
                let _ = write!(out, "{},{},{} ", cell.glyph, cell.fg, cell.texture);
            }
            out.push('\n');
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphCandidate {
    pub glyph: u8,
    pub distance: f64,
    pub fg: u8,
    pub bg: u8,
    pub texture: u8,
}

impl GlyphCandidate {
    pub fn cell(self) -> PetsciiCell {
        PetsciiCell {
            glyph: self.glyph,
            fg: self.fg,
            bg: self.bg,
            texture: self.texture,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pix_grid_validates_cell_count() {
        assert!(PetsciiGrid::new(
            2,
            1,
            vec![PetsciiCell {
                glyph: 0,
                fg: 1,
                bg: 0,
                texture: 1
            }]
        )
        .is_err());
    }

    #[test]
    fn pix_serialization_has_expected_shape() {
        let grid = PetsciiGrid::new(
            1,
            1,
            vec![PetsciiCell {
                glyph: 7,
                fg: 5,
                bg: 2,
                texture: 1,
            }],
        )
        .unwrap();
        assert_eq!(
            grid.to_pix_string(),
            "width=1,height=1,texture=255\n7,5,1,2 \n"
        );
    }
}
