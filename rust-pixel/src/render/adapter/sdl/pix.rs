use crate::render::adapter::sdl::color::Color;
use crate::render::adapter::sdl::shader::Shader;
use crate::render::adapter::sdl::texture::Frame;
use crate::render::adapter::sdl::texture::Texture;
use crate::render::adapter::sdl::transform::Transform;
use glow::HasContext;
use sdl2::video::Window;
use sdl2::Sdl;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
pub enum RenderMode {
    None = -1,
    Surfaces = 0,
    PixCells = 1,
}

pub struct Pix {
    // OpenGL 上下文
    pub gl: glow::Context,

    // SDL 上下文和窗口
    pub sdl_context: Sdl,
    pub window: Window,

    // 着色器列表
    pub shaders: Vec<Shader>,

    // 纹理和精灵
    pub textures: Vec<Texture>,
    pub sprites: HashMap<String, Vec<Frame>>,

    // 变换栈
    pub transform_stack: Vec<Transform>,
    pub transform_at: usize,
    pub transform_dirty: bool,

    // 实例缓冲区
    pub instance_buffer: Vec<f32>,
    pub instance_buffer_capacity: usize,
    pub instance_buffer_at: isize,
    pub instance_count: usize,

    // 渲染模式
    pub render_mode: RenderMode,

    // OpenGL 缓冲区和顶点数组对象
    pub vao_cells: Option<glow::NativeVertexArray>,
    pub instances_vbo: Option<glow::NativeBuffer>,
    pub quad_vbo: Option<glow::NativeBuffer>,
    pub ubo: Option<glow::NativeBuffer>,

    // Uniform Buffer 内容
    pub ubo_contents: [f32; 12],

    // 当前状态
    pub current_shader: Option<usize>,
    pub current_shader_core: Option<usize>,
    pub current_texture_atlas: Option<glow::NativeTexture>,
    pub current_texture_surface: Option<glow::NativeTexture>,
    // pub surface: Option<Texture>,

    // 画布尺寸
    pub canvas_width: u32,
    pub canvas_height: u32,

    // 清除颜色
    pub clear_color: Color,
}

