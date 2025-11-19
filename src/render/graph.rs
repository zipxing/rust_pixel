//! # Graphics Rendering Core Module
//!
//! This module contains the core data structures, constants and functions for
//! RustPixel's graphics rendering system. After the WGPU refactoring, this module
//! plays a more important role by providing unified data structures across backends.
//!
//! ## 🏗️ Module Responsibilities
//!
//! ### Core Data Structures
//! - **UnifiedColor**: Cross-backend color representation supporting RGBA float format
//! - **UnifiedTransform**: Unified 2D transformation matrix for sprite and texture transforms
//! - **RenderCell**: GPU-ready rendering unit data
//!
//! ### Texture and Symbol Management
//! - **PIXEL_TEXTURE_FILE**: Symbol texture file path constant
//! - **PIXEL_SYM_WIDTH/HEIGHT**: Global configuration for symbol dimensions
//! - Texture coordinate calculation and symbol index conversion
//!
//! ### Rendering Pipeline Abstraction
//! - **draw_all_graph()**: Unified graphics rendering entry point
//! - Buffer to RenderCell conversion logic
//! - Sprite rendering and Logo animation support
//!
//! ## 🚀 Design Benefits
//!
//! ### Cross-Backend Compatibility
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    graph.rs (This Module)                   │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │           Unified Data Structures                      │ │
//! │  │  ┌─────────────┬─────────────┬───────────────────────┐ │ │
//! │  │  │UnifiedColor │UnifiedTrans-│      RenderCell       │ │ │
//! │  │  │(RGBA)       │form (2D)    │   (GPU-ready)         │ │ │
//! │  │  └─────────────┴─────────────┴───────────────────────┘ │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! │                           │                                 │
//! │                           ▼                                 │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │              Backend Adapters                          │ │
//! │  │  ┌────────┬─────────┬─────────┬─────────┬────────────┐ │ │
//! │  │  │  SDL   │  Winit  │  Winit  │   Web   │  Crossterm │ │ │
//! │  │  │  +GL   │   +GL   │  +WGPU  │  +WebGL │    (Text)  │ │ │
//! │  │  └────────┴─────────┴─────────┴─────────┴────────────┘ │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Zero-Cost Abstractions
//! - **Compile-time specialization**: Each backend can optimize to best performance
//! - **Direct memory mapping**: RenderCell directly corresponds to GPU buffer format
//! - **No virtual function overhead**: Performance improvements after removing trait objects
//!
//! ## 📊 Symbol Texture System
//!
//! RustPixel uses a unified symbol texture to render characters and graphic elements:

use crate::{
    render::{buffer::Buffer, sprite::Sprites, style::Color, AdapterBase},
    util::{ARect, PointF32, PointI32, PointU16, Rand},
    LOGO_FRAME,
};
use std::sync::OnceLock;

/// Symbol texture file path
///
/// The symbol texture contains 8x8 blocks, each block containing 16x16 symbols,
/// totaling 128×128 symbols. This texture serves as the character atlas for
/// rendering text and symbols.
///
/// Layout:
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                   Symbols Texture Layout                    │
/// │                                                             │
/// │  ┌─────────┬─────────┬─────────┬─────────┬─────────┐        │
/// │  │Block 0,0│Block 1,0│Block 2,0│Block 3,0│Block 4,0│ ...    │
/// │  │16x16    │16x16    │16x16    │16x16    │16x16    │        │
/// │  │Symbols  │Symbols  │Symbols  │Symbols  │Symbols  │        │
/// │  ├─────────┼─────────┼─────────┼─────────┼─────────┤        │
/// │  │Block 0,1│Block 1,1│Block 2,1│Block 3,1│Block 4,1│ ...    │
/// │  │16x16    │16x16    │16x16    │16x16    │16x16    │        │
/// │  │Symbols  │Symbols  │Symbols  │Symbols  │Symbols  │        │
/// │  └─────────┴─────────┴─────────┴─────────┴─────────┘        │
/// │                           ...                               │
/// └─────────────────────────────────────────────────────────────┘
/// ```
pub const PIXEL_TEXTURE_FILE: &str = "assets/pix/symbols.png";

/// Symbol width (in pixels) resolved from the symbol atlas (8 pixels)
///
/// Initialized exactly once during adapter initialization. Accessing this
/// before initialization will panic with "lazylock init".
///
/// Note: Both Sprite and TUI layers use the same width (8 pixels).
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();

/// Symbol height (in pixels) resolved from the symbol atlas (8 pixels for Sprite)
///
/// Initialized exactly once during adapter initialization. Accessing this
/// before initialization will panic with "lazylock init".
///
/// Note: 
/// - Sprite layer: uses this value directly (8 pixels)
/// - TUI layer: uses double this value (16 pixels = PIXEL_SYM_HEIGHT * 2)
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();

/// Calculate the width of a single symbol (in pixels) based on the full texture width
///
/// # Parameters
/// - `width`: Total texture width
///
/// # Returns
/// Width of a single symbol
///
/// Calculates the width of a single 8x8 sprite cell based on texture dimensions.
/// The texture is organized as a 128x128 grid (128 columns × 128 rows).
pub fn init_sym_width(width: u32) -> f32 {
    const TEXTURE_GRID_SIZE: f32 = 128.0;
    width as f32 / TEXTURE_GRID_SIZE
}

