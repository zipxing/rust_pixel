// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::gl::{
    color::GlColor,
    shader::GlShader,
    shader_source::{FRAGMENT_SRC_SYMBOLS, VERTEX_SRC_SYMBOLS},
    texture::{GlCell, GlTexture},
    transform::GlTransform,
    GlRender, GlRenderBase,
};
use crate::render::adapter::{RenderCell, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
use glow::HasContext;
use log::info;

pub struct GlRenderSymbols {
    pub base: GlRenderBase,
    instance_buffer: Vec<f32>,
    instance_buffer_capacity: usize,
    instance_buffer_at: isize,
    instance_count: usize,
    ubo_contents: [f32; 12],
    pub symbols: Vec<GlCell>,
    pub transform_stack: GlTransform,
    pub transform_dirty: bool,
}

impl GlRender for GlRenderSymbols {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        let base = GlRenderBase {
            id: 0,
            shader: vec![],
            shader_binded: false,
            vao: None,
            gl_buffers: vec![],
            textures: vec![],
            textures_binded: false,
            canvas_width,
            canvas_height,
        };
        let mut ubo_contents = [0.0f32; 12];
        ubo_contents[8] = 1.0;
        ubo_contents[9] = 1.0;
        ubo_contents[10] = 1.0;
        ubo_contents[11] = 1.0;

        Self {
            base,
            instance_buffer: vec![0.0; 1024],
            instance_buffer_capacity: 1024,
            instance_buffer_at: -1,
            instance_count: 0,
            ubo_contents,
            symbols: vec![],
            transform_stack: GlTransform::new_with_values(
                1.0,
                0.0,
                0.0,
                0.0,
                -1.0,
                canvas_height as f32,
            ),
            transform_dirty: true,
        }
    }

    fn get_base(&mut self) -> &mut GlRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, gl: &glow::Context, ver: &str) {
        let rbs = self.get_base();
        rbs.shader.push(GlShader::new(
            gl,
            ver,
            VERTEX_SRC_SYMBOLS,
            FRAGMENT_SRC_SYMBOLS,
        ));
    }

    fn create_buffer(&mut self, gl: &glow::Context) {
        unsafe {
            let vao_symbolss = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao_symbolss));

            let instances_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances_vbo));
            let instance_buffer_capacity = 1024;
            gl.buffer_data_size(
                glow::ARRAY_BUFFER,
                (instance_buffer_capacity * std::mem::size_of::<f32>()) as i32,
                glow::DYNAMIC_DRAW,
            );

            let quad_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            let quad_vertices: [f32; 8] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0];
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                quad_vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            let ubo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(ubo));
            gl.buffer_data_size(glow::UNIFORM_BUFFER, 48, glow::DYNAMIC_DRAW);
            gl.bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(ubo));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances_vbo));

            let stride = 64;

            // Attribute 1
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, stride, 0);
            gl.vertex_attrib_divisor(1, 1);

            // Attribute 2
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 16);
            gl.vertex_attrib_divisor(2, 1);

            // Attribute 3
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, stride, 32);
            gl.vertex_attrib_divisor(3, 1);

            // Attribute 4 (color)
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 4, glow::FLOAT, false, stride, 48);
            gl.vertex_attrib_divisor(4, 1);

            gl.bind_vertex_array(None);

            self.base.vao = Some(vao_symbolss);
            self.base.gl_buffers.clear();
            self.base.gl_buffers = vec![instances_vbo, quad_vbo, ubo];
        }
    }

    fn prepare_draw(&mut self, gl: &glow::Context) {
        let size = 16u32;

        if !self.base.textures_binded {
            unsafe {
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.base.textures[0]));
            }
            self.base.textures_binded = true;
        }

        if self.transform_dirty {
            self.draw(gl);
            self.send_uniform_buffer(gl);
        }

        if !self.base.shader_binded {
            self.draw(gl);
            self.base.shader[0].bind(gl);
            self.base.shader_binded = true;
        }

        if (self.instance_buffer_at + size as isize) as usize >= self.instance_buffer_capacity {
            self.instance_buffer_capacity *= 2;
            self.instance_buffer
                .resize(self.instance_buffer_capacity, 0.0);

            unsafe {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.base.gl_buffers[0]));
                gl.buffer_data_size(
                    glow::ARRAY_BUFFER,
                    (self.instance_buffer_capacity * std::mem::size_of::<f32>()) as i32,
                    glow::DYNAMIC_DRAW,
                );
            }
        }

        self.instance_count += 1;
    }

    fn draw(&mut self, gl: &glow::Context) {
        if self.instance_count == 0 {
            return;
        }

        unsafe {
            // instances_vbo
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.base.gl_buffers[0]));
            gl.buffer_sub_data_u8_slice(
                glow::ARRAY_BUFFER,
                0,
                self.instance_buffer[0..=(self.instance_buffer_at as usize)]
                    .align_to::<u8>()
                    .1,
            );

            gl.bind_vertex_array(self.base.vao);
            gl.draw_arrays_instanced(glow::TRIANGLE_FAN, 0, 4, self.instance_count as i32);

            self.instance_buffer_at = -1;
            self.instance_count = 0;
            self.base.shader_binded = false;
            self.base.textures_binded = false;
        }
    }

    fn cleanup(&mut self, gl: &glow::Context) {}
}

