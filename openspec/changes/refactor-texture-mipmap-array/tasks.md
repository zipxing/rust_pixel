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

- [x] 2.1.1 修改 `src/render/adapter/wgpu/texture.rs` ✅
  - 新增 `WgpuTextureArray` 结构
  - `from_layers()`: 从多层 RGBA 数据创建 Texture2DArray
  - 创建 `TextureViewDimension::D2Array` view
  - Sampler 使用 Linear 过滤

- [x] 2.1.2 修改 `src/render/adapter/wgpu/pixel.rs` ✅
  - 新增 `symbol_texture_array: Option<WgpuTextureArray>` 字段
  - `load_symbol_texture_array()` 加载 Texture2DArray
  - `is_layered()` 检测方法
  - `create_bind_group()` 支持 legacy/layered 两种模式
  - `create_shader()` bind group layout 条件切换 D2/D2Array
  - Fragment shader 条件选择 `texture_2d` 或 `texture_2d_array`

### 2.2 资源加载

- [x] 2.2.1 修改 `src/init.rs` ✅
  - 新增 `PixelLayerData` 结构 + `PIXEL_LAYER_DATA` OnceLock
  - 新增 `init_layered_pixel_assets()`: 加载 JSON + 加载所有 layer PNG + 校验尺寸
  - 新增 `has_layered_assets()` 检测函数
  - 新增 `is_layered_mode()` / `get_pixel_layer_data()` 访问器
  - 设置 PIXEL_SYM_WIDTH/HEIGHT = 16.0

- [x] 2.2.2 修改 `src/render/symbol_map.rs` ✅
  - 新增 `MipUV`, `Tile`, `LayeredSymbolMap` 结构
  - `from_json()` 解析 JSON v2，UV 归一化 (pixel_coords / layer_size)
  - `resolve(&str) -> &Tile` 查询，DEFAULT_TILE fallback
  - 全局静态 `GLOBAL_LAYERED_SYMBOL_MAP` + init/get 函数
  - `PIXEL_LAYERED_SYMBOL_MAP_FILE` 常量

- [x] 2.2.3 LayeredSymbolMap 单元测试 ✅（11 个测试全部通过）
  - test_layered_parse_basic: JSON 解析 + 统计
  - test_layered_resolve_tui/sprite_pua/emoji/cjk: 4 种符号类型查询
  - test_layered_resolve_unknown: 未知符号返回 DEFAULT_TILE
  - test_layered_uv_range: UV 归一化范围 0.0-1.0
  - test_layered_layer_bounds: layer 索引 < layer_count
  - test_layered_version_check: 版本号校验
  - test_layered_stats: symbol_count 统计
  - test_tile_size: Tile 结构大小验证

### 2.3 管线适配

- [x] 2.3.1 更新 WGPU pipeline layout ✅
  - Bind group layout 条件切换 D2/D2Array
  - 新增 `SYMBOLS_INSTANCED_FRAGMENT_SHADER_LAYERED` (texture_2d_array)
  - 去除 MSDF path，仅保留 Bitmap + Glow + Bold
  - `create_shader()` 根据 `is_layered()` 选择 shader
  - 更新 `render_core.rs` + `winit_wgpu_adapter.rs` + `winit_common.rs`
  - 更新 `macros.rs` 自动检测 layered assets
  - 更新 `lib.rs` 导出新的 layered API

- [x] 2.3.2 验证 Texture2DArray 加载正确 ✅
  - `cargo pixel r petview wg` 运行成功，无 WGPU 验证错误
  - 23 层 Texture2DArray 正确加载 (2048×2048 each)
  - Pipeline 验证通过（bind group D2Array 匹配 shader texture_2d_array）
  - Phase 2 stub: symbol frames 使用 legacy layout（Phase 3 替换为 LayeredSymbolMap lookup）

## 3. Phase 3: Shader 改造 + MSDF 移除（预计 2-3 天）

### 3.1 Shader 改造

- [x] 3.1.1 修改 `src/render/adapter/wgpu/shader_source.rs` ✅
  - 新增 `SYMBOLS_INSTANCED_VERTEX_SHADER_LAYERED`: v_layer @interpolate(flat) 输出
  - 更新 `SYMBOLS_INSTANCED_FRAGMENT_SHADER_LAYERED`: textureSampleLevel() 使用 texture_2d_array + layer index
  - Layered fragment shader 去除 MSDF 分支，仅保留 Bitmap + Glow + Bold
  - Legacy shader 保留完整 MSDF 支持（兼容旧模式）
  - Legacy vertex shader 更新 attribute layout（a4@loc4, color@loc5）

### 3.2 Per-Instance Data 扩展

- [x] 3.2.1 修改 `src/render/adapter/wgpu/render_symbols.rs` ✅
  - `WgpuSymbolInstance` 扩展：新增 `a4: [f32; 4]`（5 × vec4 = 80 bytes）
  - 更新 `desc()`: 5 attributes (a1@loc1, a2@loc2, a3@loc3, a4@loc4, color@loc5)
  - 新增 `layered_tiles: Vec<Tile>`, `is_layered: bool` 字段
  - 新增 `load_layered_frames()` 方法（调用 build_layered_tiles()）
  - 新增 `draw_layered_instance()` / `draw_layered_instance_with_glow()` 方法
  - 拆分 `generate_instances_from_render_cells()` → legacy/layered 两条路径

