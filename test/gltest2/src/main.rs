use bytemuck::cast_slice;
use glow::HasContext;
use std::rc::Rc;

pub struct Renderer {
    gl: Rc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    textures: Vec<glow::Texture>,
    width: i32,
    height: i32,
    windowed_context: Option<glutin::WindowedContext<glutin::PossiblyCurrent>>,
    event_loop: Option<glutin::event_loop::EventLoop<()>>,
}

impl Renderer {
    /// 初始化渲染器，支持窗口和无窗口模式
    pub fn new(width: i32, height: i32, windowed: bool) -> Self {
        if windowed {
            let (gl, windowed_context, event_loop) = Self::create_windowed_context(width, height);
            let gl = Rc::new(gl);
            let program = Self::create_shader_program(&gl);
            let vao = unsafe { gl.create_vertex_array().unwrap() };
            unsafe { gl.bind_vertex_array(Some(vao)) };

            Self {
                gl,
                program,
                vao,
                textures: Vec::new(),
                width,
                height,
                windowed_context: Some(windowed_context),
                event_loop: Some(event_loop),
            }
        } else {
            let (gl, _) = Self::create_headless_context(width, height);
            let gl = Rc::new(gl);
            let program = Self::create_shader_program(&gl);
            let vao = unsafe { gl.create_vertex_array().unwrap() };
            unsafe { gl.bind_vertex_array(Some(vao)) };

            Self {
                gl,
                program,
                vao,
                textures: Vec::new(),
                width,
                height,
                windowed_context: None,
                event_loop: None,
            }
        }
    }

    /// 创建窗口模式的 OpenGL 上下文
    fn create_windowed_context(
        width: i32,
        height: i32,
    ) -> (
        glow::Context,
        glutin::WindowedContext<glutin::PossiblyCurrent>,
        glutin::event_loop::EventLoop<()>,
    ) {
        let event_loop = glutin::event_loop::EventLoop::new();
        let wb = glutin::window::WindowBuilder::new()
            .with_title("Renderer")
            .with_inner_size(glutin::dpi::PhysicalSize::new(width, height));
        let windowed_context = glutin::ContextBuilder::new()
            .with_vsync(false)
            .build_windowed(wb, &event_loop)
            .unwrap();

        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        let gl = unsafe {
            glow::Context::from_loader_function(|s| windowed_context.get_proc_address(s))
        };
        (gl, windowed_context, event_loop)
    }

    /// 创建无窗口模式的 OpenGL 上下文
    fn create_headless_context(
        width: i32,
        height: i32,
    ) -> (glow::Context, glutin::Context<glutin::PossiblyCurrent>) {
        let event_loop = glutin::event_loop::EventLoop::new();
        let cb = glutin::ContextBuilder::new();
        let context = cb
            .build_headless(
                &event_loop,
                glutin::dpi::PhysicalSize::new(width as u32, height as u32),
            )
            .unwrap();

        let context = unsafe { context.make_current().unwrap() };
        let gl = unsafe { glow::Context::from_loader_function(|s| context.get_proc_address(s)) };
        (gl, context)
    }

