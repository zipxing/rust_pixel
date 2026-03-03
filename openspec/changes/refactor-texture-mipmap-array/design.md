# 设计文档：纹理系统重构 — Mipmap Bitmap + Texture2DArray

## Context

RustPixel 的 GPU 渲染管线使用单张 8192×8192 纹理图集存储所有符号（Sprite/TUI/Emoji/CJK），通过实例化渲染在单次 draw call 中渲染所有 Cell。TUI 和 CJK 字符使用 MSDF/SDF 距离场来支持任意缩放，但实际效果不佳——边缘模糊、毛刺明显，不如对应分辨率的 bitmap 直接渲染。

**约束条件：**
- 必须保持单 draw call 渲染（性能关键）
- 必须支持 WebGPU（WASM 目标）
- App 层 API 保持不变（`set_graph_sym()`、`set_symbol()` 等对外接口无感知）
- 先全量预加载，动态加载后续迭代

## Goals / Non-Goals

**Goals:**
- 去除 SDF/MSDF，全部使用多分辨率 bitmap 渲染
- 用 Texture2DArray (多张 2048×2048) 替代单张 8192×8192
- CPU 端 mipmap 选择，根据屏幕像素大小选择最佳分辨率
- 减少 GPU 内存占用（从 ~256MB 降至 ~64-80MB）
- 提升渲染质量（bitmap 在目标分辨率下比 SDF 更清晰）

**Non-Goals:**
- 不改动 App 层 API（`set_graph_sym()`、`set_symbol()` 等对外接口不变）
- 不实现运行时动态加载（全量预加载）
- 不改名 Cell → Tile

## Decisions

### Decision 1: Texture2DArray 替代单张大图

**选择：** 使用 `wgpu::TextureDimension::D2` + `depth_or_array_layers > 1` 创建 Texture2DArray

**理由：**
- 保持单 draw call（layer index 作为 per-instance attribute 传入 vertex shader）
- 2048×2048 每层 = 16MB RGBA，典型 app 需 3-5 层 = 48-80MB
- WebGPU 原生支持 `texture_2d_array<f32>`（WebGL2 也支持 `TEXTURE_2D_ARRAY`）
- 比多个独立 texture bind group 简单，无需切换绑定

**替代方案：**
1. 保持单张大图 + 去 SDF → 质量提升但内存不改善 ❌
2. 多独立纹理 + 多 draw call → 性能下降 ❌
3. 虚拟纹理 (Sparse Texture) → 过于复杂，不适合 2D 引擎 ❌

**创建方式：**
```rust
pub struct WgpuTextureArray {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub layer_count: u32,
    pub width: u32,   // 2048
    pub height: u32,  // 2048
}

// 创建时
let texture = device.create_texture(&wgpu::TextureDescriptor {
    size: wgpu::Extent3d {
        width: 2048,
        height: 2048,
        depth_or_array_layers: layer_count,
    },
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8Unorm,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    mip_level_count: 1,  // 不使用 GPU mipmap，CPU 端选择
    sample_count: 1,
    ..
});

// View 必须指定 D2Array 维度
let view = texture.create_view(&wgpu::TextureViewDescriptor {
    dimension: Some(wgpu::TextureViewDimension::D2Array),
    ..
});
```

### Decision 2: Mipmap 级别设计

**选择：** 根据符号类型定义不同的 mipmap 级别

| 符号类型 | Level 0 (高) | Level 1 (中) | Level 2 (低) |
|---------|-------------|-------------|-------------|
| Sprite (1×1) | 64×64 | 32×32 | 16×16 |
| TUI (1×2) | 64×128 | 32×64 | 16×32 |
| Emoji (2×2) | 128×128 | 64×64 | 32×32 |
| CJK (2×2) | 128×128 | 64×64 | 32×32 |

**理由：**
- 所有符号类型统一为 3 级 mipmap，架构一致
- Level 0 提供高分辨率质量（5K 全屏场景下 Sprite 约 48 物理像素/cell，64×64 提供下采样质量）
- Level 1 匹配当前基础分辨率（兼容现有视觉效果）
- Level 2 用于极小缩放或远距离渲染

**所有 mipmap level 的符号混合打包到同一组 Texture2DArray layers 中**，通过 DP 优化的 shelf-packing 算法最小化层数（详见 Decision 8）。

**基准单元：PIXEL_SYMBOL_SIZE = 16**

Mipmap 倍率以 16px 为基准单元（1 base unit = 16px），各级别像素尺寸 = cell_size × mip_scale × 16：

