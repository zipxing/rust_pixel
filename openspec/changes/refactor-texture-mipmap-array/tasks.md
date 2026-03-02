# 任务清单：纹理系统重构 — Mipmap Bitmap + Texture2DArray

## 1. Phase 1: 去除 SDF + 工具改造（预计 2-3 天）

### 1.1 工具配置更新

- [x] 1.1.1 修改 `tools/cargo-pixel/src/symbols/config.rs` ✅
  - 新增 `LayeredTextureConfig` 结构
  - 定义 mipmap level 配置（各符号类型的分辨率）
  - 定义 layer 大小（2048）和打包参数

- [x] 1.1.2 修改 `tools/cargo-pixel/src/symbols/mod.rs` + `command.rs` ✅
  - 新增 `--layered` 命令行参数
  - 区分旧模式（单张大图）和新模式（layered）

### 1.2 去除 SDF 生成

- [x] 1.2.1 修改 `tools/cargo-pixel/src/symbols/font.rs` ✅
  - 新增 `MipBitmaps` 结构、`generate_mip_levels()`（gamma-correct Lanczos3）
  - 新增 `generate_sprite_mips()`（Nearest neighbor 像素画）
  - 新增 `render_tui_bitmaps()`、`render_emoji_bitmaps()`、`render_cjk_bitmaps()`
  - macOS CoreText + fontdue 双路径分发
  - 不触碰旧 SDF 代码，layered 模式完全跳过 SDF

- [x] 1.2.2 标记 `tools/cargo-pixel/src/symbols/edt.rs` 为可选/废弃 ✅
  - EDT（Euclidean Distance Transform）不再用于 layered 模式
  - 保留代码，旧模式仍可调用

### 1.3 Layer 打包算法

- [x] 1.3.1 修改 `tools/cargo-pixel/src/symbols/texture.rs` ✅
  - 实现 `dp_fill_layer()`（alloc 数组方案，修正了回溯溢出 bug）
  - 实现 `pack_all_layers()` 迭代填充
  - 实现 `pack_layered()` 完整流程：统计需求→DP 打包→生成层图→生成 JSON
  - 实现 `pua_symbol()` PUA key 生成

- [x] 1.3.2 实现 `layered_symbol_map.json` 生成 ✅
  - version=2, layer_size, layer_count, layer_files
  - symbol 字符串为 key（PUA/Unicode）
  - 每个符号含 cell_w, cell_h, mips[3] = {layer, x, y, w, h}

### 1.4 测试 + 验证

- [x] 1.4.1 DP shelf-packing 单元测试 ✅
  - 16 个单元测试全部通过
  - `dp_fill_layer`: 7 个测试（single_type/exact_capacity/mixed/overflow/prioritize_large/empty/minimal）
  - `pack_all_layers`: 6 个测试（single_layer/two_layers/zero_waste/full_production=84层/level1_only/typical_app）
  - UV 坐标 + PUA key 生成: 3 个测试

- [ ] 1.4.2 JSON 生成单元测试（详见 tests.md §6）
  - version=2 标识
  - PUA key 编码/Unicode key 直接使用
  - 所有符号有 3 个 mip level
  - layer_count 与 layer_files 一致
  - 坐标范围合法（x+w ≤ 2048, y+h ≤ 2048）

- [ ] 1.4.3 运行工具，检查输出 PNG 图片质量
  - 各 mipmap level 的 bitmap 应清晰无 SDF 伪影
  - 对比旧 SDF 输出，确认 bitmap 质量更好

- [ ] 1.4.4 检查 `layered_symbol_map.json` 格式正确性
  - 所有符号都有对应的 mipmap level 记录
  - layer/UV 坐标与实际 PNG 内容一致

## 2. Phase 2: Texture2DArray 基础设施（预计 3-4 天）

### 2.1 GPU 纹理支持

- [ ] 2.1.1 修改 `src/render/adapter/wgpu/texture.rs`
  - 新增 `WgpuTextureArray` 结构
  - `from_layer_images()`: 从多张 PNG 创建 Texture2DArray
  - 创建 `TextureViewDimension::D2Array` view
  - Sampler 使用 Nearest 过滤（与当前一致）

- [ ] 2.1.2 修改 `src/render/adapter/wgpu/pixel.rs`
  - `load_symbol_texture_internal()` 支持 layered 模式
  - 检测 `layered_symbol_map.json` 存在时走新路径
  - 创建 Texture2DArray bind group
  - 更新 bind group layout: `texture_2d` → `texture_2d_array`

### 2.2 资源加载

