use glow::HasContext;
use glutin::ContextBuilder;
use std::env;

const WIDTH: u32 = 40;
const HEIGHT: u32 = 40;

fn check_gl_error(gl: &glow::Context, label: &str) {
    unsafe {
        let error = gl.get_error();
        if error != glow::NO_ERROR {
            println!("OpenGL Error [{}]: {:?}", label, error);
        } else {
            println!("No OpenGL Error [{}]", label);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let headless = args.contains(&"--headless".to_string());

    let img = image::open("your_image.png").expect("Failed to load image");
    let img_data = img.to_rgba8();
    let img_width = img.width();
    let img_height = img.height();
    let img_raw = img_data.into_raw();
    println!("@@@@@{} {}", img_width, img_height);

    let el = glutin::event_loop::EventLoop::new();

    if headless {
        let size = glutin::dpi::PhysicalSize::new(WIDTH, HEIGHT);
        let window_context = ContextBuilder::new()
            .with_gl_debug_flag(true) // Enable debugging
            .build_headless(&el, size)
            .expect("Failed to create headless context");

        let window_context = unsafe { window_context.make_current().unwrap() };

        render(&window_context, &img_raw, img_width, img_height, true);
    } else {
        let window_builder = glutin::window::WindowBuilder::new()
            .with_title("OpenGL with Glow")
            .with_inner_size(glutin::dpi::PhysicalSize::new(WIDTH, HEIGHT));

        let window_context = ContextBuilder::new()
            .with_gl_debug_flag(true) // Enable debugging
            .build_windowed(window_builder, &el)
            .expect("Failed to create windowed context");

        let window_context = unsafe { window_context.make_current().unwrap() };

        el.run(move |event, _, control_flow| {
            *control_flow = glutin::event_loop::ControlFlow::Wait;
            match event {
                glutin::event::Event::WindowEvent { event, .. } => match event {
                    glutin::event::WindowEvent::CloseRequested => {
                        *control_flow = glutin::event_loop::ControlFlow::Exit
                    }
                    _ => (),
                },
                glutin::event::Event::MainEventsCleared => {
                    render(&window_context, &img_raw, img_width, img_height, false);
                    window_context.swap_buffers().unwrap();
                }
                _ => (),
            }
        });
    }
}

fn render(
    context: &glutin::Context<glutin::PossiblyCurrent>,
    img_raw: &[u8],
    img_width: u32,
    img_height: u32,
    headless: bool,
) {
    let gl =
        unsafe { glow::Context::from_loader_function(|s| context.get_proc_address(s) as *const _) };

    unsafe {
        // 设置视口
        gl.viewport(0, 0, WIDTH as i32, HEIGHT as i32);
        check_gl_error(&gl, "Viewport");

        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        //gl.clear(glow::COLOR_BUFFER_BIT);
        check_gl_error(&gl, "Clear Screen");

        // 顶点着色器和片段着色器代码
        let vertex_shader_source = r#"
            #version 330
            in vec2 a_position;
            in vec2 a_tex_coord;
            out vec2 v_tex_coord;
            void main() {
                v_tex_coord = vec2(a_tex_coord.x, 1.0 - a_tex_coord.y);
                gl_Position = vec4(a_position, 0.0, 1.0);
            }
        "#;

        let fragment_shader_source = r#"
            #version 330
            in vec2 v_tex_coord;
            uniform sampler2D u_texture;
            out vec4 color;
            void main() {
                color = texture(u_texture, v_tex_coord);
            }
        "#;

        // 编译并链接着色器程序
        let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
        gl.shader_source(vertex_shader, vertex_shader_source);
        gl.compile_shader(vertex_shader);
        check_gl_error(&gl, "Vertex Shader Compile");
        if !gl.get_shader_compile_status(vertex_shader) {
            panic!(
                "Vertex shader compilation failed: {}",
                gl.get_shader_info_log(vertex_shader)
            );
        }

        let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
        gl.shader_source(fragment_shader, fragment_shader_source);
        gl.compile_shader(fragment_shader);
        check_gl_error(&gl, "Fragment Shader Compile");
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
        check_gl_error(&gl, "Program Link");
        if !gl.get_program_link_status(program) {
            panic!(
                "Shader program linking failed: {}",
                gl.get_program_info_log(program)
            );
        }

        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        // 顶点数据和索引
        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
        ];

        let indices: [u32; 6] = [
            0, 1, 2, // 三角形 1
            2, 3, 0, // 三角形 2
        ];

        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        check_gl_error(&gl, "VAO Bind");

        let vertex_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            &vertices.align_to::<u8>().1,
            glow::STATIC_DRAW,
        );
        check_gl_error(&gl, "Vertex Buffer Data");

        let index_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            &indices.align_to::<u8>().1,
            glow::STATIC_DRAW,
        );
        check_gl_error(&gl, "Index Buffer Data");

        let pos_attrib = gl.get_attrib_location(program, "a_position").unwrap();
        let tex_attrib = gl.get_attrib_location(program, "a_tex_coord").unwrap();
        gl.enable_vertex_attrib_array(pos_attrib);
        gl.enable_vertex_attrib_array(tex_attrib);
        check_gl_error(&gl, "Enable Vertex Attribs");

        gl.vertex_attrib_pointer_f32(pos_attrib, 2, glow::FLOAT, false, 16, 0);
        gl.vertex_attrib_pointer_f32(tex_attrib, 2, glow::FLOAT, false, 16, 8);
        check_gl_error(&gl, "Set Vertex Attrib Pointers");

        let (render_texture, framebuffer) = if headless {
            // **创建帧缓冲区并绑定** 在上传纹理之前
            let render_texture = gl.create_texture().unwrap();
            let framebuffer = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            check_gl_error(&gl, "Framebuffer Bind");

            gl.bind_texture(glow::TEXTURE_2D, Some(render_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                WIDTH as i32,
                HEIGHT as i32,
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
            (Some(render_texture), Some(framebuffer))
        } else {
            (None, None)
        };

        // 纹理上传：将 PNG 图像上传到 OpenGL
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
        check_gl_error(&gl, "Upload Texture Data");

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.use_program(Some(program));
        check_gl_error(&gl, "Use Program and Bind Texture");

        gl.uniform_1_i32(gl.get_uniform_location(program, "u_texture").as_ref(), 0);
        check_gl_error(&gl, "Set Uniform");

        // 渲染到帧缓冲区
        gl.viewport(0, 0, WIDTH as i32, HEIGHT as i32); // 设置视口为帧缓冲区大小
        gl.clear(glow::COLOR_BUFFER_BIT);
        check_gl_error(&gl, "Clear Framebuffer");
        gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);
        check_gl_error(&gl, "Draw Elements");

        if headless {
            let mut pixels = vec![0u8; WIDTH as usize * HEIGHT as usize * 4];
            gl.read_pixels(
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );
            check_gl_error(&gl, "Read Pixels");

            println!("pixel {:?}", pixels);
        }

        // 清理资源
        gl.delete_vertex_array(vao);
        gl.delete_buffer(vertex_buffer);
        gl.delete_buffer(index_buffer);
        gl.delete_program(program);
        gl.delete_texture(texture);
        if render_texture != None {
            gl.delete_texture(render_texture.unwrap());
        }
        if framebuffer != None {
            gl.delete_framebuffer(framebuffer.unwrap());
        }
    }
}
