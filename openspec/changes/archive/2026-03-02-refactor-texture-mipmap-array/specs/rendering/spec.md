## REMOVED Requirements

### Requirement: SDF/MSDF 字体渲染
**Reason**: SDF/MSDF 渲染质量不佳，边缘模糊和毛刺明显，不如对应分辨率的 bitmap 直接渲染。去除后改用多分辨率 bitmap + Texture2DArray 架构。
**Migration**: 所有字符类型（TUI/CJK/Emoji/Sprite）统一使用 bitmap 渲染，通过 mipmap 多分辨率保证不同缩放下的显示质量。

## MODIFIED Requirements

### Requirement: 统一纹理图集
渲染系统 SHALL 使用 Texture2DArray（多张 2048×2048 纹理层）替代单张 8192×8192 纹理图集，保持单次 draw call 渲染所有 Cell。各符号类型以多分辨率 bitmap 形式存储在不同 mipmap level 中。

#### Scenario: WGPU 适配器使用 Texture2DArray
- **WHEN** WGPU 适配器初始化时检测到 `layered_symbol_map.json` 存在
- **THEN** 加载多张 2048×2048 层 PNG 创建 `texture_2d_array<f32>`
- **AND** bind group layout binding 1 使用 `texture_2d_array<f32>` 类型
- **AND** per-instance data 包含 layer index 指定每个 Cell 的纹理层
- **AND** 保持单次 `draw_indexed_instanced()` 渲染所有 Cell

#### Scenario: 内存使用改善
- **WHEN** 全量加载 Level 1 mipmap 时
- **THEN** GPU 内存占用约 96MB（6 层 × 16MB）
- **AND** 显著低于当前 8192×8192 单图的 ~256MB

## ADDED Requirements

### Requirement: 多分辨率 Bitmap Mipmap
渲染系统 SHALL 为每种符号类型提供多个分辨率级别的 bitmap，用于在不同缩放场景下选择最佳渲染质量。

#### Scenario: Sprite 符号 Mipmap 级别
- **WHEN** Sprite 符号（PUA 编码，1×1 cell）被渲染时
- **THEN** 系统提供 Level 0 (64×64)、Level 1 (32×32)、Level 2 (16×16) 三个分辨率
- **AND** 纹理坐标已在 `set_symbol()` 时缓存到 `Tile.mips[]`

#### Scenario: TUI 字符 Mipmap 级别
- **WHEN** TUI 字符（ASCII/Box Drawing/Braille，1×2 cell）被渲染时
- **THEN** 系统提供 Level 0 (64×128)、Level 1 (32×64)、Level 2 (16×32) 三个分辨率
- **AND** 纹理坐标已在 `set_symbol()` 时缓存到 `Tile.mips[]`

#### Scenario: Emoji/CJK Mipmap 级别
- **WHEN** Emoji 或 CJK 字符（2×2 cell）被渲染时
- **THEN** 系统提供 Level 0 (128×128)、Level 1 (64×64)、Level 2 (32×32) 三个分辨率
- **AND** 纹理坐标已在 `set_symbol()` 时缓存到 `Tile.mips[]`

### Requirement: CPU 端 Mipmap 级别选择
渲染系统 SHALL 在 CPU 端根据每个 Cell 的最终屏幕像素大小，选择最合适的 mipmap level 进行渲染。

#### Scenario: 大窗口高分辨率选择
- **WHEN** Cell 的最终屏幕像素高度 >= 48px
- **THEN** 选择 Level 0（最高分辨率）
- **AND** 使用 Level 0 对应的 (layer, UV) 进行采样

#### Scenario: 标准窗口选择
- **WHEN** Cell 的最终屏幕像素高度在 24-48px 之间
- **THEN** 选择 Level 1（中等分辨率）
- **AND** 使用 Level 1 对应的 (layer, UV) 进行采样

#### Scenario: 小窗口低分辨率选择
- **WHEN** Cell 的最终屏幕像素高度 < 24px
- **THEN** 选择 Level 2（最低分辨率）
- **AND** 所有符号类型均提供 Level 2

