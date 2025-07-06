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
fn vs_main(vertex_input: VertexInput, instance: InstanceInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Calculate UV coordinates: uv = a1.zw + position * a2.xy
    output.uv = instance.a1.zw + vertex_input.position * instance.a2.xy;
    
    // Apply the exact same transformation as OpenGL:
    // vec2 transformed = (((vertex - a1.xy) * mat2(a2.zw, a3.xy) + a3.zw) * mat2(tw.xy, th.xy) + vec2(tw.z, th.z)) / vec2(tw.w, th.w) * 2.0;
    
    // Step 1: vertex - a1.xy
    let step1 = vertex_input.position - instance.a1.xy;
    
    // Step 2: step1 * mat2(a2.zw, a3.xy)
    let mat2_1 = mat2x2<f32>(instance.a2.zw, instance.a3.xy);
    let step2 = mat2_1 * step1;
    
    // Step 3: step2 + a3.zw
    let step3 = step2 + instance.a3.zw;
    
    // Step 4: step3 * mat2(tw.xy, th.xy)
    let mat2_2 = mat2x2<f32>(uniforms.transform.tw.xy, uniforms.transform.th.xy);
    let step4 = mat2_2 * step3;
    
    // Step 5: step4 + vec2(tw.z, th.z)
    let step5 = step4 + vec2<f32>(uniforms.transform.tw.z, uniforms.transform.th.z);
    
    // Step 6: step5 / vec2(tw.w, th.w) * 2.0
    let step6 = (step5 / vec2<f32>(uniforms.transform.tw.w, uniforms.transform.th.w)) * 2.0;
    
    // Step 7: final transform - vec2(1.0, 1.0)
    let transformed = step6 - vec2<f32>(1.0, 1.0);
    
    output.clip_position = vec4<f32>(transformed, 0.0, 1.0);
    
    // Color modulation: colorj = color * colorFilter
    output.colorj = instance.color * uniforms.transform.colorFilter;
    
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

/// Vertex shader for pixel rendering with uniform transformations
pub const PIXEL_UNIFORM_VERTEX_SHADER: &str = r#"
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

struct Uniforms {
    transform: mat4x4<f32>,
    color_filter: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Apply uniform transformation
    output.clip_position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    // 暂时去掉colorFilter，直接使用原始颜色
    output.color = input.color;
    
    return output;
}
"#;

/// Fragment shader for pixel rendering with texture sampling
pub const PIXEL_TEXTURE_FRAGMENT_SHADER: &str = r#"
@group(0) @binding(1)
var t_symbols: texture_2d<f32>;
@group(0) @binding(2)
var s_symbols: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_symbols, s_symbols, input.tex_coords);
    
    if (tex_color.a < 0.1) {
        discard;
    }
    
    return tex_color * input.color;
}
"#;

/// Instanced vertex shader for symbols (matches OpenGL behavior exactly)
pub const SYMBOLS_INSTANCED_VERTEX_SHADER: &str = r#"
// Vertex input (base quad geometry)
struct VertexInput {
    @location(0) position: vec2<f32>,
}

// Instance input (per-symbol data, matches OpenGL layout)
struct InstanceInput {
    @location(1) a1: vec4<f32>,  // origin_x, origin_y, uv_left, uv_top
    @location(2) a2: vec4<f32>,  // uv_width, uv_height, m00*width, m10*height
    @location(3) a3: vec4<f32>,  // m01*width, m11*height, m20, m21
    @location(4) color: vec4<f32>, // r, g, b, a
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colorj: vec4<f32>,
}

// Transform uniform (matches OpenGL layout)
struct Transform {
    tw: vec4<f32>,      // [m00, m10, m20, canvas_width]
    th: vec4<f32>,      // [m01, m11, m21, canvas_height]
    colorFilter: vec4<f32>, // [r, g, b, a]
}

@group(0) @binding(0)
var<uniform> transform: Transform;

