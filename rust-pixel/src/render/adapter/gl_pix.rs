// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::gl_color::GlColor;
use crate::render::adapter::gl_shader::GlShader;
use crate::render::adapter::gl_texture::{GlCell, GlRenderTexture, GlTexture};
use crate::render::adapter::gl_transform::GlTransform;
use crate::render::adapter::{RenderCell, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
use glow::HasContext;
use log::info;

#[derive(Clone, Copy, PartialEq)]
pub enum GlRenderMode {
    None = -1,
    PixCells = 0,
    Transition = 1,
    General2D = 2,
}

pub struct GlPix {
    pub render_mode: GlRenderMode,

    pub shaders: Vec<GlShader>,
    pub render_textures: Vec<GlRenderTexture>,

    pub symbols: Vec<GlCell>,

    pub transform_stack: GlTransform,
    pub transform_dirty: bool,

    // symbols shader buffers...
    pub vao_symbols: glow::VertexArray,
    pub instances_vbo: glow::Buffer,
    pub instance_buffer: Vec<f32>,
    pub instance_buffer_capacity: usize,
    pub instance_buffer_at: isize,
    pub instance_count: usize,
    pub quad_vbo: glow::Buffer,
    pub ubo: glow::Buffer,
    pub ubo_contents: [f32; 12],

    // trans shader buffers...
    pub vao_trans: glow::VertexArray,
    pub vbo_trans: glow::Buffer,
    pub ebo_trans: glow::Buffer,

    // general2d shader buffers...
    pub vao_general2d: glow::VertexArray,
    pub vbo_general2d: glow::Buffer,
    pub ebo_general2d: glow::Buffer,

    pub current_texture_atlas: Option<glow::Texture>,

    pub canvas_width: u32,
    pub canvas_height: u32,

    pub clear_color: GlColor,
}

impl GlPix {
    pub fn new(
        gl: &glow::Context,
        ver: &str,
        canvas_width: i32,
        canvas_height: i32,
        texw: i32,
        texh: i32,
        texdata: &[u8],
    ) -> Self {
        // symbolss shader...
        let vertex_shader_src = r#"
        precision mediump float;
        layout(location=0) in vec2 vertex;
        layout(location=1) in vec4 a1;
        layout(location=2) in vec4 a2;
        layout(location=3) in vec4 a3;
        layout(location=4) in vec4 color;
        layout(std140) uniform transform {
            vec4 tw;
            vec4 th;
            vec4 colorFilter;
        };
        out vec2 uv;
        out vec4 colorj;
        void main() {
            uv = a1.zw + vertex * a2.xy;
            vec2 transformed = (((vertex - a1.xy) * mat2(a2.zw, a3.xy) + a3.zw) * mat2(tw.xy, th.xy) + vec2(tw.z, th.z)) / vec2(tw.w, th.w) * 2.0;
            gl_Position = vec4(transformed - vec2(1.0, 1.0), 0.0, 1.0);
            colorj = color * colorFilter;
        }
        "#;

        let fragment_shader_src = r#"
        precision mediump float;
        uniform sampler2D source;
        layout(std140) uniform transform {
            vec4 tw;
            vec4 th;
            vec4 colorFilter;
        };
        in vec2 uv;
        in vec4 colorj;
        layout(location=0) out vec4 color;
        void main() {
            color = texture(source, uv) * colorj;
        }
        "#;

        // trans shader ...
        let vertex_shader_src2 = r#"
            precision mediump float;
            layout(location = 0) in vec2 aPos;
            layout(location = 1) in vec2 aTexCoord;
            out vec2 TexCoord;
            void main() {
                gl_Position = vec4(aPos, 0.0, 1.0);
                TexCoord = aTexCoord;
            }
        "#;
        let fs = r#"
            const float PI = 3.14159265358;

            vec4 transition (vec2 uv) {
                    float time = progress;
                    float stime = sin(time * PI / 2.);
                    float phase = time * PI * 3.0;
                    float y = (abs(cos(phase))) * (1.0 - stime);
                    float d = uv.y - y;
                    vec4 from = getFromColor(vec2(uv.x, uv.y + (1.0 - y)));
                    // vec4 from = getFromColor(uv);
                    vec4 to = getToColor(uv);
                    vec4 mc = mix( to, from, step(d, 0.0) );
                    return mc;
            }
        "#;

        let fragment_shader_src2 = &format!(
            r#"
            precision highp float;
            out vec4 FragColor;
            in vec2 TexCoord;
            uniform sampler2D texture1;
            uniform sampler2D texture2;
            uniform float progress;
            vec4 getFromColor(vec2 uv) {{ return texture(texture1, uv); }}
            vec4 getToColor(vec2 uv) {{ return texture(texture2, uv); }}
            {}
            void main() {{ FragColor =  transition(TexCoord); }}
            "#,
            fs
        );

        let vertex_shader_src3 = r#"
            precision mediump float;
    layout(location = 0) in vec2 aPos;        // 顶点坐标
    layout(location = 1) in vec2 aTexCoord;   // 纹理坐标

    out vec2 TexCoord;  // 传递给片段着色器的纹理坐标

    uniform mat4 transform;  // 变换矩阵
    uniform vec4 area;       // 纹理的采样区域 (x, y, width, height) [0.0, 1.0]

    void main()
    {
        // 纹理的 UV 坐标映射到指定区域
        TexCoord = vec2(
            mix(area.x, area.x + area.z, aTexCoord.x),
            mix(area.y, area.y + area.w, aTexCoord.y)
        );

        // 使用变换矩阵对顶点坐标进行变换
        gl_Position = transform * vec4(aPos, 0.0, 1.0);
    }
    "#;

        let fragment_shader_src3 = r#"
            precision mediump float;
    out vec4 FragColor;

    in vec2 TexCoord;

    uniform sampler2D texture1;  // 输入的纹理
    uniform vec4 color;          // 渲染颜色，包含透明度

    void main()
    {
        // 从纹理中采样颜色
        vec4 texColor = texture(texture1, TexCoord);

        // 应用颜色和透明度
        FragColor = texColor * color;
    }
    "#;
    info!("AAADDDWEE1111.........");

        let shader_symbols = GlShader::new(&gl, ver, vertex_shader_src, fragment_shader_src);
    info!("AAADDDWEE22222.........");
        let shader_trans = GlShader::new(&gl, ver, vertex_shader_src2, fragment_shader_src2);
    info!("AAADDDWEE33333.........");
        let shader_general2d = GlShader::new(&gl, ver, vertex_shader_src3, fragment_shader_src3);
    info!("AAADDDWEE44444.........");

        let (vao_trans, vbo_trans, ebo_trans) =
            unsafe { create_trans_buffers(&gl, shader_trans.program) };
        let (vao_general2d, vbo_general2d, ebo_general2d) =
            unsafe { create_general2d_buffers(&gl, shader_general2d.program) };
        let (vao_symbols, instances_vbo, quad_vbo, ubo) = unsafe { create_symbols_buffers(&gl) };

        let shaders = vec![shader_symbols, shader_trans, shader_general2d];

        // 初始化缓冲区
        unsafe {
            gl.enable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
        }

        let mut ubo_contents = [0.0f32; 12];
        ubo_contents[8] = 1.0;
        ubo_contents[9] = 1.0;
        ubo_contents[10] = 1.0;
        ubo_contents[11] = 1.0;

        let mut render_textures = vec![];
        // create 4 render texture for gl transition...
        for _i in 0..4 {
            let rt = GlRenderTexture::new(gl, canvas_width as u32, canvas_height as u32).unwrap();
            render_textures.push(rt);
        }

        let mut s = Self {
            canvas_width: canvas_width as u32,
            canvas_height: canvas_height as u32,
            shaders,
            quad_vbo,
            instances_vbo,
            vao_symbols,
            ubo,
            ubo_contents,
            vao_trans,
            vbo_trans,
            ebo_trans,
            vao_general2d,
            vbo_general2d,
            ebo_general2d,
            transform_stack: GlTransform::new_with_values(
                1.0,
                0.0,
                0.0,
                0.0,
                -1.0,
                canvas_height as f32,
            ),
            transform_dirty: true,
            instance_buffer_capacity: 1024,
            instance_buffer_at: -1,
            instance_buffer: vec![0.0; 1024],
            instance_count: 0,
            render_mode: GlRenderMode::None,
            current_texture_atlas: None,
            clear_color: GlColor::new(1.0, 1.0, 1.0, 0.0),
            symbols: vec![],
            render_textures,
        };

        s.set_clear_color(GlColor::new(0.0, 0.0, 0.0, 1.0));

        // init gl_symbols
        // for texture_path in texs {
        // info!("gl_pix load texture...{}", texture_path);
        // let img = image::open(texture_path).map_err(|e| e.to_string()).unwrap().to_rgba8();
        // let width = img.width();
        // let height = img.height();
        info!("aaaa1111111111111");

        let mut sprite_sheet = GlTexture::new(gl, texw, texh, texdata).unwrap();
        sprite_sheet.bind(gl);
        for i in 0..32 {
            for j in 0..32 {
                let symbol = s.make_symbols_frame(
                    &mut sprite_sheet,
                    j as f32 * (PIXEL_SYM_WIDTH + 1.0),
                    i as f32 * (PIXEL_SYM_HEIGHT + 1.0),
                    PIXEL_SYM_WIDTH,
                    PIXEL_SYM_HEIGHT,
                    8.0,
                    8.0,
                );
                s.symbols.push(symbol);
            }
        }
        // }
        s
    }

    pub fn prepare_draw_trans(&mut self, gl: &glow::Context) {
        unsafe {
            gl.bind_vertex_array(Some(self.vao_trans));
        }
    }

    pub fn prepare_render_symbols(&mut self, gl: &glow::Context, mode: GlRenderMode, size: usize) {
        if self.transform_dirty {
            self.flush(gl);
            self.send_uniform_buffer(gl);
        }

        if self.render_mode != mode {
            self.flush(gl);
            self.render_mode = mode;
            self.shaders[mode as usize].bind(gl);
        }

        if (self.instance_buffer_at + size as isize) as usize >= self.instance_buffer_capacity {
            self.instance_buffer_capacity *= 2;
            self.instance_buffer
                .resize(self.instance_buffer_capacity, 0.0);

            unsafe {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.instances_vbo));
                gl.buffer_data_size(
                    glow::ARRAY_BUFFER,
                    (self.instance_buffer_capacity * std::mem::size_of::<f32>()) as i32,
                    glow::DYNAMIC_DRAW,
                );
            }
        }

        self.instance_count += 1;
    }

    fn send_uniform_buffer(&mut self, gl: &glow::Context) {
        let transform = self.transform_stack;
        self.ubo_contents[0] = transform.m00;
        self.ubo_contents[1] = transform.m10;
        self.ubo_contents[2] = transform.m20;
        self.ubo_contents[4] = transform.m01;
        self.ubo_contents[5] = transform.m11;
        self.ubo_contents[6] = transform.m21;
        self.ubo_contents[3] = self.canvas_width as f32;
        self.ubo_contents[7] = self.canvas_height as f32;

        unsafe {
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.ubo));
            gl.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                0,
                &self.ubo_contents.align_to::<u8>().1,
            );
        }

        self.transform_dirty = false;
    }

    pub fn bind(&mut self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
        }
    }

    pub fn bind_render_texture(&mut self, gl: &glow::Context, idx: usize) {
        unsafe {
            gl.bind_framebuffer(
                glow::FRAMEBUFFER,
                Some(self.render_textures[idx].framebuffer),
            );
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
        }
    }

    pub fn clear(&mut self, gl: &glow::Context) {
        self.flush(gl);

        unsafe {
            gl.clear_color(
                self.clear_color.r * self.ubo_contents[8],
                self.clear_color.g * self.ubo_contents[9],
                self.clear_color.b * self.ubo_contents[10],
                self.clear_color.a * self.ubo_contents[11],
            );
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn flush(&mut self, gl: &glow::Context) {
        if self.instance_count == 0 {
            return;
        }

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.instances_vbo));
            gl.buffer_sub_data_u8_slice(
                glow::ARRAY_BUFFER,
                0,
                &self.instance_buffer[0..=(self.instance_buffer_at as usize)]
                    .align_to::<u8>()
                    .1,
            );

            gl.bind_vertex_array(Some(self.vao_symbols));
            gl.draw_arrays_instanced(glow::TRIANGLE_FAN, 0, 4, self.instance_count as i32);

            self.instance_buffer_at = -1;
            self.instance_count = 0;
        }
    }

    pub fn bind_texture_atlas(&mut self, gl: &glow::Context, texture: glow::Texture) {
        if Some(texture) == self.current_texture_atlas {
            return;
        }

        unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        }

        self.current_texture_atlas = Some(texture);
    }

    pub fn make_symbols_frame(
        &mut self,
        sheet: &mut GlTexture,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        x_origin: f32,
        y_origin: f32,
    ) -> GlCell {
        let origin_x = x_origin / width;
        let origin_y = y_origin / height;
        let tex_width = sheet.width as f32;
        let tex_height = sheet.height as f32;

        let uv_left = x / tex_width;
        let uv_top = y / tex_height;
        let uv_width = width / tex_width;
        let uv_height = height / tex_height;

        let frame = GlCell {
            texture: sheet.texture,
            width,
            height,
            origin_x,
            origin_y,
            uv_left,
            uv_top,
            uv_width,
            uv_height,
        };

        frame
    }

    pub fn set_clear_color(&mut self, color: GlColor) {
        self.clear_color = color;
    }

    pub fn draw_general2d(
        &mut self,
        gl: &glow::Context,
        rtidx: usize,
        // texture: glow::Texture,
        area: [f32; 4],
        transform: &GlTransform,
        color: &GlColor,
    ) {
        self.flush(gl); // 确保之前的绘制命令已经执行

        // 使用 General2D 着色器
        self.shaders[GlRenderMode::General2D as usize].bind(gl);
        self.render_mode = GlRenderMode::General2D;

        // 绑定 VAO
        unsafe {
            gl.bind_vertex_array(Some(self.vao_general2d));

            // 设置 uniform
            let shader_program = self.shaders[GlRenderMode::General2D as usize].program;

            // 绑定纹理
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.render_textures[rtidx].texture));
            self.current_texture_atlas = Some(self.render_textures[rtidx].texture);
            let tex_loc = gl.get_uniform_location(shader_program, "texture1");
            gl.uniform_1_i32(tex_loc.as_ref(), 0);

            // 设置变换矩阵
            let transform_loc = gl.get_uniform_location(shader_program, "transform");
            gl.uniform_matrix_4_f32_slice(
                transform_loc.as_ref(),
                false,
                &[
                    transform.m00,
                    transform.m01,
                    0.0,
                    0.0,
                    transform.m10,
                    transform.m11,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    transform.m20,
                    transform.m21,
                    0.0,
                    1.0,
                ],
            );

            let area_loc = gl.get_uniform_location(shader_program, "area");
            gl.uniform_4_f32_slice(area_loc.as_ref(), &area);

            let color_loc = gl.get_uniform_location(shader_program, "color");
            gl.uniform_4_f32_slice(color_loc.as_ref(), &[color.r, color.g, color.b, color.a]);

            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);

            gl.bind_vertex_array(None);
        }
    }

    pub fn render_trans_frame(
        &mut self,
        gl: &glow::Context,
        width: u32,
        height: u32,
        progress: f32,
    ) {
        unsafe {
            self.prepare_draw_trans(gl);
            gl.viewport(0, 0, width as i32, height as i32);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.use_program(Some(self.shaders[1].program));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.render_textures[0].texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.shaders[1].program, "texture1")
                    .as_ref(),
                0,
            );

            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.render_textures[1].texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.shaders[1].program, "texture2")
                    .as_ref(),
                1,
            );

            let lb = gl.get_uniform_location(self.shaders[1].program, "progress");
            gl.uniform_1_f32(lb.as_ref(), progress);

            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);
        }
    }

    pub fn render_rbuf(
        &mut self,
        gl: &glow::Context,
        rbuf: &[RenderCell],
        ratio_x: f32,
        ratio_y: f32,
    ) {
        self.bind_texture_atlas(gl, self.symbols[0].texture);
        for r in rbuf {
            let mut transform = GlTransform::new();
            transform.translate(r.x + r.cx - PIXEL_SYM_WIDTH, r.y + r.cy - PIXEL_SYM_HEIGHT);
            if r.angle != 0.0 {
                transform.rotate(r.angle);
            }
            transform.translate(
                -r.cx + PIXEL_SYM_WIDTH / 2.0,
                -r.cy + PIXEL_SYM_HEIGHT / 2.0,
            );
            transform.scale(1.0 / ratio_x, 1.0 / ratio_y);

            if let Some(b) = r.bcolor {
                let back_color = GlColor::new(b.0, b.1, b.2, b.3);
                // fill instance buffer for opengl instance rendering
                self.render_symbol(gl, 320, &transform, &back_color);
            }

            let color = GlColor::new(r.fcolor.0, r.fcolor.1, r.fcolor.2, r.fcolor.3);
            // fill instance buffer for opengl instance rendering
            self.render_symbol(gl, r.texsym, &transform, &color);
        }
    }

    pub fn render_symbol(
        &mut self,
        gl: &glow::Context,
        sym: usize,
        transform: &GlTransform,
        color: &GlColor,
    ) {
        self.prepare_render_symbols(gl, GlRenderMode::PixCells, 16);
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
}

