use glow::{
    HasContext, NativeBuffer, NativeFramebuffer, NativeTexture, NativeVertexArray, Program,
};

pub struct GlTransition {
    pub program: Program,
    pub vao: NativeVertexArray,
    pub vbuf: NativeBuffer,
    pub ibuf: NativeBuffer,
    pub rt: NativeTexture,
    pub fbuf: NativeFramebuffer,
    pub width: u32,
    pub height: u32,
    pub texture1: Option<NativeTexture>,
    pub texture2: Option<NativeTexture>,
    pub pixels: Vec<u8>,
}

impl GlTransition {
    pub fn new(gl: &glow::Context, width: u32, height: u32) -> Self {
        unsafe {
            let fs = r#"
                uniform float bounces = 3.0;
                const float PI = 3.14159265358;

                vec4 transition (vec2 uv) {
                    float time = progress;
                    float stime = sin(time * PI / 2.);
                    float phase = time * PI * bounces;
                    float y = (abs(cos(phase))) * (1.0 - stime);
                    float d = uv.y - y;
                    // vec4 from = getFromColor(vec2(uv.x, uv.y + (1.0 - y)));
                    vec4 from = getFromColor(uv);
                    vec4 to = getToColor(uv);
                    vec4 mc = mix( to, from, step(d, 0.0) );
                    return mc;
            }"#;

            // create shaders and buffers...
            let program = create_shaders(&gl, fs);
            let (vao, vbuf, ibuf) = create_buffers(&gl, program);
            let (rt, fbuf) = create_render_texture(&gl, width, height);
            let texture1 = None;
            let texture2 = None;
            Self {
                program,
                vao,
                vbuf,
                ibuf,
                rt,
                fbuf,
                width,
                height,
                texture1,
                texture2,
                pixels: vec![],
            }
        }
    }

    // call if u want update texture...
    pub fn set_texture(&mut self, gl: &glow::Context, img1: &[u8], img2: &[u8]) {
        unsafe {
            if let Some(texture1) = self.texture1 {
                gl.delete_texture(texture1);
            }
            if let Some(texture2) = self.texture2 {
                gl.delete_texture(texture2);
            }
            let w = self.width;
            let h = self.height;
            self.texture1 = Some(create_texture(&gl, w, h, &img1));
            self.texture2 = Some(create_texture(&gl, w, h, &img2));
        }
    }

    // render and output pixels data...
    pub fn render_frame(&mut self, gl: &glow::Context, p: f32) {
        unsafe {
            let w = self.width;
            let h = self.height;
            if let (Some(t1), Some(t2)) = (self.texture1, self.texture2) {
                self.pixels = render_frame(&gl, self.program, t1, t2, w, h, p);
            }
        }
    }

    // clean handle...
    pub fn clean(&mut self, gl: &glow::Context) {
        unsafe {
            if let (Some(t1), Some(t2)) = (self.texture1, self.texture2) {
                cleanup(
                    &gl,
                    self.program,
                    self.vao,
                    self.vbuf,
                    self.ibuf,
                    t1,
                    t2,
                    self.rt,
                    self.fbuf,
                );
            }
        }
    }
}

fn check_gl_error(gl: &glow::Context, label: &str) {
    unsafe {
        let error = gl.get_error();
        if error != glow::NO_ERROR {
            println!("OpenGL Error [{}]: {:?}", label, error);
        }
    }
}

unsafe fn render_frame(
    gl: &glow::Context,
    program: glow::Program,
    texture1: glow::NativeTexture,
    texture2: glow::NativeTexture,
    width: u32,
    height: u32,
    progress: f32,
) -> Vec<u8> {
    gl.viewport(0, 0, width as i32, height as i32);
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(glow::COLOR_BUFFER_BIT);

    gl.use_program(Some(program));

    gl.active_texture(glow::TEXTURE0);
    gl.bind_texture(glow::TEXTURE_2D, Some(texture1));
    gl.uniform_1_i32(gl.get_uniform_location(program, "texture1").as_ref(), 0);

    gl.active_texture(glow::TEXTURE1);
    gl.bind_texture(glow::TEXTURE_2D, Some(texture2));
    gl.uniform_1_i32(gl.get_uniform_location(program, "texture2").as_ref(), 1);

    // 设置progress
    let lb = gl.get_uniform_location(program, "progress");
    gl.uniform_1_f32(lb.as_ref(), progress);

    gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);

    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    gl.read_pixels(
        0,
        0,
        width as i32,
        height as i32,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        glow::PixelPackData::Slice(&mut pixels),
    );
    // println!("pixel {:?}", pixels);
    pixels
}