| 类型 | cell | mip0 (×4) | mip1 (×2) | mip2 (×1) |
|------|------|-----------|-----------|-----------|
| Sprite (1×1) | 1×1 | 4×4 (64px) | 2×2 (32px) | 1×1 (16px) |
| TUI (1×2) | 1×2 | 4×8 (64×128) | 2×4 (32×64) | 1×2 (16×32) |
| Emoji (2×2) | 2×2 | 8×8 (128px) | 4×4 (64px) | 2×2 (32px) |
| CJK (2×2) | 2×2 | 8×8 (128px) | 4×4 (64px) | 2×2 (32px) |

旧架构中 `PIXEL_SYM_WIDTH = texture_width / 256`（动态计算），新架构中改为固定常量 `PIXEL_SYMBOL_SIZE = 16`，布局代码使用 `PIXEL_SYM_WIDTH = PIXEL_SYMBOL_SIZE * 2`（32.0）保持与现有 cell_width()/cell_height() 接口兼容。

### Decision 3: CPU 端 Mipmap 选择

**选择：** 在 `WgpuSymbolRenderer::generate_instances_from_render_cells()` 中选择 mipmap level

**理由：**
- 此时已知每个 cell 的最终屏幕像素大小（`RenderCell.w` × `RenderCell.h` × viewport scale）
- 简单的阈值判断即可，无需 GPU 计算
- 每个 cell 可独立选择不同的 mipmap level（比 GPU 统一 mipmap 更灵活）

**选择逻辑：**
```rust
fn select_mip_level(screen_pixel_h: f32, sym_type: SymbolType) -> u8 {
    match sym_type {
        SymbolType::Sprite => {
            if screen_pixel_h >= 48.0 { 0 }       // 64×64
            else if screen_pixel_h >= 24.0 { 1 }   // 32×32
            else { 2 }                              // 16×16
        }
        SymbolType::Tui => {
            if screen_pixel_h >= 96.0 { 0 }  // 64×128
            else if screen_pixel_h >= 48.0 { 1 } // 32×64
            else { 2 }                        // 16×32
        }
        SymbolType::Emoji | SymbolType::Cjk => {
            if screen_pixel_h >= 96.0 { 0 }  // 128×128
            else if screen_pixel_h >= 48.0 { 1 } // 64×64
            else { 2 }                        // 32×32
        }
    }
}
```

### Decision 4: Tile 缓存解析后的纹理坐标

**选择：** Glyph 改名为 Tile，不再存储 `(block, idx)`，改为缓存 3 级 mipmap 的 `(layer, uv)` 纹理坐标

**当前 Glyph（4 bytes，将改名为 Tile）：**
```rust
pub struct Glyph {
    pub block: u8,    // 纹理 block 索引
    pub idx: u8,      // block 内符号索引
    pub width: u8,    // cell 宽度 (1 or 2)
    pub height: u8,   // cell 高度 (1 or 2)
}
```

**新 Tile（~56 bytes）：**
```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct MipUV {
    pub layer: u16,   // Texture2DArray 层索引
    pub uv_x: f32,    // 归一化 UV (0.0-1.0)
    pub uv_y: f32,
    pub uv_w: f32,
    pub uv_h: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Tile {
    pub width: u8,          // cell 宽度 (1 or 2)
    pub height: u8,         // cell 高度 (1 or 2)
    pub mips: [MipUV; 3],   // Level 0/1/2 的纹理坐标
}
```

**核心机制：symbol 字符串是万能 key**

Cell 的 `symbol` 字段（PUA 编码或 Unicode 字符串）直接作为 `LayeredSymbolMap` 的查找 key：

```rust
// Cell::compute_tile() — set_symbol() 时自动调用
fn compute_tile(&mut self) {
    // symbol 字符串统一查表，不区分 PUA/Unicode
    self.tile = get_layered_symbol_map().resolve(&self.symbol);
}
```

**App 层完全无感：**
```rust
// Sprite — PUA 编码自动作为 key
buf.set_graph_sym(x, y, block, idx, fg);
// → cellsym_block(block, idx) → PUA "\u{F0342}"
// → cell.set_symbol("\u{F0342}")
// → compute_tile() → resolve("\u{F0342}") → Tile with 3 mips

// TUI/Emoji/CJK — Unicode 字符串直接作为 key
cell.set_symbol("A");    // → resolve("A")
cell.set_symbol("😀");   // → resolve("😀")
cell.set_symbol("中");   // → resolve("中")
```

