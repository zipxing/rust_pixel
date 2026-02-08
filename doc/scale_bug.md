# GPU 缩放清晰度 Bug 修复记录

## 问题背景

在 mdpt 演示工具开发过程中，图形模式下出现两类缩放渲染问题：

1. **符号纹理缩放模糊/有网格线** — 纹理图集采样策略不当，导致相邻符号像素泄漏
2. **非整数缩放时出现 1 像素间隙** — 独立舍入每个单元格的尺寸导致累积误差

两个问题互相关联：修复网格线需要切换 LINEAR 过滤，但 LINEAR 过滤又引入了纹理边界采样问题，需要半纹素内缩配合解决。

## 涉及提交

| 提交 | 说明 |
|------|------|
| `55f71be` | LINEAR 过滤 + 半纹素内缩 + 精度升级 + 网格平铺修复 |
| `777a4c6` | 修复 LINEAR 引入的黑线 bug（per-cell 缩放条件判断） |
| `09efb24` | BOLD 加粗着色器（利用 LINEAR 过滤实现亚像素采样） |

## 涉及文件

```
src/render/adapter/gl/texture.rs          # OpenGL 纹理采样器配置
src/render/adapter/gl/shader_source.rs    # GLSL 着色器（精度 + BOLD）
src/render/adapter/gl/render_symbols.rs   # GL 符号渲染（半纹素 + BOLD 编码）
src/render/adapter/wgpu/texture.rs        # WGPU 纹理采样器配置
src/render/adapter/wgpu/shader_source.rs  # WGSL 着色器（精度 + BOLD）
src/render/adapter/wgpu/render_symbols.rs # WGPU 符号渲染（半纹素 + BOLD 编码）
src/render/graph.rs                       # 网格平铺算法修复
```

---

## 修复 1：分层纹理采样策略

### 问题

所有纹理统一使用 NEAREST 过滤。当缩放比例不是整数时（如 1.2x），NEAREST 采样导致字符边缘出现锯齿，视觉效果粗糙。

### 方案

区分 **渲染纹理 (RT)** 和 **符号纹理 (Atlas)** 的采样策略：

| 纹理类型 | 过滤模式 | 原因 |
|----------|---------|------|
| 渲染纹理 (RT) | **Nearest** | RT 是像素级合成目标，LINEAR 会在 RT 边界产生模糊 |
| 符号纹理 (Atlas) | **Linear** | 非整数缩放时提供平滑插值，字符边缘更清晰 |

### GL 实现 (`gl/texture.rs`)

```rust
// 渲染纹理 — 保持 NEAREST
gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);

// 符号纹理 — 改为 LINEAR
gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
```

### WGPU 实现 (`wgpu/texture.rs`)

RT 采样器保持 `FilterMode::Nearest`，符号纹理采样器同理。WGPU 的采样器通过 `SamplerDescriptor` 配置。

---

## 修复 2：半纹素内缩（Half-texel Inset）

### 问题

启用 LINEAR 过滤后，纹理图集中的符号边界出现**网格线伪影**。原因是双线性插值在 UV 边界会采样到相邻符号的像素。

```
┌─────┬─────┐
│  A  │  B  │   ← UV 边界处，LINEAR 会混合 A 和 B 的像素
├─────┼─────┤      → 产生可见的网格线
│  C  │  D  │
└─────┴─────┘
```

### 方案

将 UV 坐标向内收缩 0.5 个纹素，确保采样范围严格在目标符号内部：

```
原始 UV:        [x/W, (x+w)/W]
半纹素内缩后:   [x/W + 0.5/W, (x+w)/W - 0.5/W]
```

### 实现（GL + WGPU 对称）

```rust
// gl/render_symbols.rs — make_symbols_frame()
// wgpu/render_symbols.rs — make_symbols_frame_custom()
let half_texel_x = 0.5 / tex_width;
let half_texel_y = 0.5 / tex_height;
let uv_left   = x / tex_width  + half_texel_x;         // 左边界向右缩 0.5px
let uv_top    = y / tex_height + half_texel_y;          // 上边界向下缩 0.5px
let uv_width  = width  / tex_width  - half_texel_x * 2.0;  // 总宽度减 1px
let uv_height = height / tex_height - half_texel_y * 2.0;  // 总高度减 1px
```

---

## 修复 3：着色器精度升级

### 问题

`precision mediump float` 在移动端和部分 GPU 上精度不足，浮点舍入导致 UV 坐标偏移，在高分辨率纹理图集上引起采样错位。

### 方案

所有着色器统一升级为 `precision highp float`（影响 6 个着色器字符串常量）。

---

## 修复 4：非整数缩放的网格平铺

### 问题