impl GlRenderSymbols {
    pub fn load_texture(&mut self, gl: &glow::Context, texw: i32, texh: i32, texdata: &[u8]) {
        let mut sprite_sheet = GlTexture::new(gl, texw, texh, texdata).unwrap();
        sprite_sheet.bind(gl);
        let th = (texh as f32 / PIXEL_SYM_HEIGHT.get().expect("lazylock init")) as usize;
        let tw = (texw as f32 / PIXEL_SYM_WIDTH.get().expect("lazylock init")) as usize;
        for i in 0..th {
            for j in 0..tw {
                let symbol = self.make_symbols_frame(
                    &mut sprite_sheet,
                    j as f32 * (PIXEL_SYM_WIDTH.get().expect("lazylock init")),
                    i as f32 * (PIXEL_SYM_HEIGHT.get().expect("lazylock init")),
                );
                self.symbols.push(symbol);
            }
        }
        info!(
            "symbols len...{}, texh={} texw={} th={} tw={}",
            self.symbols.len(),
            texh,
            texw,
            th,
            tw
        );
        self.base.textures.clear();
        self.base.textures.push(self.symbols[0].texture);
        self.base.textures_binded = false;
    }

    fn send_uniform_buffer(&mut self, gl: &glow::Context) {
        let transform = self.transform_stack;
        self.ubo_contents[0] = transform.m00;
        self.ubo_contents[1] = transform.m10;
        self.ubo_contents[2] = transform.m20;
        self.ubo_contents[4] = transform.m01;
        self.ubo_contents[5] = transform.m11;
        self.ubo_contents[6] = transform.m21;
        self.ubo_contents[3] = self.base.canvas_width as f32;
        self.ubo_contents[7] = self.base.canvas_height as f32;

        unsafe {
            // ubo
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.base.gl_buffers[2]));
            gl.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                0,
                self.ubo_contents.align_to::<u8>().1,
            );
        }

        self.transform_dirty = false;
    }

    // fill instance buffer...
    fn draw_symbol(
        &mut self,
        gl: &glow::Context,
        sym: usize,
        transform: &GlTransform,
        color: &GlColor,
    ) {
        self.prepare_draw(gl);
        let frame = &self.symbols[sym];
        let instance_buffer = &mut self.instance_buffer;

        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.origin_x;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.origin_y;

        // UV attributes
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_left;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_top;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_width;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_height;

        // Transform attributes
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m00 * frame.width;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m10 * frame.height;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m01 * frame.width;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m11 * frame.height;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m20;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m21;

        // Color
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.r;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.g;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.b;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.a;
    }

    pub fn render_rbuf(
        &mut self,
        gl: &glow::Context,
        rbuf: &[RenderCell],
        ratio_x: f32,
        ratio_y: f32,
    ) {
        // info!("ratiox....{} ratioy....{}", ratio_x, ratio_y);
        for r in rbuf {
            let mut transform = GlTransform::new();

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
                let back_color = GlColor::new(b.0, b.1, b.2, b.3);
                // fill instance buffer for opengl instance rendering
                self.draw_symbol(gl, 1280, &transform, &back_color);
            }

            let color = GlColor::new(r.fcolor.0, r.fcolor.1, r.fcolor.2, r.fcolor.3);
            // fill instance buffer for opengl instance rendering
            self.draw_symbol(gl, r.texsym, &transform, &color);
        }
        self.draw(gl);
    }

    fn make_symbols_frame(&mut self, sheet: &mut GlTexture, x: f32, y: f32) -> GlCell {
        let origin_x = 1.0;
        let origin_y = 1.0;
        let tex_width = sheet.width as f32;
        let tex_height = sheet.height as f32;

        let uv_left = x / tex_width;
        let uv_top = y / tex_height;
        let uv_width = PIXEL_SYM_WIDTH.get().expect("lazylock init") / tex_width;
        let uv_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / tex_height;

        GlCell {
            texture: sheet.texture,
            width: *PIXEL_SYM_WIDTH.get().expect("lazylock init"),
            height: *PIXEL_SYM_HEIGHT.get().expect("lazylock init"),
            origin_x,
            origin_y,
            uv_left,
            uv_top,
            uv_width,
            uv_height,
        }
    }
}
