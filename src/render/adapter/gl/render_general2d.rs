// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::gl::{
    shader::GlShader,
    shader_source::{GENERAL2D_FRAGMENT_SRC, GENERAL2D_VERTEX_SRC},
    GlRender, GlRenderBase,
};
use crate::render::pixel_renderer::{UnifiedColor, UnifiedTransform};
use glow::HasContext;
// use log::info;

pub struct GlRenderGeneral2d {
    pub base: GlRenderBase,
    pub area: [f32; 4],
    pub transform: UnifiedTransform,
    pub color: UnifiedColor,
}

impl GlRender for GlRenderGeneral2d {
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
            area: [0.0, 0.0, 0.0, 0.0],
            transform: UnifiedTransform::new(),
            color: UnifiedColor::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    fn get_base(&mut self) -> &mut GlRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, gl: &glow::Context, ver: &str) {
        let rbs = self.get_base();
        rbs.shader.clear();
        rbs.shader.push(GlShader::new(
            gl,
            ver,
            GENERAL2D_VERTEX_SRC,
            GENERAL2D_FRAGMENT_SRC,
        ));
    }

    fn create_buffer(&mut self, gl: &glow::Context) {
        let vertices: [f32; 16] = [
            // positions  // texCoords
            -1.0, -1.0, 0.0, 0.0, // 左下角
            1.0, -1.0, 1.0, 0.0, // 右下角
            1.0, 1.0, 1.0, 1.0, // 右上角
            -1.0, 1.0, 0.0, 1.0, // 左上角
        ];
        let indices: [u32; 6] = [0, 1, 2, 2, 3, 0];

        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            let ebo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
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
            self.base.gl_buffers = vec![vbo, ebo];
        }
    }

    fn prepare_draw(&mut self, gl: &glow::Context) {
        self.base.shader[0].bind(gl);
        unsafe {
            gl.bind_vertex_array(self.base.vao);

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.base.textures[0]));
            let tex_loc = gl.get_uniform_location(self.base.shader[0].program, "texture1");
            gl.uniform_1_i32(tex_loc.as_ref(), 0);

            let transform_loc = gl.get_uniform_location(self.base.shader[0].program, "transform");
            gl.uniform_matrix_4_f32_slice(
                transform_loc.as_ref(),
                false,
                &[
                    self.transform.m00,
                    self.transform.m01,
                    0.0,
                    0.0,
                    self.transform.m10,
                    self.transform.m11,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    self.transform.m20,
                    self.transform.m21,
                    0.0,
                    1.0,
                ],
            );

            let area_loc = gl.get_uniform_location(self.base.shader[0].program, "area");
            gl.uniform_4_f32_slice(area_loc.as_ref(), &self.area);

            let color_loc = gl.get_uniform_location(self.base.shader[0].program, "color");
            gl.uniform_4_f32_slice(
                color_loc.as_ref(),
                &[self.color.r, self.color.g, self.color.b, self.color.a],
            );
        }
    }

    fn draw(&mut self, gl: &glow::Context) {
        unsafe {
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);
        }
    }

    fn cleanup(&mut self, gl: &glow::Context) {}
}

impl GlRenderGeneral2d {
    pub fn set_texture(&mut self, gl: &glow::Context, tex1: glow::Texture) -> &mut Self {
        // textures...
        self.base.textures.clear();
        self.base.textures.push(tex1);
        self.base.textures_binded = false;
        self
    }

    pub fn set_area(&mut self, area: &[f32; 4]) -> &mut Self {
        self.area = *area;
        self
    }

    pub fn set_transform(&mut self, transform: &UnifiedTransform) -> &mut Self {
        self.transform = *transform;
        self
    }

    pub fn set_color(&mut self, color: &UnifiedColor) -> &mut Self {
        self.color = *color;
        self
    }
}
