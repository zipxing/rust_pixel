// RustPixel
// copyright zipxing@hotmail.com 2022～2025

use crate::render::adapter::gl::{
    shader::GlShader,
    shader_source::{get_trans_fragment_src, VERTEX_SRC_TRANS},
    GlRender, GlRenderBase,
};
use glow::HasContext;

pub struct GlRenderTransition {
    pub base: GlRenderBase,
    pub shader_idx: usize,
    pub width: u32,
    pub height: u32,
    pub progress: f32,
}

impl GlRender for GlRenderTransition {
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

        Self {
            base,
            shader_idx: 0,
            width: 0,
            height: 0,
            progress: 0.0,
        }
    }

    fn get_base(&mut self) -> &mut GlRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, gl: &glow::Context, ver: &str) {
        let rbs = self.get_base();
        let fss = get_trans_fragment_src();
        for f in &fss {
            rbs.shader.push(GlShader::new(gl, ver, VERTEX_SRC_TRANS, f));
        }
    }

    fn create_buffer(&mut self, gl: &glow::Context) {
        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
        ];
        let indices: [u32; 6] = [0, 1, 2, 2, 3, 0];

        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vertex_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            let index_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            let program = self.base.shader[0].program;
            let pos_attrib = gl.get_attrib_location(program, "aPos").unwrap();
            let tex_attrib = gl.get_attrib_location(program, "aTexCoord").unwrap();
            gl.enable_vertex_attrib_array(pos_attrib);
            gl.enable_vertex_attrib_array(tex_attrib);

            gl.vertex_attrib_pointer_f32(pos_attrib, 2, glow::FLOAT, false, 16, 0);
            gl.vertex_attrib_pointer_f32(tex_attrib, 2, glow::FLOAT, false, 16, 8);

            gl.bind_vertex_array(None);

            self.base.vao = Some(vao);
            self.base.gl_buffers.clear();
            self.base.gl_buffers = vec![vertex_buffer, index_buffer];
        }
    }

    fn prepare_draw(&mut self, gl: &glow::Context) {
        self.base.shader[self.shader_idx].bind(gl);
        unsafe {
            gl.bind_vertex_array(self.base.vao);
            gl.viewport(0, 0, self.width as i32, self.height as i32);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.base.textures[0]));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.base.shader[self.shader_idx].program, "texture1")
                    .as_ref(),
                0,
            );

            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.base.textures[1]));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.base.shader[self.shader_idx].program, "texture2")
                    .as_ref(),
                1,
            );
            let lb = gl.get_uniform_location(self.base.shader[self.shader_idx].program, "progress");
            gl.uniform_1_f32(lb.as_ref(), self.progress);
        }
    }

    fn draw(&mut self, gl: &glow::Context) {
        unsafe {
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);
        }
    }

    fn cleanup(&mut self, gl: &glow::Context) {}
}

impl GlRenderTransition {
    pub fn set_texture(&mut self, w: u32, h: u32, tex1: glow::Texture, tex2: glow::Texture) {
        // textures...
        self.base.textures.clear();
        self.base.textures.push(tex1);
        self.base.textures.push(tex2);
        self.base.textures_binded = false;

        // width, height...
        self.width = w;
        self.height = h;
    }

    pub fn draw_trans(&mut self, gl: &glow::Context, shader_idx: usize, progress: f32) {
        self.shader_idx = shader_idx;
        self.progress = progress;
        self.prepare_draw(gl);
        self.draw(gl);
    }
}
