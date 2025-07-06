// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Symbol Renderer Module
//! 
//! Extracted symbol rendering logic from WgpuPixelRender.
//! Handles vertex generation from RenderCell data and coordinate transformations.

use super::*;
use crate::render::adapter::RenderCell;
use crate::render::style::Color;

/// Symbol rendering helper for WgpuPixelRender
pub struct WgpuSymbolRenderer {
    canvas_width: f32,
    canvas_height: f32,
    ratio_x: f32,
    ratio_y: f32,
}

impl WgpuSymbolRenderer {
    /// Create new symbol renderer with canvas dimensions
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            canvas_width: canvas_width as f32,
            canvas_height: canvas_height as f32,
            ratio_x: 1.0,
            ratio_y: 1.0,
        }
    }
    
    /// Set the ratio parameters for coordinate transformation
    pub fn set_ratio(&mut self, ratio_x: f32, ratio_y: f32) {
        self.ratio_x = ratio_x;
        self.ratio_y = ratio_y;
    }

    /// Generate vertices from processed render cells
    /// 
    /// This is the main rendering interface that converts RenderCell data
    /// into GPU-ready vertex data with proper coordinate transformations.
    pub fn generate_vertices_from_render_cells(&self, render_cells: &[RenderCell]) -> Vec<super::pixel::WgpuVertex> {
        let mut vertices = Vec::new();
        
        // Constants for texture atlas layout
        // symbols.png is 1024x1024 pixels with 128x128 symbol positions
        // Each symbol occupies 8x8 pixels (1024÷128=8)
        const PIXELS_PER_SYMBOL: f32 = 8.0; // Each symbol is 8x8 pixels
        const TEXTURE_SIZE: f32 = 1024.0; // Total texture size in pixels
        
        // Canvas dimensions for NDC conversion
        let window_width = self.canvas_width;
        let window_height = self.canvas_height;
        
        // Get symbol dimensions from global constants (matches OpenGL version)
        let sym_width = crate::render::adapter::PIXEL_SYM_WIDTH.get().expect("lazylock init");
        let sym_height = crate::render::adapter::PIXEL_SYM_HEIGHT.get().expect("lazylock init");
        
        // Convert render cells to vertices
        for render_cell in render_cells {
            // Apply the same transformation chain as OpenGL version
            // The RenderCell coordinates contain PIXEL_SYM_WIDTH/HEIGHT offset which needs OpenGL-style transform
            
            // Use the RenderCell coordinates directly (they already contain proper positioning)
            // Apply simple transformation for position and size
            let base_x = render_cell.x;
            let base_y = render_cell.y;
            let width = render_cell.w as f32;
            let height = render_cell.h as f32;
            
            // Calculate basic quad bounds
            let left = base_x;
            let right = base_x + width;
            let top = base_y;
            let bottom = base_y + height;
            
            // Handle rotation around the center point if needed
            let (left_ndc, right_ndc, top_ndc, bottom_ndc) = if render_cell.angle != 0.0 {
                // Define the quad corners in world space
                let corners = vec![
                    (left, bottom),   // bottom-left
                    (right, bottom),  // bottom-right
                    (right, top),     // top-right
                    (left, top),      // top-left
                ];
                
                // Apply rotation around the center point (use the center from RenderCell)
                let center_x = render_cell.x + render_cell.cx;
                let center_y = render_cell.y + render_cell.cy;
                
                let rotated_corners: Vec<(f32, f32)> = corners.iter().map(|(x, y)| {
                    // Translate to origin (center point)
                    let translated_x = x - center_x;
                    let translated_y = y - center_y;
                    
                                         // Apply rotation (use negative angle to match OpenGL coordinate system)
                     let cos_angle = (-render_cell.angle).cos();
                     let sin_angle = (-render_cell.angle).sin();
                    
                    let rotated_x = translated_x * cos_angle - translated_y * sin_angle;
                    let rotated_y = translated_x * sin_angle + translated_y * cos_angle;
                    
                    // Translate back
                    (rotated_x + center_x, rotated_y + center_y)
                }).collect();
                
                // Convert to NDC
                let ndc_corners: Vec<(f32, f32)> = rotated_corners.iter().map(|(x, y)| {
                    let x_ndc = (x / window_width) * 2.0 - 1.0;
                    let y_ndc = 1.0 - (y / window_height) * 2.0;
                    (x_ndc, y_ndc)
                }).collect();
                
                (ndc_corners[0], ndc_corners[1], ndc_corners[2], ndc_corners[3])
            } else {
                // No rotation - simple conversion to NDC
                let left_ndc = (left / window_width) * 2.0 - 1.0;
                let right_ndc = (right / window_width) * 2.0 - 1.0;
                let top_ndc = 1.0 - (top / window_height) * 2.0;
                let bottom_ndc = 1.0 - (bottom / window_height) * 2.0;
                
                ((left_ndc, bottom_ndc), (right_ndc, bottom_ndc), (right_ndc, top_ndc), (left_ndc, top_ndc))
            };
            
            // First, render background if it exists (similar to GL mode)
            if let Some(bcolor) = render_cell.bcolor {
                // Use a solid block symbol for background (index 1280 like GL mode)
                // Calculate texture coordinates for background symbol (solid block)
                let bg_texsym = 1280; // Same as GL mode - should be a solid block symbol
                
                let symbols_per_row = 128;
                let bg_symbol_x = (bg_texsym % symbols_per_row) as f32;
                let bg_symbol_y = (bg_texsym / symbols_per_row) as f32;
                
                let bg_pixel_x = bg_symbol_x * PIXELS_PER_SYMBOL;
                let bg_pixel_y = bg_symbol_y * PIXELS_PER_SYMBOL;
                
                let bg_tex_left = bg_pixel_x / TEXTURE_SIZE;
                let bg_tex_right = (bg_pixel_x + PIXELS_PER_SYMBOL) / TEXTURE_SIZE;
                let bg_tex_top = bg_pixel_y / TEXTURE_SIZE;
                let bg_tex_bottom = (bg_pixel_y + PIXELS_PER_SYMBOL) / TEXTURE_SIZE;
                
                // Use background color
                let bg_color = [bcolor.0, bcolor.1, bcolor.2, bcolor.3];
                
                // Create background quad vertices (6 vertices for 2 triangles)
                vertices.push(super::pixel::WgpuVertex {
                    position: [left_ndc.0, left_ndc.1],  // bottom-left
                    tex_coords: [bg_tex_left, bg_tex_bottom],
                    color: bg_color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [right_ndc.0, right_ndc.1],  // bottom-right
                    tex_coords: [bg_tex_right, bg_tex_bottom],
                    color: bg_color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [top_ndc.0, top_ndc.1],  // top-right
                    tex_coords: [bg_tex_right, bg_tex_top],
                    color: bg_color,
                });
                
                // Second triangle for background
                vertices.push(super::pixel::WgpuVertex {
                    position: [left_ndc.0, left_ndc.1],  // bottom-left
                    tex_coords: [bg_tex_left, bg_tex_bottom],
                    color: bg_color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [top_ndc.0, top_ndc.1],  // top-right
                    tex_coords: [bg_tex_right, bg_tex_top],
                    color: bg_color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [bottom_ndc.0, bottom_ndc.1],  // top-left
                    tex_coords: [bg_tex_left, bg_tex_top],
                    color: bg_color,
                });
            }
            
            // Then render the foreground symbol
            let color = [render_cell.fcolor.0, render_cell.fcolor.1, render_cell.fcolor.2, render_cell.fcolor.3];
            
            // Calculate texture coordinates from texsym field using OpenGL-compatible method
            // texsym directly indexes into the 128x128 symbol grid
            let texsym = render_cell.texsym;
            
            // Calculate symbol grid position (matches OpenGL make_symbols_frame logic)
            let symbols_per_row = 128;
            let symbol_x = (texsym % symbols_per_row) as f32;
            let symbol_y = (texsym / symbols_per_row) as f32;
            
            // Convert to pixel coordinates (each symbol is 8x8 pixels in 1024x1024 texture)
            let pixel_x = symbol_x * PIXELS_PER_SYMBOL;
            let pixel_y = symbol_y * PIXELS_PER_SYMBOL;
            
            // Convert to normalized texture coordinates (0.0-1.0)
            // Match OpenGL uv calculation: uv_left = x / tex_width
            let tex_left = pixel_x / TEXTURE_SIZE;
            let tex_right = (pixel_x + PIXELS_PER_SYMBOL) / TEXTURE_SIZE;
            let tex_top = pixel_y / TEXTURE_SIZE;
            let tex_bottom = (pixel_y + PIXELS_PER_SYMBOL) / TEXTURE_SIZE;
            
            // Create foreground quad vertices (using triangle list, so need 6 vertices per quad)
            // Use the calculated (potentially rotated) corner positions
            // Corners are: [bottom-left, bottom-right, top-right, top-left]
            vertices.push(super::pixel::WgpuVertex {
                position: [left_ndc.0, left_ndc.1],  // bottom-left
                tex_coords: [tex_left, tex_bottom],
                color,
            });
            vertices.push(super::pixel::WgpuVertex {
                position: [right_ndc.0, right_ndc.1],  // bottom-right
                tex_coords: [tex_right, tex_bottom],
                color,
            });
            vertices.push(super::pixel::WgpuVertex {
                position: [top_ndc.0, top_ndc.1],  // top-right
                tex_coords: [tex_right, tex_top],
                color,
            });
            
            // Second triangle for foreground
            vertices.push(super::pixel::WgpuVertex {
                position: [left_ndc.0, left_ndc.1],  // bottom-left
                tex_coords: [tex_left, tex_bottom],
                color,
            });
            vertices.push(super::pixel::WgpuVertex {
                position: [top_ndc.0, top_ndc.1],  // top-right
                tex_coords: [tex_right, tex_top],
                color,
            });
            vertices.push(super::pixel::WgpuVertex {
                position: [bottom_ndc.0, bottom_ndc.1],  // top-left
                tex_coords: [tex_left, tex_top],
                color,
            });
        }
        
        vertices
    }

    /// Generate vertices from game buffer (alternative interface)
    /// 
    /// This method processes the raw game buffer and converts it to vertex data.
    /// Less efficient than render_cells method but provides compatibility.
    pub fn generate_vertices_from_buffer(&self, buffer: &crate::render::buffer::Buffer) -> Vec<super::pixel::WgpuVertex> {
        let mut vertices = Vec::new();
        
        // Buffer dimensions
        let buffer_width = buffer.area.width as f32;
        let buffer_height = buffer.area.height as f32;
        
        // Calculate scaling factors
        let scale_x = self.canvas_width / buffer_width;
        let scale_y = self.canvas_height / buffer_height;
        
        // Symbol texture atlas constants
        const SYMBOLS_PER_ROW: u32 = 16; // 16x16 grid in texture
        const SYMBOL_SIZE: f32 = 1.0 / 16.0; // Each symbol is 1/16 of texture
        
        // Process each cell in the buffer
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.get(x, y);
                
                // Skip empty cells (space character or transparent)
                if cell.symbol == " " || cell.fg == Color::Reset {
                    continue;
                }
                
                // Calculate screen position
                let screen_x = x as f32 * scale_x;
                let screen_y = y as f32 * scale_y;
                let cell_width = scale_x;
                let cell_height = scale_y;
                
                // Convert to NDC coordinates
                let left = (screen_x / self.canvas_width) * 2.0 - 1.0;
                let right = ((screen_x + cell_width) / self.canvas_width) * 2.0 - 1.0;
                let top = 1.0 - (screen_y / self.canvas_height) * 2.0;
                let bottom = 1.0 - ((screen_y + cell_height) / self.canvas_height) * 2.0;
                
                // Calculate texture coordinates based on character
                // Map ASCII characters to texture atlas positions
                let char_index = if let Some(ch) = cell.symbol.chars().next() {
                    ch as u32
                } else {
                    32 // space character
                };
                let tex_x = (char_index % SYMBOLS_PER_ROW) as f32;
                let tex_y = (char_index / SYMBOLS_PER_ROW) as f32;
                
                let tex_left = tex_x * SYMBOL_SIZE;
                let tex_right = (tex_x + 1.0) * SYMBOL_SIZE;
                let tex_top = tex_y * SYMBOL_SIZE;
                let tex_bottom = (tex_y + 1.0) * SYMBOL_SIZE;
                
                // Use cell color
                let (r, g, b, a) = cell.fg.get_rgba();
                let color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                
                // Create quad (6 vertices for 2 triangles)
                vertices.push(super::pixel::WgpuVertex {
                    position: [left, bottom],
                    tex_coords: [tex_left, tex_bottom],
                    color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [right, bottom],
                    tex_coords: [tex_right, tex_bottom],
                    color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [right, top],
                    tex_coords: [tex_right, tex_top],
                    color,
                });
                
                // Second triangle
                vertices.push(super::pixel::WgpuVertex {
                    position: [left, bottom],
                    tex_coords: [tex_left, tex_bottom],
                    color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [right, top],
                    tex_coords: [tex_right, tex_top],
                    color,
                });
                vertices.push(super::pixel::WgpuVertex {
                    position: [left, top],
                    tex_coords: [tex_left, tex_top],
                    color,
                });
            }
        }
        
        vertices
    }

    /// Update canvas dimensions (for window resize)
    pub fn update_canvas_size(&mut self, width: u32, height: u32) {
        self.canvas_width = width as f32;
        self.canvas_height = height as f32;
        // ratio parameters remain unchanged
    }
} 