当缩放比例导致单元格宽高为非整数时（如 16px / 1.2 = 13.333px），独立舍入每个单元格的尺寸会产生累积误差，在相邻单元格之间形成 1 像素间隙：

```
独立舍入：每格都 round(13.333) = 13px
    格0: [0, 13)  格1: [13, 26)  格2: [26, 39) ...
    理想位置: 0, 13.333, 26.666, 40.0
    实际位置: 0, 13, 26, 39
    → 在 40px 处少了 1px，产生间隙
```

### 方案

**累积舍入法**：宽度 = `round(下一格位置) - round(当前格位置)`

```
round(0)     = 0
round(13.33) = 13    → 格0宽度 = 13-0  = 13
round(26.67) = 27    → 格1宽度 = 27-13 = 14  ← 自动补偿
round(40.0)  = 40    → 格2宽度 = 40-27 = 13
```

宽度交替为 13, 14, 13, 14...，相邻单元格完美平铺无间隙。

### 实现 (`graph.rs`)

**`render_helper_with_scale()`** — 网格布局模式：

```rust
let cell_f_w = w as f32 / r.x * scale_x;
let cell_f_h = h as f32 / r.y * scale_y;
let this_x = (dstx as f32 * cell_f_w).round();
let this_y = (dsty as f32 * cell_f_h).round();
let next_x = ((dstx as f32 + 1.0) * cell_f_w).round();
let next_y = ((dsty as f32 + 1.0) * cell_f_h).round();
// 宽高 = 下一格位置 - 当前格位置
(this_x, this_y, (next_x - this_x) as u32, (next_y - this_y) as u32)
```

**`render_buffer_to_cells()`** — X 方向后处理修正：

```rust
if x_center_offset.abs() < 0.01 {
    let this_x_f = cumulative_x + x_center_offset;
    let next_x_f = this_x_f + grid_advance;
    let corrected_w = next_x_f.round() as i32 - this_x_f.round() as i32;
    if corrected_w > 0 {
        s2.w = corrected_w as u32;
    }
}
```

### 修复 4a：per-cell 缩放的条件排除 (`777a4c6`)

上述平铺修正对于普通单元格有效，但对 **per-cell 缩放**的元素（如 0.5x 缩放的 emoji bullet）会错误地扩大尺寸。因为这些元素本来就比格子小，不需要平铺填满。

**Y 方向**：当 `y_offset != 0` 时跳过平铺修正

```rust
let h_val = if y_offset.abs() < 0.01 {
    (next_y_f.round() - this_y_f.round()) as u32  // 平铺修正
} else {
    (h as f32 / r.y * scale_y).round() as u32     // 独立舍入
};
```

**X 方向**：当 `x_center_offset != 0` 时跳过宽度修正

```rust
// 只在格子满占时应用平铺修正
if x_center_offset.abs() < 0.01 { ... }
```

---

## 附带功能：BOLD 加粗着色器 (`09efb24`)

利用 LINEAR 过滤已启用这一前提，在 GPU 着色器中实现文字加粗效果。

### 编码方式

通过 `origin_x` 的符号位传递 BOLD 标志（零额外带宽）：

```rust
// CPU 端：编码
instance_buffer[at] = if is_bold { -frame.origin_x } else { frame.origin_x };

// 着色器端：解码
v_bold = a1.x < 0.0 ? 1.0 : 0.0;
vec2 origin = abs(a1.xy);  // 取绝对值恢复真实坐标
```

### 着色器实现

采样当前像素左右各 0.5 纹素的颜色，取 max 使字形横向加粗：

```glsl
if (v_bold > 0.5) {
    ivec2 ts = textureSize(source, 0);
    float dx = 0.5 / float(ts.x);
    texColor = max(texColor, texture(source, uv + vec2(dx, 0.0)));
    texColor = max(texColor, texture(source, uv + vec2(-dx, 0.0)));
    texColor.a = smoothstep(0.05, 0.8, texColor.a);
}
```

GL（GLSL）和 WGPU（WGSL）两套着色器同步实现。

---

## 修复链路总结

```
问题：NEAREST 缩放锯齿
  │
  ├─→ 方案：符号纹理改 LINEAR
  │     │
  │     └─→ 新问题：LINEAR 采样越界产生网格线
  │           │
  │           ├─→ 方案：半纹素内缩 UV 坐标
  │           └─→ 方案：着色器精度 mediump → highp
  │
  └─→ 利用 LINEAR：实现 BOLD 加粗着色器

问题：非整数缩放产生 1px 间隙
  │
  ├─→ 方案：累积舍入法（round(next) - round(this)）
  │
  └─→ 新问题：per-cell 缩放元素被错误扩大
        │
        └─→ 方案：条件排除（y_offset / x_center_offset 判断）
```