- [x] 3.2.2 修改 UV 坐标计算 ✅
  - Layered 路径从 `layered_tiles[texsym]` 查 Tile → mips[1] 读取 layer + UV
  - 新增 `build_layered_tiles()` 到 symbol_map.rs（线性 texsym → Tile 反向映射）
  - 新增 SymbolMap iterator 方法: `iter_tui()`, `iter_emoji()`, `iter_cjk()`

- [x] 3.2.3 修改 `src/render/adapter/wgpu/pixel.rs` ✅
  - `create_shader()` 根据 `is_layered()` 选择 vertex/fragment shader
  - `load_symbol_texture_array()` 调用 `load_layered_frames()` 替代 Phase 2 stub

### 3.3 清理 MSDF 代码

- [ ] 3.3.1 删除 MSDF 相关字段和逻辑（layered 路径已不使用 MSDF，legacy 路径保留）
  - 删除 `WgpuSymbolRenderer.msdf_enabled`
  - 删除 `origin_y` sign bit 编码 MSDF flag 的逻辑
  - 删除 `is_msdf_symbol()` 函数
  - 删除 `WgpuRenderCore` 中 MSDF 相关代码
  - 更新 `Adapter` trait 中 `msdf_enabled` 相关方法

### 3.4 验证

- [x] 3.4.1 运行 `cargo pixel r petview wg` ✅
  - 确认 TUI/CJK/Emoji 渲染正确（bitmap 路径）
  - 修复 PIXEL_SYM_WIDTH/HEIGHT: 16→32（匹配 legacy 8192/256=32）
  - 确认 Sprite 渲染正常

- [ ] 3.4.2 运行 `cargo pixel r mdpt wg`
  - 确认 TUI 文字渲染清晰
  - 确认 CJK 字符渲染正确
  - 确认代码高亮和图表渲染正常

## 4. Phase 4: Mipmap 选择逻辑（预计 2-3 天）

### 4.1 PIXEL_SYMBOL_SIZE 基准常量

- [x] 4.1.1 引入 `PIXEL_SYMBOL_SIZE = 16` 常量 ✅
  - `src/render/graph.rs`: `pub const PIXEL_SYMBOL_SIZE: f32 = 16.0`
  - `init_layered_pixel_assets()`: `PIXEL_SYM_WIDTH/HEIGHT = PIXEL_SYMBOL_SIZE * 2`
  - `src/render.rs`: 导出 `PIXEL_SYMBOL_SIZE`
  - Legacy 模式保留 `init_sym_width(texture_width)` 兼容

### 4.2 Mipmap 选择实现

- [x] 4.2.1 修改 `src/render/adapter/wgpu/render_symbols.rs` ✅
  - 新增 `select_mip_level(screen_pixel_h, cell_h) -> usize`
  - 阈值：per_unit_h >= 48 → mip0, >= 24 → mip1, else mip2
  - 在 `generate_instances_layered()` 中动态选择 mip level（替换固定 mip_level=1）
  - BG fill 固定 mip1（纯色块，mip 无关）

- [ ] 4.2.2 Mipmap 选择单元测试
  - `select_mip_level` 边界值测试
  - 返回值范围验证（0-2）

### 4.3 Glyph→Tile 改名 + 重构

- [ ] 4.3.1 修改 `src/render/cell.rs`
  - Glyph 改名为 Tile
  - Tile 从 `(block, idx, width, height)` 改为 `(width, height, mips: [MipUV; 3])`
  - `compute_glyph()` 改名 `compute_tile()`，调用 `get_layered_symbol_map().resolve(&self.symbol)`
  - `get_glyph()` 改名 `get_tile()`
  - Cell 的 `glyph` 字段改名 `tile`
  - 移除 `block`/`idx` 字段和相关 PUA→block 解码逻辑
  - 移除 `tui_idx()`/`emoji_idx()`/`cjk_idx()` 等旧查找调用

- [ ] 4.3.2 修改 `src/render/graph.rs`
  - 所有 `get_glyph()` 调用改为 `get_tile()`
  - `glyph.block`/`glyph.idx` 引用更新

- [ ] 4.3.3 修改 `src/render.rs`
  - re-export `Tile` 替代 `Glyph`

- [ ] 4.3.4 Tile + PUA 兼容链路单元测试
  - PUA 编码/解码 round-trip
  - `is_double_width()` / `is_double_height()` 逻辑
  - `set_symbol` → `compute_tile` 链路

- [ ] 4.3.5 验证 App 层 API 兼容性
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
- Phase 2: ✅ 7/7 (100%) — Texture2DArray 基础 + 测试 + pipeline 验证通过
- Phase 3: 🟡 6/8 (75%) — Shader 改造 + per-instance layer index（MSDF cleanup + mdpt 验证 pending）
- Phase 4: 🟡 2/9 (22%) — PIXEL_SYMBOL_SIZE ✅ + Mipmap 选择 ✅ + Tile 重构 pending
- Phase 5: ⬜ 0/4 (0%) — 文档清理

**总进度：🟡 21/37 (57%)**
**其中单元测试：2/6 个测试任务完成（Phase 1 DP 16 例 + Phase 2 LayeredSymbolMap 11 例 = 27/45 通过）**
