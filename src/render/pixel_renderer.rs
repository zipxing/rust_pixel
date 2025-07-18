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

use std::any::Any;

// Conditional imports based on feature flags
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
use glow;

#[cfg(feature = "wgpu")]
use wgpu;

/// Unified render context for different graphics backends
///
/// This enum provides a type-safe way to pass backend-specific context
/// information to the unified pixel renderer interface.
pub enum RenderContext<'a> {
    /// OpenGL rendering context
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    OpenGL {
        gl: &'a glow::Context,
    },
    /// WGPU rendering context  
    #[cfg(feature = "wgpu")]
    Wgpu {
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        encoder: &'a mut wgpu::CommandEncoder,
        view: Option<&'a wgpu::TextureView>,
    },
}

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

// /// Unified Pixel Renderer Interface
// ///
// /// This trait provides a unified interface for all graphics mode pixel renderers,
// /// abstracting over OpenGL and WGPU backends while maintaining type safety and performance.
// ///
// /// ## Design Principles
// /// - **Backend Agnostic**: Same interface works with OpenGL and WGPU
// /// - **Type Safe**: Use associated types for backend-specific data
// /// - **Performance**: Zero-cost abstractions where possible
// /// - **Extensible**: Easy to add new backends or rendering features
// ///
// /// ## Typical Usage
// /// ```rust,ignore
// /// // Usage in adapter (pseudo-code)
// /// fn draw_render_textures_to_screen(&mut self) {
// ///     let renderer = self.get_pixel_renderer();
// ///     
// ///     // RT2 - main buffer (full screen)
// ///     if !renderer.get_render_texture_hidden(2) {
// ///         let transform = UnifiedTransform::new();
// ///         let color = UnifiedColor::white();
// ///         renderer.render_texture_to_screen(&context, 2, [0.0, 0.0, 1.0, 1.0], &transform, &color)?;
// ///     }
// ///     
// ///     // RT3 - transition effects (scaled)
// ///     if !renderer.get_render_texture_hidden(3) {
// ///         let mut transform = UnifiedTransform::new();
// ///         transform.scale(pw / pcw, ph / pch);
// ///         let color = UnifiedColor::white();
// ///         renderer.render_texture_to_screen(&context, 3, area, &transform, &color)?;
// ///     }
// /// }
// /// ```
// pub trait PixelRenderer {
//     /// Get canvas dimensions
//     fn get_canvas_size(&self) -> (u32, u32);
    
//     /// Render texture to screen using General2D pipeline
//     ///
//     /// This method renders a render texture to the current render target
//     /// with specified area mapping, transformation, and color modulation.
//     /// Each implementation manages its own rendering context internally.
//     ///
//     /// # Parameters
//     /// - `rtidx`: Render texture index (0-3)
//     /// - `area`: Texture area mapping [x, y, width, height] in texture coordinates (0.0-1.0)
//     /// - `transform`: 2D transformation matrix
//     /// - `color`: Color modulation (1.0, 1.0, 1.0, 1.0 = no change)
//     ///
//     /// # Returns
//     /// Result indicating success or rendering error
//     fn render_texture_to_screen(
//         &mut self,
//         rtidx: usize,
//         area: [f32; 4],
//         transform: &UnifiedTransform,
//         color: &UnifiedColor,
//     ) -> Result<(), String>;
    
//     /// Render transition frame with effects
//     ///
//     /// Applies transition effects between render textures using specialized shaders.
//     /// Each implementation manages its own rendering context internally.
//     ///
//     /// # Parameters
//     /// - `shader_idx`: Transition shader index (0-6 for different effects)
//     /// - `progress`: Transition progress (0.0 = start, 1.0 = end)
//     ///
//     /// # Returns
//     /// Result indicating success or rendering error
//     fn render_transition_frame(
//         &mut self,
//         shader_idx: usize,
//         progress: f32,
//     ) -> Result<(), String>;
    
//     /// Get render texture hidden state
//     ///
//     /// # Parameters
//     /// - `rtidx`: Render texture index (0-3)
//     ///
//     /// # Returns
//     /// True if the render texture is hidden from final composition
//     fn get_render_texture_hidden(&self, rtidx: usize) -> bool;
    
//     /// Set render texture hidden state
//     ///
//     /// Controls whether a render texture participates in final screen composition.
//     /// Hidden render textures are not drawn during screen composition.
//     ///
//     /// # Parameters
//     /// - `rtidx`: Render texture index (0-3)
//     /// - `hidden`: True to hide, false to show
//     fn set_render_texture_hidden(&mut self, rtidx: usize, hidden: bool);
    
//     /// Render symbol buffer to specified render texture
//     ///
//     /// This method renders RenderCell data to a specific render texture using
//     /// the symbols rendering pipeline. Each implementation manages its own rendering context internally.
//     ///
//     /// # Parameters
//     /// - `rbuf`: RenderCell data array
//     /// - `rtidx`: Target render texture index
//     /// - `ratio_x`: Horizontal scaling ratio
//     /// - `ratio_y`: Vertical scaling ratio
//     ///
//     /// # Returns
//     /// Result indicating success or rendering error
//     fn render_symbols_to_texture(
//         &mut self,
//         rbuf: &[crate::render::graph::RenderCell],
//         rtidx: usize,
//         ratio_x: f32,
//         ratio_y: f32,
//     ) -> Result<(), String>;
    
//     /// Set clear color for render targets
//     ///
//     /// # Parameters
//     /// - `color`: Clear color
//     fn set_clear_color(&mut self, color: &UnifiedColor);
    
//     /// Clear current render target
//     ///
//     /// Each implementation manages its own rendering context internally.
//     fn clear(&mut self);
    
//     /// Bind render texture as current render target
//     ///
//     /// # Parameters
//     /// - `rtidx`: Render texture index (None for screen/default framebuffer)
//     fn bind_render_target(&mut self, rtidx: Option<usize>);
    
//     /// Get type-erased reference for downcasting
//     ///
//     /// This allows adapter code to downcast to specific renderer types
//     /// when backend-specific functionality is needed.
//     fn as_any(&mut self) -> &mut dyn Any;
// }

// // Note: Type conversion methods are implemented in the respective backend files
// // (gl/pixel.rs and wgpu/pixel.rs) to avoid complex conditional compilation issues. 
