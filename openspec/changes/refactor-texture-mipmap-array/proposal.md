# Refactor: 纹理系统 — Mipmap Bitmap + Texture2DArray

## Why

当前纹理系统存在两个核心问题：

1. **SDF/MSDF 渲染质量不佳** — TUI 字符使用 MSDF（msdfgen 生成），CJK 字符使用 SDF（EDT 距离场），实际渲染效果尤其在缩放边缘处不如直接 bitmap。Fragment shader 中 `median3()` + `smoothstep()` 的抗锯齿路径引入了模糊和毛刺
2. **8192×8192 大图浪费内存** — 单张 RGBA 纹理约 256MB 全量加载进 GPU 内存，即使大部分 app 只使用了少量符号（如仅用 Sprite block 0-3 + TUI）

## What Changes

### 核心改动

1. **去除 SDF/MSDF 渲染路径**
   - 删除 fragment shader 中的 `median3()` + `smoothstep()` MSDF 路径
   - 删除 per-instance data 中 `origin_y` sign bit 的 MSDF 标记
   - 删除 `msdf_enabled` 字段及相关逻辑
   - 所有字符类型统一使用 bitmap 渲染

2. **多分辨率 Bitmap（Mipmap）**
   - Sprite: 64×64 (high), 32×32 (mid), 16×16 (low) — 3 级
   - TUI/Braille: 64×128 (high), 32×64 (mid), 16×32 (low) — 3 级
   - Emoji: 128×128 (high), 64×64 (mid), 32×32 (low) — 3 级
   - CJK: 128×128 (high), 64×64 (mid), 32×32 (low) — 3 级
   - 高分辨率 bitmap 提供更好的渲染质量
   - 低分辨率 bitmap 节省带宽和提升缓存命中率

3. **Texture2DArray 替代单张大图**
   - 多张 2048×2048 纹理层（layer），组成 `texture_2d_array<f32>`
   - 保持单 draw call 渲染（layer index 编码在 per-instance data 中）
   - 典型 app 仅需 3-5 层 = 48-80MB，显著优于 256MB
   - WebGPU/WebGL2 均原生支持 Texture2DArray

4. **工具改造（cargo pixel symbols）**
   - 去除 SDF 生成管道（EDT、msdfgen 调用）
   - 支持多分辨率 bitmap 渲染和混合级别打包（DP 优化 shelf-packing）
   - 各 mipmap 级别混合打包到同一组 2048×2048 层，最小化层数
   - 输出 `layers/layer_N.png` + `layered_symbol_map.json`

5. **Glyph 改名 Tile，重构为纹理坐标缓存**
   - Glyph 改名为 Tile（纹理瓦片），不再存储 `(block, idx)`，改为缓存 3 级 mipmap 的 `(layer, uv)` 纹理坐标
   - Cell 的 `symbol` 字符串（PUA 或 Unicode）作为 `LayeredSymbolMap` 的统一查找 key
   - `set_symbol()` 时一次性解析纹理坐标到 Tile，渲染时零查找
   - App 层 API（`set_graph_sym()`、`set_symbol()` 等）完全不变

6. **CPU 端 Mipmap 级别选择**
   - 在 `generate_instances_from_render_cells()` 阶段
   - 根据 cell 最终屏幕像素大小选择 mipmap level
   - 直接从 `tile.mips[level]` 读取纹理坐标，零查找开销

### **BREAKING**: Shader 接口变更
- `@binding(1)` 从 `texture_2d<f32>` 变为 `texture_2d_array<f32>`
- Per-instance data 中 MSDF sign bit 改为 layer index

### **INTERNAL**: Glyph→Tile 改名 + 结构变更
- Glyph 改名为 Tile
- Tile 从 `(block, idx, width, height)` 改为 `(width, height, mips[3])`
- `mips[i]` 包含 `(layer, uv_x, uv_y, uv_w, uv_h)`
- App 层无感知（Tile 是内部类型）

## Impact

### Affected Specs
- **rendering**（重大修改）— 纹理架构、shader 管线、渲染数据格式

### Affected Code

**工具层**
- `tools/cargo-pixel/src/symbols/font.rs` — 去除 SDF，多分辨率 bitmap 渲染
- `tools/cargo-pixel/src/symbols/texture.rs` — 2048 层打包算法
- `tools/cargo-pixel/src/symbols/config.rs` — LayeredTextureConfig
- `tools/cargo-pixel/src/symbols/edt.rs` — 可废弃（EDT/SDF 不再需要）

**引擎核心**
- `src/render/adapter/wgpu/texture.rs` — 新增 WgpuTextureArray
- `src/render/adapter/wgpu/pixel.rs` — 加载 Texture2DArray，更新 bind group
- `src/render/adapter/wgpu/shader_source.rs` — 去 MSDF，array 采样
- `src/render/adapter/wgpu/render_symbols.rs` — layer index + mipmap 选择
- `src/render/symbol_map.rs` — 解析 layered JSON，多 mipmap 查找 API
- `src/init.rs` — 多层纹理加载

**内部变更（App 无感知）**
- `src/render/cell.rs` — Glyph→Tile 改名 + 结构重构（缓存 3 级 MipUV）
- `src/render/cell.rs` — `compute_glyph()`→`compute_tile()` 改为查 LayeredSymbolMap

**不受影响**
- App 层 API（`set_graph_sym()`、`set_symbol()` 等）— 保持不变
- App 代码 — 无需修改

### Compatibility
- WGPU 适配器切换到新 Texture2DArray
- Web 适配器共享 WgpuRenderCore，自动受益
- App 层代码无需修改

## Timeline

- **Phase 1**（2-3 天）：去除 SDF + 工具改造
- **Phase 2**（3-4 天）：Texture2DArray 基础设施
- **Phase 3**（2-3 天）：Shader 改造 + MSDF 移除
- **Phase 4**（2-3 天）：Mipmap 选择逻辑

**总计：9-13 天**