impl Pix {
    pub fn new(
        gl: glow::Context,
        sdl_context: Sdl,
        window: Window,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Self {
        Self {
            gl,
            sdl_context,
            window,
            shaders: Vec::new(),
            textures: Vec::new(),
            sprites: HashMap::new(),
            transform_stack: vec![Transform::new_with_values(
                1.0,
                0.0,
                0.0,
                0.0,
                -1.0,
                canvas_height as f32,
            )],
            transform_at: 0,
            transform_dirty: true,

            instance_buffer: Vec::new(),
            instance_buffer_capacity: 1024,
            instance_buffer_at: -1,
            instance_count: 0,

            render_mode: RenderMode::None,
            vao_cells: None,
            instances_vbo: None,
            quad_vbo: None,
            ubo: None,
            ubo_contents: [0.0; 12],
            current_shader: None,
            current_shader_core: None,
            current_texture_atlas: None,
            current_texture_surface: None,
            // surface: None,
            canvas_width,
            canvas_height,
            clear_color: Color::new(1.0, 1.0, 1.0, 0.0),
        }
    }

    pub fn init(&mut self) {
        unsafe {
            // create instance buffer...
            let instances_vbo = self.gl.create_buffer().unwrap();
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances_vbo));
            self.instance_buffer = vec![0.0; self.instance_buffer_capacity];
            self.gl.buffer_data_size(
                glow::ARRAY_BUFFER,
                (self.instance_buffer_capacity * std::mem::size_of::<f32>()) as i32,
                glow::DYNAMIC_DRAW,
            );
            self.instances_vbo = Some(instances_vbo);

            // create quad vbo...
            let quad_vbo = self.gl.create_buffer().unwrap();
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            let quad_vertices: [f32; 8] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0];
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                &quad_vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );
            self.quad_vbo = Some(quad_vbo);

            // create Uniform Buffer Object (UBO)
            let ubo = self.gl.create_buffer().unwrap();
            self.gl.bind_buffer(glow::UNIFORM_BUFFER, Some(ubo));
            self.gl
                .buffer_data_size(glow::UNIFORM_BUFFER, 48, glow::DYNAMIC_DRAW);
            self.gl
                .bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(ubo));
            self.ubo = Some(ubo);

            // create VAO
            let vao_cells = self.gl.create_vertex_array().unwrap();
            self.gl.bind_vertex_array(Some(vao_cells));
            self.vao_cells = Some(vao_cells);

            // set vertex attrib...
            self.gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(self.quad_vbo.unwrap()));
            self.gl.enable_vertex_attrib_array(0);
            self.gl
                .vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

            // instance vertex attrib
            self.gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(self.instances_vbo.unwrap()));
            let stride = 64;
            for i in 0..4 {
                self.gl.enable_vertex_attrib_array(1 + i);
                self.gl.vertex_attrib_pointer_f32(
                    1 + i,
                    4,
                    glow::FLOAT,
                    false,
                    stride,
                    (i * 16) as i32,
                );
                self.gl.vertex_attrib_divisor(1 + i, 1);
            }

            self.gl.bind_vertex_array(None);

            // set opengl state...
            self.gl.enable(glow::BLEND);
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );

            self.ubo_contents[8] = 1.0;
            self.ubo_contents[9] = 1.0;
            self.ubo_contents[10] = 1.0;
            self.ubo_contents[11] = 1.0;
        }
    }

    fn push_identity(&mut self) {
        self.transform_stack.push(Transform::new());
        self.transform_dirty = true;
    }

    pub fn push(&mut self) {
        let current_transform = self.transform_stack.last().unwrap().clone();
        self.transform_stack.push(current_transform);
    }

    pub fn pop(&mut self) {
        if self.transform_stack.len() > 1 {
            self.transform_stack.pop();
            self.transform_dirty = true;
        }
    }

    pub fn prepare_draw(&mut self, mode: RenderMode, size: usize, shader: Option<&mut Shader>) {
        if self.transform_dirty {
            self.flush();
            self.send_uniform_buffer();
        }

        if self.render_mode != mode {
            self.flush();
            self.render_mode = mode;
            if let Some(shader) = shader {
                shader.bind(&self.gl);
            } else {
                self.shaders[mode as usize].bind(&self.gl);
            }
        }

        if (self.instance_buffer_at as usize) + size >= self.instance_buffer_capacity {
            self.instance_buffer_capacity *= 2;
            self.instance_buffer.resize(self.instance_buffer_capacity, 0.0);

            unsafe {
                self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.instances_vbo.unwrap()));
                self.gl.buffer_data_size(
                    glow::ARRAY_BUFFER,
                    (self.instance_buffer_capacity * std::mem::size_of::<f32>()) as i32,
                    glow::DYNAMIC_DRAW,
                );
            }
        }

        self.instance_count += 1;
    }

    fn send_uniform_buffer(&mut self) {
        let transform = self.transform_stack.last().unwrap();
        self.ubo_contents[0] = transform.m00;
        self.ubo_contents[1] = transform.m10;
        self.ubo_contents[2] = transform.m20;
        self.ubo_contents[4] = transform.m01;
        self.ubo_contents[5] = transform.m11;
        self.ubo_contents[6] = transform.m21;
        self.ubo_contents[3] = self.canvas_width as f32;
        self.ubo_contents[7] = self.canvas_height as f32;

        unsafe {
            self.gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.ubo.unwrap()));
            self.gl.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                0,
                &self.ubo_contents.align_to::<u8>().1,
            );
        }

        self.transform_dirty = false;
    }

    pub fn clear(&mut self) {
        self.flush();

        unsafe {
            self.gl.clear_color(
                self.clear_color.r * self.ubo_contents[8],
                self.clear_color.g * self.ubo_contents[9],
                self.clear_color.b * self.ubo_contents[10],
                self.clear_color.a * self.ubo_contents[11],
            );
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn flush(&mut self) {
        if self.instance_count == 0 {
            return;
        }

        unsafe {
            self.gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(self.instances_vbo.unwrap()));
            self.gl.buffer_sub_data_u8_slice(
                glow::ARRAY_BUFFER,
                0,
                &self.instance_buffer[0..=(self.instance_buffer_at as usize)].align_to::<u8>().1,
            );

            match self.render_mode {
                RenderMode::PixCells => {
                    self.gl.bind_vertex_array(Some(self.vao_cells.unwrap()));
                    self.gl.draw_arrays_instanced(
                        glow::TRIANGLE_FAN,
                        0,
                        4,
                        self.instance_count as i32,
                    );
                }
                _ => {}
            }

            self.instance_buffer_at = -1;
            self.instance_count = 0;
        }
    }

    pub fn bind_texture_atlas(&mut self, texture: glow::NativeTexture) {
        if Some(texture) == self.current_texture_atlas {
            return;
        }

        self.flush();

        unsafe {
            self.gl.active_texture(glow::TEXTURE0);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        }

        self.current_texture_atlas = Some(texture);
    }
}