/// Calculate the height of a single symbol (in pixels) based on the full texture height
///
/// # Parameters
/// - `height`: Total texture height
///
/// # Returns
/// Height of a single symbol
///
/// Calculates the height of a single 8x8 sprite cell based on texture dimensions.
/// The texture is organized as a 128x128 grid (128 columns × 128 rows).
pub fn init_sym_height(height: u32) -> f32 {
    const TEXTURE_GRID_SIZE: f32 = 128.0;
    height as f32 / TEXTURE_GRID_SIZE
}

/// Logo display width (in characters)
pub const PIXEL_LOGO_WIDTH: usize = 27;

/// Logo display height (in characters)
///
/// The logo is displayed during startup to show the project identity.
/// Uses RGB format storage with 3 bytes per pixel.
pub const PIXEL_LOGO_HEIGHT: usize = 12;

/// RustPixel Logo data
///
/// Predefined logo image data in RGB format, 3 bytes per pixel.
/// Displayed during game startup stage to provide brand identification.
///
/// Data format: [R, G, B, R, G, B, ...]
/// Dimensions: 27 × 12 pixels
pub const PIXEL_LOGO: [u8; PIXEL_LOGO_WIDTH * PIXEL_LOGO_HEIGHT * 3] = [
    32, 15, 1, 32, 202, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 239, 1, 32, 15, 1, 100, 239, 1, 32,
    239, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 165, 1, 32,
    165, 0, 32, 87, 1, 32, 15, 1, 18, 202, 1, 21, 202, 1, 19, 202, 1, 20, 202, 1, 32, 15, 1, 47,
    239, 1, 47, 239, 1, 116, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 87, 1, 32, 165, 0, 32, 165, 1, 32, 240, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 100, 239, 1, 81, 49, 1, 47, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15,
    1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15, 0, 32, 87, 1, 32, 15, 0, 32, 165, 0,
    47, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 238, 1, 47,
    238, 1, 47, 238, 1, 47, 239, 1, 100, 239, 1, 46, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1,
    104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1, 84,
    239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160,
    49, 1, 81, 49, 1, 32, 15, 1, 160, 86, 1, 32, 15, 1, 160, 49, 1, 47, 236, 1, 47, 236, 1, 46,
    234, 1, 160, 49, 1, 47, 239, 1, 81, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 47,
    239, 1, 160, 49, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 87, 1, 160, 45,
    1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 160, 45, 1, 32, 15, 1, 160, 45, 1, 32, 235, 1, 116, 235, 1,
    160, 45, 1, 47, 236, 1, 160, 45, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 46, 234, 1, 32, 15, 1,
    46, 234, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 1,
    32, 15, 0, 32, 197, 1, 160, 147, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 160, 147, 1, 32, 15,
    1, 160, 147, 1, 32, 235, 1, 116, 235, 1, 46, 235, 1, 81, 147, 1, 47, 239, 1, 47, 239, 1, 100,
    239, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 47, 239, 1, 32, 15, 1, 160, 147, 1,
    32, 239, 1, 84, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 32, 239, 1, 160, 147, 1, 47,
    239, 1, 104, 239, 1, 104, 240, 1, 160, 147, 1, 32, 15, 1, 160, 147, 1, 32, 15, 1, 116, 235, 1,
    160, 147, 1, 47, 239, 1, 160, 147, 1, 47, 239, 1, 47, 239, 1, 160, 147, 1, 104, 238, 1, 104,
    238, 1, 104, 238, 1, 104, 238, 1, 47, 242, 1, 160, 147, 1, 47, 239, 1, 104, 239, 1, 104, 239,
    1, 104, 239, 1, 47, 239, 1, 84, 239, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 81,
    214, 1, 47, 239, 1, 81, 214, 1, 47, 239, 1, 160, 214, 1, 47, 239, 1, 32, 0, 1, 46, 235, 1, 160,
    214, 1, 47, 236, 1, 81, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 47, 242, 1,
    81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 47, 239, 1, 32, 165, 1, 160, 214,
    1, 103, 239, 1, 32, 242, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 0, 1,
    32, 0, 1, 32, 87, 1, 32, 87, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 165, 0, 32,
    165, 0, 160, 214, 1, 103, 239, 1, 32, 242, 1, 32, 97, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97,
    0, 32, 165, 0, 32, 15, 1, 90, 214, 1, 47, 239, 1, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32,
    0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32,
    15, 1, 32, 15, 1, 32, 15, 1,
];

/// 🎨 Unified Color Representation
///
/// This struct provides cross-backend color abstraction, one of the core data structures
/// after the WGPU refactoring. Supports color representation and conversion for all
/// graphics backends (OpenGL, WGPU, WebGL).
///
/// ## 🔄 Cross-Backend Compatibility
///
/// ```text
/// UnifiedColor (RGBA f32)
///      │
///      ├─→ OpenGL: glColor4f(r, g, b, a)
///      ├─→ WGPU: wgpu::Color { r, g, b, a }
///      ├─→ WebGL: gl.uniform4f(location, r, g, b, a)
///      └─→ Crossterm: Color::Rgb { r: u8, g: u8, b: u8 }
/// ```
///
/// ## 🚀 Performance Features
/// - **Compile-time optimization**: Zero-cost abstraction, fully inlinable by compiler
/// - **Cache-friendly**: Compact memory layout (16 bytes)
/// - **SIMD compatible**: 4 f32 aligned, suitable for vectorization
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

