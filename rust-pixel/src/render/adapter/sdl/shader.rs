use std::collections::HashMap;
use glow::HasContext;

pub struct ShaderCore {
    pub program: glow::NativeProgram,
}

impl ShaderCore {
    pub fn new(gl: &glow::Context, vertex_source: &str, fragment_source: &str) -> Self {
        unsafe {
            let vertex_shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            gl.shader_source(vertex_shader, vertex_source);
            gl.compile_shader(vertex_shader);
            if !gl.get_shader_compile_status(vertex_shader) {
                panic!("Vertex Shader Compilation Error: {}", gl.get_shader_info_log(vertex_shader));
            }

            let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            gl.shader_source(fragment_shader, fragment_source);
            gl.compile_shader(fragment_shader);
            if !gl.get_shader_compile_status(fragment_shader) {
                panic!(
                    "Fragment Shader Compilation Error: {}",
                    gl.get_shader_info_log(fragment_shader)
                );
            }

            let program = gl.create_program().unwrap();
            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("Program Linking Error: {}", gl.get_program_info_log(program));
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

    pub fn get_program(&self) -> glow::NativeProgram {
        self.program
    }
}

pub struct Shader {
    pub core: ShaderCore,
    pub uniforms: HashMap<String, UniformValue>,
}

impl Shader {
    pub fn new(core: ShaderCore, uniforms: HashMap<String, UniformValue>) -> Self {
        Self { core, uniforms }
    }

    pub fn bind(&mut self, gl: &glow::Context) {
        self.core.bind(gl);
        for (name, uniform) in &self.uniforms {
            let location = unsafe { gl.get_uniform_location(self.core.get_program(), name) };
            if let Some(loc) = location {
                match uniform {
                    UniformValue::Int(value) => unsafe {
                        gl.uniform_1_i32(Some(&loc), *value);
                    },
                    UniformValue::Float(value) => unsafe {
                        gl.uniform_1_f32(Some(&loc), *value);
                    },
                    // 添加其他类型的 UniformValue
                }
            }
        }
    }

    pub fn set_uniform(&mut self, name: &str, value: UniformValue) {
        self.uniforms.insert(name.to_string(), value);
    }
}

pub enum UniformValue {
    Int(i32),
    Float(f32),
    // 添加其他类型
}

