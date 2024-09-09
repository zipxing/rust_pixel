use glow::HasContext;
use glutin::ContextBuilder;
const WIDTH : u32 = 8;
const HEIGHT : u32 = 6;

fn main() {
    // 创建无窗口的 OpenGL 上下文
    let el = glutin::event_loop::EventLoop::new();
    let size = glutin::dpi::PhysicalSize::new(WIDTH, HEIGHT);
    let window_context = ContextBuilder::new()
        .build_headless(&el, size)
        .expect("Failed to create headless context");

    // 将上下文设为当前上下文
    let window_context = unsafe { window_context.make_current().unwrap() };

    // 使用 glow 创建 OpenGL 上下文
    let gl = unsafe {
        glow::Context::from_loader_function(|s| window_context.get_proc_address(s) as *const _)
    };

    // 渲染操作
    unsafe {
        gl.viewport(0, 0, WIDTH as i32, HEIGHT as i32);
        // 设置清除颜色
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        // 创建一个简单的顶点着色器和片段着色器
        let vertex_shader_source = r#"
            #version 330
            in vec2 a_position;
            void main() {
                gl_Position = vec4(a_position, 0.0, 1.0);
            }
        "#;

        let fragment_shader_source = r#"
            #version 330
            out vec4 color;
            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);  // 红色
            }
        "#;

        // 编译顶点着色器
        let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
        gl.shader_source(vertex_shader, vertex_shader_source);
        gl.compile_shader(vertex_shader);
        if !gl.get_shader_compile_status(vertex_shader) {
            panic!("Vertex shader compilation failed: {}", gl.get_shader_info_log(vertex_shader));
        }

        // 编译片段着色器
        let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
        gl.shader_source(fragment_shader, fragment_shader_source);
        gl.compile_shader(fragment_shader);
        if !gl.get_shader_compile_status(fragment_shader) {
            panic!("Fragment shader compilation failed: {}", gl.get_shader_info_log(fragment_shader));
        }

        // 链接着色器程序
        let program = gl.create_program().unwrap();
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("Shader program linking failed: {}", gl.get_program_info_log(program));
        }
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        // 创建顶点缓冲区
        let vertices: [f32; 6] = [
            -0.5, -0.5,  // 左下角
            0.5, -0.5,   // 右下角
            0.0, 0.5,    // 顶点
        ];

        let vertex_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &vertices.align_to::<u8>().1, glow::STATIC_DRAW);

        // 创建顶点数组对象
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let pos_attrib = gl.get_attrib_location(program, "a_position").unwrap();
        gl.enable_vertex_attrib_array(pos_attrib);
        gl.vertex_attrib_pointer_f32(pos_attrib, 2, glow::FLOAT, false, 8, 0);

        // 使用着色器程序
        gl.use_program(Some(program));

        // 创建帧缓冲区并绑定
        let framebuffer = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

        // 创建渲染目标纹理
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
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
            Some(texture),
            0,
        );

        // 检查帧缓冲区是否完整
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            panic!("Framebuffer is not complete");
        }

        // 绘制三角形到帧缓冲区
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.flush();

        // 读取帧缓冲区的像素数据
        let mut pixels = vec![0u8; WIDTH as usize * HEIGHT as usize * 4]; // 每个像素4个字节（RGBA）
        gl.read_pixels(
            0,
            0,
            WIDTH as i32,
            HEIGHT as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(&mut pixels),
        );

        // 打印左下角的像素颜色
        println!("First pixel color: {:?}", pixels);

        // 清理资源
        gl.delete_vertex_array(vao);
        gl.delete_buffer(vertex_buffer);
        gl.delete_program(program);
        gl.delete_texture(texture);
        gl.delete_framebuffer(framebuffer);
    }
}