/// 🔄 Unified 2D Transformation Matrix
///
/// This struct provides cross-backend 2D transformation abstraction, supporting
/// translation, scaling, rotation and other operations. After the WGPU refactoring,
/// it became the unified transformation representation for all graphics backends.
///
/// ## 📐 Matrix Layout
///
/// ```text
/// │m00  m01  m20│   │sx   0   tx│   Translation: (tx, ty)
/// │m10  m11  m21│ = │0   sy   ty│   Scale:       (sx, sy)  
/// │ 0    0    1 │   │0    0    1│   Rotation:    cos/sin in m00,m01,m10,m11
/// ```
///
/// ## 🔄 Backend Conversion
///
/// ```text
/// UnifiedTransform (2D Matrix)
///      │
///      ├─→ OpenGL: glUniformMatrix3fv(uniform, matrix)
///      ├─→ WGPU: bytemuck::cast_slice(&transform.to_array())
///      ├─→ WebGL: gl.uniformMatrix3fv(location, false, matrix)
///      └─→ Sprites: Apply to position/scale directly
/// ```
///
/// ## ⚡ Use Cases
/// - **Sprite transformation**: Position, scaling, rotation animations
/// - **UI layout**: Relative positioning of panels and controls
/// - **Effect rendering**: Particle systems and transition effects
/// - **Camera**: View transformation and projection matrices
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnifiedTransform {
    pub m00: f32,
    pub m01: f32,
    pub m10: f32,
    pub m11: f32,
    pub m20: f32,
    pub m21: f32,
}

impl UnifiedTransform {
    /// Create identity transform
    pub fn new() -> Self {
        Self {
            m00: 1.0,
            m01: 0.0,
            m10: 0.0,
            m11: 1.0,
            m20: 0.0,
            m21: 0.0,
        }
    }

    /// Create transform with specific values  
    /// Parameters are in same order as field definition: m00, m01, m10, m11, m20, m21
    pub fn new_with_values(m00: f32, m01: f32, m10: f32, m11: f32, m20: f32, m21: f32) -> Self {
        Self {
            m00,
            m01,
            m10,
            m11,
            m20,
            m21,
        }
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
        self.m00 = 1.0;
        self.m01 = 0.0;
        self.m10 = 0.0;
        self.m11 = 1.0;
        self.m20 = 0.0;
        self.m21 = 0.0;
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

        self.m00 = new_m00;
        self.m01 = new_m01;
        self.m10 = new_m10;
        self.m11 = new_m11;
        self.m20 = new_m20;
        self.m21 = new_m21;
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

/// GPU rendering unit structure
///
/// RenderCell serves as the intermediate data format between game buffers and
/// the GPU rendering pipeline. This design provides the following advantages:
///
/// ## Design Benefits
/// - **GPU optimization**: Data pre-formatted for efficient GPU upload
/// - **Batch processing**: Multiple units can be rendered in single draw calls
/// - **Flexible rendering**: Supports rotation, scaling and complex effects
/// - **Memory efficient**: Compact representation for large scenes
///
/// ## Rendering Pipeline Integration
/// ```text
/// ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
/// │   Buffer    │───►│ RenderCell  │───►│ OpenGL/GPU  │
/// │(Characters) │    │   Array     │    │  Rendering  │
/// └─────────────┘    └─────────────┘    └─────────────┘
/// ```
///
/// Each RenderCell contains all information needed to render a character or sprite,
/// including color, position, rotation and texture coordinates.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RenderCell {
    /// Foreground color RGBA components (0.0-1.0 range)
    ///
    /// Used for character/symbol rendering. Alpha component controls
    /// transparency and blending operations.
    pub fcolor: (f32, f32, f32, f32),

    /// Optional background color RGBA components
    ///
    /// When present, renders a colored background behind the symbol.
    /// If None, background is transparent.
    pub bcolor: Option<(f32, f32, f32, f32)>,

    /// Packed texture and symbol index value
    ///
    /// - High bits: Texture index (which texture to use)
    /// - Low bits: Symbol index (which character/symbol in texture)
    pub texsym: usize,

    /// Screen-space X position (in pixels)
    ///
    /// Note: This value is derived from high-level destination rectangle `s`
    /// produced by helper functions (e.g., `render_helper*`). It may be offset
    /// relative to the original logical top-left to cooperate with the backend
    /// transform chain, which applies additional translation and scaling.
    pub x: f32,

    /// Screen-space Y position (in pixels)
    ///
    /// See notes in `x` for how this value cooperates with the backend transform
    /// chain for final positioning.
    pub y: f32,

    /// Destination width (in pixels)
    ///
    /// This is the final display width for the symbol instance after ratio-based
    /// adjustments performed in helper functions.
    pub w: u32,

    /// Destination height (in pixels)
    ///
    /// This is the final display height for the symbol instance after ratio-based
    /// adjustments performed in helper functions.
    pub h: u32,

    /// Rotation angle (radians)
    ///
    /// Used for sprite rotation effects. 0.0 means no rotation.
    pub angle: f32,

    /// Rotation center X coordinate
    ///
    /// Defines the pivot point for rotation.
    pub cx: f32,

    /// Rotation center Y coordinate
    ///
    /// Defines the pivot point for rotation.
    pub cy: f32,
}

pub struct Graph {
    /// Physical window width in pixels
    pub pixel_w: u32,

    /// Physical window height in pixels
    pub pixel_h: u32,

    /// Horizontal scaling ratio for different DPI displays
    ///
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_x: f32,

    /// Vertical scaling ratio for different DPI displays
    ///
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_y: f32,

    /// Render flag controlling immediate vs buffered rendering
    ///
    /// - true: Direct rendering to screen (normal mode)
    /// - false: Buffered rendering for external access (used for FFI/WASM)
    pub rflag: bool,

