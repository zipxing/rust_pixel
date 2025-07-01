// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Transform Module
//! 
//! Implements 2D transformation matrix utilities for WGPU rendering pipeline.
//! Provides matrix operations for scaling, rotation, translation, and
//! projection transformations used in 2D graphics rendering.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
/// WGPU-compatible 4x4 transformation matrix
/// 
/// Represents a 4x4 transformation matrix in column-major order (compatible with WGSL).
/// The `#[repr(C)]` attribute ensures consistent memory layout for GPU upload.
/// 
/// Matrix layout:
/// ```text
/// [ m[0]  m[4]  m[8]   m[12] ]   [ xx  xy  xz  tx ]
/// [ m[1]  m[5]  m[9]   m[13] ] = [ yx  yy  yz  ty ]
/// [ m[2]  m[6]  m[10]  m[14] ]   [ zx  zy  zz  tz ]
/// [ m[3]  m[7]  m[11]  m[15] ]   [ wx  wy  wz  tw ]
/// ```
pub struct WgpuTransform {
    /// Matrix data in column-major order
    pub matrix: [f32; 16],
}

impl WgpuTransform {
    /// Create a new identity transformation matrix
    /// 
    /// # Returns
    /// Identity transformation matrix (no transformation applied)
    pub fn new() -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0, 0.0,  // Column 0
                0.0, 1.0, 0.0, 0.0,  // Column 1
                0.0, 0.0, 1.0, 0.0,  // Column 2
                0.0, 0.0, 0.0, 1.0,  // Column 3
            ],
        }
    }

    /// Create an identity transformation matrix
    /// 
    /// # Returns
    /// Identity transformation matrix
    pub fn identity() -> Self {
        Self::new()
    }

    /// Create a translation matrix
    /// 
    /// # Parameters
    /// - `x`: Translation along X-axis
    /// - `y`: Translation along Y-axis
    /// 
    /// # Returns
    /// Translation transformation matrix
    pub fn translation(x: f32, y: f32) -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0, 0.0,  // Column 0
                0.0, 1.0, 0.0, 0.0,  // Column 1
                0.0, 0.0, 1.0, 0.0,  // Column 2
                x,   y,   0.0, 1.0,  // Column 3
            ],
        }
    }

    /// Create a scaling matrix
    /// 
    /// # Parameters
    /// - `x`: Scale factor along X-axis
    /// - `y`: Scale factor along Y-axis
    /// 
    /// # Returns
    /// Scaling transformation matrix
    pub fn scaling(x: f32, y: f32) -> Self {
        Self {
            matrix: [
                x,   0.0, 0.0, 0.0,  // Column 0
                0.0, y,   0.0, 0.0,  // Column 1
                0.0, 0.0, 1.0, 0.0,  // Column 2
                0.0, 0.0, 0.0, 1.0,  // Column 3
            ],
        }
    }

    /// Create a rotation matrix
    /// 
    /// # Parameters
    /// - `angle_radians`: Rotation angle in radians
    /// 
    /// # Returns
    /// Rotation transformation matrix
    pub fn rotation(angle_radians: f32) -> Self {
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();
        
        Self {
            matrix: [
                cos_a, sin_a, 0.0, 0.0,  // Column 0
                -sin_a, cos_a, 0.0, 0.0,  // Column 1
                0.0,    0.0,   1.0, 0.0,  // Column 2
                0.0,    0.0,   0.0, 1.0,  // Column 3
            ],
        }
    }

    /// Create an orthographic projection matrix
    /// 
    /// # Parameters
    /// - `left`: Left clipping plane
    /// - `right`: Right clipping plane
    /// - `bottom`: Bottom clipping plane
    /// - `top`: Top clipping plane
    /// 
    /// # Returns
    /// Orthographic projection matrix
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        let width = right - left;
        let height = top - bottom;
        
        Self {
            matrix: [
                2.0 / width, 0.0,          0.0, 0.0,  // Column 0
                0.0,         2.0 / height, 0.0, 0.0,  // Column 1
                0.0,         0.0,          1.0, 0.0,  // Column 2
                -(right + left) / width, -(top + bottom) / height, 0.0, 1.0,  // Column 3
            ],
        }
    }

    /// Apply translation to this matrix
    /// 
    /// # Parameters
    /// - `x`: Translation along X-axis
    /// - `y`: Translation along Y-axis
    pub fn translate(&mut self, x: f32, y: f32) {
        let translation = Self::translation(x, y);
        *self = *self * translation;
    }

    /// Apply scaling to this matrix
    /// 
    /// # Parameters
    /// - `x`: Scale factor along X-axis
    /// - `y`: Scale factor along Y-axis
    pub fn scale(&mut self, x: f32, y: f32) {
        let scaling = Self::scaling(x, y);
        *self = *self * scaling;
    }

    /// Apply rotation to this matrix
    /// 
    /// # Parameters
    /// - `angle_radians`: Rotation angle in radians
    pub fn rotate(&mut self, angle_radians: f32) {
        let rotation = Self::rotation(angle_radians);
        *self = *self * rotation;
    }

    /// Get matrix data as byte array for GPU upload
    /// 
    /// # Returns
    /// 64-byte array representing the matrix
    pub fn as_bytes(&self) -> [u8; 64] {
        bytemuck::cast(self.matrix)
    }

    /// Get matrix data as reference to f32 array
    /// 
    /// # Returns
    /// Reference to the internal matrix array
    pub fn as_array(&self) -> &[f32; 16] {
        &self.matrix
    }
}

impl std::ops::Mul for WgpuTransform {
    type Output = WgpuTransform;

    /// Matrix multiplication
    /// 
    /// Multiplies two transformation matrices together.
    /// Order matters: `a * b` applies transformation `b` first, then `a`.
    fn mul(self, rhs: WgpuTransform) -> Self::Output {
        let a = &self.matrix;
        let b = &rhs.matrix;
        
        let mut result = [0.0f32; 16];
        
        // Matrix multiplication in column-major order
        for col in 0..4 {
            for row in 0..4 {
                result[col * 4 + row] = 
                    a[0 * 4 + row] * b[col * 4 + 0] +
                    a[1 * 4 + row] * b[col * 4 + 1] +
                    a[2 * 4 + row] * b[col * 4 + 2] +
                    a[3 * 4 + row] * b[col * 4 + 3];
            }
        }
        
        WgpuTransform { matrix: result }
    }
}

impl Default for WgpuTransform {
    /// Default to identity matrix
    fn default() -> Self {
        Self::identity()
    }
}

impl From<[f32; 16]> for WgpuTransform {
    /// Create transform from matrix array
    fn from(matrix: [f32; 16]) -> Self {
        Self { matrix }
    }
}

impl Into<[f32; 16]> for WgpuTransform {
    /// Convert to matrix array
    fn into(self) -> [f32; 16] {
        self.matrix
    }
}

// Implement bytemuck traits for safe casting to GPU data
unsafe impl bytemuck::Pod for WgpuTransform {}
unsafe impl bytemuck::Zeroable for WgpuTransform {} 