@vertex
fn vs_main(vertex_input: VertexInput, instance: InstanceInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Calculate UV coordinates: uv = a1.zw + position * a2.xy
    output.uv = instance.a1.zw + vertex_input.position * instance.a2.xy;
    
    // Apply the same transformation chain as OpenGL:
    // transformed = (((vertex - a1.xy) * mat2(a2.zw, a3.xy) + a3.zw) * mat2(tw.xy, th.xy) + vec2(tw.z, th.z)) / vec2(tw.w, th.w) * 2.0
    // gl_Position = vec4(transformed - vec2(1.0, 1.0), 0.0, 1.0);
    
    // Step 1: vertex - a1.xy
    let vertex_centered = vertex_input.position - instance.a1.xy;
    
    // Step 2: * mat2(a2.zw, a3.xy) + a3.zw
    let transform_matrix = mat2x2<f32>(instance.a2.zw, instance.a3.xy);
    let transformed_local = transform_matrix * vertex_centered + instance.a3.zw;
    
    // Step 3: * mat2(tw.xy, th.xy) + vec2(tw.z, th.z)
    let global_matrix = mat2x2<f32>(transform.tw.xy, transform.th.xy);
    let transformed_global = global_matrix * transformed_local + vec2<f32>(transform.tw.z, transform.th.z);
    
    // Step 4: / vec2(tw.w, th.w) * 2.0
    let normalized = (transformed_global / vec2<f32>(transform.tw.w, transform.th.w)) * 2.0;
    
    // Step 5: - vec2(1.0, 1.0) to convert to NDC coordinates
    let ndc_pos = normalized - vec2<f32>(1.0, 1.0);
    
    output.clip_position = vec4<f32>(ndc_pos, 0.0, 1.0);
    
    // Color modulation: colorj = color * colorFilter
    output.colorj = instance.color * transform.colorFilter;
    
    return output;
}
"#;

/// Instanced fragment shader for symbols (matches OpenGL behavior)
pub const SYMBOLS_INSTANCED_FRAGMENT_SHADER: &str = r#"
@group(0) @binding(1)
var source: texture_2d<f32>;
@group(0) @binding(2)
var source_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colorj: vec4<f32>,
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(source, source_sampler, input.uv);
    return tex_color * input.colorj;
}
"#;

// Transition shaders (converted from OpenGL GLSL to WGSL)

// Complete General2D shader combining vertex and fragment stages
pub const GENERAL2D_SHADER: &str = r#"
// Common structures and uniforms
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct General2dUniforms {
    transform: mat4x4<f32>,
    area: vec4<f32>,     // [x, y, width, height]
    color: vec4<f32>,    // [r, g, b, a]
}

@group(0) @binding(0)
var<uniform> uniforms: General2dUniforms;

@group(0) @binding(1)
var texture_input: texture_2d<f32>;

@group(0) @binding(2)
var texture_sampler: sampler;

// Vertex shader
@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply area mapping to texture coordinates
    out.tex_coords = vec2<f32>(
        mix(uniforms.area.x, uniforms.area.x + uniforms.area.z, vertex.tex_coords.x),
        mix(uniforms.area.y, uniforms.area.y + uniforms.area.w, vertex.tex_coords.y)
    );
    
    // Apply transform matrix to vertex position
    out.clip_position = uniforms.transform * vec4<f32>(vertex.position, 0.0, 1.0);
    
    return out;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(texture_input, texture_sampler, input.tex_coords);
    return tex_color * uniforms.color;
}
"#;

// For backward compatibility, provide separate vertex and fragment shaders
// (these are now just parts of the complete shader above)
pub const GENERAL2D_VERTEX_SRC: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct General2dUniforms {
    transform: mat4x4<f32>,
    area: vec4<f32>,     // [x, y, width, height]
    color: vec4<f32>,    // [r, g, b, a]
}

@group(0) @binding(0)
var<uniform> uniforms: General2dUniforms;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply area mapping to texture coordinates
    out.tex_coords = vec2<f32>(
        mix(uniforms.area.x, uniforms.area.x + uniforms.area.z, vertex.tex_coords.x),
        mix(uniforms.area.y, uniforms.area.y + uniforms.area.w, vertex.tex_coords.y)
    );
    
    // Apply transform matrix to vertex position
    out.clip_position = uniforms.transform * vec4<f32>(vertex.position, 0.0, 1.0);
    
    return out;
}
"#;

// Fragment shader (without duplicate definitions)
pub const GENERAL2D_FRAGMENT_SRC: &str = r#"
@group(0) @binding(1)
var texture_input: texture_2d<f32>;

@group(0) @binding(2)
var texture_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(texture_input, texture_sampler, input.tex_coords);
    return tex_color * uniforms.color;
}
"#;

// Transition effect shaders (converted from OpenGL GLSL to WGSL)
// These support 7 different transition effects matching the OpenGL version

// Common transition vertex shader
pub const TRANSITION_VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}
"#;

// Transition 0: Squares effect
pub const TRANSITION_SQUARES_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct TransitionUniforms {
    progress: f32,
    _padding1: vec3<f32>,
    _padding2: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TransitionUniforms;

@group(0) @binding(1)
var texture1: texture_2d<f32>;

@group(0) @binding(2)
var texture2: texture_2d<f32>;

@group(0) @binding(3)
var texture_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}

