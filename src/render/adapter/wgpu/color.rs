// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Color Module
//! 
//! Provides color representation and operations for WGPU rendering.
//! Compatible with OpenGL GlColor interface for easy porting.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WgpuColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl WgpuColor {
    /// Create a new color
    /// 
    /// # Parameters
    /// - `r`: Red component (0.0-1.0)
    /// - `g`: Green component (0.0-1.0)
    /// - `b`: Blue component (0.0-1.0)
    /// - `a`: Alpha component (0.0-1.0)
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a white color
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    /// Create a black color
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// Create a transparent color
    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Create a red color
    pub fn red() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0)
    }

    /// Create a green color
    pub fn green() -> Self {
        Self::new(0.0, 1.0, 0.0, 1.0)
    }

    /// Create a blue color
    pub fn blue() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }

    /// Convert to RGBA array for uniforms
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to WGPU color for clear operations
    pub fn to_wgpu_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

impl Default for WgpuColor {
    fn default() -> Self {
        Self::white()
    }
}

impl From<(f32, f32, f32, f32)> for WgpuColor {
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Self::new(r, g, b, a)
    }
}

impl From<[f32; 4]> for WgpuColor {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self::new(r, g, b, a)
    }
}

impl Into<[f32; 4]> for WgpuColor {
    fn into(self) -> [f32; 4] {
        self.to_array()
    }
} 