    /// Render buffer storing RenderCell array for buffered mode
    ///
    /// When rflag is false, rendered data is stored rbuf instead of
    /// being directly drawn to screen. Used for external access to
    /// rendering data (e.g., Python FFI, WASM exports).
    pub rbuf: Vec<RenderCell>,
    // pixel_renderer field removed - all adapters now use direct renderers
}

impl Graph {
    /// Create new graphics rendering context
    ///
    /// Initializes all graphics mode related data structures and rendering state.
    /// Render flag defaults to true (direct rendering to screen).
    pub fn new() -> Self {
        Self {
            pixel_w: 0,
            pixel_h: 0,
            ratio_x: 1.0,
            ratio_y: 1.0,
            rflag: true,
            rbuf: Vec::new(),
            // pixel_renderer field removed - all adapters now use direct renderers
        }
    }

    /// Set X-axis scaling ratio
    ///
    /// Used for handling scaling adaptation for different DPI displays.
    /// This value affects pixel width calculation and rendering coordinate conversion.
    ///
    /// # Parameters
    /// - `rx`: X-axis scaling ratio (1.0 for standard ratio)
    pub fn set_ratiox(&mut self, rx: f32) {
        self.ratio_x = rx;
    }

    /// Set Y-axis scaling ratio
    ///
    /// Used for handling scaling adaptation for different DPI displays.
    /// This value affects pixel height calculation and rendering coordinate conversion.
    ///
    /// # Parameters
    /// - `ry`: Y-axis scaling ratio (1.0 for standard ratio)
    pub fn set_ratioy(&mut self, ry: f32) {
        self.ratio_y = ry;
    }

    /// Calculate and set pixel dimensions based on current settings
    ///
    /// Calculates actual pixel width and height based on cell count, symbol dimensions
    /// and scaling ratios. This is the core method for graphics mode window size calculation.
    ///
    /// # Parameters
    /// - `cell_w`: Game area width (character cell count)
    /// - `cell_h`: Game area height (character cell count)
    ///
    /// # Calculation Formula
    /// ```text
    /// pixel_w = (cell_w + 2) * symbol_width / ratio_x
    /// pixel_h = (cell_h + 2) * symbol_height / ratio_y
    /// ```
    /// Where +2 reserves space for borders
    pub fn set_pixel_size(&mut self, cell_w: u16, cell_h: u16) {
        self.pixel_w = ((cell_w + 2) as f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init")
            / self.ratio_x) as u32;
        self.pixel_h = ((cell_h + 2) as f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init")
            / self.ratio_y) as u32;
    }

    /// Get single character cell width (pixels)
    ///
    /// Calculates actual pixel width of a single character cell based on symbol
    /// texture dimensions and current X-axis scaling ratio. This value is used
    /// for precise position calculation and rendering layout.
    ///
    /// # Returns
    /// Pixel width of a single character cell
    pub fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH.get().expect("lazylock init") / self.ratio_x
    }

    /// Get single character cell height (pixels)
    ///
    /// Calculates actual pixel height of a single character cell based on symbol
    /// texture dimensions and current Y-axis scaling ratio. This value is used
    /// for precise position calculation and rendering layout.
    ///
    /// # Returns
    /// Pixel height of a single character cell
    pub fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT.get().expect("lazylock init") / self.ratio_y
    }
}

