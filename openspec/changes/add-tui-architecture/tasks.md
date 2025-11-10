## 1. Implementation

- [x] 1.1 创建 TUI 符号纹理资源 (`assets/pix/symbols_tui.png`)，布局与 `symbols.png` 相同，每个 cell 为 8x16 像素（1:2 宽高比）
- [x] 1.2 扩展 `MouseEvent` 结构，添加 `column_tui`、`row_tui` 字段用于 TUI 坐标，保留原有 `column`、`row` 用于 Sprite 坐标
- [x] 1.3 实现双坐标转换逻辑，在所有 adapter 的鼠标事件处理中同时计算两套坐标（`winit_common.rs`、`sdl_adapter.rs`、`web_adapter.rs`）
- [x] 1.4 添加 TUI 符号纹理加载支持，扩展 `PIXEL_SYM_WIDTH/HEIGHT` 为可配置的 TUI 和 Sprite 两套尺寸
- [x] 1.5 修改 `render_main_buffer` 使用 TUI 符号尺寸（8x16）进行渲染，确保瘦高字符正确显示
- [ ] 1.6 确保 TUI 层（Main Buffer）在渲染顺序上位于所有 Pixel Sprites 之后（最上层）
- [ ] 1.7 修改 `Cell.get_cell_info()` 方法，将 `Cell.modifier` 信息传递到渲染管线
- [ ] 1.8 在渲染管线中实现 BOLD 效果（RGB 值乘以 1.3，限制在 1.0 以内）
- [ ] 1.9 在渲染管线中实现 DIM 效果（Alpha 值乘以 0.6）
- [ ] 1.10 在渲染管线中实现 HIDDEN 效果（Alpha 值设为 0.0）
- [ ] 1.11 在渲染管线中实现 REVERSED 效果（前景色和背景色交换）
- [ ] 1.12 扩展 `RenderCell` 结构添加 `modifier: u16` 字段，用于需要着色器支持的效果（ITALIC、UNDERLINED、CROSSED_OUT）
- [ ] 1.13 更新所有图形后端的着色器（OpenGL、WGPU），添加 modifier 字段支持
- [ ] 1.14 在顶点着色器中实现 ITALIC 效果（倾斜变换）
- [ ] 1.15 在片段着色器中实现 UNDERLINED（底部线条）、CROSSED_OUT（中间线条）效果
- [ ] 1.16 更新 UI 组件使用 `column_tui`、`row_tui` 进行鼠标事件处理，确保 TUI 交互正确
- [ ] 1.17 在 `apps/ui_demo` 中验证 TUI 架构，测试 TUI 界面与游戏精灵的混合渲染和交互
- [ ] 1.18 确保 TUI 架构始终启用，支持应用自由选择使用 Main Buffer（TUI）或仅使用 Pixel Sprites

## 2. Testing

- [ ] 2.1 验证文本模式行为保持不变
- [ ] 2.2 验证图形模式下 TUI 层始终在最上层
- [ ] 2.3 验证双坐标系统正确性（TUI 和 Sprite 坐标独立准确）
- [ ] 2.4 验证 TUI 符号纹理正确加载和渲染
- [ ] 2.5 验证单次 draw call 性能保持不变
- [ ] 2.6 验证 BOLD 修饰符在图形模式下正确渲染（颜色强度增强）
- [ ] 2.7 验证 ITALIC 修饰符在图形模式下正确渲染（字符倾斜）
- [ ] 2.8 验证 UNDERLINED 修饰符在图形模式下正确渲染（底部线条）
- [ ] 2.9 验证 DIM 修饰符在图形模式下正确渲染（透明度降低）
- [ ] 2.10 验证 REVERSED 修饰符在图形模式下正确渲染（前景背景色交换）
- [ ] 2.11 验证 CROSSED_OUT 修饰符在图形模式下正确渲染（中间线条）
- [ ] 2.12 验证 HIDDEN 修饰符在图形模式下正确渲染（完全透明）
- [ ] 2.13 验证多个修饰符组合效果正确（如 BOLD + ITALIC + UNDERLINED）
- [ ] 2.14 验证 BLINK 修饰符在图形模式下被正确忽略（无闪烁效果）
- [ ] 2.15 验证样式修饰符在文本模式下继续使用 crossterm 正常工作

## 3. Documentation

- [ ] 3.1 更新 `README_UI_FRAMEWORK.md` 说明 TUI 架构设计
- [ ] 3.2 添加 TUI 符号纹理创建指南
- [ ] 3.3 更新鼠标事件处理文档，说明双坐标系统用法
- [ ] 3.4 添加 TUI 样式修饰符使用指南，说明图形模式下的样式效果
- [ ] 3.5 更新着色器实现文档，说明样式修饰符的技术实现细节
- [ ] 3.6 添加样式修饰符兼容性说明（文本模式 vs 图形模式差异）

