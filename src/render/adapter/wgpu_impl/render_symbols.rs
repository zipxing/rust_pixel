// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::{
    wgpu_impl::{
        color::WgpuColor,
        shader::WgpuShader,
        shader_source::{VERTEX_SRC_SYMBOLS, FRAGMENT_SRC_SYMBOLS},
        texture::{WgpuTexture, WgpuCell},
        transform::WgpuTransform,
        WgpuRender, WgpuRenderBase,
    },
    RenderCell, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
};
use log::info;
use wgpu::util::DeviceExt;

pub struct WgpuRenderSymbols {
    pub base: WgpuRenderBase,
    instance_buffer: Vec<f32>,
    instance_buffer_capacity: usize,
    instance_count: usize,
    pub symbols: Vec<WgpuCell>,
    pub transform_stack: WgpuTransform,
    pub transform_dirty: bool,
    
    // WGPU specific
    vertex_buffer: Option<wgpu::Buffer>,
    instance_buffer_gpu: Option<wgpu::Buffer>,
    uniform_buffer: Option<wgpu::Buffer>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    uniform_bind_group: Option<wgpu::BindGroup>,
    texture_bind_group: Option<wgpu::BindGroup>,
}

impl WgpuRender for WgpuRenderSymbols {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        let base = WgpuRenderBase::new(0, canvas_width, canvas_height);
        
        Self {
            base,
            instance_buffer: vec![0.0; 1024],
            instance_buffer_capacity: 1024,
            instance_count: 0,
            symbols: vec![],
            transform_stack: WgpuTransform::orthographic(
                0.0,
                canvas_width as f32,
                canvas_height as f32,
                0.0,
            ),
            transform_dirty: true,
            
            vertex_buffer: None,
            instance_buffer_gpu: None,
            uniform_buffer: None,
            bind_group_layout: None,
            texture_bind_group_layout: None,
            uniform_bind_group: None,
            texture_bind_group: None,
        }
    }

    fn get_base(&mut self) -> &mut WgpuRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        let shader = WgpuShader::new(
            device,
            "symbols_shader",
            VERTEX_SRC_SYMBOLS,
            FRAGMENT_SRC_SYMBOLS,
        );

        // Create bind group layouts
        self.bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("symbols_uniform_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        }));

        self.texture_bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("symbols_texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
        }));

        // Create render pipeline
        let vertex_layout = &[
            wgpu::VertexBufferLayout {
                array_stride: 8 * 4, // 8 floats * 4 bytes
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2, // position
                    },
                    wgpu::VertexAttribute {
                        offset: 2 * 4,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2, // tex_coords
                    },
                    wgpu::VertexAttribute {
                        offset: 4 * 4,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x2, // uv_offset
                    },
                    wgpu::VertexAttribute {
                        offset: 6 * 4,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x2, // uv_size
                    },
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: 12 * 4, // 12 floats * 4 bytes
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 4,
                        format: wgpu::VertexFormat::Float32x4, // transform
                    },
                    wgpu::VertexAttribute {
                        offset: 4 * 4,
                        shader_location: 5,
                        format: wgpu::VertexFormat::Float32x4, // color
                    },
                ],
            },
        ];

        let bind_group_layouts = &[
            self.bind_group_layout.as_ref().unwrap(),
            self.texture_bind_group_layout.as_ref().unwrap(),
        ];

        let pipeline = shader.create_render_pipeline(
            device,
            format,
            vertex_layout,
            bind_group_layouts,
            Some("symbols_render_pipeline"),
        );

        self.base.render_pipeline = Some(pipeline);
        self.base.shader = Some(shader);
    }

    fn create_buffer(&mut self, device: &wgpu::Device) {
        // Create vertex buffer for quad
        let quad_vertices: [f32; 32] = [
            // position, tex_coords, uv_offset, uv_size
            0.0, 0.0,   0.0, 0.0,   0.0, 0.0,   1.0, 1.0,
            0.0, 1.0,   0.0, 1.0,   0.0, 0.0,   1.0, 1.0,
            1.0, 1.0,   1.0, 1.0,   0.0, 0.0,   1.0, 1.0,
            1.0, 0.0,   1.0, 0.0,   0.0, 0.0,   1.0, 1.0,
        ];

        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("symbols_vertex_buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        // Create instance buffer
        self.instance_buffer_gpu = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("symbols_instance_buffer"),
            size: (self.instance_buffer_capacity * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        // Create uniform buffer
        self.uniform_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("symbols_uniform_buffer"),
            size: 64, // 4x4 matrix
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.base.buffers.push(self.vertex_buffer.as_ref().unwrap().clone());
        self.base.buffers.push(self.instance_buffer_gpu.as_ref().unwrap().clone());
        self.base.buffers.push(self.uniform_buffer.as_ref().unwrap().clone());
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.transform_dirty {
            self.send_uniform_buffer(device, queue);
        }

        if self.instance_count > 0 {
            // Update instance buffer
            queue.write_buffer(
                self.instance_buffer_gpu.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&self.instance_buffer[0..self.instance_count * 12]),
            );
        }
    }

    fn draw(&mut self, render_pass: &mut wgpu::RenderPass) {
        if self.instance_count == 0 {
            return;
        }

        if let Some(pipeline) = &self.base.render_pipeline {
            render_pass.set_pipeline(pipeline);
            
            if let Some(uniform_bind_group) = &self.uniform_bind_group {
                render_pass.set_bind_group(0, uniform_bind_group, &[]);
            }
            
            if let Some(texture_bind_group) = &self.texture_bind_group {
                render_pass.set_bind_group(1, texture_bind_group, &[]);
            }
            
            if let Some(vertex_buffer) = &self.vertex_buffer {
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            }
            
            if let Some(instance_buffer) = &self.instance_buffer_gpu {
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
            }
            
            render_pass.draw(0..4, 0..self.instance_count as u32);
        }

        self.instance_count = 0;
    }

    fn cleanup(&mut self, device: &wgpu::Device) {
        // Cleanup resources if needed
    }
}