/// Convert high-level element data to a GPU-ready RenderCell
///
/// This function converts individual game elements (characters, sprites, etc.) into
/// a GPU-ready RenderCell. It handles:
/// - Texture/symbol indexing and packing (texsym)
/// - Color normalization (u8 → f32)
/// - Destination rectangle mapping (position and size)
/// - Rotation and rotation center
///
/// ## Conversion Process
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Data Transformation                        │
/// │                                                             │
/// │  Game Data Input:                                           │
/// │  ├── Colors (u8 RGBA) ────────► Normalized (f32 RGBA)       │
/// │  ├── Texture & Symbol Index ──► Packed texsym value         │
/// │  ├── Screen Rectangle ─────────► Position & dimensions      │
/// │  ├── Rotation angle ───────────► Angle + center point       │
/// │  └── Background color ─────────► Optional background        │
/// │                                                             │
/// │                       ▼                                     │
/// │               ┌─────────────────────┐                       │
/// │               │    RenderCell       │                       │
/// │               │   (GPU-ready)       │                       │
/// │               └─────────────────────┘                       │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// # Parameters
/// - `rbuf`: Target RenderCell vector to append to
/// - `fc`: Foreground color as (R,G,B,A) in 0-255 range
/// - `bgc`: Optional background color
/// - `texidx`: Texture region identifier (0=TUI, 255=Emoji, 1-254=Sprite)
/// - `symidx`: Symbol index within the region (0-255 for most, 0-1023 for TUI)
/// - `s`: Destination rectangle in screen space (pixels). The helper functions
///        already apply ratio-based sizing and spacing; this function may derive
///        an offset from it to cooperate with backend transform chain.
/// - `angle`: Rotation angle in degrees (will be converted to radians internally)
/// - `ccp`: Center point for rotation
///
/// # Unified Texture Layout (1024x1024)
/// - **TUI Region** (rows 0-127): 128 cols × 8 rows = 1024 chars (8x16 pixels each)
///   - Index range: 0-1023 (linear)
/// - **Emoji Region** (rows 128-191): 64 cols × 4 rows = 256 emojis (16x16 pixels each)
///   - Index range: 1024-1279 (linear)
/// - **Sprite Region** (rows 192-1023): 128 cols × 104 rows = 13312 chars (8x8 pixels each)
///   - Index range: 1280-14591 (linear)
pub fn push_render_buffer(
    rbuf: &mut Vec<RenderCell>,
    fc: &(u8, u8, u8, u8),
    bgc: &Option<(u8, u8, u8, u8)>,
    texidx: usize,
    symidx: usize,
    s: ARect,
    angle: f64,
    ccp: &PointI32,
) {
    let mut wc = RenderCell {
        fcolor: (
            fc.0 as f32 / 255.0,
            fc.1 as f32 / 255.0,
            fc.2 as f32 / 255.0,
            fc.3 as f32 / 255.0,
        ),
        ..Default::default()
    };
    if let Some(bc) = bgc {
        wc.bcolor = Some((
            bc.0 as f32 / 255.0,
            bc.1 as f32 / 255.0,
            bc.2 as f32 / 255.0,
            bc.3 as f32 / 255.0,
        ));
    } else {
        wc.bcolor = None;
    }
    
    // Calculate final texture symbol index
    //
    // The texture is traversed in row-major order (128 cols × 128 rows):
    // - Rows 0-95 (indices 0-12287): 8×8 Sprites (block-based layout)
    // - Rows 96-127, Cols 0-79 (indices 12288-16383): 8×16 TUI characters
    // - Rows 96-127, Cols 80-127 (indices 16384-17919): 16×16 Emoji
    //
    // For Sprite region (block-based layout for backward compatibility):
    let x = symidx as u32 % 16u32 + (texidx as u32 % 8u32) * 16u32;
    let y = symidx as u32 / 16u32 + (texidx as u32 / 8u32) * 16u32;
    wc.texsym = (y * 128u32 + x) as usize;
    // Derive the instance anchor from the destination rectangle produced by helper functions.
    //
    // The backend transform chain applies additional translation and ratio-compensation.
    // Here we set the instance anchor relative to the destination rectangle to match that chain.
    wc.x = s.x as f32 + s.w as f32;
    wc.y = s.y as f32 + s.h as f32;
    wc.w = s.w;
    wc.h = s.h;
    if angle == 0.0 {
        wc.angle = angle as f32;
    } else {
        let mut aa = (1.0 - angle / 180.0) * std::f64::consts::PI;
        let pi2 = std::f64::consts::PI * 2.0;
        while aa < 0.0 {
            aa += pi2;
        }
        while aa > pi2 {
            aa -= pi2;
        }
        wc.angle = aa as f32;
    }
    wc.cx = ccp.x as f32;
    wc.cy = ccp.y as f32;
    rbuf.push(wc);
}

pub fn render_helper(
    cell_w: u16,
    r: PointF32,
    i: usize,
    sh: &(u8, u8, Color, Color),
    p: PointU16,
    is_border: bool,
    use_tui: bool,
) -> (ARect, usize, usize) {
    render_helper_with_scale(cell_w, r, i, sh, p, is_border, use_tui, 1.0, 1.0)
}

/// Enhanced helper that returns destination rectangle and symbol indices with per-sprite scaling.
///
/// Returns: (dest_rect, texture_id, symbol_id)
pub fn render_helper_with_scale(
    cell_w: u16,
    r: PointF32,
    i: usize,
    sh: &(u8, u8, Color, Color),
    p: PointU16,
    is_border: bool,
    use_tui: bool, // Use TUI characters (8×16) instead of Sprite characters (8×8)
    scale_x: f32,  // Sprite scaling along X (unitless, 1.0 means no scaling)
    scale_y: f32,  // Sprite scaling along Y (unitless, 1.0 means no scaling)
) -> (ARect, usize, usize) {
    let w = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as i32;  // 8 pixels
    // Height depends on character type:
    // - Sprite: 8 pixels
    // - TUI: 16 pixels (double height)
    let h = if use_tui {
        (*PIXEL_SYM_HEIGHT.get().expect("lazylock init") * 2.0) as i32 // TUI: 16 pixels
    } else {
        *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as i32 // Sprite: 8 pixels
    };
    
    let dstx = i as u16 % cell_w;
    let dsty = i as u16 / cell_w;
    
    let tx = sh.1 as usize;

    // Apply per-sprite scaling to the destination render area
    let scaled_w = (w as f32 / r.x * scale_x) as u32;
    let scaled_h = (h as f32 / r.y * scale_y) as u32;

    // Apply position scaling: scale both symbol size and spacing to avoid overlaps
    let base_x = (dstx + if is_border { 0 } else { 1 }) as f32 * (w as f32 / r.x);
    let base_y = (dsty + if is_border { 0 } else { 1 }) as f32 * (h as f32 / r.y);

    let scaled_x = base_x * scale_x;
    let scaled_y = base_y * scale_y;

    (
        // Destination rectangle in the render texture (with sprite scaling applied
        // to both size and position)
        ARect {
            x: scaled_x as i32 + p.x as i32,
            y: scaled_y as i32 + p.y as i32,
            w: scaled_w,
            h: scaled_h,
        },
        // texture id
        tx,
        // sym id (original index, not offset)
        sh.0 as usize,
    )
}