- [ ] 2.2.1 修改 `src/init.rs`
  - 新增 `init_layered_pixel_assets()` 函数
  - 加载 `layered_symbol_map.json`
  - 加载各 mipmap level 的 layer PNG 文件
  - 存储到 `PIXEL_TEXTURE_LAYERS: OnceLock<Vec<Vec<u8>>>`

- [ ] 2.2.2 修改 `src/render/symbol_map.rs`
  - 新增 `LayeredSymbolMap` 结构
  - 解析 `layered_symbol_map.json`（symbol 字符串作为 key）
  - 提供 `resolve(symbol: &str) -> Tile` 查询（返回含 3 级 MipUV 的 Tile）
  - 内部使用 `HashMap<String, Tile>` 存储
  - 替换旧 `SymbolMap`

- [ ] 2.2.3 LayeredSymbolMap 单元测试（详见 tests.md §3）
  - JSON 解析（version/layer_size/symbol_count）
  - resolve() 查询 4 种符号类型（Sprite PUA/TUI ASCII/Emoji/CJK）
  - 未知符号返回默认 Tile
  - UV 归一化范围验证（0.0-1.0）
  - layer 索引边界验证（< layer_count）

### 2.3 管线适配

- [ ] 2.3.1 更新 WGPU pipeline layout
  - Bind group 0 binding 1: `texture_2d_array<f32>`
  - Bind group 0 binding 2: sampler（不变）
  - 创建新的 pipeline（或条件切换）

- [ ] 2.3.2 验证 Texture2DArray 加载正确
  - 创建简单测试：加载 layers，渲染单个符号到屏幕
  - 确认 UV 坐标和 layer index 映射正确

## 3. Phase 3: Shader 改造 + MSDF 移除（预计 2-3 天）

### 3.1 Shader 改造

- [ ] 3.1.1 修改 `src/render/adapter/wgpu/shader_source.rs`
  - Vertex shader: 新增 `v_layer: i32` 输出（`@interpolate(flat)`）
  - Fragment shader: `textureSampleLevel()` 使用 `texture_2d_array` + layer index
  - 删除 `median3()` 函数
  - 删除 MSDF 渲染分支（`if v_msdf > 0.5` 整段）
  - 保留 Bold 和 Glow 效果路径

### 3.2 Per-Instance Data 扩展

- [ ] 3.2.1 修改 `src/render/adapter/wgpu/render_symbols.rs`
  - `WgpuSymbolInstance` 扩展：新增 `a4: [f32; 4]`（layer_index, mip_level, 0, 0）
  - 更新 vertex buffer layout: `array_stride` 从 48 → 64 bytes
  - 新增 `@location(6): a4` vertex attribute
  - `generate_instances_from_render_cells()` 中填充 layer_index

- [ ] 3.2.2 修改 UV 坐标计算
  - 原来从 `self.symbols[linear_index]` 查 SymbolFrame（基于大图）
  - 改为直接从 `cell.tile.mips[mip_level]` 读取 layer + UV（零查找）

### 3.3 清理 MSDF 代码

- [ ] 3.3.1 删除 MSDF 相关字段和逻辑
  - 删除 `WgpuSymbolRenderer.msdf_enabled`
  - 删除 `origin_y` sign bit 编码 MSDF flag 的逻辑
  - 删除 `is_msdf_symbol()` 函数
  - 删除 `WgpuRenderCore` 中 MSDF 相关代码
  - 更新 `Adapter` trait 中 `msdf_enabled` 相关方法

### 3.4 验证

- [ ] 3.4.1 运行 `cargo pixel r petview wg`
  - 确认 TUI/CJK/Emoji 渲染正确（bitmap 路径）
  - 对比旧 MSDF 渲染，确认质量提升
  - 确认 Sprite 渲染无变化

- [ ] 3.4.2 运行 `cargo pixel r mdpt wg`
  - 确认 TUI 文字渲染清晰
  - 确认 CJK 字符渲染正确
  - 确认代码高亮和图表渲染正常

## 4. Phase 4: Mipmap 选择逻辑（预计 2-3 天）

### 4.1 Mipmap 选择实现

- [ ] 4.1.1 修改 `src/render/adapter/wgpu/render_symbols.rs`
  - 在 `generate_instances_from_render_cells()` 中添加 mipmap 选择逻辑
  - 根据 `cell_height * viewport_scale` 计算屏幕像素大小
  - 选择 mipmap level → 直接从 `cell.tile.mips[level]` 读取 (layer, UV)
  - 填充 per-instance data（UV + layer_index 来自 Tile 缓存，零查找）