    /// 创建 Shader 程序
    fn create_shader_program(gl: &glow::Context) -> glow::Program {
        unsafe {
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

            let fragment_shader_source = r#"
                #version 330 core
                out vec4 FragColor;
                in vec2 TexCoord;
                uniform sampler2D texture1;
                uniform sampler2D texture2;
                uniform float progress;
                const float PI = 3.14159265358;
                void main() {
                    float time = progress;
                    float stime = sin(time * PI / 2.);
                    float phase = time * PI * 6.0;
                    float y = (abs(cos(phase))) * (1.0 - stime);
                    float d = TexCoord.y - y;
                    vec4 color1 = texture(texture1, TexCoord);
                    vec4 color2 = texture(texture2, TexCoord);
                    FragColor = mix(color1, color2, step(d, 0.0));
                }
            "#;

            let program = gl.create_program().expect("Cannot create program");

            let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            gl.shader_source(vs, vertex_shader_source);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                panic!(
                    "Vertex shader compilation failed: {}",
                    gl.get_shader_info_log(vs)
                );
            }
            gl.attach_shader(program, vs);

            let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            gl.shader_source(fs, fragment_shader_source);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                panic!(
                    "Fragment shader compilation failed: {}",
                    gl.get_shader_info_log(fs)
                );
            }
            gl.attach_shader(program, fs);

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!(
                    "Shader linking failed: {}",
                    gl.get_program_info_log(program)
                );
            }

            gl.delete_shader(vs);
            gl.delete_shader(fs);

            program
        }
    }

    /// 加载纹理
    pub fn load_textures(&mut self, image_paths: &[&str]) {
        use image::GenericImageView;

        for (_i, &path) in image_paths.iter().enumerate() {
            let img = image::open(path).expect("Failed to load image");
            let data = img.flipv().to_rgba8();
            let (width, height) = img.dimensions();

            unsafe {
                let texture = self.gl.create_texture().unwrap();
                self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                self.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    width as i32,
                    height as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(&data),
                );
                self.gl.generate_mipmap(glow::TEXTURE_2D);

                self.textures.push(texture);
            }
        }
    }

    /// 运行事件循环（窗口模式）
    pub fn run(mut self) {
        let event_loop = self.event_loop.take().unwrap();
        let windowed_context = self.windowed_context.take().unwrap();
        let gl = self.gl.clone();
        let program = self.program;
        let vao = self.vao;
        let textures = self.textures.clone();
        let width = self.width;
        let height = self.height;
        let mut p = 0.0f32;

        let target_fps = 60.0;
        let frame_duration = std::time::Duration::from_secs_f64(1.0 / target_fps);
        let mut last_frame_time = std::time::Instant::now();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = glutin::event_loop::ControlFlow::Poll;

            match event {
                glutin::event::Event::MainEventsCleared => {
                    let now = std::time::Instant::now();
                    if now - last_frame_time >= frame_duration {
                        // 请求重绘窗口
                        windowed_context.window().request_redraw();
                        last_frame_time = now;
                    }
                }

                glutin::event::Event::RedrawRequested(_) => {
                    unsafe {
                        gl.viewport(0, 0, width, height);
                        gl.clear_color(0.0, 0.0, 0.0, 1.0);
                        gl.clear(glow::COLOR_BUFFER_BIT);

                        gl.use_program(Some(program));

                        // 绑定纹理
                        for (i, &texture) in textures.iter().enumerate() {
                            gl.active_texture(glow::TEXTURE0 + i as u32);
                            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                            let location =
                                gl.get_uniform_location(program, &format!("texture{}", i + 1));
                            gl.uniform_1_i32(location.as_ref(), i as i32);
                        }

                        // 设置progress
                        let lb = gl.get_uniform_location(program, "progress");
                        gl.uniform_1_f32(lb.as_ref(), p);
                        p += 0.01;
                        if p >= 1.0 {
                            p = 0.0;
                        }

                        // 绘制全屏四边形
                        Self::draw_fullscreen_quad_internal(&gl, vao);

                        windowed_context.swap_buffers().unwrap();
                    }
                }
                glutin::event::Event::WindowEvent { event, .. } => match event {
                    glutin::event::WindowEvent::CloseRequested => {
                        *control_flow = glutin::event_loop::ControlFlow::Exit;
                    }
                    _ => (),
                },
                _ => (),
            }
        });
    }

    /// 渲染并读取像素数据（无窗口模式）
    pub fn render_and_read_pixels(&self) -> Vec<u8> {
        unsafe {
            let p = 0.0f32;
            // 创建帧缓冲区
            let fbo = self.gl.create_framebuffer().unwrap();
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));

            // 创建纹理附件
            let texture = self.gl.create_texture().unwrap();
            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                self.width,
                self.height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );

            // 检查帧缓冲区完整性
            if self.gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                panic!("Framebuffer is not complete");
            }

            // 渲染到帧缓冲区
            self.gl.viewport(0, 0, self.width, self.height);
            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            self.gl.use_program(Some(self.program));

            // 绑定纹理
            for (i, &texture) in self.textures.iter().enumerate() {
                self.gl.active_texture(glow::TEXTURE0 + i as u32);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                let location = self
                    .gl
                    .get_uniform_location(self.program, &format!("texture{}", i + 1));
                self.gl.uniform_1_i32(location.as_ref(), i as i32);
            }

            // 绘制全屏四边形
            Self::draw_fullscreen_quad_internal(&self.gl, self.vao);

            // 设置progress
            let lb = self.gl.get_uniform_location(self.program, "progress");
            self.gl.uniform_1_f32(lb.as_ref(), p);

            // 读取像素数据
            let mut pixels = vec![0u8; (self.width * self.height * 4) as usize];
            self.gl.read_pixels(
                0,
                0,
                self.width,
                self.height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );

            // 清理
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.delete_framebuffer(fbo);
            self.gl.delete_texture(texture);

            pixels
        }
    }

    /// 绘制全屏四边形（内部使用）
    fn draw_fullscreen_quad_internal(gl: &glow::Context, vao: glow::VertexArray) {
        unsafe {
            gl.bind_vertex_array(Some(vao));

            let vertices: [f32; 24] = [
                // Positions   // TexCoords
                -1.0, 1.0, 0.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0,
                1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
            ];

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, cast_slice(&vertices), glow::STATIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);

            gl.draw_arrays(glow::TRIANGLES, 0, 6);

            gl.delete_buffer(vbo);
            gl.bind_vertex_array(None);
        }
    }
}

fn main() {
    let width = 800;
    let height = 600;
    let windowed = true; // 设置为 false 则为无窗口模式

    let mut renderer = Renderer::new(width, height, windowed);
    renderer.load_textures(&["texture1.png", "texture2.png"]);

    if windowed {
        // 窗口模式，运行事件循环
        renderer.run();
    } else {
        // 无窗口模式，直接渲染并读取像素数据
        let pixels = renderer.render_and_read_pixels();

        // 处理像素数据，例如保存为图像
        image::save_buffer(
            "output.png",
            &pixels,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        )
        .unwrap();

        println!("渲染完成！");
    }
}
