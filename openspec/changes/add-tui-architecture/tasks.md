## 1. Implementation

- [x] 1.1 创建符号纹理 2048x2048 (`assets/pix/symbols.png`)，使用 block-based 布局：
  - Sprite 符号：16x16 方形字符
  - TUI 符号：16x32 瘦高字符
  - Emoji 符号：32x32 彩色图像
- [x] 1.2 保持所有 adapter 的鼠标事件处理不变（已按 16 像素计算）
  - `column = pixel_x / 16`（16 像素宽度，TUI 和 Sprite 共享）
  - `row = pixel_y / 16`（16 像素高度，Sprite 坐标系）
  - TUI 层使用时：`column_tui = column`, `row_tui = row / 2`
- [x] 1.4 修改纹理加载逻辑，解析统一纹理（2048x2048），初始化 `PIXEL_SYM_WIDTH=16.0/HEIGHT=16.0`（TUI 使用 WIDTH 和 HEIGHT*2）
  - WGPU 和 OpenGL 渲染器的 `load_texture` 已更新为 block-based 布局
- [x] 1.5 在 `render_helper_tui` 中实现 TUI 区域索引计算
  - 使用 `PIXEL_SYM_WIDTH` (16px) 和 `PIXEL_SYM_HEIGHT * 2` (32px)
- [x] 1.6 在 `cell.rs` 中创建 `EMOJI_MAP: HashMap<String, u16>`，映射常用 Emoji 到纹理索引
  - 选择 256 个最常用 Emoji（表情、符号、食物、自然、对象等）+ 128 个预留空间
  - 实现 `is_prerendered_emoji(symbol: &str) -> bool`
  - 实现 `emoji_texidx(symbol: &str) -> Option<u16>`
- [x] 1.7 在 `buffer.rs` 的 `set_stringn` 中实现 Emoji 双宽字符（wcwidth=2）处理
  - 使用 `unicode-width` 检测字符宽度
  - 预制 Emoji：第一格存储 Emoji，第二格设为空白
  - 未预制 Emoji：显示空白占位符，占 2 格
- [x] 1.8 在 `graph.rs` 中实现 `render_helper_emoji` 函数
  - Destination 宽度为 `cell_width * 2.0`（占 2 格）
  - Source 尺寸为 32x32 像素
- [x] 1.9 在 `render_helper` 中实现 Sprite 区域索引计算
  - 向后兼容，现有 Sprite 代码无需修改
- [x] 1.10 确保 TUI 层（Main Buffer）在渲染顺序上位于所有 Pixel Sprites 之后（最上层）
- [x] 1.11 修改 `Cell.get_cell_info()` 方法，将 `Cell.modifier` 信息传递到渲染管线
  - `CellInfo` 类型已包含 modifier 字段
- [x] 1.12 在渲染管线中实现 BOLD 效果（RGB 值乘以 1.3，限制在 1.0 以内）
  - 在 `render_symbols.rs` 中实现（WGPU 和 OpenGL）
- [x] 1.13 在渲染管线中实现 DIM 效果（Alpha 值乘以 0.6）
  - 在 `render_symbols.rs` 中实现（WGPU 和 OpenGL）
- [x] 1.14 在渲染管线中实现 HIDDEN 效果（Alpha 值设为 0.0）
  - 在 `render_symbols.rs` 中实现（WGPU 和 OpenGL）
- [x] 1.15 在渲染管线中实现 REVERSED 效果（前景色和背景色交换）
  - 在 `render_symbols.rs` 中实现（WGPU 和 OpenGL）
- [x] 1.16 扩展 `RenderCell` 结构添加 `modifier: u16` 字段，用于需要着色器支持的效果（ITALIC、UNDERLINED、CROSSED_OUT）
  - `RenderCell.modifier` 字段已存在并被使用
- [x] 1.17 更新所有图形后端添加 modifier 字段支持（在 CPU 端实现，无需修改着色器）
  - WGPU: `render_symbols.rs` 中处理所有 modifier 效果
  - OpenGL: `render_symbols.rs` 中处理所有 modifier 效果
- [x] 1.18 实现 ITALIC 效果（通过 `UnifiedTransform::skew_x()` 倾斜变换实现）
  - 添加 `skew_x()` 方法到 `UnifiedTransform`
  - 倾斜角度约 12°（shear factor ≈ 0.21）
- [x] 1.19 实现 UNDERLINED（底部线条）、CROSSED_OUT（中间线条）效果
  - 使用符号 1280（背景填充）绘制线条
  - UNDERLINED: 在单元格底部 90% 位置绘制
  - CROSSED_OUT: 在单元格中间 46% 位置绘制