**渲染时零查找：**
```rust
// generate_instances_from_render_cells() 中
let mip_level = select_mip_level(screen_pixel_h);
let uv = cell.tile.mips[mip_level];  // 直接读取，不查表
// uv.layer → per-instance data
// uv.uv_x/y/w/h → per-instance UV
```

**理由：**
- `set_symbol()` 调用频率远低于每帧渲染，查表开销在设置时一次性完成
- 渲染时直接从 Tile 读 MipUV，零额外开销
- block+idx 概念自然消失在 Tile 内部，对外 API 不变
- Tile 变大（4→56 bytes），Buffer(80×50) 从 16KB→224KB，可接受
- 改名 Glyph→Tile 更准确反映其含义：纹理图集中的瓦片，而非字体字形

### Decision 5: Per-Instance Data 编码 Layer Index

**选择：** 扩展 instance 大小到 64 bytes（加一个 vec4）

**当前 per-instance layout (48 bytes)：**
```
a1: [origin_x, origin_y(sign=MSDF), uv_left, uv_top]
a2: [uv_width, uv_height, m00*w, m10*h]
a3: [m01*w, m11*h, tx, ty]
color: [r, g, b, a]
```

**新 layout (64 bytes)：**
```
a1: [origin_x, origin_y, uv_left, uv_top]      // MSDF sign bit 移除
a2: [uv_width, uv_height, m00*w, m10*h]
a3: [m01*w, m11*h, tx, ty]
a4: [layer_index, reserved, reserved, reserved]  // 新增
color: [r, g, b, a]
```

UV 和 layer_index 直接从 `tile.mips[mip_level]` 读取填充，无需查表。

### Decision 6: 工具输出格式

**选择：** `cargo pixel symbols --layered` 生成新格式

**输出结构：**
```
assets/pix/
├── layers/
│   ├── layer_0.png            # 2048×2048，各级别混合打包
│   ├── layer_1.png
│   ├── layer_2.png
│   └── ...                    # 总层数由 DP 打包算法决定
└── layered_symbol_map.json    # 新格式
```

**layered_symbol_map.json 结构：**

Symbol 字符串作为统一 key（PUA 编码用 `\uXXXXX` 格式，Unicode 字符直接使用）：

```json
{
  "version": 2,
  "layer_size": 2048,
  "layer_count": 84,
  "layer_files": [
    "layers/layer_0.png",
    "layers/layer_1.png",
    "..."
  ],
  "symbols": {
    "\uDB80\uDC00": {
      "w": 1, "h": 1,
      "mip0": {"layer": 0, "x": 0, "y": 0, "w": 64, "h": 64},
      "mip1": {"layer": 62, "x": 128, "y": 0, "w": 32, "h": 32},
      "mip2": {"layer": 80, "x": 0, "y": 0, "w": 16, "h": 16}
    },
    "A": {
      "w": 1, "h": 2,
      "mip0": {"layer": 40, "x": 0, "y": 0, "w": 64, "h": 128},
      "mip1": {"layer": 62, "x": 0, "y": 64, "w": 32, "h": 64},
      "mip2": {"layer": 80, "x": 16, "y": 0, "w": 16, "h": 32}
    },
    "😀": {
      "w": 2, "h": 2,
      "mip0": {"layer": 55, "x": 0, "y": 0, "w": 128, "h": 128},
      "mip1": {"layer": 62, "x": 0, "y": 0, "w": 64, "h": 64},
      "mip2": {"layer": 80, "x": 32, "y": 0, "w": 32, "h": 32}
    },
    "中": {
      "w": 2, "h": 2,
      "mip0": {"layer": 55, "x": 128, "y": 0, "w": 128, "h": 128},
      "mip1": {"layer": 62, "x": 64, "y": 0, "w": 64, "h": 64},
      "mip2": {"layer": 80, "x": 64, "y": 0, "w": 32, "h": 32}
    }
  }
}
```

> **注意：** layer 索引是全局的（所有 mipmap level 混合打包），同一符号的 3 个 mip level 可能分布在不同层中。

### Decision 7: Shader 改造

**选择：** 全新 fragment shader，去除 MSDF 路径

**当前 fragment shader 关键路径：**
```wgsl
// MSDF 路径（将被删除）
if (input.v_msdf > 0.5) {
    let d = median3(texColor.r, texColor.g, texColor.b);
    let w = max(fwidth(d), 0.03);
    let alpha = smoothstep(0.5 - w, 0.5 + w, d);
    texColor = vec4<f32>(1.0, 1.0, 1.0, alpha);
}
```

