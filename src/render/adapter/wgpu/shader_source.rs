// RustPixel
// copyright zipxing@hotmail.com 2022～2025

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
    @location(1) a1: vec4<f32>,  // origin_x (sign=bold flag), origin_y, uv_left, uv_top
    @location(2) a2: vec4<f32>,  // uv_width, uv_height, m00*width, m10*width
    @location(3) a3: vec4<f32>,  // m01*height, m11*height, m20, m21
    @location(4) color: vec4<f32>, // r, g, b, a
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colorj: vec4<f32>,
    @location(2) v_bold: f32,
    @location(3) v_msdf: f32,
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

    // Extract bold flag from origin_x sign (negative = bold)
    output.v_bold = select(0.0, 1.0, instance.a1.x < 0.0);
    // Extract MSDF flag from origin_y sign (negative = MSDF)
    output.v_msdf = select(0.0, 1.0, instance.a1.y < 0.0);
    let origin = abs(instance.a1.xy);

    // Calculate UV coordinates: uv = a1.zw + position * a2.xy
    output.uv = instance.a1.zw + vertex_input.position * instance.a2.xy;

    // Apply the same transformation chain as OpenGL:
    // transformed = (((vertex - origin) * mat2(a2.zw, a3.xy) + a3.zw) * mat2(tw.xy, th.xy) + vec2(tw.z, th.z)) / vec2(tw.w, th.w) * 2.0
    // gl_Position = vec4(transformed - vec2(1.0, 1.0), 0.0, 1.0);

    // Step 1: vertex - origin
    let vertex_centered = vertex_input.position - origin;

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

    // Step 6: Flip Y coordinate to match WGPU coordinate system (Y-axis up in OpenGL, Y-axis down in WGPU)
    output.clip_position = vec4<f32>(ndc_pos.x, -ndc_pos.y, 0.0, 1.0);

    // Color modulation: colorj = color * colorFilter
    output.colorj = instance.color * transform.colorFilter;

    return output;
}
"#;

/// Instanced fragment shader for symbols (matches OpenGL behavior)
/// Note: Uses textureSampleLevel instead of textureSample in non-uniform control flow
/// to comply with WebGPU uniform control flow requirements for derivative operations.
/// Supports MSDF (Multi-channel Signed Distance Field) for TUI/CJK characters.
pub const SYMBOLS_INSTANCED_FRAGMENT_SHADER: &str = r#"
@group(0) @binding(1)
var source: texture_2d<f32>;
@group(0) @binding(2)
var source_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colorj: vec4<f32>,
    @location(2) v_bold: f32,
    @location(3) v_msdf: f32,
}

fn median3(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var texColor = textureSampleLevel(source, source_sampler, input.uv, 0.0);

    // Precompute all fwidth() calls in uniform control flow (required by WebGPU WGSL)
    let d = median3(texColor.r, texColor.g, texColor.b);
    let w_msdf = max(fwidth(d), 0.03);
    let edge_alpha = fwidth(texColor.a);

    if (input.v_msdf > 0.5) {
        // === MSDF path ===
        var threshold = 0.5;
        if (input.v_bold > 0.5) {
            threshold = 0.45; // Bold: lower threshold to expand glyph
        }
        let alpha = smoothstep(threshold - w_msdf, threshold + w_msdf, d);
        return vec4<f32>(input.colorj.rgb, input.colorj.a * alpha);
    } else {
        // === Bitmap path (existing logic) ===
        if (input.v_bold > 0.5) {
            let ts = vec2<f32>(textureDimensions(source));
            let dx = 0.35 / ts.x;
            texColor = max(texColor, textureSampleLevel(source, source_sampler, input.uv + vec2<f32>(dx, 0.0), 0.0));
            texColor = max(texColor, textureSampleLevel(source, source_sampler, input.uv + vec2<f32>(-dx, 0.0), 0.0));
            texColor.a = smoothstep(0.15, 0.95, texColor.a);
        }
        // Alpha edge sharpening
        if (edge_alpha > 0.001) {
            texColor.a = smoothstep(0.5 - edge_alpha, 0.5 + edge_alpha, texColor.a);
        }
        return texColor * input.colorj;
    }
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
    params: vec4<f32>,   // [sharpness, 0, 0, 0]
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

// Fragment shader with RCAS (Robust Contrast-Adaptive Sharpening)
// Based on AMD FidelityFX CAS, uses luma-based weighting to avoid color fringing
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(texture_input, texture_sampler, input.tex_coords);
    var result = tex_color;

    let sharpness = uniforms.params.x;
    if (sharpness > 0.0) {
        let tex_size = vec2<f32>(textureDimensions(texture_input));
        let texel = 1.0 / tex_size;

        // Sample 4 neighbors (cross pattern)
        let n = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(0.0, -texel.y));
        let s = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(0.0, texel.y));
        let e = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(texel.x, 0.0));
        let w = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(-texel.x, 0.0));

        // RCAS: compute luma for perceptually-correct sharpening
        let luma = vec3<f32>(0.299, 0.587, 0.114);
        let luma_n = dot(n.rgb, luma);
        let luma_s = dot(s.rgb, luma);
        let luma_e = dot(e.rgb, luma);
        let luma_w = dot(w.rgb, luma);
        let luma_c = dot(tex_color.rgb, luma);

        // Local contrast via luma min/max
        let mn = min(min(luma_n, luma_s), min(luma_e, luma_w));
        let mx = max(max(luma_n, luma_s), max(luma_e, luma_w));

        // Peak-limited weight to prevent over-sharpening
        // mix(8,5) maps sharpness-adaptive range; tighter limit in high-contrast
        let peak = -1.0 / mix(8.0, 5.0, clamp(min(mn, 1.0 - mx), 0.0, 1.0));
        let wt = clamp(peak * (luma_n + luma_s + luma_e + luma_w - 4.0 * luma_c), -0.25, 0.0) * sharpness;
        let weight = 1.0 / (1.0 + 4.0 * wt);

        // Apply: weighted blend of neighbors and center
        result = vec4<f32>(
            clamp((wt * (n.rgb + s.rgb + e.rgb + w.rgb) + tex_color.rgb) * weight, vec3<f32>(0.0), vec3<f32>(1.0)),
            tex_color.a
        );
    }

    return result * uniforms.color;
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
    params: vec4<f32>,   // [sharpness, 0, 0, 0]
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
    var result = tex_color;

    let sharpness = uniforms.params.x;
    if (sharpness > 0.0) {
        let tex_size = vec2<f32>(textureDimensions(texture_input));
        let texel = 1.0 / tex_size;
        let n = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(0.0, -texel.y));
        let s = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(0.0, texel.y));
        let e = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(texel.x, 0.0));
        let w = textureSample(texture_input, texture_sampler, input.tex_coords + vec2<f32>(-texel.x, 0.0));
        let mn = min(min(n, s), min(e, w));
        let mx = max(max(n, s), max(e, w));
        let d = mx - mn;
        let amp = clamp(vec4<f32>(1.0) - d, vec4<f32>(0.0), vec4<f32>(1.0)) * sharpness;
        let avg = (n + s + e + w) * 0.25;
        result = clamp(tex_color + (tex_color - avg) * amp, vec4<f32>(0.0), vec4<f32>(1.0));
    }

    return result * uniforms.color;
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
    // 翻转Y坐标以适配WGPU坐标系（Y轴向下）
    let flipped_p = vec2<f32>(p.x, 1.0 - p.y);
    let flipped_center = vec2<f32>(center.x, 1.0 - center.y);
    let o = (flipped_p - flipped_center) / (1.6 * size);
    let a = o.x * o.x + o.y * o.y - 0.3;
    return step(a * a * a, o.x * o.x * o.y * o.y * o.y);
}