impl WgpuRenderSymbols {
    pub fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texw: u32, texh: u32, texdata: &[u8]) {
        let texture = WgpuTexture::new(
            device,
            queue,
            texw,
            texh,
            texdata,
            Some("symbols_texture"),
        ).unwrap();

        // Create texture bind group
        if let Some(layout) = &self.texture_bind_group_layout {
            self.texture_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(texture.get_sampler()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(texture.get_view()),
                    },
                ],
                label: Some("symbols_texture_bind_group"),
            }));
        }

        // Create symbols from texture
        let th = (texh as f32 / PIXEL_SYM_HEIGHT.get().expect("lazylock init")) as usize;
        let tw = (texw as f32 / PIXEL_SYM_WIDTH.get().expect("lazylock init")) as usize;
        
        for i in 0..th {
            for j in 0..tw {
                let symbol = self.make_symbols_frame(
                    &texture,
                    j as f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init"),
                    i as f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
                );
                self.symbols.push(symbol);
            }
        }
        
        info!("symbols len...{}, texh={} texw={} th={} tw={}", self.symbols.len(), texh, texw, th, tw);
        
        self.base.textures.push(texture.texture);
        self.base.texture_views.push(texture.view);
        self.base.samplers.push(texture.sampler);
    }

    fn send_uniform_buffer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let transform_matrix = self.transform_stack.to_array();
        
        if let Some(uniform_buffer) = &self.uniform_buffer {
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&transform_matrix));
        }

        // Create uniform bind group if not exists
        if self.uniform_bind_group.is_none() {
            if let (Some(layout), Some(buffer)) = (&self.bind_group_layout, &self.uniform_buffer) {
                self.uniform_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                    label: Some("symbols_uniform_bind_group"),
                }));
            }
        }

        self.transform_dirty = false;
    }

    pub fn render_rbuf(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
        rbuf: &[RenderCell],
        ratio_x: f32,
        ratio_y: f32,
    ) {
        for r in rbuf {
            let mut transform = WgpuTransform::new();

            transform.translate(
                r.x + r.cx - PIXEL_SYM_WIDTH.get().expect("lazylock init"),
                r.y + r.cy - PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
            );
            if r.angle != 0.0 {
                transform.rotate(r.angle);
            }
            transform.translate(
                -r.cx + PIXEL_SYM_WIDTH.get().expect("lazylock init") / ratio_x,
                -r.cy + PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ratio_y,
            );
            transform.scale(1.0 / ratio_x, 1.0 / ratio_y);

            if let Some(b) = r.bcolor {
                let back_color = WgpuColor::new(b.0, b.1, b.2, b.3);
                self.draw_symbol(1280, &transform, &back_color);
            }

            let color = WgpuColor::new(r.fcolor.0, r.fcolor.1, r.fcolor.2, r.fcolor.3);
            self.draw_symbol(r.texsym, &transform, &color);
        }
        
        self.draw(render_pass);
    }

    fn draw_symbol(&mut self, sym: usize, transform: &WgpuTransform, color: &WgpuColor) {
        if sym >= self.symbols.len() {
            return;
        }

        let frame = &self.symbols[sym];
        
        // Ensure we have enough space in instance buffer
        if (self.instance_count + 1) * 12 > self.instance_buffer_capacity {
            self.instance_buffer_capacity *= 2;
            self.instance_buffer.resize(self.instance_buffer_capacity, 0.0);
        }

        let base_idx = self.instance_count * 12;
        
        // Transform data (4 floats)
        self.instance_buffer[base_idx] = transform.m00 * frame.width;
        self.instance_buffer[base_idx + 1] = transform.m10 * frame.height;
        self.instance_buffer[base_idx + 2] = transform.m01 * frame.width;
        self.instance_buffer[base_idx + 3] = transform.m11 * frame.height;
        
        // Color data (4 floats)
        self.instance_buffer[base_idx + 4] = color.r;
        self.instance_buffer[base_idx + 5] = color.g;
        self.instance_buffer[base_idx + 6] = color.b;
        self.instance_buffer[base_idx + 7] = color.a;
        
        // UV data (4 floats)
        self.instance_buffer[base_idx + 8] = frame.uv_left;
        self.instance_buffer[base_idx + 9] = frame.uv_top;
        self.instance_buffer[base_idx + 10] = frame.uv_width;
        self.instance_buffer[base_idx + 11] = frame.uv_height;

        self.instance_count += 1;
    }

    fn make_symbols_frame(&mut self, sheet: &WgpuTexture, x: f32, y: f32) -> WgpuCell {
        let origin_x = 1.0;
        let origin_y = 1.0;
        let tex_width = sheet.width as f32;
        let tex_height = sheet.height as f32;

        let uv_left = x / tex_width;
        let uv_top = y / tex_height;
        let uv_width = PIXEL_SYM_WIDTH.get().expect("lazylock init") / tex_width;
        let uv_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / tex_height;

        WgpuCell::new(
            sheet.texture.clone(),
            sheet.view.clone(),
            *PIXEL_SYM_WIDTH.get().expect("lazylock init"),
            *PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
            origin_x,
            origin_y,
            uv_left,
            uv_top,
            uv_width,
            uv_height,
        )
    }
} 