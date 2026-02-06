# Texture2DArray Implementation Plan for RustPixel WGPU Renderer

## Current Architecture Analysis

### Instanced Rendering (Single Draw Call)

Current WGPU implementation already uses instanced rendering to achieve single draw call for all symbols:

```
draw_indexed(0..6, 0, 0..instance_count)
```

All symbols share the same quad geometry (6 vertices), differentiated only by per-instance attributes.

### Current Data Structure

**Shader (shader_source.rs):**
```wgsl
struct InstanceInput {
    @location(1) a1: vec4<f32>,  // origin_x, origin_y, uv_left, uv_top
    @location(2) a2: vec4<f32>,  // uv_width, uv_height, m00*width, m10*width
    @location(3) a3: vec4<f32>,  // m01*height, m11*height, m20, m21
    @location(4) color: vec4<f32>,
}
```

**Rust (render_symbols.rs):**
```rust
pub struct WgpuSymbolInstance {
    pub a1: [f32; 4],
    pub a2: [f32; 4],
    pub a3: [f32; 4],
    pub color: [f32; 4],
}
```

### Current Limitation

- Single 4096x4096 texture atlas
- Not enough capacity for all symbol types:
  - Sprite: 16x16 pixels
  - TUI: 16x32 pixels
  - Emoji: 32x32 pixels
  - CJK: 32x32 pixels (large character set)

---

## Texture2DArray Solution

### Concept

Use `texture_2d_array<f32>` instead of `texture_2d<f32>`. This allows multiple texture layers while maintaining **single draw call**.

### Layer Organization

| Layer | Content | Symbol Size | Capacity |
|-------|---------|-------------|----------|
| 0 | Sprite symbols | 16x16 | 65536 symbols |
| 1 | TUI symbols | 16x32 | 32768 symbols |
| 2 | Emoji | 32x32 | 16384 symbols |
| 3+ | CJK characters | 32x32 | 16384/layer |

Each layer is 4096x4096, can add more layers as needed.

---

## Implementation Changes

### 1. Shader Modification (shader_source.rs)

**Before:**
```wgsl
@group(0) @binding(1)
var source: texture_2d<f32>;
```

**After:**
```wgsl
@group(0) @binding(1)
var source: texture_2d_array<f32>;

struct InstanceInput {
    @location(1) a1: vec4<f32>,
    @location(2) a2: vec4<f32>,
    @location(3) a3: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) tex_layer: u32,  // NEW: texture layer index
}
```

**Fragment Shader Change:**
```wgsl
// Before
let tex_color = textureSample(source, source_sampler, in.uv);

// After
let tex_color = textureSample(source, source_sampler, in.uv, i32(in.tex_layer));
```

**Vertex Shader - Pass layer to fragment:**
```wgsl
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_layer: u32,  // NEW
};

// In vertex main:
out.tex_layer = instance.tex_layer;
```

### 2. Rust Instance Struct (render_symbols.rs)

**Before:**
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgpuSymbolInstance {
    pub a1: [f32; 4],
    pub a2: [f32; 4],
    pub a3: [f32; 4],
    pub color: [f32; 4],
}
```

**After:**
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgpuSymbolInstance {
    pub a1: [f32; 4],
    pub a2: [f32; 4],
    pub a3: [f32; 4],
    pub color: [f32; 4],
    pub tex_layer: u32,
    pub _padding: [u32; 3],  // Align to 16 bytes
}
```

**Update desc() method:**
```rust
impl WgpuSymbolInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuSymbolInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // a1
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // a2
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // a3
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // tex_layer (NEW)
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}
```

### 3. Symbol Frame (render_symbols.rs)

**Add layer field to WgpuSymbolFrame:**
```rust
pub struct WgpuSymbolFrame {
    pub uv: (f32, f32, f32, f32),  // (left, top, width, height)
    pub tex_layer: u32,            // NEW: which texture layer
}
```

### 4. Texture Creation (texture.rs or relevant file)

**Create Texture Array:**
```rust
let texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("Symbol Texture Array"),
    size: wgpu::Extent3d {
        width: 4096,
        height: 4096,
        depth_or_array_layers: layer_count,  // e.g., 4 layers
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8UnormSrgb,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
});

// Create view for array
let view = texture.create_view(&wgpu::TextureViewDescriptor {
    dimension: Some(wgpu::TextureViewDimension::D2Array),
    ..Default::default()
});
```

**Upload to specific layer:**
```rust
queue.write_texture(
    wgpu::ImageCopyTexture {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d {
            x: 0,
            y: 0,
            z: layer_index,  // Which layer to write to
        },
        aspect: wgpu::TextureAspect::All,
    },
    &image_data,
    wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(4096 * 4),
        rows_per_image: Some(4096),
    },
    wgpu::Extent3d {
        width: 4096,
        height: 4096,
        depth_or_array_layers: 1,
    },
);
```

### 5. Symbol Map (symbol_map.rs or relevant file)

**Update symbol lookup to include layer:**
```rust
pub struct SymbolInfo {
    pub uv: (f32, f32, f32, f32),
    pub layer: u32,
}

// When looking up a symbol:
fn get_symbol_info(symbol_type: SymbolType, index: usize) -> SymbolInfo {
    match symbol_type {
        SymbolType::Sprite => SymbolInfo {
            uv: calculate_uv(index, 16, 16),
            layer: 0,
        },
        SymbolType::Tui => SymbolInfo {
            uv: calculate_uv(index, 16, 32),
            layer: 1,
        },
        SymbolType::Emoji => SymbolInfo {
            uv: calculate_uv(index, 32, 32),
            layer: 2,
        },
        SymbolType::Cjk => {
            let (layer, local_index) = calculate_cjk_layer(index);
            SymbolInfo {
                uv: calculate_uv(local_index, 32, 32),
                layer: 3 + layer,
            }
        },
    }
}
```

---

## Implementation Order

1. **shader_source.rs** - Update WGSL shaders
2. **render_symbols.rs** - Update WgpuSymbolInstance and WgpuSymbolFrame
3. **Texture creation** - Switch to texture_2d_array
4. **Symbol loading** - Update to populate different layers
5. **Symbol lookup** - Include layer in frame info

---

## Benefits

- **Single draw call maintained** - No performance regression
- **Massive capacity increase** - Each layer adds 4096x4096 pixels
- **Clean organization** - Each symbol type has dedicated layer
- **Easy extension** - Add more layers as needed for CJK expansion

---

## Notes

- Ensure proper 16-byte alignment for instance struct (using padding)
- Test with `wgpu::Features::TEXTURE_BINDING_ARRAY` if needed (usually not required for basic 2d_array)
- Layer index passed as `u32` in instance data, converted to `i32` for textureSample
