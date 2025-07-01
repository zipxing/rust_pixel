// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU WGSL Shader Source Module
//! 
//! Contains WGSL shader source code for all RustPixel rendering operations.

/// Basic vertex shader for pixel rendering (simplified - no uniforms)
pub const PIXEL_VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // Simple pass-through, assuming positions are already in clip space
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    output.color = input.color;
    return output;
}
"#;

/// Basic fragment shader for pixel rendering (simplified - no textures)
pub const PIXEL_FRAGMENT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simply return the vertex color without texture sampling
    return input.color;
}
"#;

/// Vertex shader for symbol rendering with instancing
pub const SYMBOLS_VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct InstanceInput {
    @location(2) instance_position: vec2<f32>,
    @location(3) instance_scale: vec2<f32>,
    @location(4) instance_color: vec4<f32>,
    @location(5) tex_offset: vec2<f32>,
    @location(6) tex_size: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct Uniforms {
    transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var output: VertexOutput;
    
    let scaled_pos = vertex.position * instance.instance_scale;
    let world_pos = scaled_pos + instance.instance_position;
    
    output.clip_position = uniforms.transform * vec4<f32>(world_pos, 0.0, 1.0);
    output.tex_coords = instance.tex_offset + vertex.tex_coords * instance.tex_size;
    output.color = instance.instance_color;
    
    return output;
}
"#;

/// Fragment shader for symbol rendering
pub const SYMBOLS_FRAGMENT_SHADER: &str = r#"
@group(0) @binding(1)
var t_symbols: texture_2d<f32>;
@group(0) @binding(2)
var s_symbols: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_symbols, s_symbols, input.tex_coords);
    if (tex_color.a < 0.1) {
        discard;
    }
    return tex_color * input.color;
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}
"#; 