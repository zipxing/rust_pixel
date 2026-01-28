## REMOVED Requirements

### Requirement: Normal Sprite 类型
**Reason**: Normal Sprite 与 Widget 系统功能重叠（都用于 TUI 内容渲染），导致概念冗余和 API 混乱。TUI 内容统一由 Widget 系统处理。
**Migration**: 使用 Widget 系统（Label/Button 等）或直接操作 `Stage.tui_sprite.content` buffer 替代 Normal Sprite。

### Requirement: Panel 容器类型
**Reason**: Panel 命名不够形象，且包含多层管理的复杂逻辑。新架构简化为 Stage（舞台）+ 两个明确容器。
**Migration**: 将所有 `Panel` 引用替换为 `Stage`，API 一致，仅类型名变化。

### Requirement: Sprites 类型名
**Reason**: `Sprites`（复数）作为类名容易与单个 `Sprite` 混淆。
**Migration**: 将所有 `Sprites` 引用替换为 `SpriteLayer`。

## ADDED Requirements

### Requirement: Stage 统一渲染容器
渲染系统 SHALL 使用 `Stage`（舞台）作为统一的渲染容器，包含 `tui_sprite` 和 `sprites` 两个明确的子容器，简化多层管理逻辑。

#### Scenario: Stage 初始化
- **当** 应用创建 Stage 时
- **则** 自动初始化 `tui_sprite`（TUI 内容载体）和 `sprites`（SpriteLayer，图形精灵容器）
- **且** 双缓冲 `buffers[2]` 用于 diff 优化
- **且** 不需要手动管理层索引或层标签

#### Scenario: Stage 渲染流程
- **当** 调用 `stage.draw(ctx)` 时
- **则** 将 `tui_sprite.content`（TUI buffer）和 `sprites`（图形精灵）一起提交到 Adapter
- **且** Adapter 负责按分层优先级渲染（TUI 层在 Sprite 层之上）
- **且** 渲染流程与之前相同，概念更清晰

#### Scenario: Stage 辅助方法
- **当** 应用需要添加图形精灵时
- **则** 使用 `stage.add_sprite(sprite, tag)` 添加到 sprites 层
- **当** 应用需要获取 TUI buffer 时
- **则** 使用 `stage.tui_buffer_mut()` 或直接访问 `stage.tui_sprite.content`

### Requirement: TUI Sprite 作为 mainbuffer 载体
渲染系统 SHALL 使用一个特殊的 Sprite（`tui_sprite`）作为 mainbuffer 的载体，所有 Widget 内容渲染到该 Sprite 的 buffer 中。

#### Scenario: Widget 渲染到 TUI Sprite
- **当** 应用使用 UIApp 或独立 Widget 渲染 TUI 内容时
- **则** 所有 Widget 通过 `render_into()` 渲染到 `stage.tui_sprite.content` buffer
- **且** TUI Sprite 的 buffer 就是 mainbuffer（TUI 层的数据源）
- **且** 不需要额外的 merge 步骤

#### Scenario: TUI Sprite 参与统一渲染
- **当** Stage 执行渲染时
- **则** TUI Sprite 的 buffer 作为 TUI 层提交给 Adapter
- **且** 与图形 Sprites 一起参与统一渲染管线
- **且** TUI 层始终在图形层之上

#### Scenario: TUI Sprite 清空和更新
- **当** 应用在每帧开始时需要更新 TUI 内容
- **则** 调用 `stage.tui_sprite.content.reset()` 清空 buffer
- **且** 然后渲染新的 Widget 内容到 buffer
- **且** Stage::draw() 使用最新的 buffer 数据

### Requirement: SpriteLayer 精灵层管理
渲染系统 SHALL 使用 `SpriteLayer` 管理图形精灵集合，去除 `is_pixel` 标记，所有 Sprite 都视为 pixel sprite。

#### Scenario: SpriteLayer 创建和管理
- **当** Stage 初始化时
- **则** 创建一个 SpriteLayer 实例用于管理所有图形精灵
- **且** SpriteLayer 提供 `add(sprite, tag)`、`get(tag)` 方法管理精灵
- **且** 不需要 `is_pixel` 字段区分精灵类型

#### Scenario: SpriteLayer 渲染
- **当** Stage 执行渲染时
- **则** SpriteLayer 中的所有 Sprite 参与图形层渲染
- **且** 每个 Sprite 按 render_weight 排序渲染
- **且** 渲染结果作为图形层提交给 Adapter

### Requirement: Sprite 统一类型（二元模型）
渲染系统 SHALL 采用 Widget + Sprite 二元模型：Widget 专门处理 TUI 内容，Sprite 专门处理图形内容。Sprite 在不同渲染模式下行为自动适配。

#### Scenario: 图形模式下 Sprite 完整功能
- **当** 应用在图形模式下使用 Sprite 时
- **则** 支持像素精确定位、旋转（angle）、透明度（alpha）、缩放（scale_x, scale_y）
- **且** 使用 `set_graph_sym()` 设置图形符号
- **且** 所有图形特性正常工作

#### Scenario: 文本模式下 Sprite 退化使用
- **当** 应用在文本模式下使用 Sprite 时
- **则** 旋转、透明度、缩放属性被忽略
- **且** 位置对齐到字符网格
- **且** 渲染内容通过字符映射显示
- **且** 不产生错误，静默降级

#### Scenario: Widget 处理 TUI 内容
- **当** 应用需要渲染文本或 UI 组件时
- **则** 使用 Widget 系统（Label、Button、List 等）
- **且** Widget 渲染到 TUI Sprite 的 buffer
- **且** 不使用 Sprite 来渲染字符内容
- **且** 职责清晰：Widget = TUI, Sprite = 图形

### Requirement: Adapter draw_all 统一接口
渲染 Adapter SHALL 接收统一的参数：`tui_buffer`（TUI 内容）和 `sprites`（SpriteLayer，图形精灵），简化渲染接口。

#### Scenario: draw_all 参数结构
- **当** Stage 调用 Adapter 渲染时
- **则** 传递 `tui_buffer: &Buffer`（TUI Sprite 的 buffer）
- **且** 传递 `sprites: &mut SpriteLayer`（单个精灵层）
- **且** 不再传递 `Vec<Sprites>`（多层列表）
- **且** 接口简化，语义明确

#### Scenario: 文本模式 Adapter 行为
- **当** CrosstermAdapter 接收 draw_all 调用时
- **则** 使用 tui_buffer 作为输出源
- **且** sprites 中的精灵按字符映射渲染到终端
- **且** 行为与之前一致

#### Scenario: 图形模式 Adapter 行为
- **当** 图形 Adapter（SDL/Glow/WGPU/Web）接收 draw_all 调用时
- **则** 将 tui_buffer 和 sprites 合并到 RenderCell 数组
- **且** TUI 层渲染在 Sprite 层之上（保持现有优先级）
- **且** 单次 draw call 渲染所有内容