/// Render pixel sprites with rotation and transformation support
///
/// This function processes individual sprite objects and converts them to renderable
/// format. It supports advanced features like rotation, scaling, and complex
/// transformations while maintaining efficient rendering performance.
///
/// ## Sprite Rendering Pipeline
/// ```text
/// ┌────────────────────────────────────────────────────────────┐
/// │                   Sprite Processing                        │
/// │                                                            │
/// │  ┌─────────────┐                                           │
/// │  │   Sprite    │                                           │
/// │  │   Object    │                                           │
/// │  │  ┌───────┐  │  ┌─────────────────────────────────────┐  │
/// │  │  │Pixels │  │  │        Transformation               │  │
/// │  │  │Array  │  │  │  ┌────────────────────────────────┐ │  │
/// │  │  └───────┘  │  │  │  1. Position calculation       │ │  │
/// │  │     │       │  │  │  2. Rotation matrix applied    │ │  │
/// │  │     ▼       │  │  │  3. Scaling based on rx/ry     │ │  │
/// │  │  ┌───────┐  │  │  │  4. Color & texture mapping    │ │  │
/// │  │  │Colors │  │  │  └────────────────────────────────┘ │  │
/// │  │  │&Flags │  │  └─────────────────────────────────────┘  │
/// │  │  └───────┘  │                     │                     │
/// │  └─────────────┘                     ▼                     │
/// │                        ┌─────────────────────┐             │
/// │                        │  Callback Function  │             │
/// │                        │ (push_render_buffer)│             │
/// │                        └─────────────────────┘             │
/// │                                 │                          │
/// │                                 ▼                          │
/// │                        ┌─────────────────────┐             │
/// │                        │    RenderCell       │             │
/// │                        │      Array          │             │
/// │                        └─────────────────────┘             │
/// └────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Features Supported
/// - **Rotation**: Full 360-degree rotation around sprite center
/// - **Scaling**: Display ratio compensation for different screen densities
/// - **Transparency**: Alpha blending and background color support
/// - **Instanced Rendering**: Efficient batch processing for multiple sprites
///
/// # Parameters
/// - `pixel_spt`: Sprite object containing pixel data and transformation info
/// - `rx`: Horizontal scaling ratio for display compensation
/// - `ry`: Vertical scaling ratio for display compensation
/// - `f`: Callback function to process each sprite pixel
pub fn render_pixel_sprites<F>(pixel_spt: &mut Sprites, rx: f32, ry: f32, mut f: F)
where
    // Callback signature: (fg_color, bg_color, dst_rect, tex_idx, sym_idx, angle, center_point)
    F: FnMut(
        &(u8, u8, u8, u8),
        &Option<(u8, u8, u8, u8)>,
        ARect,
        usize,
        usize,
        f64,
        PointI32,
    ),
{
    // sort by render_weight...
    pixel_spt.update_render_index();
    for si in &pixel_spt.render_index {
        let s = &pixel_spt.sprites[si.0];
        if s.is_hidden() {
            continue;
        }
        let px = s.content.area.x;
        let py = s.content.area.y;
        let pw = s.content.area.width;
        let ph = s.content.area.height;

        for (i, cell) in s.content.content.iter().enumerate() {
            let sh = &cell.get_cell_info();
            let (s2, texidx, symidx) = render_helper_with_scale(
                pw,
                PointF32 { x: rx, y: ry },
                i,
                sh,
                PointU16 { x: px, y: py },
                false,
                false,    // Pixel sprites use Sprite characters (8×8)
                s.scale_x, // 应用sprite的X轴缩放
                s.scale_y, // 应用sprite的Y轴缩放
            );
            let x = i % pw as usize;
            let y = i / pw as usize;
            // Center point for rotation — matching the scaled position.
            // Since we scaled both symbol size and spacing, the rotation center must scale accordingly.
            let w = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as f32;
            let h = *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as f32;

            let original_offset_x = (pw as f32 / 2.0 - x as f32) * w / rx;
            let original_offset_y = (ph as f32 / 2.0 - y as f32) * h / ry;

            // Apply the same scaling to the rotation center offset
            let ccp = PointI32 {
                x: (original_offset_x * s.scale_x) as i32,
                y: (original_offset_y * s.scale_y) as i32,
            };
            let mut fc = sh.2.get_rgba();
            fc.3 = s.alpha;
            let bc;
            if sh.3 != Color::Reset {
                let mut brgba = sh.3.get_rgba();
                brgba.3 = s.alpha;
                bc = Some(brgba);
            } else {
                bc = None;
            }
            f(&fc, &bc, s2, texidx, symidx, s.angle, ccp);
        }
    }
}