unsafe fn create_shaders(gl: &glow::Context, fssrc: &str) -> glow::Program {
    let vertex_shader_source = r#"
            #version 330 core
            layout(location = 0) in vec2 aPos;
            layout(location = 1) in vec2 aTexCoord;
            out vec2 TexCoord;
            void main() {
                gl_Position = vec4(aPos, 0.0, 1.0);
                TexCoord = aTexCoord;
            }
    "#;

    let fragment_shader_source = &format!(
        r#"
            #version 330 core
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
        fssrc
    );

    let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
    gl.shader_source(vertex_shader, vertex_shader_source);
    gl.compile_shader(vertex_shader);
    if !gl.get_shader_compile_status(vertex_shader) {
        panic!(
            "Vertex shader compilation failed: {}",
            gl.get_shader_info_log(vertex_shader)
        );
    }

    let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
    gl.shader_source(fragment_shader, fragment_shader_source);
    gl.compile_shader(fragment_shader);
    if !gl.get_shader_compile_status(fragment_shader) {
        panic!(
            "Fragment shader compilation failed: {}",
            gl.get_shader_info_log(fragment_shader)
        );
    }

    let program = gl.create_program().unwrap();
    gl.attach_shader(program, vertex_shader);
    gl.attach_shader(program, fragment_shader);
    gl.link_program(program);
    assert!(gl.get_program_link_status(program));

    gl.delete_shader(vertex_shader);
    gl.delete_shader(fragment_shader);

    program
}

unsafe fn create_render_texture(
    gl: &glow::Context,
    width: u32,
    height: u32,
) -> (NativeTexture, NativeFramebuffer) {
    let render_texture = gl.create_texture().unwrap();
    let framebuffer = gl.create_framebuffer().unwrap();
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
    check_gl_error(&gl, "Framebuffer Bind");

    gl.bind_texture(glow::TEXTURE_2D, Some(render_texture));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA as i32,
        width as i32,
        height as i32,
        0,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        None,
    );
    gl.framebuffer_texture_2d(
        glow::FRAMEBUFFER,
        glow::COLOR_ATTACHMENT0,
        glow::TEXTURE_2D,
        Some(render_texture),
        0,
    );
    check_gl_error(&gl, "Attach Render Texture to Framebuffer");

    if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
        panic!("Framebuffer is not complete");
    }
    (render_texture, framebuffer)
}

unsafe fn create_buffers(
    gl: &glow::Context,
    program: glow::Program,
) -> (NativeVertexArray, NativeBuffer, NativeBuffer) {
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

    (vao, vertex_buffer, index_buffer)
}

unsafe fn create_texture(
    gl: &glow::Context,
    img_width: u32,
    img_height: u32,
    img_raw: &[u8],
) -> NativeTexture {
    let texture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
        glow::LINEAR as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::LINEAR as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
    );

    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA as i32,
        img_width as i32,
        img_height as i32,
        0,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        Some(img_raw),
    );

    texture
}

unsafe fn cleanup(
    gl: &glow::Context,
    program: glow::Program,
    vao: glow::NativeVertexArray,
    vertex_buffer: glow::NativeBuffer,
    index_buffer: glow::NativeBuffer,
    texture1: glow::NativeTexture,
    texture2: glow::NativeTexture,
    rt: NativeTexture,
    fb: NativeFramebuffer,
) {
    gl.delete_vertex_array(vao);
    gl.delete_buffer(vertex_buffer);
    gl.delete_buffer(index_buffer);
    gl.delete_program(program);
    gl.delete_texture(texture1);
    gl.delete_texture(texture2);
    gl.delete_texture(rt);
    gl.delete_framebuffer(fb);
}
