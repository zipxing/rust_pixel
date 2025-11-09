## 1. Implementation

- [ ] 1.1 创建 TUI 符号纹理资源 (`assets/pix/symbols_tui.png`)，布局与 `symbols.png` 相同，每个 cell 为 8x16 像素（1:2 宽高比）
- [ ] 1.2 扩展 `MouseEvent` 结构，添加 `column_tui`、`row_tui` 字段用于 TUI 坐标，保留原有 `column`、`row` 用于 Sprite 坐标
- [ ] 1.3 实现双坐标转换逻辑，在所有 adapter 的鼠标事件处理中同时计算两套坐标（`winit_common.rs`、`sdl_adapter.rs`、`web_adapter.rs`）
- [ ] 1.4 添加 TUI 符号纹理加载支持，扩展 `PIXEL_SYM_WIDTH/HEIGHT` 为可配置的 TUI 和 Sprite 两套尺寸
- [ ] 1.5 修改 `render_main_buffer` 使用 TUI 符号尺寸（8x16）进行渲染，确保瘦高字符正确显示
- [ ] 1.6 确保 TUI 层（Main Buffer）在渲染顺序上位于所有 Pixel Sprites 之后（最上层）
- [ ] 1.7 更新 UI 组件使用 `column_tui`、`row_tui` 进行鼠标事件处理，确保 TUI 交互正确
- [ ] 1.8 在 `apps/ui_demo` 中验证 TUI 架构，测试 TUI 界面与游戏精灵的混合渲染和交互
- [ ] 1.9 确保 TUI 架构始终启用，支持应用自由选择使用 Main Buffer（TUI）或仅使用 Pixel Sprites

## 2. Testing

- [ ] 2.1 验证文本模式行为保持不变
- [ ] 2.2 验证图形模式下 TUI 层始终在最上层
- [ ] 2.3 验证双坐标系统正确性（TUI 和 Sprite 坐标独立准确）
- [ ] 2.4 验证 TUI 符号纹理正确加载和渲染
- [ ] 2.5 验证单次 draw call 性能保持不变

## 3. Documentation

- [ ] 3.1 更新 `README_UI_FRAMEWORK.md` 说明 TUI 架构设计
- [ ] 3.2 添加 TUI 符号纹理创建指南
- [ ] 3.3 更新鼠标事件处理文档，说明双坐标系统用法