#### Scenario: 每个 Cell 独立选择
- **WHEN** 同一帧中存在不同缩放比例的 Cell（如 per-cell scale 不同）
- **THEN** 每个 Cell 独立计算屏幕像素大小并选择 mipmap level
- **AND** 同一 draw call 中可以混合使用不同 level 的纹理层

### Requirement: Layered Symbol Map 格式
资源工具 SHALL 生成 `layered_symbol_map.json` 配置文件，描述所有符号在 Texture2DArray 各层中的位置信息。

#### Scenario: JSON 格式版本标识
- **WHEN** 引擎加载 symbol map 时
- **THEN** 检查 `"version": 2` 标识新 layered 格式
- **AND** `"version": 1` 或无 version 字段表示旧格式

#### Scenario: 符号位置查询
- **WHEN** 需要查询符号的纹理位置时
- **THEN** 使用 Cell 的 `symbol` 字符串（PUA 或 Unicode）作为 key 查找 `LayeredSymbolMap`
- **AND** 返回 3 级 mipmap 的 (layer_index, uv_x, uv_y, uv_w, uv_h)
- **AND** layer_index 指定 Texture2DArray 中的层序号
- **AND** uv 坐标为归一化值 (0.0-1.0) 在 2048×2048 层中的位置

#### Scenario: 工具输出新格式
- **WHEN** 运行 `cargo pixel symbols --layered` 时
- **THEN** 输出 `layers/layer_N.png` 多层 PNG 文件（各级别混合打包）
- **AND** 输出 `layered_symbol_map.json` 新格式
- **AND** JSON 以 symbol 字符串（PUA 编码或 Unicode 字符）为 key
- **AND** 各 mipmap 级别的符号混合打包到同一组层中，layer 索引为全局索引

#### Scenario: 混合级别打包
- **WHEN** 工具执行 shelf-packing 打包时
- **THEN** 不同 mipmap 级别（Level 0/1/2）的符号混合排入同一 2048×2048 层
- **AND** 使用 DP 优化的 shelf 高度组合，使每层利用率最大化
- **AND** 同一符号的 3 个 mip level 可能分布在不同层中

### Requirement: Tile 缓存纹理坐标（Glyph→Tile 改名）
Tile（原 Glyph）SHALL 在 `set_symbol()` 时一次性解析并缓存 3 级 mipmap 的纹理坐标，渲染时零查找。

#### Scenario: set_symbol 时解析纹理坐标
- **WHEN** 调用 `cell.set_symbol(symbol)` 时（包括 `set_graph_sym` 内部调用）
- **THEN** `compute_tile()` 使用 `symbol` 字符串查找 `LayeredSymbolMap`
- **AND** 将 3 级 mipmap 的 `(layer, uv)` 缓存到 `Tile.mips[3]`
- **AND** PUA 编码（Sprite）和 Unicode 字符（TUI/Emoji/CJK）统一走同一条路径

#### Scenario: 渲染时直接读取 Tile
- **WHEN** `generate_instances_from_render_cells()` 生成 per-instance data 时
- **THEN** 根据屏幕像素大小选择 mip_level
- **AND** 直接从 `cell.tile.mips[mip_level]` 读取 `(layer, uv)`
- **AND** 无需查表，零额外开销

### Requirement: Texture2DArray GPU 资源管理
WGPU 适配器 SHALL 支持创建和管理 Texture2DArray 类型的 GPU 纹理资源。

#### Scenario: 创建 Texture2DArray
- **WHEN** WGPU 适配器使用 layered 模式初始化时
- **THEN** 创建 `wgpu::Texture` with `depth_or_array_layers > 1`
- **AND** 创建 `TextureViewDimension::D2Array` 类型的 view
- **AND** 将各层 PNG 数据写入对应的 array layer

#### Scenario: Bind Group 配置
- **WHEN** 创建符号渲染 bind group 时
- **THEN** binding 1 使用 `texture_2d_array<f32>` 类型
- **AND** sampler 使用 Nearest 过滤（与当前一致）
- **AND** bind group layout 与 shader 声明匹配
