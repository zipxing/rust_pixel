use glow::HasContext;
use glow::NativeBuffer;
use glow::NativeFramebuffer;
use glow::NativeTexture;
use glow::NativeVertexArray;
use glutin::ContextBuilder;

pub struct GlTransition {
    pub gl: glow::Context,
    pub program: glow::Program,
    pub texture1: glow::NativeTexture,
    pub texture2: glow::NativeTexture,
    pub headless: bool,
}

impl GlTransition {
    pub fn new(width: u32, height: u32, img_raw: &[u8], img_raw2: &[u8], headless: bool) -> Self {
        let el = glutin::event_loop::EventLoop::new();
        if headless {
            let size = glutin::dpi::PhysicalSize::new(width, height);
            let context = ContextBuilder::new()
                .with_gl_debug_flag(true)
                .build_headless(&el, size)
                .expect("Failed to create headless context");
            let window_context = unsafe { context.make_current().unwrap() };
            let gl = unsafe {
                glow::Context::from_loader_function(|s| {
                    window_context.get_proc_address(s) as *const _
                })
            };
            unsafe {
                // 创建着色器、缓冲区和纹理 (仅初始化一次)
                let program = create_shaders(&gl);
                let (vao, vertex_buffer, index_buffer) = create_buffers(&gl, program);
                let (render_texture, framebuffer) =
                    create_render_texture(&gl, width, height, headless);
                let texture1 = create_texture(&gl, width, height, &img_raw);
                let texture2 = create_texture(&gl, width, height, &img_raw2);
                return Self {
                    gl,
                    program,
                    texture1,
                    texture2,
                    headless,
                };
            }
        } else {
            let window_builder = glutin::window::WindowBuilder::new()
                .with_title("OpenGL with Glow")
                .with_inner_size(glutin::dpi::PhysicalSize::new(width, height));

            let context = ContextBuilder::new()
                .with_gl_debug_flag(true)
                .build_windowed(window_builder, &el)
                .expect("Failed to create windowed context");
            let window_context = unsafe { context.make_current().unwrap() };
            let gl = unsafe {
                glow::Context::from_loader_function(|s| {
                    window_context.get_proc_address(s) as *const _
                })
            };
            // 创建着色器、缓冲区和纹理 (仅初始化一次)
            let program = unsafe { create_shaders(&gl) };
            let (vao, vertex_buffer, index_buffer) = unsafe { create_buffers(&gl, program) };
            let texture1 = unsafe { create_texture(&gl, width, height, &img_raw) };
            let texture2 = unsafe { create_texture(&gl, width, height, &img_raw2) };
            return Self {
                gl,
                program,
                texture1,
                texture2,
                headless,
            };

            // // 事件循环
            // el.run(move |event, _, control_flow| {
            //     *control_flow = glutin::event_loop::ControlFlow::Wait;
            //     match event {
            //         glutin::event::Event::WindowEvent { event, .. } => match event {
            //             glutin::event::WindowEvent::CloseRequested => {
            //                 *control_flow = glutin::event_loop::ControlFlow::Exit;
            //                 // 在程序退出时清理资源
            //                 unsafe {
            //                     cleanup(
            //                         &gl,
            //                         program,
            //                         vao,
            //                         vertex_buffer,
            //                         index_buffer,
            //                         texture1,
            //                         texture2,
            //                         None,
            //                         None,
            //                     );
            //                 }
            //             }
            //             _ => (),
            //         },
            //         glutin::event::Event::MainEventsCleared => {
            //             // 每帧渲染
            //             unsafe {
            //                 render_frame(&gl, program, texture1, texture2, headless);
            //             }
            //             window_context.swap_buffers().unwrap();
            //         }
            //         _ => (),
            //     }
            // });
        };
    }

    pub fn render_frame(self: &mut Self, w: u32, h: u32) {
        unsafe {
            render_frame(
                &self.gl,
                self.program,
                self.texture1,
                self.texture2,
                w,
                h,
                self.headless,
            );
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
    headless: bool,
) {
    gl.viewport(0, 0, width as i32, height as i32);
    // 使用已有的着色器和纹理进行渲染
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(glow::COLOR_BUFFER_BIT);

    gl.use_program(Some(program));

    // 绑定第一个纹理
    gl.active_texture(glow::TEXTURE0);
    gl.bind_texture(glow::TEXTURE_2D, Some(texture1));
    gl.uniform_1_i32(gl.get_uniform_location(program, "u_texture1").as_ref(), 0);

    // 绑定第二个纹理
    gl.active_texture(glow::TEXTURE1);
    gl.bind_texture(glow::TEXTURE_2D, Some(texture2));
    gl.uniform_1_i32(gl.get_uniform_location(program, "u_texture2").as_ref(), 1);

    gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);

    if headless {
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
        println!("pixel {:?}", pixels);
    }
}

unsafe fn create_shaders(gl: &glow::Context) -> glow::Program {
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
        uniform sampler2D u_texture1;
        uniform sampler2D u_texture2;
        out vec4 color;
        void main() {
            vec4 tex1 = texture(u_texture1, v_tex_coord);
            vec4 tex2 = texture(u_texture2, v_tex_coord);
            color = mix(tex1, tex2, 0.5);  // 混合两个纹理
        }
    "#;

    let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
    gl.shader_source(vertex_shader, vertex_shader_source);
    gl.compile_shader(vertex_shader);
    assert!(gl.get_shader_compile_status(vertex_shader));

    let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
    gl.shader_source(fragment_shader, fragment_shader_source);
    gl.compile_shader(fragment_shader);
    assert!(gl.get_shader_compile_status(fragment_shader));

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
    headless: bool,
) -> (Option<NativeTexture>, Option<NativeFramebuffer>) {
    if headless {
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
        (Some(render_texture), Some(framebuffer))
    } else {
        (None, None)
    }
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

    let pos_attrib = gl.get_attrib_location(program, "a_position").unwrap();
    let tex_attrib = gl.get_attrib_location(program, "a_tex_coord").unwrap();
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
    rt: Option<NativeTexture>,
    fb: Option<NativeFramebuffer>,
) {
    gl.delete_vertex_array(vao);
    gl.delete_buffer(vertex_buffer);
    gl.delete_buffer(index_buffer);
    gl.delete_program(program);
    gl.delete_texture(texture1);
    gl.delete_texture(texture2);
    if rt != None {
        gl.delete_texture(rt.unwrap());
    }
    if fb != None {
        gl.delete_framebuffer(fb.unwrap());
    }
}
