## Context

rust_pixel 目前支持文本模式（终端）和图形模式（SDL/OpenGL/WGPU/WebGL），但在图形模式下缺乏对 TUI（Terminal User Interface）风格界面的良好支持。终端字符通常是瘦高的（1:2 宽高比），而图形模式使用的是正方形符号（1:1）。这导致在图形模式下无法真实模拟终端 UI 的视觉效果。

**约束条件：**
- 必须保持文本模式完全向后兼容
- 必须保持单次 draw call 的高性能渲染
- 必须支持 TUI 和游戏精灵的混合渲染
- 符号纹理布局必须与现有 `symbols.png` 保持一致（128x128 网格）

**相关方：**
- 游戏开发者：需要在图形模式下使用 TUI 界面
- UI 框架用户：需要正确的字符宽高比和鼠标交互
- 性能敏感应用：需要保持高效的渲染性能

## Goals / Non-Goals

**Goals:**
- 在图形模式下支持瘦高字符（1:2）的 TUI 渲染
- 提供清晰的 TUI 层和游戏精灵层分离
- 实现双坐标系统，正确处理 TUI 和游戏区域的鼠标事件
- TUI 层永远渲染在最上层，确保界面可见性
- 保持单次 draw call 的渲染性能

**Non-Goals:**
- 不改变文本模式的任何行为
- 不引入复杂的窗口管理或布局系统
- 不支持可变宽度字符（如 CJK 全角字符的特殊处理）
- 不实现专业 GUI 框架的高级特性（如矢量绘制、富文本排版）

## Decisions

### Decision 1: 独立的 TUI 符号纹理

**选择：** 创建独立的 `symbols_tui.png` 文件，每个 cell 为 8x16 像素

**理由：**
- 与现有 `symbols.png`（16x16）分离，避免混淆
- 保持相同的 128x128 网格布局，便于符号索引复用
- 允许独立优化 TUI 字符的视觉效果

**替代方案：**
- 在单个纹理中混合 1:1 和 1:2 符号 → 复杂度高，索引混乱
- 运行时缩放 1:1 符号 → 视觉效果差，失真明显

### Decision 2: 双坐标系统

**选择：** `MouseEvent` 同时提供 `(column, row)` 和 `(column_tui, row_tui)` 两套坐标

**理由：**
- 职责分离：TUI 组件用 TUI 坐标，游戏对象用 Sprite 坐标
- 应用层自主选择，无需复杂的坐标转换逻辑
- 向后兼容：现有代码继续使用 `column/row`

**替代方案：**
- 单一坐标 + 区域判断 → 需要应用层维护区域映射，复杂度高
- 动态坐标类型 → 需要运行时类型检查，性能和易用性差

### Decision 3: TUI 层渲染顺序

**选择：** Main Buffer（TUI 层）在 `generate_render_buffer` 中最后添加到 `RenderCell` 数组

**理由：**
- GPU 按顺序渲染，后添加的在上层
- 确保 TUI 界面（如菜单、对话框）始终可见
- 无需修改 shader 或引入深度测试

**替代方案：**
- 使用 Z-index 或深度缓冲 → 增加渲染复杂度，违背简单原则
- 应用层控制渲染顺序 → 容易出错，不够健壮

### Decision 4: 符号尺寸配置

**选择：** 扩展为两套全局配置：
```rust
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();   // Sprite: 16
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();  // Sprite: 16
pub static PIXEL_TUI_WIDTH: OnceLock<f32> = OnceLock::new();   // TUI: 8
pub static PIXEL_TUI_HEIGHT: OnceLock<f32> = OnceLock::new();  // TUI: 16
```

**理由：**
- 清晰区分 TUI 和 Sprite 的符号尺寸
- 保持现有代码对 `PIXEL_SYM_*` 的使用不变
- 允许未来支持其他宽高比（如 2:3）

**替代方案：**
- 单一尺寸 + 缩放因子 → 不够直观，容易混淆
- 运行时查表 → 性能开销，不必要的复杂度

### Decision 5: 渲染管线集成

**选择：** 修改 `render_main_buffer` 使用 TUI 符号尺寸，但仍合并到统一的 `RenderCell` 数组

**理由：**
- 保持单次 draw call 的高性能
- 复用现有的实例化渲染管线
- 最小化 shader 修改（已支持可变尺寸的 `RenderCell.w/h`）

**替代方案：**
- 分离 TUI 和 Sprite 的 draw call → 性能下降，违背设计目标
- 使用多个 render pass → 过度设计，不符合简单原则

## Risks / Trade-offs

### Risk 1: 符号纹理资源增加

**风险：** 新增 `symbols_tui.png` 增加约 256KB 资源大小

**缓解措施：**
- 按需加载：仅在启用 TUI 模式时加载
- 使用压缩纹理格式（如 PNG 压缩）
- 对于不使用 TUI 的应用，无额外开销

### Risk 2: 鼠标坐标计算复杂度

**风险：** 双坐标计算可能引入性能开销或精度问题

**缓解措施：**
- 坐标转换是简单的除法运算，开销可忽略
- 在输入事件层一次性计算，后续无额外开销
- 添加单元测试验证坐标精度

### Risk 3: 向后兼容性

**风险：** 现有应用可能受到 `MouseEvent` 结构变化影响

**缓解措施：**
- 保留原有 `column/row` 字段，现有代码无需修改
- 新增字段使用默认值（与 `column/row` 相同）
- 添加配置选项，默认禁用 TUI 模式

## Migration Plan

### Phase 1: 基础设施（不影响现有应用）
1. 添加 `PIXEL_TUI_WIDTH/HEIGHT` 全局配置
2. 扩展 `MouseEvent` 结构（向后兼容）
3. 实现双坐标转换逻辑

### Phase 2: TUI 渲染支持
1. 创建 `symbols_tui.png` 资源
2. 修改 `render_main_buffer` 支持 TUI 符号
3. 调整渲染顺序确保 TUI 在上层

### Phase 3: 应用集成
1. 更新 UI 组件使用 TUI 坐标
2. 在 `ui_demo` 中验证
3. 提供配置选项和文档

### Rollback Plan
- Phase 1 可随时回滚（仅添加代码，未修改行为）
- Phase 2 需要移除 TUI 符号加载逻辑
- Phase 3 需要恢复 UI 组件的坐标使用

## Open Questions

1. **TUI 符号纹理内容：** 是否需要为 TUI 专门设计字符集，还是复用现有符号？
   - **建议：** 初期复用现有符号，后续根据需要优化

2. **混合渲染性能：** 在大量 TUI 元素和游戏精灵混合时，单次 draw call 是否仍然高效？
   - **建议：** 在 `ui_demo` 中添加压力测试场景

3. **多分辨率支持：** 不同 DPI 下，8x16 的 TUI 字符是否需要特殊处理？
   - **建议：** 复用现有的 `ratio_x/ratio_y` 缩放机制

4. **TUI 模式配置：** 应该在编译时还是运行时选择 TUI 模式？
   - **建议：** 运行时配置，提供 `enable_tui_mode()` API