/// Main buffer rendering with character-to-pixel conversion
///
/// This function processes the main game buffer containing character data and
/// converts it to renderable format. It follows the principle.md design where
/// characters are the fundamental rendering unit, with each character mapped
/// to symbols in the texture atlas.
///
/// ## Buffer Rendering Process
/// ```text
/// ┌────────────────────────────────────────────────────────────┐
/// │                   Main Buffer Processing                   │
/// │                                                            │
/// │  ┌─────────────────────┐                                   │
/// │  │      Buffer         │                                   │
/// │  │   ┌─────────────┐   │                                   │
/// │  │   │ Character   │   │    ┌─────────────────────────────┐│
/// │  │   │   Grid      │   │    │   Per-Character Process     ││
/// │  │   │             │   │    │                             ││
/// │  │   │ ┌─┬─┬─┬─┐   │   │    │ 1. Read character data      ││
/// │  │   │ │A│B│C│D│   │   │    │ 2. Extract colors & symbol  ││
/// │  │   │ ├─┼─┼─┼─┤   │──────► │ 3. Calculate screen pos     ││
/// │  │   │ │E│F│G│H│   │   │    │ 4. Map to texture coords    ││
/// │  │   │ ├─┼─┼─┼─┤   │   │    │ 5. Call render callback     ││
/// │  │   │ │I│J│K│L│   │   │    │                             ││
/// │  │   │ └─┴─┴─┴─┘   │   │    └─────────────────────────────┘│
/// │  │   └─────────────┘   │                     │             │
/// │  └─────────────────────┘                     ▼             │
/// │                                ┌─────────────────────┐     │
/// │                                │   RenderCell Array  │     │
/// │                                │   (GPU-ready data)  │     │
/// │                                └─────────────────────┘     │
/// └────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Character Data Structure
/// Each character in the buffer contains:
/// - **Symbol Index**: Which character/symbol to display
/// - **Texture Index**: Which texture sheet to use
/// - **Foreground Color**: Primary character color
/// - **Background Color**: Optional background fill color
/// - **Style Flags**: Bold, italic, underline, etc.
///
/// # Parameters
/// - `buf`: Game buffer containing character grid data
/// - `width`: Buffer width in characters
/// - `rx`: Horizontal scaling ratio for display adaptation
/// - `ry`: Vertical scaling ratio for display adaptation
/// - `border`: Include border rendering (for windowed modes)
/// - `use_tui`: Use TUI characters (8×16) instead of Sprite characters (8×8)
/// - `f`: Callback function to process each character
pub fn render_main_buffer<F>(buf: &Buffer, width: u16, rx: f32, ry: f32, border: bool, use_tui: bool, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, usize, usize),
{
    for (i, cell) in buf.content.iter().enumerate() {
        // symidx, texidx, fg, bg
        let sh = cell.get_cell_info();
        
        // Pass use_tui flag directly to render_helper
        // - false: 8×8 Sprite characters (for pixel sprites, backward compatibility)
        // - true: 8×16 TUI characters (for UI components)
        let (s2, texidx, symidx) = render_helper(
            width,
            PointF32 { x: rx, y: ry },
            i,
            &sh,
            PointU16 { x: 0, y: 0 },
            border,
            use_tui,
        );
        
        let fc = sh.2.get_rgba();
        let bc = if sh.3 != Color::Reset {
            Some(sh.3.get_rgba())
        } else {
            None
        };
        f(&fc, &bc, s2, texidx, symidx);
    }
}

/// Render window borders (windowed display modes)
///
/// This function renders decorative borders around the game area for SDL and Winit
/// modes. The border provides visual separation between the game content and the
/// desktop environment, creating a more polished windowed gaming experience.
///
/// ## Border Layout
/// ```text
/// ┌───────────────────────────────────────────────────────┐
/// │                      Window Border                    │
/// │  ┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  ├─┤                 Game Content Area           ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                     80 x 40                 ├─┤  │
/// │  ├─┤                  Character Grid             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  └─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘  │
/// └───────────────────────────────────────────────────────┘
/// ```
///
/// The border consists of:
/// - **Top/Bottom Edges**: Horizontal line characters
/// - **Left/Right Edges**: Vertical line characters
/// - **Corners**: Corner junction characters
/// - **Consistent Styling**: Matches the game's visual theme
///
/// # Parameters
/// - `cell_w`: Game area width in characters
/// - `cell_h`: Game area height in characters
/// - `rx`: Horizontal scaling ratio
/// - `ry`: Vertical scaling ratio
/// - `f`: Callback function to render each border character
pub fn render_border<F>(cell_w: u16, cell_h: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, usize, usize),
{
    let sh_top = (102u8, 1u8, Color::Indexed(7), Color::Reset);
    let sh_other = (24u8, 2u8, Color::Indexed(7), Color::Reset);
    let sh_close = (214u8, 1u8, Color::Indexed(7), Color::Reset);

    for n in 0..cell_h as usize + 2 {
        for m in 0..cell_w as usize + 2 {
            if n != 0 && n != cell_h as usize + 1 && m != 0 && m != cell_w as usize + 1 {
                continue;
            }
            let rsh;
            if n == 0 {
                if m as u16 <= cell_w {
                    rsh = &sh_top;
                } else {
                    rsh = &sh_close;
                }
            } else {
                rsh = &sh_other;
            }
            let (s2, texidx, symidx) = render_helper(
                cell_w + 2,
                PointF32 { x: rx, y: ry },
                n * (cell_w as usize + 2) + m,
                rsh,
                PointU16 { x: 0, y: 0 },
                true,
                false, // Border uses Sprite characters (8×8)
            );
            let fc = rsh.2.get_rgba();
            let bc = None;
            f(&fc, &bc, s2, texidx, symidx);
        }
    }
}

