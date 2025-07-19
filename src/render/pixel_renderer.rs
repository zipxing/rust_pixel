// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # Unified Pixel Renderer Interface
//!
//! This module defines a unified interface for all graphics mode pixel renderers,
//! providing a common abstraction over OpenGL and WGPU backends while maintaining
//! their specific performance characteristics.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Adapter Layer (Unchanged)                  │
//! │  ┌─────────────┬─────────────┬─────────────┬─────────────┐  │
//! │  │     SDL     │    Winit    │     Web     │  Crossterm  │  │
//! │  │   Adapter   │   Adapter   │   Adapter   │   Adapter   │  │
//! │  └─────────────┴─────────────┴─────────────┴─────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │            Unified Graphics Renderer Layer (NEW)            │
//! │                                                             │
//! │                PixelRenderer Trait                         │
//! │  ┌───────────────────────┬───────────────────────────────┐  │
//! │  │    OpenGL Backend     │        WGPU Backend           │  │
//! │  │                       │                               │  │
//! │  │  impl PixelRenderer   │   impl PixelRenderer          │  │
//! │  │  for GlPixel          │   for WgpuPixelRender         │  │
//! │  └───────────────────────┴───────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```

/// Unified color representation for graphics rendering
///
/// This structure provides a backend-agnostic color representation
/// that can be converted to specific backend color types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnifiedColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl UnifiedColor {
    /// Create a new color
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    
    /// Create white color
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
    
    /// Create black color
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
    
    /// Convert to array format
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
    

}

/// Unified 2D transformation matrix
///
/// This structure provides a backend-agnostic 2D transformation
/// that can be converted to specific backend transform types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnifiedTransform {
    pub m00: f32, pub m01: f32,
    pub m10: f32, pub m11: f32, 
    pub m20: f32, pub m21: f32,
}

impl UnifiedTransform {
    /// Create identity transform
    pub fn new() -> Self {
        Self {
            m00: 1.0, m01: 0.0,
            m10: 0.0, m11: 1.0,
            m20: 0.0, m21: 0.0,
        }
    }
    
    /// Create transform with specific values  
    /// Parameters are in same order as field definition: m00, m01, m10, m11, m20, m21
    pub fn new_with_values(m00: f32, m01: f32, m10: f32, m11: f32, m20: f32, m21: f32) -> Self {
        Self { m00, m01, m10, m11, m20, m21 }
    }
    
    /// Apply scaling transformation
    pub fn scale(&mut self, x: f32, y: f32) {
        // Correct scaling (matches WGPU behavior)
        self.m00 *= x;
        self.m10 *= y;
        self.m01 *= x;
        self.m11 *= y;
    }
    
    /// Apply translation transformation
    pub fn translate(&mut self, x: f32, y: f32) {
        // Correct matrix multiplication for translation (matches WGPU behavior)
        self.m20 += self.m00 * x + self.m10 * y;
        self.m21 += self.m01 * x + self.m11 * y;
    }
    
    /// Apply rotation (angle in radians)
    pub fn rotate(&mut self, angle: f32) {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        let m00 = self.m00;
        let m01 = self.m01;
        let m10 = self.m10;
        let m11 = self.m11;
        
        // Match WGPU's working rotation calculation:
        self.m00 = m00 * cos_a - m10 * sin_a;
        self.m10 = m00 * sin_a + m10 * cos_a;
        self.m01 = m01 * cos_a - m11 * sin_a;
        self.m11 = m01 * sin_a + m11 * cos_a;
    }
    
    /// Reset to identity matrix
    pub fn identity(&mut self) {
        self.m00 = 1.0; self.m01 = 0.0;
        self.m10 = 0.0; self.m11 = 1.0;
        self.m20 = 0.0; self.m21 = 0.0;
    }
    
    /// Set from another transform
    pub fn set(&mut self, other: &UnifiedTransform) {
        *self = *other;
    }
    
    /// Create a copy of this transform
    pub fn copy(&self) -> Self {
        *self
    }
    
    /// Multiply with another transform
    pub fn multiply(&mut self, other: &UnifiedTransform) {
        let new_m00 = self.m00 * other.m00 + self.m01 * other.m10;
        let new_m01 = self.m00 * other.m01 + self.m01 * other.m11;
        let new_m10 = self.m10 * other.m00 + self.m11 * other.m10;
        let new_m11 = self.m10 * other.m01 + self.m11 * other.m11;
        let new_m20 = self.m20 * other.m00 + self.m21 * other.m10 + other.m20;
        let new_m21 = self.m20 * other.m01 + self.m21 * other.m11 + other.m21;
        
        self.m00 = new_m00; self.m01 = new_m01;
        self.m10 = new_m10; self.m11 = new_m11;
        self.m20 = new_m20; self.m21 = new_m21;
    }
    
    /// Convert to 4x4 matrix for GPU uniforms (column-major order)
    pub fn to_matrix4(&self) -> [[f32; 4]; 4] {
        [
            [self.m00, self.m01, 0.0, 0.0],
            [self.m10, self.m11, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [self.m20, self.m21, 0.0, 1.0],
        ]
    }
    

}

impl Default for UnifiedTransform {
    fn default() -> Self {
        Self::new()
    }
}