unsafe fn create_trans_buffers(
    gl: &glow::Context,
    program: glow::Program,
) -> (glow::VertexArray, glow::Buffer, glow::Buffer) {
    let vertices: [f32; 16] = [
        -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
    ];
    let indices: [u32; 6] = [0, 1, 2, 2, 3, 0];

    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(vao));

    let vertex_buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
    gl.buffer_data_u8_slice(
        glow::ARRAY_BUFFER,
        &vertices.align_to::<u8>().1,
        glow::STATIC_DRAW,
    );

    let index_buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
    gl.buffer_data_u8_slice(
        glow::ELEMENT_ARRAY_BUFFER,
        &indices.align_to::<u8>().1,
        glow::STATIC_DRAW,
    );

    let pos_attrib = gl.get_attrib_location(program, "aPos").unwrap();
    let tex_attrib = gl.get_attrib_location(program, "aTexCoord").unwrap();
    gl.enable_vertex_attrib_array(pos_attrib);
    gl.enable_vertex_attrib_array(tex_attrib);

    gl.vertex_attrib_pointer_f32(pos_attrib, 2, glow::FLOAT, false, 16, 0);
    gl.vertex_attrib_pointer_f32(tex_attrib, 2, glow::FLOAT, false, 16, 8);

    gl.bind_vertex_array(None);

    (vao, vertex_buffer, index_buffer)
}

