// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! WGPU Symbol Renderer Module using Instanced Rendering
//! 
//! This module implements instanced rendering for symbols, exactly matching
//! the OpenGL version's behavior. Each symbol is drawn as an instance of
//! a base quad geometry with instance-specific transform and color data.

use crate::render::adapter::{RenderCell, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
use crate::render::graph::UnifiedTransform;

/// Symbol instance data for WGPU (matches OpenGL layout exactly)
/// This corresponds to the per-instance attributes in OpenGL version
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuSymbolInstance {
    /// a1: [origin_x, origin_y, uv_left, uv_top]
    pub a1: [f32; 4],
    /// a2: [uv_width, uv_height, m00*width, m10*height]  
    pub a2: [f32; 4],
    /// a3: [m01*width, m11*height, m20, m21]
    pub a3: [f32; 4],
    /// color: [r, g, b, a]
    pub color: [f32; 4],
}

unsafe impl bytemuck::Pod for WgpuSymbolInstance {}
unsafe impl bytemuck::Zeroable for WgpuSymbolInstance {}

/// Transform uniform data (matches OpenGL UBO layout)
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuTransformUniforms {
    /// tw: [m00, m10, m20, canvas_width] 
    pub tw: [f32; 4],
    /// th: [m01, m11, m21, canvas_height]
    pub th: [f32; 4],
    /// colorFilter: [r, g, b, a]
    pub color_filter: [f32; 4],
}

unsafe impl bytemuck::Pod for WgpuTransformUniforms {}
unsafe impl bytemuck::Zeroable for WgpuTransformUniforms {}

/// Base quad vertex for instanced rendering
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuQuadVertex {
    /// Vertex position in unit quad coordinates (0,0) to (1,1)
    pub position: [f32; 2],
}

unsafe impl bytemuck::Pod for WgpuQuadVertex {}
unsafe impl bytemuck::Zeroable for WgpuQuadVertex {}

/// Symbol frame data (equivalent to OpenGL GlCell)
#[derive(Clone, Debug)]
pub struct WgpuSymbolFrame {
    pub width: f32,
    pub height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub uv_left: f32,
    pub uv_top: f32,
    pub uv_width: f32,
    pub uv_height: f32,
}

/// WGPU Symbol Renderer using instanced rendering
/// 
/// This renderer exactly matches the OpenGL GlRenderSymbols behavior:
/// - Uses a single base quad geometry
/// - Each symbol is an instance with its own transform and UV data
/// - Performs the same complex transformation chain in the vertex shader
pub struct WgpuSymbolRenderer {
    canvas_width: f32,
    canvas_height: f32,
    
    /// Transform stack (equivalent to OpenGL transform_stack)
    transform_stack: UnifiedTransform,
    
    /// Symbol frames (equivalent to OpenGL symbols)
    symbols: Vec<WgpuSymbolFrame>,
    
    /// Current instance buffer data
    instance_buffer: Vec<WgpuSymbolInstance>,
    
    /// Instance count for current frame
    instance_count: usize,
    
    /// Max instances capacity
    max_instances: usize,
    
    /// Ratio parameters for coordinate transformation
    pub ratio_x: f32,
    pub ratio_y: f32,
}




impl WgpuSymbolRenderer {
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        // Initialize transform stack to match OpenGL version exactly:
        // UnifiedTransform::new_with_values(1.0, 0.0, 0.0, -1.0, 0.0, canvas_height as f32)
        let transform_stack = UnifiedTransform::new_with_values(
            1.0, 0.0,                    // m00, m01
            0.0, -1.0,                   // m10, m11  
            0.0, canvas_height as f32    // m20, m21
        );
        
        let max_instances = (canvas_width * canvas_height) as usize; // Conservative estimate
        
        Self {
            canvas_width: canvas_width as f32,
            canvas_height: canvas_height as f32,
            transform_stack,
            symbols: Vec::new(),
            instance_buffer: Vec::with_capacity(max_instances),
            instance_count: 0,
            max_instances,
            ratio_x: 1.0,
            ratio_y: 1.0,
        }
    }
    
    /// Load symbol texture and create symbol frames (equivalent to OpenGL load_texture)
    pub fn load_texture(&mut self, texw: i32, texh: i32, _texdata: &[u8]) {
        let sym_width = *PIXEL_SYM_WIDTH.get().expect("lazylock init");
        let sym_height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init");
        
        let th = (texh as f32 / sym_height) as usize;
        let tw = (texw as f32 / sym_width) as usize;
        
        self.symbols.clear();
        for i in 0..th {
            for j in 0..tw {
                let frame = self.make_symbols_frame(
                    texw as f32, texh as f32,
                    j as f32 * sym_width,
                    i as f32 * sym_height,
                );
                self.symbols.push(frame);
            }
        }
        
        // Symbols loaded successfully (debug output removed for performance)
    }
    
    /// Create symbol frame (equivalent to OpenGL make_symbols_frame)
    fn make_symbols_frame(&self, tex_width: f32, tex_height: f32, x: f32, y: f32) -> WgpuSymbolFrame {
        let sym_width = *PIXEL_SYM_WIDTH.get().expect("lazylock init");
        let sym_height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init");
        
        let origin_x = 1.0;
        let origin_y = 1.0;
        let uv_left = x / tex_width;
        let uv_top = y / tex_height;
        let uv_width = sym_width / tex_width;
        let uv_height = sym_height / tex_height;
        
        WgpuSymbolFrame {
            width: sym_width,
            height: sym_height,
            origin_x,
            origin_y,
            uv_left,
            uv_top,
            uv_width,
            uv_height,
        }
    }
    
    /// Generate instance data from render cells (equivalent to OpenGL render_rbuf)
    pub fn generate_instances_from_render_cells(&mut self, render_cells: &[RenderCell], ratio_x: f32, ratio_y: f32) {
        self.instance_buffer.clear();
        self.instance_count = 0;
        
        for r in render_cells {
            // Use the same transformation chain as OpenGL using UnifiedTransform methods
            let mut transform = self.transform_stack;
            let w = PIXEL_SYM_WIDTH.get().expect("lazylock init") / ratio_x;
            let h = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ratio_y;

            transform.translate(
                r.x + r.cx - w,
                r.y + r.cy - h,
            );
            if r.angle != 0.0 {
                transform.rotate(r.angle);
            }
            transform.translate(
                -r.cx + w,
                -r.cy + h,
            );
            transform.scale(1.0 / ratio_x, 1.0 / ratio_y);
            
            // Draw background if it exists
            if let Some(b) = r.bcolor {
                self.draw_symbol_instance(1280, &transform, [b.0, b.1, b.2, b.3]);
            }
            
            // Draw foreground symbol
            self.draw_symbol_instance(r.texsym, &transform, [r.fcolor.0, r.fcolor.1, r.fcolor.2, r.fcolor.3]);
        }
    }
    
    /// Add a symbol instance (equivalent to OpenGL draw_symbol)
    fn draw_symbol_instance(&mut self, sym: usize, transform: &UnifiedTransform, color: [f32; 4]) {
        if self.instance_count >= self.max_instances {
            // Instance limit reached (debug output removed for performance)
            return;
        }
        if sym >= self.symbols.len() {
            // Symbol index out of bounds (debug output removed for performance)
            return;
        }
        
        let frame = &self.symbols[sym];
        
        // Create instance data matching OpenGL layout exactly
        let instance = WgpuSymbolInstance {
            // a1: [origin_x, origin_y, uv_left, uv_top]
            a1: [frame.origin_x, frame.origin_y, frame.uv_left, frame.uv_top],
            
            // a2: [uv_width, uv_height, m00*width, m10*height]
            a2: [
                frame.uv_width, 
                frame.uv_height,
                transform.m00 * frame.width,
                transform.m10 * frame.height
            ],
            
            // a3: [m01*width, m11*height, m20, m21]
            a3: [
                transform.m01 * frame.width,
                transform.m11 * frame.height,
                transform.m20,
                transform.m21
            ],
            
            // color: [r, g, b, a]
            color,
        };
        
        self.instance_buffer.push(instance);
        self.instance_count += 1;
    }
    
    /// Get base quad vertices (equivalent to OpenGL quad vertices)
    pub fn get_base_quad_vertices() -> &'static [WgpuQuadVertex] {
        // Base quad vertices in unit coordinates (matches OpenGL TRIANGLE_FAN order)
        // OpenGL uses: [0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]
        &[
            WgpuQuadVertex { position: [0.0, 0.0] }, // Bottom-left
            WgpuQuadVertex { position: [0.0, 1.0] }, // Top-left  
            WgpuQuadVertex { position: [1.0, 1.0] }, // Top-right
            WgpuQuadVertex { position: [1.0, 0.0] }, // Bottom-right
        ]
    }
    
    /// Get base quad indices for triangle list rendering
    pub fn get_base_quad_indices() -> &'static [u16] {
        // Convert TRIANGLE_FAN to TRIANGLE_LIST: (0,1,2) and (2,3,0)
        &[0, 1, 2, 2, 3, 0]
    }
    
    /// Get current instance buffer data
    pub fn get_instance_buffer(&self) -> &[WgpuSymbolInstance] {
        &self.instance_buffer[..self.instance_count]
    }
    
    /// Get instance count
    pub fn get_instance_count(&self) -> u32 {
        self.instance_count as u32
    }
    
    /// Get transform uniforms (equivalent to OpenGL UBO)
    pub fn get_transform_uniforms(&self) -> WgpuTransformUniforms {
        WgpuTransformUniforms {
            // tw: [m00, m10, m20, canvas_width]
            tw: [
                self.transform_stack.m00,
                self.transform_stack.m10, 
                self.transform_stack.m20,
                self.canvas_width
            ],
            // th: [m01, m11, m21, canvas_height]
            th: [
                self.transform_stack.m01,
                self.transform_stack.m11,
                self.transform_stack.m21,
                self.canvas_height
            ],
            // colorFilter: [r, g, b, a] - default to white (no filtering)
            color_filter: [1.0, 1.0, 1.0, 1.0],
        }
    }
    
    /// Update canvas dimensions
    pub fn update_canvas_size(&mut self, width: u32, height: u32) {
        self.canvas_width = width as f32;
        self.canvas_height = height as f32;
        
        // Update transform stack canvas height (matches OpenGL behavior)
        self.transform_stack.m21 = height as f32;
    }
    
    /// Set ratio parameters for coordinate transformation
    pub fn set_ratio(&mut self, ratio_x: f32, ratio_y: f32) {
        self.ratio_x = ratio_x;
        self.ratio_y = ratio_y;
    }
    
    /// Generate vertices from buffer (legacy method for compatibility)
    /// This method exists for backward compatibility but is no longer used
    /// in the instanced rendering pipeline
    pub fn generate_vertices_from_buffer(&self, _buffer: &crate::render::buffer::Buffer) -> Vec<crate::render::adapter::wgpu::pixel::WgpuVertex> {
        // Return empty vector since we now use instanced rendering
        Vec::new()
    }
}

impl WgpuQuadVertex {
    /// Vertex buffer layout descriptor for base quad
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuQuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl WgpuSymbolInstance {
    /// Instance buffer layout descriptor
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuSymbolInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // a1: vec4<f32> at location 1
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // a2: vec4<f32> at location 2
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // a3: vec4<f32> at location 3
                wgpu::VertexAttribute {
                    offset: (2 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // color: vec4<f32> at location 4
                wgpu::VertexAttribute {
                    offset: (3 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