**新 fragment shader：**
```wgsl
@group(0) @binding(1)
var t_symbols: texture_2d_array<f32>;
@group(0) @binding(2)
var s_symbols: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // texture_2d_array 采样：coords + layer_index + lod
    var texColor = textureSampleLevel(
        t_symbols, s_symbols,
        input.uv,
        i32(input.v_layer),  // layer index from per-instance data
        0.0                  // LOD = 0 (CPU 已选择 mipmap level)
    );

    // Bold 效果（保留）
    if (input.v_bold > 0.5) {
        let ts = vec2<f32>(textureDimensions(t_symbols).xy);
        let dx = 0.35 / ts.x;
        texColor = max(texColor, textureSampleLevel(t_symbols, s_symbols,
            input.uv + vec2<f32>(dx, 0.0), i32(input.v_layer), 0.0));
        texColor = max(texColor, textureSampleLevel(t_symbols, s_symbols,
            input.uv + vec2<f32>(-dx, 0.0), i32(input.v_layer), 0.0));
    }

    // Glow 效果（保留）
    if (input.v_glow > 0.5) { ... }

    return texColor * input.color;
}
```

### Decision 8: 混合级别 Shelf-Packing 算法（DP 优化）

**选择：** 所有 mipmap 级别的符号混合打包到同一组 2048×2048 层中，使用 DP 优化的 shelf-packing 最小化总层数

**核心思想：**
不同 mipmap 级别和符号类型的 bitmap 高度不同（16, 32, 64, 128 像素），将它们混合排列到同一 2048 高度的层中。每个 shelf（行）的高度由该行放入的符号高度决定。使用动态规划找到每层的最优 shelf 高度组合，使利用率最大化、总层数最小化。

#### Step 1: 统计各高度的 shelf 需求

对每个 (symbol_type, mip_level) 组合，计算需要多少行 shelf：

```
shelf_height = 符号的像素高度
symbols_per_row = floor(2048 / symbol_width)
rows_needed = ceil(symbol_count / symbols_per_row)
demand[shelf_height] += rows_needed
```

**全量统计结果：**

| shelf 高度 | 需求行数 | 来源明细 |
|-----------|---------|---------|
| 128 | 384 | TUI L0 (80) + Emoji L0 (48) + CJK L0 (256) |
| 64 | 1472 | Sprite L0 (1280) + TUI L1 (40) + Emoji L1 (24) + CJK L1 (128) |
| 32 | 736 | Sprite L1 (640) + TUI L2 (20) + Emoji L2 (12) + CJK L2 (64) |
| 16 | 320 | Sprite L2 (320) |

总高度 = 384×128 + 1472×64 + 736×32 + 320×16 = 49152 + 94208 + 23552 + 5120 = **172032 像素**

理论最小层数 = ceil(172032 / 2048) = **84 层**

#### Step 2: DP 求解单层最优 shelf 组合

对每一层，用 DP 在剩余需求约束下，找到使总填充高度最接近 2048 的 shelf 组合。

**DP 建模：** 有限背包问题（Bounded Knapsack）
- 背包容量 C = 2048 / 16 = **128 个单位**（以 16 为基本单位）
- 物品类型 4 种：大小分别为 8, 4, 2, 1 单位（对应 128, 64, 32, 16 像素）
- 每种物品数量有限（= 该高度的剩余 shelf 行数）
- 目标：最大化填充量（等价于最小化浪费）

```rust
/// 对一层，在剩余需求约束下，求最优 shelf 组合
/// 使用 alloc 数组追踪每个容量下的完整分配方案，避免回溯错误
pub fn dp_fill_layer(remaining: &mut [u32; 4]) -> Vec<(usize, u32)> {
    const SHELF_UNITS: [u16; 4] = [8, 4, 2, 1]; // 128/16, 64/16, 32/16, 16/16
    let capacity: usize = 128; // 2048 / 16

    // dp[c] = 容量 c 时的最大填充量
    // alloc[c][i] = 容量 c 的最优解中，类型 i 使用了多少个
    let mut dp = vec![0u32; capacity + 1];
    let mut alloc = vec![[0u32; 4]; capacity + 1];

    for i in 0..4 {
        let u = SHELF_UNITS[i] as usize;
        let avail = remaining[i].min((capacity / u) as u32);
        if avail == 0 { continue; }

        // 二进制分组优化
        let mut k = 1u32;
        let mut left = avail;
        while left > 0 {
            let batch = k.min(left);
            let batch_units = u * batch as usize;
            for c in (batch_units..=capacity).rev() {
                let prev = c - batch_units;
                let new_fill = dp[prev] + batch_units as u32;
                if new_fill > dp[c] {
                    dp[c] = new_fill;
                    alloc[c] = alloc[prev]; // 继承前一状态的分配
                    alloc[c][i] += batch;   // 加上本次分配
                }
            }
            left -= batch;
            k *= 2;
        }
    }

    // 找到填充量最大的容量
    let best_c = (0..=capacity).max_by_key(|&c| dp[c]).unwrap_or(0);

    // 从 alloc 直接读取结果，扣减 remaining
    let mut result = vec![];
    for i in 0..4 {
        let count = alloc[best_c][i];
        if count > 0 {
            result.push((i, count));
            remaining[i] -= count;
        }
    }
    result
}
```