unsafe fn create_symbols_buffers(
    gl: &glow::Context,
    // program: glow::Program,
) -> (glow::VertexArray, glow::Buffer, glow::Buffer, glow::Buffer) {
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
        &quad_vertices.align_to::<u8>().1,
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

    (vao_symbolss, instances_vbo, quad_vbo, ubo)
}

unsafe fn create_general2d_buffers(
    gl: &glow::Context,
    program: glow::Program,
) -> (glow::VertexArray, glow::Buffer, glow::Buffer) {
    let vertices: [f32; 16] = [
        // positions  // texCoords
        0.0, 0.0, 0.0, 0.0, // 左下角
        1.0, 0.0, 1.0, 0.0, // 右下角
        1.0, 1.0, 1.0, 1.0, // 右上角
        0.0, 1.0, 0.0, 1.0, // 左上角
    ];
    let indices: [u32; 6] = [0, 1, 2, 2, 3, 0];

    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(vao));

    let vbo = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    gl.buffer_data_u8_slice(
        glow::ARRAY_BUFFER,
        &vertices.align_to::<u8>().1,
        glow::STATIC_DRAW,
    );

    let ebo = gl.create_buffer().unwrap();
    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
    gl.buffer_data_u8_slice(
        glow::ELEMENT_ARRAY_BUFFER,
        &indices.align_to::<u8>().1,
        glow::STATIC_DRAW,
    );

    let pos_attrib = gl.get_attrib_location(program, "aPos").unwrap();
    let tex_attrib = gl.get_attrib_location(program, "aTexCoord").unwrap();
    gl.enable_vertex_attrib_array(pos_attrib);
    gl.enable_vertex_attrib_array(tex_attrib);

    gl.vertex_attrib_pointer_f32(pos_attrib, 2, glow::FLOAT, false, 16, 0);
    gl.vertex_attrib_pointer_f32(tex_attrib, 2, glow::FLOAT, false, 16, 8);

    gl.bind_vertex_array(None);

    (vao, vbo, ebo)
}
