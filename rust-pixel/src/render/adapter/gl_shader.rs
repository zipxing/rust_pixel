// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use glow::HasContext;
use log::info;

#[derive(Clone)]
pub struct GlShader {
    pub program: glow::Program,
}

impl GlShader {
    pub fn new(gl: &glow::Context, ver: &str, vertex_source: &str, fragment_source: &str) -> Self {
        unsafe {
            let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            gl.shader_source(vertex_shader, &format!("{}\n{}", ver, vertex_source));
            gl.compile_shader(vertex_shader);
            if !gl.get_shader_compile_status(vertex_shader) {
                info!(
                    "Vertex Shader Compilation Error: {}",
                    gl.get_shader_info_log(vertex_shader)
                );
            }

            let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            gl.shader_source(fragment_shader, &format!("{}\n{}", ver, fragment_source));
            gl.compile_shader(fragment_shader);
            if !gl.get_shader_compile_status(fragment_shader) {
                info!(
                    "Fragment Shader Compilation Error: {}",
                    gl.get_shader_info_log(fragment_shader)
                );
            }

            let program = gl.create_program().unwrap();
            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!(
                    "Program Linking Error: {}",
                    gl.get_program_info_log(program)
                );
            }
            gl.detach_shader(program, vertex_shader);
            gl.detach_shader(program, fragment_shader);
            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);

            Self { program }
        }
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(Some(self.program));
        }
    }

    pub fn get_program(&self) -> glow::Program {
        self.program
    }
}