- [ ] 4.1.2 Mipmap 选择 + Per-Instance 编码单元测试（详见 tests.md §2, §5）
  - `select_mip_level` 4 种符号类型的边界值测试
  - 常见屏幕场景（1080p/1440p/Retina 2x/5K）
  - 返回值范围验证（0-2）
  - Per-instance 大小 = 64 bytes
  - layer_index f32 编码精度验证
  - Tile.mips → instance UV 填充正确性

### 4.2 Glyph→Tile 改名 + 重构

- [ ] 4.2.1 修改 `src/render/cell.rs`
  - Glyph 改名为 Tile
  - 新增 `MipUV` 结构：`(layer: u16, uv_x: f32, uv_y: f32, uv_w: f32, uv_h: f32)`
  - Tile 从 `(block, idx, width, height)` 改为 `(width, height, mips: [MipUV; 3])`
  - `compute_glyph()` 改名 `compute_tile()`，调用 `get_layered_symbol_map().resolve(&self.symbol)`
  - `get_glyph()` 改名 `get_tile()`
  - Cell 的 `glyph` 字段改名 `tile`
  - 移除 `block`/`idx` 字段和相关 PUA→block 解码逻辑
  - 移除 `tui_idx()`/`emoji_idx()`/`cjk_idx()` 等旧查找调用

- [ ] 4.2.2 修改 `src/render/graph.rs`
  - 所有 `get_glyph()` 调用改为 `get_tile()`
  - `glyph.block`/`glyph.idx` 引用更新
  - `glyph_height` 参数名可保留（语义仍通用）

- [ ] 4.2.3 修改 `src/render.rs`
  - re-export `Tile` 替代 `Glyph`

- [ ] 4.2.4 Tile + PUA 兼容链路单元测试（详见 tests.md §4）
  - PUA 编码/解码 round-trip（block=0..159, idx=0..255）
  - `is_pua_sprite` 判断正确
  - Tile 结构大小 ≤ 64 bytes
  - MipUV Copy 语义
  - `is_double_width()` / `is_double_height()` 逻辑
  - `set_symbol` → `compute_tile` 链路

- [ ] 4.2.5 验证 App 层 API 兼容性
  - `set_graph_sym(block, idx)` → `cellsym_block()` → `set_symbol()` → `compute_tile()` 链路正常
  - `set_symbol("A")`/`set_symbol("中")` 等 TUI/CJK 路径正常
  - 所有 app 无需修改代码

### 4.3 验证

- [ ] 4.3.1 测试不同窗口大小下的 mipmap 切换
  - 窗口放大 → 应选择高分辨率 level
  - 窗口缩小 → 应选择低分辨率 level
  - 切换应平滑无闪烁

- [ ] 4.3.2 测试 HiDPI/Retina 显示器
  - 确认 DPI 缩放因子正确纳入 mipmap 选择

- [ ] 4.3.3 性能测试
  - 对比重构前后 CPU 占用
  - 对比 GPU 内存占用
  - 确认 FPS 无下降

## 5. 文档和清理（预计 1 天）

### 5.1 文档更新

- [ ] 5.1.1 更新 `CLAUDE.md` 纹理系统说明
  - 纹理架构从单张大图改为 Texture2DArray
  - Mipmap 级别说明
  - 去除 SDF/MSDF 相关描述

- [ ] 5.1.2 更新 `doc/sdf.md` → `doc/texture-system.md`
  - 记录新的多分辨率 bitmap 架构
  - 记录 Texture2DArray 使用方式
  - 记录 mipmap 选择策略

- [ ] 5.1.3 更新 `openspec/project.md`
  - 更新 Important Constraints 中的纹理描述
  - 去除 "MSDF/SDF 字体" 相关约束

### 5.2 代码清理

- [ ] 5.2.1 清理废弃的 SDF 相关代码
  - 确认 EDT 代码不再被引用
  - 移除 msdfgen 工具调用代码

- [ ] 5.2.2 运行 cargo clippy 和 cargo test
  - 确保无 warning
  - 确保所有测试通过

## 进度追踪

- Phase 1: 🟡 6/9 (67%) — 工具改造 + 测试（核心代码完成，待 e2e 验证）
- Phase 2: ⬜ 0/7 (0%) — Texture2DArray 基础 + 测试
- Phase 3: ⬜ 0/7 (0%) — Shader 改造
- Phase 4: ⬜ 0/11 (0%) — Tile 重构 + Mipmap 选择 + 测试
- Phase 5: ⬜ 0/4 (0%) — 文档清理

**总进度：🟡 6/38 (16%)**
**其中单元测试：1/6 个测试任务完成，16/45 个测试用例通过（详见 tests.md）**
