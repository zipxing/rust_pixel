## 1. Implementation

- [ ] 1.1 创建或扩展符号纹理为 1024x1024 (`assets/pix/symbols.png`)，包含三个区域：
  - 行 0-127（128px）：TUI 符号（8x16 瘦高字符，1024 个）
  - 行 128-191（64px）：预制 Emoji（16x16 彩色图像，256 个）
  - 行 192-1023（832px）：Sprite 符号（8x8 方形字符，13,312 个）
- [ ] 1.2 保持所有 adapter 的鼠标事件处理不变（已按 8 像素计算）
  - `column = pixel_x / 8`（8 像素宽度，TUI 和 Sprite 共享）
  - `row = pixel_y / 8`（8 像素高度，Sprite 坐标系）
  - TUI 层使用时：`column_tui = column`, `row_tui = row / 2`
- [ ] 1.4 修改纹理加载逻辑，解析统一纹理（1024x1024），初始化 `PIXEL_TUI_WIDTH=8.0/HEIGHT=16.0` 和 `PIXEL_SYM_WIDTH=8.0/HEIGHT=8.0`
- [ ] 1.5 在 `render_helper_tui` 中实现 TUI 区域索引计算（符号索引 0-1023，对应行 0-127）
  - 计算公式：`char_x = symidx % 128`, `char_y = symidx / 128`, `pixel_x = char_x * 8`, `pixel_y = char_y * 16`
  - 符号索引 = `symidx`（直接线性索引）
- [ ] 1.6 在 `cell.rs` 中创建 `EMOJI_MAP: HashMap<String, u16>`，映射常用 Emoji 到纹理索引
  - 选择 175 个最常用 Emoji（表情、符号、食物、自然、对象等）+ 81 个预留空间
  - Emoji 索引范围：1024-1279（Emoji 区域）
  - 实现 `is_prerendered_emoji(symbol: &str) -> bool`
  - 实现 `emoji_texidx(symbol: &str) -> Option<u16>`
- [ ] 1.7 在 `buffer.rs` 的 `set_stringn` 中实现 Emoji 双宽字符（wcwidth=2）处理
  - 使用 `unicode-width` 检测字符宽度
  - 预制 Emoji：第一格存储 Emoji，第二格设为空白
  - 未预制 Emoji：显示空白占位符，占 2 格
- [ ] 1.8 在 `graph.rs` 中实现 `render_helper_emoji` 函数
  - 计算公式：`relative_idx = emoji_idx - 1024`, `emoji_x = (relative_idx % 64) * 16`, `emoji_y = 128 + (relative_idx / 64) * 16`
  - Destination 宽度为 `cell_width * 2.0`（占 2 格）
  - Source 尺寸为 16x16 像素
- [ ] 1.9 在 `render_helper` 中实现 Sprite 区域索引计算（符号索引 1280-14591，对应行 192-1023）
  - 计算公式：`relative_idx = symidx - 1280`, `char_x = relative_idx % 128`, `char_y = relative_idx / 128`, `pixel_x = char_x * 8`, `pixel_y = 192 + char_y * 8`
  - 符号索引 = `symidx`（直接线性索引）
- [ ] 1.10 确保 TUI 层（Main Buffer）在渲染顺序上位于所有 Pixel Sprites 之后（最上层）
- [ ] 1.11 修改 `Cell.get_cell_info()` 方法，将 `Cell.modifier` 信息传递到渲染管线
- [ ] 1.12 在渲染管线中实现 BOLD 效果（RGB 值乘以 1.3，限制在 1.0 以内）
- [ ] 1.13 在渲染管线中实现 DIM 效果（Alpha 值乘以 0.6）
- [ ] 1.14 在渲染管线中实现 HIDDEN 效果（Alpha 值设为 0.0）
- [ ] 1.15 在渲染管线中实现 REVERSED 效果（前景色和背景色交换）
- [ ] 1.16 扩展 `RenderCell` 结构添加 `modifier: u16` 字段，用于需要着色器支持的效果（ITALIC、UNDERLINED、CROSSED_OUT）
- [ ] 1.17 更新所有图形后端的着色器（OpenGL、WGPU），添加 modifier 字段支持
- [ ] 1.18 在顶点着色器中实现 ITALIC 效果（倾斜变换）
- [ ] 1.19 在片段着色器中实现 UNDERLINED（底部线条）、CROSSED_OUT（中间线条）效果
- [ ] 1.20 验证 UI 组件鼠标事件处理（水平直接使用 column，垂直使用 row / 2）
- [ ] 1.21 在 `apps/ui_demo` 中验证 TUI 架构，测试 TUI 界面、Emoji 和游戏精灵的混合渲染和交互
- [ ] 1.22 确保 TUI 架构始终启用，支持应用自由选择使用 Main Buffer（TUI）或仅使用 Pixel Sprites

## 2. Testing

- [ ] 2.1 验证文本模式行为保持不变
- [ ] 2.2 验证图形模式下 TUI 层始终在最上层
- [ ] 2.3 验证统一坐标系统正确性（水平通用，TUI 垂直除以 2，Sprite 直接使用）
- [ ] 2.4 验证统一纹理的三个区域（TUI、Emoji、Sprite）正确加载和渲染
- [ ] 2.5 验证 TUI 字符显示为 8x16 瘦高形状，Sprite 字符显示为 8x8 方形
- [ ] 2.6 验证预制 Emoji 正确渲染为 16x16 彩色图像，占 2 格宽度
- [ ] 2.7 验证未预制 Emoji 显示为空白占位符，占 2 格宽度
- [ ] 2.8 验证 Emoji 映射表正确识别常用 Emoji（175 个 + 81 个预留）
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
- [ ] 3.2 添加统一符号纹理布局规范文档（TUI、Emoji、Sprite 三区域）
- [ ] 3.3 创建 Emoji 使用指南文档
  - 说明预制 Emoji 的选择标准（175 个常用 Emoji + 81 个预留）
  - 列出所有支持的 Emoji 及其分类
  - 说明如何在 TUI 中使用 Emoji
  - 说明未预制 Emoji 的显示行为
- [ ] 3.4 更新鼠标事件处理文档，说明统一坐标系统（水平通用，TUI 垂直除以 2）
- [ ] 3.5 添加 TUI 样式修饰符使用指南，说明图形模式下的样式效果
- [ ] 3.6 更新着色器实现文档，说明样式修饰符的技术实现细节
- [ ] 3.7 添加样式修饰符兼容性说明（文本模式 vs 图形模式差异）