- [x] 1.20 验证 UI 组件鼠标事件处理（水平直接使用 column，垂直使用 row / 2）
  - `winit_common.rs` 中已正确处理 TUI 模式坐标转换
  - 所有 UI 组件使用 `mouse_event.column` 和 `mouse_event.row` 进行事件处理
- [x] 1.21 在 `apps/ui_demo` 中验证 TUI 架构，测试 TUI 界面、Emoji 和游戏精灵的混合渲染和交互
- [x] 1.22 确保 TUI 架构始终启用，支持应用自由选择使用 Main Buffer（TUI）或仅使用 Pixel Sprites

## 2. Testing

- [ ] 2.1 验证文本模式行为保持不变
- [ ] 2.2 验证图形模式下 TUI 层始终在最上层
- [ ] 2.3 验证统一坐标系统正确性（水平通用，TUI 垂直除以 2，Sprite 直接使用）
- [ ] 2.4 验证统一纹理的三个区域（TUI、Emoji、Sprite）正确加载和渲染
- [ ] 2.5 验证 TUI 字符显示为 16x32 瘦高形状，Sprite 字符显示为 16x16 方形
- [ ] 2.6 验证预制 Emoji 正确渲染为 32x32 彩色图像，占 2 格宽度
- [ ] 2.7 验证未预制 Emoji 显示为空白占位符，占 2 格宽度
- [ ] 2.8 验证 Emoji 映射表正确识别常用 Emoji（256 个 active + 128 个预留）
- [ ] 2.9 验证单次 draw call 性能保持不变
- [ ] 2.10 验证 BOLD 修饰符在图形模式下正确渲染（颜色强度增强）
- [ ] 2.11 验证 ITALIC 修饰符在图形模式下正确渲染（字符倾斜）
- [ ] 2.12 验证 UNDERLINED 修饰符在图形模式下正确渲染（底部线条）
- [ ] 2.13 验证 DIM 修饰符在图形模式下正确渲染（透明度降低）
- [ ] 2.14 验证 REVERSED 修饰符在图形模式下正确渲染（前景背景色交换）
- [ ] 2.15 验证 CROSSED_OUT 修饰符在图形模式下正确渲染（中间线条）
- [ ] 2.16 验证 HIDDEN 修饰符在图形模式下正确渲染（完全透明）
- [ ] 2.17 验证多个修饰符组合效果正确（如 BOLD + ITALIC + UNDERLINED）
- [ ] 2.18 验证 BLINK 修饰符在图形模式下被正确忽略（无闪烁效果）
- [ ] 2.19 验证样式修饰符在文本模式下继续使用 crossterm 正常工作

## 3. Documentation

- [ ] 3.1 更新 `README_UI_FRAMEWORK.md` 说明 TUI 架构设计
- [ ] 3.2 添加统一符号纹理布局规范文档（Block-Based 布局：Sprite、TUI、Emoji 三区域）
  - 说明 Sprite 区域保持不变（向后兼容）
  - 说明 Block-based 管理的优势（便于编辑器 UI）
- [ ] 3.3 创建 Emoji 使用指南文档
  - 说明预制 Emoji 的选择标准（256 个 active + 128 个预留）
  - 列出所有支持的 Emoji 及其分类
  - 说明如何在 TUI 中使用 Emoji
  - 说明未预制 Emoji 的显示行为
- [ ] 3.4 更新鼠标事件处理文档，说明统一坐标系统（水平通用，TUI 垂直除以 2）
- [ ] 3.5 添加 TUI 样式修饰符使用指南，说明图形模式下的样式效果
- [ ] 3.6 更新着色器实现文档，说明样式修饰符的技术实现细节
- [ ] 3.7 添加样式修饰符兼容性说明（文本模式 vs 图形模式差异）

## Implementation Status

✅ **Section 1 Implementation**: 全部完成 (22/22 tasks)
- 2048x2048 统一纹理已实现
- Block-based 布局：48 Sprite blocks + 5 TUI blocks + 3 Emoji blocks
- Grid 坐标系统：128×128 grid units，每个 16x16 像素
- 所有适配器（SDL、Glow、WGPU、Web）已支持新架构
- TUI 样式修饰符（BOLD、ITALIC、UNDERLINED、DIM、REVERSED、CROSSED_OUT、HIDDEN）已实现

⏳ **Section 2 Testing**: 待完成 (0/19 tasks)

⏳ **Section 3 Documentation**: 待完成 (0/7 tasks)
