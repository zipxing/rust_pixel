// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Color Management Module
//! 
//! Provides color space conversions, blending operations, and GPU-friendly
//! color format handling for the WGPU pipeline.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
/// WGPU-compatible color structure with RGBA components
/// 
/// Represents a color with red, green, blue, and alpha components.
/// Each component is a 32-bit floating point value in the range [0.0, 1.0].
/// 
/// The `#[repr(C)]` attribute ensures consistent memory layout for GPU upload,
/// making it safe to cast to byte arrays for uniform buffers.
pub struct WgpuColor {
    /// Red component (0.0 - 1.0)
    pub r: f32,
    /// Green component (0.0 - 1.0)  
    pub g: f32,
    /// Blue component (0.0 - 1.0)
    pub b: f32,
    /// Alpha component (0.0 - 1.0)
    pub a: f32,
}

impl WgpuColor {
    /// Create a new color with specified RGBA components
    /// 
    /// # Parameters
    /// - `r`: Red component (0.0 - 1.0)
    /// - `g`: Green component (0.0 - 1.0)
    /// - `b`: Blue component (0.0 - 1.0)
    /// - `a`: Alpha component (0.0 - 1.0)
    /// 
    /// # Returns
    /// New WgpuColor instance
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a black color (0, 0, 0, 1)
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// Create a white color (1, 1, 1, 1)
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    /// Create a transparent color (0, 0, 0, 0)
    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Convert from u8 RGBA values (0-255) to normalized float values (0.0-1.0)
    /// 
    /// # Parameters
    /// - `r`: Red component (0 - 255)
    /// - `g`: Green component (0 - 255)
    /// - `b`: Blue component (0 - 255)
    /// - `a`: Alpha component (0 - 255)
    /// 
    /// # Returns
    /// New WgpuColor instance with normalized values
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Convert to u8 RGBA values (0-255) from normalized float values (0.0-1.0)
    /// 
    /// # Returns
    /// Tuple of (r, g, b, a) as u8 values
    pub fn to_u8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    /// Convert to byte array for GPU upload
    /// 
    /// # Returns
    /// 16-byte array representing the color in RGBA float format
    pub fn as_bytes(&self) -> [u8; 16] {
        bytemuck::cast([self.r, self.g, self.b, self.a])
    }

    /// Convert to wgpu::Color for clear operations
    /// 
    /// # Returns
    /// wgpu::Color instance compatible with render pass clear operations
    pub fn to_wgpu_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

// Implement bytemuck traits for safe casting to GPU data
unsafe impl bytemuck::Pod for WgpuColor {}
unsafe impl bytemuck::Zeroable for WgpuColor {}

impl Default for WgpuColor {
    /// Default to black color
    fn default() -> Self {
        Self::black()
    }
}

impl From<(f32, f32, f32, f32)> for WgpuColor {
    /// Convert from (r, g, b, a) tuple
    fn from(rgba: (f32, f32, f32, f32)) -> Self {
        Self::new(rgba.0, rgba.1, rgba.2, rgba.3)
    }
}

impl From<(u8, u8, u8, u8)> for WgpuColor {
    /// Convert from (r, g, b, a) u8 tuple
    fn from(rgba: (u8, u8, u8, u8)) -> Self {
        Self::from_u8(rgba.0, rgba.1, rgba.2, rgba.3)
    }
}

impl Into<[f32; 4]> for WgpuColor {
    /// Convert to [f32; 4] array for shader uniforms
    fn into(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
} 