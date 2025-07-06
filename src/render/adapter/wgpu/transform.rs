// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Transform Module
//! 
//! Provides transformation matrix operations for WGPU rendering.
//! Compatible with OpenGL GlTransform interface for easy porting.

#[derive(Debug, Clone, Copy)]
pub struct WgpuTransform {
    pub m00: f32,
    pub m10: f32,
    pub m20: f32,
    pub m01: f32,
    pub m11: f32,
    pub m21: f32,
}

impl Default for WgpuTransform {
    fn default() -> Self {
        Self::new()
    }
}

impl WgpuTransform {
    /// Create a new identity transform
    pub fn new() -> Self {
        Self {
            m00: 1.0,
            m10: 0.0,
            m20: 0.0,
            m01: 0.0,
            m11: 1.0,
            m21: 0.0,
        }
    }

    /// Create a transform with specific values
    pub fn new_with_values(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self {
            m00: a,
            m10: b,
            m20: c,
            m01: d,
            m11: e,
            m21: f,
        }
    }

    /// Reset to identity matrix
    pub fn identity(&mut self) {
        self.m00 = 1.0;
        self.m10 = 0.0;
        self.m20 = 0.0;
        self.m01 = 0.0;
        self.m11 = 1.0;
        self.m21 = 0.0;
    }

    /// Set from another transform
    pub fn set(&mut self, other: &WgpuTransform) {
        *self = *other;
    }

    /// Create a copy of this transform
    pub fn copy(&self) -> Self {
        *self
    }

    /// Multiply with another transform
    pub fn multiply(&mut self, other: &WgpuTransform) {
        let m00 = self.m00 * other.m00 + self.m10 * other.m01;
        let m10 = self.m00 * other.m10 + self.m10 * other.m11;
        let m20 = self.m20 + self.m00 * other.m20 + self.m10 * other.m21;
        let m01 = self.m01 * other.m00 + self.m11 * other.m01;
        let m11 = self.m01 * other.m10 + self.m11 * other.m11;
        let m21 = self.m21 + self.m01 * other.m20 + self.m11 * other.m21;

        self.m00 = m00;
        self.m10 = m10;
        self.m20 = m20;
        self.m01 = m01;
        self.m11 = m11;
        self.m21 = m21;
    }

    /// Apply translation
    pub fn translate(&mut self, x: f32, y: f32) {
        self.m20 += self.m00 * x + self.m10 * y;
        self.m21 += self.m01 * x + self.m11 * y;
    }

    /// Apply scaling
    pub fn scale(&mut self, x: f32, y: f32) {
        self.m00 *= x;
        self.m10 *= x;
        self.m01 *= y;
        self.m11 *= y;
    }

    /// Apply rotation (angle in radians)
    pub fn rotate(&mut self, angle: f32) {
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let new_transform = WgpuTransform::new_with_values(cos_a, sin_a, 0.0, -sin_a, cos_a, 0.0);
        self.multiply(&new_transform);
    }

    /// Convert to 4x4 matrix for WGPU uniforms (column-major order)
    pub fn to_matrix4(&self) -> [[f32; 4]; 4] {
        [
            [self.m00, self.m01, 0.0, 0.0],
            [self.m10, self.m11, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [self.m20, self.m21, 0.0, 1.0],
        ]
    }

    /// Convert to flat array for uniform buffer
    pub fn to_array(&self) -> [f32; 16] {
        [
            self.m00, self.m01, 0.0, 0.0,
            self.m10, self.m11, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            self.m20, self.m21, 0.0, 1.0,
        ]
    }
} 