/// Render the RustPixel logo animation with dynamic effects
///
/// This function renders the animated RustPixel logo during the startup sequence.
/// It provides a visually appealing introduction to the framework with dynamic
/// effects and proper centering across different screen resolutions.
///
/// ## Logo Animation Sequence
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Logo Animation Timeline                  │
/// │                                                             │
/// │  Stage 0 ────────────────────────────────► LOGO_FRAME       │
/// │    │                                            │           │
/// │    ▼                                            ▼           │
/// │  ┌─────────────────┐                    ┌─────────────────┐ │
/// │  │  Logo Display   │                    │  Start Game     │ │
/// │  │                 │                    │   Rendering     │ │
/// │  │  ┌───────────┐  │                    │                 │ │
/// │  │  │ ██████    │  │   Dynamic Effects: │                 │ │
/// │  │  │ ██  ██    │  │   - Random colors  │                 │ │
/// │  │  │ ██████    │  │   - Centered pos   │                 │ │
/// │  │  │ ██  ██    │  │   - Smooth trans   │                 │ │
/// │  │  │ ██  ██    │  │   - Frame timing   │                 │ │
/// │  │  └───────────┘  │                    │                 │ │
/// │  └─────────────────┘                    └─────────────────┘ │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Rendering Features
/// - **Centered Positioning**: Automatically centers on any screen size
/// - **Dynamic Colors**: Randomly generated color effects for visual appeal
/// - **Smooth Animation**: Frame-based timing for consistent display
/// - **High-DPI Support**: Proper scaling for different display densities
/// - **Cross-platform**: Works consistently across SDL, Winit, and Web modes
///
/// ## Logo Data Processing
/// The function processes the PIXEL_LOGO constant array where each character
/// is represented by 3 bytes: [symbol_id, texture_id, flags]. The logo is
/// dynamically positioned and colored based on the current animation stage.
///
/// # Parameters
/// - `srx`: Screen horizontal scaling ratio
/// - `sry`: Screen vertical scaling ratio
/// - `spw`: Screen physical width in pixels
/// - `sph`: Screen physical height in pixels
/// - `rd`: Random number generator for color effects
/// - `stage`: Current animation stage (0 to LOGO_FRAME)
/// - `f`: Callback function to render each logo character
pub fn render_logo<F>(srx: f32, sry: f32, spw: u32, sph: u32, rd: &mut Rand, stage: u32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, usize, usize),
{
    let rx = srx * 1.0;
    let ry = sry * 1.0;
    for y in 0usize..PIXEL_LOGO_HEIGHT {
        for x in 0usize..PIXEL_LOGO_WIDTH {
            let sci = y * PIXEL_LOGO_WIDTH + x;
            let symw = PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
            let symh = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

            let (mut s2, texidx, symidx) = render_helper(
                PIXEL_LOGO_WIDTH as u16,
                PointF32 { x: rx, y: ry },
                sci,
                &(
                    PIXEL_LOGO[sci * 3],
                    PIXEL_LOGO[sci * 3 + 2],
                    Color::Indexed(PIXEL_LOGO[sci * 3 + 1]),
                    Color::Reset,
                ),
                PointU16 {
                    x: spw as u16 / 2 - (PIXEL_LOGO_WIDTH as f32 / 2.0 * symw) as u16,
                    y: sph as u16 / 2 - (PIXEL_LOGO_HEIGHT as f32 / 2.0 * symh) as u16,
                },
                false,
                false, // Logo uses Sprite characters (8×8)
            );
            let fc = Color::Indexed(PIXEL_LOGO[sci * 3 + 1]).get_rgba();

            let randadj = 12 - (rd.rand() % 24) as i32;
            let sg = LOGO_FRAME as u8 / 3;
            let r: u8;
            let g: u8;
            let b: u8;
            let a: u8;
            if stage <= sg as u32 {
                r = (stage as u8).saturating_mul(10);
                g = (stage as u8).saturating_mul(10);
                b = (stage as u8).saturating_mul(10);
                a = 255;
                s2.x += randadj;
            } else if stage <= sg as u32 * 2 {
                r = fc.0;
                g = fc.1;
                b = fc.2;
                a = 255;
            } else {
                let cc = (stage as u8 - sg * 2).saturating_mul(10);
                r = fc.0.saturating_sub(cc);
                g = fc.1.saturating_sub(cc);
                b = fc.2.saturating_sub(cc);
                a = 255;
            }
            f(&(r, g, b, a), s2, texidx, symidx);
        }
    }
}

// merge main buffer & pixel sprites to render buffer...
pub fn generate_render_buffer(
    cb: &Buffer,
    _pb: &Buffer,
    ps: &mut Vec<Sprites>,
    stage: u32,
    base: &mut AdapterBase,
) -> Vec<RenderCell> {
    let mut rbuf = vec![];
    let width = cb.area.width;
    let pz = PointI32 { x: 0, y: 0 };

    // render logo...
    if stage <= LOGO_FRAME {
        render_logo(
            base.gr.ratio_x,
            base.gr.ratio_y,
            base.gr.pixel_w,
            base.gr.pixel_h,
            &mut base.rd,
            stage,
            |fc, s2, texidx, symidx| {
                push_render_buffer(&mut rbuf, fc, &None, texidx, symidx, s2, 0.0, &pz);
            },
        );
        return rbuf;
    }

    let rx = base.gr.ratio_x;
    let ry = base.gr.ratio_y;
    let mut rfunc = |fc: &(u8, u8, u8, u8),
                     bc: &Option<(u8, u8, u8, u8)>,
                     s2: ARect,
                     texidx: usize,
                     symidx: usize| {
        push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
    };

    // render windows border, for sdl, winit and wgpu mode
    #[cfg(graphics_backend)]
    render_border(base.cell_w, base.cell_h, rx, ry, &mut rfunc);

    // render main buffer...
    // Use TUI characters (8×16) for UI components in graphics mode
    if stage > LOGO_FRAME {
        render_main_buffer(cb, width, rx, ry, false, true, &mut rfunc);
    }

    // render pixel_sprites...
    if stage > LOGO_FRAME {
        for item in ps {
            if item.is_pixel && !item.is_hidden {
                render_pixel_sprites(
                    item,
                    rx,
                    ry,
                    |fc, bc, s2, texidx, symidx, angle, ccp| {
                        push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, angle, &ccp);
                    },
                );
            }
        }
    }
    rbuf
}