**关键设计决策**：使用 `alloc[c][i]` 数组而非 `choice[c]` 回溯。二进制分组会覆盖 choice 条目，导致回溯时多次扣减同一物品，引发溢出。`alloc` 方案在每个容量下保存完整分配快照，空间 O(128×4)，完全消除回溯错误。

#### Step 3: 迭代填充所有层

```rust
fn pack_all_layers(demands: &[u32; 4]) -> Vec<LayerConfig> {
    let mut remaining = demands.clone();
    let mut layers = vec![];

    while remaining.iter().any(|&r| r > 0) {
        let shelves = dp_fill_layer(&mut remaining);
        layers.push(LayerConfig { shelves });
    }
    layers
}
```

#### Step 4: 在每层内按 shelf 顺序放置符号

```
对每层的每个 shelf (height, count):
    当前 y = 上一个 shelf 的 y + height
    在 shelf 内横向排列符号（x += symbol_width）
    记录每个符号的 (layer_index, x, y) → 归一化为 UV
```

**为什么 DP 有效：** 所有 shelf 高度是 16 的倍数（16, 32, 64, 128），2048 也是 16 的倍数，所以 DP 总能找到精确填满 2048 的组合（零浪费），直到最后一层。实际总层数 = ceil(总高度 / 2048) = 理论最小值。

**容量估算：**

| 场景 | 层数 | GPU 内存 |
|-----|------|---------|
| 全量加载（所有 3 级 mipmap） | ~84 层 | ~1344MB |
| 仅 Level 1（中分辨率） | ~17 层 | ~272MB |
| 典型 app（少量 Sprite + TUI） | ~5 层 | ~80MB |

> **注意：** 全量 3 级加载内存较大，但多数 app 只用少量符号。后续动态加载可按需加载。
> 初始实现先全量生成，按 app 配置选择加载哪些 mipmap level。

## Risks / Trade-offs

### Risk 1: 全量加载内存可能不降反升
**缓解：** 初始只加载 Level 1，与当前分辨率匹配。Level 0/2 按需后续添加。

### Risk 2: Texture2DArray 在旧 GPU 上兼容性
**缓解：** WebGPU 和 WebGL2 均原生支持。wgpu 在所有后端（Vulkan/Metal/DX12）均支持。

### Risk 3: Per-instance data 扩展影响性能
**缓解：** 从 48 → 64 bytes 仅增加 33%。现代 GPU instance buffer 带宽远超此需求。

### Risk 4: 打包算法效率
**缓解：** 混合级别 DP shelf-packing 保证零浪费（所有高度为 16 的倍数，总能精确填满 2048）。层数 = 理论最小值。

## Migration Plan

1. **Phase 1**: 工具改造，输出新格式（旧格式同时保留）
2. **Phase 2**: WGPU 适配器支持 Texture2DArray 加载
3. **Phase 3**: Shader 改造，去除 MSDF
4. **Phase 4**: CPU mipmap 选择
5. 每个 Phase 完成后验证：`cargo pixel r petview wg`

**回滚方案：** 如果新路径有问题，WGPU 可回退到旧单纹理路径（通过检测 `layered_symbol_map.json` 是否存在来切换）。

## Open Questions

1. **Layer 数量上限** — Texture2DArray 的 `depth_or_array_layers` 上限取决于 GPU。WebGPU 规范保证至少 256 层，通常 2048+，不是问题。
2. **初始只加载 Level 1 是否足够** — 或者是否需要 Level 0 + Level 1 双级。可以在实现后通过 A/B 对比决定。
3. **Web/WASM 下 layer PNG 加载方式** — 需要异步 fetch 多个文件 vs 打包成单文件。可考虑将所有层 concat 为一个 bin 文件。