fn getFromColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture1, texture_sampler, uv);
}

fn getToColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture2, texture_sampler, uv);
}

fn transition(uv: vec2<f32>) -> vec4<f32> {
    let squaresMin = vec2<i32>(20, 20);
    let steps = 50;
    let d = min(uniforms.progress, 1.0 - uniforms.progress);
    let dist = select(d, ceil(d * f32(steps)) / f32(steps), steps > 0);
    let squareSize = 2.0 * dist / vec2<f32>(squaresMin);
    let p = select(uv, (floor(uv / squareSize) + 0.5) * squareSize, dist > 0.0);
    return mix(getFromColor(p), getToColor(p), uniforms.progress);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return transition(input.tex_coords);
}
"#;

// Transition 1: Heart effect
pub const TRANSITION_HEART_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct TransitionUniforms {
    progress: f32,
    _padding1: vec3<f32>,
    _padding2: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TransitionUniforms;

@group(0) @binding(1)
var texture1: texture_2d<f32>;

@group(0) @binding(2)
var texture2: texture_2d<f32>;

@group(0) @binding(3)
var texture_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}

fn getFromColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture1, texture_sampler, uv);
}

fn getToColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture2, texture_sampler, uv);
}

fn inHeart(p: vec2<f32>, center: vec2<f32>, size: f32) -> f32 {
    if (size == 0.0) { return 0.0; }
    let o = (p - center) / (1.6 * size);
    let a = o.x * o.x + o.y * o.y - 0.3;
    return step(a * a * a, o.x * o.x * o.y * o.y * o.y);
}

fn transition(uv: vec2<f32>) -> vec4<f32> {
    return mix(
        getFromColor(uv),
        getToColor(uv),
        inHeart(uv, vec2<f32>(0.5, 0.4), uniforms.progress)
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return transition(input.tex_coords);
}
"#;

// Transition 2: Noise effect
pub const TRANSITION_NOISE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct TransitionUniforms {
    progress: f32,
    _padding1: vec3<f32>,
    _padding2: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TransitionUniforms;

@group(0) @binding(1)
var texture1: texture_2d<f32>;

@group(0) @binding(2)
var texture2: texture_2d<f32>;

@group(0) @binding(3)
var texture_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}

fn getFromColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture1, texture_sampler, uv);
}

fn getToColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture2, texture_sampler, uv);
}

fn noise(co: vec2<f32>) -> f32 {
    let a = 12.9898;
    let b = 78.233;
    let c = 43758.5453;
    let dt = dot(co * uniforms.progress, vec2<f32>(a, b));
    let sn = dt % 3.14159265;
    return fract(sin(sn) * c);
}

fn transition(p: vec2<f32>) -> vec4<f32> {
    if (uniforms.progress < 0.05) {
        return getFromColor(p);
    } else if (uniforms.progress > (1.0 - 0.05)) {
        return getToColor(p);
    } else {
        return vec4<f32>(vec3<f32>(noise(p)), 1.0);
    }
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return transition(input.tex_coords);
}
"#;

// Transition 3: Zoom rotate effect
pub const TRANSITION_ZOOM_ROTATE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct TransitionUniforms {
    progress: f32,
    _padding1: vec3<f32>,
    _padding2: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TransitionUniforms;

@group(0) @binding(1)
var texture1: texture_2d<f32>;

@group(0) @binding(2)
var texture2: texture_2d<f32>;

@group(0) @binding(3)
var texture_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}

fn getFromColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture1, texture_sampler, uv);
}

fn getToColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture2, texture_sampler, uv);
}

fn transition(uv: vec2<f32>) -> vec4<f32> {
    let phase = select(uniforms.progress * 2.0, (uniforms.progress - 0.5) * 2.0, uniforms.progress < 0.5);
    let angleOffset = select(mix(0.0, 6.0 * 0.03927, phase), mix(-6.0 * 0.03927, 0.0, phase), uniforms.progress < 0.5);
    let newScale = select(mix(1.0, 1.2, phase), mix(1.2, 1.0, phase), uniforms.progress < 0.5);
    let center = vec2<f32>(0.0, 0.0);
    let assumedCenter = vec2<f32>(0.5, 0.5);
    var p = (uv - vec2<f32>(0.5, 0.5)) / newScale * vec2<f32>(1.2, 1.0);
    let angle = atan2(p.y, p.x) + angleOffset;
    let dist = distance(center, p);
    p.x = cos(angle) * dist / 1.2 + 0.5;
    p.y = sin(angle) * dist + 0.5;
    let c = select(getFromColor(p), getToColor(p), uniforms.progress < 0.5);
    return c + select(mix(0.0, 1.0, phase), mix(1.0, 0.0, phase), uniforms.progress < 0.5);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return transition(input.tex_coords);
}
"#;