fn transition(uv: vec2<f32>) -> vec4<f32> {
    return mix(
        getFromColor(uv),
        getToColor(uv),
        inHeart(uv, vec2<f32>(0.5, 0.4), uniforms.progress)  // Restore original value, flip handled internally in inHeart
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

const GRID: f32 = 16.0;
const RADIUS: f32 = 0.5;

fn hash(co: vec2<f32>) -> f32 {
    let dt = dot(co, vec2<f32>(12.9898, 78.233));
    return fract(sin(dt % 3.14159265) * 43758.5453);
}

fn transition(p: vec2<f32>) -> vec4<f32> {
    let cell = floor(p * GRID);
    let local = fract(p * GRID) - 0.5;
    let dist = length(local);
    let n = hash(cell);
    let grow = smoothstep(n - 0.1, n + 0.1, uniforms.progress * 1.2 - 0.1);
    let circle = 1.0 - smoothstep(RADIUS * grow - 0.05, RADIUS * grow, dist);
    return mix(getFromColor(p), getToColor(p), circle);
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
    let phase = select((uniforms.progress - 0.5) * 2.0, uniforms.progress * 2.0, uniforms.progress < 0.5);
    let angleOffset = select(mix(-6.0 * 0.03927, 0.0, phase), mix(0.0, 6.0 * 0.03927, phase), uniforms.progress < 0.5);
    let newScale = select(mix(1.2, 1.0, phase), mix(1.0, 1.2, phase), uniforms.progress < 0.5);
    let center = vec2<f32>(0.0, 0.0);
    let assumedCenter = vec2<f32>(0.5, 0.5);
    var p = (uv - vec2<f32>(0.5, 0.5)) / newScale * vec2<f32>(1.2, 1.0);
    let angle = atan2(p.y, p.x) + angleOffset;
    let dist = distance(center, p);
    p.x = cos(angle) * dist / 1.2 + 0.5;
    p.y = sin(angle) * dist + 0.5;
    let c = select(getToColor(p), getFromColor(p), uniforms.progress < 0.5);
    
    // Fix color value calculation to avoid exceeding [0,1] range causing all white
    let brightness_factor = select(mix(1.0, 0.0, phase), mix(0.0, 1.0, phase), uniforms.progress < 0.5);
    return mix(c, vec4<f32>(1.0, 1.0, 1.0, 1.0), brightness_factor * 0.3); // Limit brightness enhancement intensity
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
    // Use GL-style y (1.0 - uv.y) for boundary check, flip scroll direction for WGPU UV
    let d = (1.0 - uv.y) - y;
    let from_color = getFromColor(vec2<f32>(uv.x, uv.y - (1.0 - y)));
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
    let offset = dir * sin(dist * amplitude - uniforms.progress * speed);
    return mix(getFromColor(p + offset), getToColor(p), uniforms.progress);
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