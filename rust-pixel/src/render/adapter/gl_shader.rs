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

// pub struct GlShader {
//     pub core: GlShaderCore,
//     pub uniforms: HashMap<String, GlUniformValue>,
// }

// impl GlShader {
//     pub fn new(core: GlShaderCore, uniforms: HashMap<String, GlUniformValue>) -> Self {
//         Self { core, uniforms }
//     }

//     pub fn bind(&mut self, gl: &glow::Context) {
//         self.core.bind(gl);
//         for (name, uniform) in &self.uniforms {
//             let location = unsafe { gl.get_uniform_location(self.core.get_program(), name) };
//             if let Some(loc) = location {
//                 match uniform {
//                     GlUniformValue::Int(value) => unsafe {
//                         gl.uniform_1_i32(Some(&loc), *value);
//                     },
//                     GlUniformValue::Float(value) => unsafe {
//                         gl.uniform_1_f32(Some(&loc), *value);
//                     },
//                     GlUniformValue::Vec2(value) => unsafe {
//                         gl.uniform_2_f32_slice(Some(&loc), value);
//                     },
//                     GlUniformValue::Vec3(value) => unsafe {
//                         gl.uniform_3_f32_slice(Some(&loc), value);
//                     },
//                     GlUniformValue::Vec4(value) => unsafe {
//                         gl.uniform_4_f32_slice(Some(&loc), value);
//                     },
//                     GlUniformValue::Mat2(value) => unsafe {
//                         gl.uniform_matrix_2_f32_slice(Some(&loc), false, value);
//                     },
//                     GlUniformValue::Mat3(value) => unsafe {
//                         gl.uniform_matrix_3_f32_slice(Some(&loc), false, value);
//                     },
//                     GlUniformValue::Mat4(value) => unsafe {
//                         gl.uniform_matrix_4_f32_slice(Some(&loc), false, value);
//                     },
//                     // other
//                 }
//             }
//         }
//     }

//     pub fn set_uniform(&mut self, name: &str, value: GlUniformValue) {
//         self.uniforms.insert(name.to_string(), value);
//     }
// }

// pub enum GlUniformValue {
//     Int(i32),
//     Float(f32),
//     Vec2([f32; 2]),
//     Vec3([f32; 3]),
//     Vec4([f32; 4]),
//     Mat2([f32; 4]),
//     Mat3([f32; 9]),
//     Mat4([f32; 16]),
// }