// Transition 4: Wave effect
pub const TRANSITION_WAVE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct TransitionUniforms {
    progress: f32,
    _padding1: vec3<f32>,
    _padding2: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TransitionUniforms;

@group(0) @binding(1)
var texture1: texture_2d<f32>;

@group(0) @binding(2)
var texture2: texture_2d<f32>;

@group(0) @binding(3)
var texture_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}

fn getFromColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture1, texture_sampler, uv);
}

fn getToColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture2, texture_sampler, uv);
}

fn transition(uv: vec2<f32>) -> vec4<f32> {
    let time = uniforms.progress;
    let stime = sin(time * 3.14159265 / 2.0);
    let phase = time * 3.14159265 * 3.0;
    let y = abs(cos(phase)) * (1.0 - stime);
    let d = uv.y - y;
    let from_color = getFromColor(vec2<f32>(uv.x, uv.y + (1.0 - y)));
    let to_color = getToColor(uv);
    return mix(to_color, from_color, step(d, 0.0));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return transition(input.tex_coords);
}
"#;

// Transition 5: Distortion effect
pub const TRANSITION_DISTORTION_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct TransitionUniforms {
    progress: f32,
    _padding1: vec3<f32>,
    _padding2: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TransitionUniforms;

@group(0) @binding(1)
var texture1: texture_2d<f32>;

@group(0) @binding(2)
var texture2: texture_2d<f32>;

@group(0) @binding(3)
var texture_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}

fn getFromColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture1, texture_sampler, uv);
}

fn getToColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture2, texture_sampler, uv);
}

fn transition(p: vec2<f32>) -> vec4<f32> {
    let size = 0.04;
    let zoom = 50.0;
    let colorSeparation = 0.3;
    
    let inv = 1.0 - uniforms.progress;
    let disp = size * vec2<f32>(cos(zoom * p.x), sin(zoom * p.y));
    let texTo = getToColor(p + inv * disp);
    let texFrom = vec4<f32>(
        getFromColor(p + uniforms.progress * disp * (1.0 - colorSeparation)).r,
        getFromColor(p + uniforms.progress * disp).g,
        getFromColor(p + uniforms.progress * disp * (1.0 + colorSeparation)).b,
        1.0
    );
    return texTo * uniforms.progress + texFrom * inv;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return transition(input.tex_coords);
}
"#;

// Transition 6: Ripple effect
pub const TRANSITION_RIPPLE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct TransitionUniforms {
    progress: f32,
    _padding1: vec3<f32>,
    _padding2: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TransitionUniforms;

@group(0) @binding(1)
var texture1: texture_2d<f32>;

@group(0) @binding(2)
var texture2: texture_2d<f32>;

@group(0) @binding(3)
var texture_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(vertex.position, 0.0, 1.0);
    output.tex_coords = vertex.tex_coords;
    return output;
}

fn getFromColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture1, texture_sampler, uv);
}

fn getToColor(uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture2, texture_sampler, uv);
}

fn transition(p: vec2<f32>) -> vec4<f32> {
    let amplitude = 30.0;
    let speed = 30.0;
    
    let dir = p - vec2<f32>(0.5);
    let dist = length(dir);
    
    if (dist > uniforms.progress) {
        return mix(getFromColor(p), getToColor(p), uniforms.progress);
    } else {
        let offset = dir * sin(dist * amplitude - uniforms.progress * speed);
        return mix(getFromColor(p + offset), getToColor(p), uniforms.progress);
    }
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return transition(input.tex_coords);
}
"#;

/// Get all transition shader sources
/// Returns a vector of 7 transition shader sources
pub fn get_transition_shaders() -> Vec<&'static str> {
    vec![
        TRANSITION_SQUARES_SHADER,
        TRANSITION_HEART_SHADER,
        TRANSITION_NOISE_SHADER,
        TRANSITION_ZOOM_ROTATE_SHADER,
        TRANSITION_WAVE_SHADER,
        TRANSITION_DISTORTION_SHADER,
        TRANSITION_RIPPLE_SHADER,
    ]
} 