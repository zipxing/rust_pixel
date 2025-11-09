## Why

当前 rust_pixel 在文本模式和图形模式下的渲染架构不够统一，特别是在图形模式下混合使用 TUI 界面和游戏精灵时，缺乏清晰的分层和坐标系统。这导致：
1. TUI 组件和游戏精灵使用相同的符号尺寸（1:1），无法模拟终端的瘦高字符（1:2）
2. 鼠标事件处理混乱，无法区分 TUI 区域和游戏区域的点击
3. 渲染层次不明确，TUI 界面无法始终保持在最上层

## What Changes

- **文本模式保持不变**：所有精灵和 UI 组件合并到 main buffer，使用统一的瘦高字符和坐标系统
- **图形模式分层架构**：
  - Main Buffer 专用于 TUI 渲染，使用瘦高字符（1:2，8x16 像素）
  - Pixel Sprites 用于游戏对象，使用正方形字符（1:1，16x16 像素）
  - TUI 层永远渲染在最上层，确保界面始终可见
- **独立符号纹理**：新增 `symbols_tui.png`，布局与 `symbols.png` 相同，但每个 cell 为 8x16 像素
- **双坐标系统**：鼠标事件同时提供 TUI 坐标（瘦字符）和 Sprite 坐标（胖字符）
- **统一渲染管线**：Main Buffer 和 Pixel Sprites 仍合并到 RenderBuffer，保持单次 draw call 性能

## Impact

- Affected specs: rendering (新增)
- Affected code:
  - `src/render/graph.rs` - 添加 TUI 符号纹理加载和双坐标计算
  - `src/render/adapter.rs` - 扩展渲染管线支持分层和 TUI 优先级
  - `src/event/input.rs` - 扩展 MouseEvent 支持双坐标系统
  - `src/render/adapter/*/render_symbols.rs` - 支持 TUI 符号渲染
  - `assets/pix/symbols_tui.png` - 新增 TUI 专用符号纹理
- Compatibility: 文本模式完全向后兼容；图形模式 TUI 架构始终启用，现有应用继续使用 Pixel Sprites 无影响
- Migration: 现有应用无需迁移，可继续仅使用 Pixel Sprites；新应用可选择使用 Main Buffer（TUI）进行界面渲染

