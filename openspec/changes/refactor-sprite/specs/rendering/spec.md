## REMOVED Requirements

### Requirement: Normal Sprite 类型
**Reason**: Normal Sprite 与 Widget 系统功能重叠（都用于 TUI 内容渲染），导致概念冗余和 API 混乱。TUI 内容统一由 Widget 系统处理。
**Migration**: 使用 Widget 系统（Label/Button 等）或直接操作 `scene.tui_buffer_mut()` 替代 Normal Sprite。

### Requirement: Panel 容器类型
**Reason**: Panel 命名不够形象。新架构使用 Scene（场景）作为统一容器。
**Migration**: 将所有 `Panel` 引用替换为 `Scene`。

### Requirement: Sprites 类型名
**Reason**: `Sprites`（复数）作为类名容易与单个 `Sprite` 混淆。
**Migration**: 将所有 `Sprites` 引用替换为 `Layer`。

### Requirement: is_pixel 标记
**Reason**: 不再需要区分 Normal Sprite 和 Pixel Sprite，所有 Sprite 统一为 pixel sprite。
**Migration**: 去除 `is_pixel: bool` 字段，`Layer::new()` 不再需要 `is_pixel` 参数。

## ADDED Requirements

### Requirement: Scene 统一渲染容器
渲染系统 SHALL 使用 `Scene`（场景）作为统一的渲染容器，保持 `layers: Vec<Layer>` 多层结构，默认初始化 tui 层和 sprite 层。

#### Scenario: Scene 初始化
- **当** 应用创建 Scene 时
- **则** 自动初始化两个层：
  - `layers[0]`: "tui" 层（render_weight: 100），包含全屏 buffer sprite
  - `layers[1]`: "sprite" 层（render_weight: 0），用于图形精灵
- **且** 双缓冲 `buffers[2]` 用于 diff 优化
- **且** 保留 `layer_tag_index` 支持按名称访问层

#### Scenario: Scene 渲染流程
- **当** 调用 `scene.draw(ctx)` 时
- **则** 按 render_weight 排序渲染所有层
- **且** Adapter 负责按分层优先级渲染（tui 层在 sprite 层之上）
- **且** 渲染流程与之前相同，概念更清晰

#### Scenario: Scene 辅助方法
- **当** 应用需要添加图形精灵时
- **则** 使用 `scene.add_sprite(sprite, tag)` 添加到 sprite 层
- **当** 应用需要获取 TUI buffer 时
- **则** 使用 `scene.tui_buffer_mut()` 获取 tui 层的 buffer

### Requirement: TUI Layer 作为 mainbuffer 载体
渲染系统 SHALL 使用 "tui" 层作为 mainbuffer 的载体，该层包含一个名为 "buffer" 的全屏 Sprite。

#### Scenario: Widget 渲染到 TUI Layer
- **当** 应用使用 UIApp 或独立 Widget 渲染 TUI 内容时
- **则** 所有 Widget 通过 `render_into()` 渲染到 `scene.tui_buffer_mut()`
- **且** TUI Layer 的 buffer sprite 就是 mainbuffer（TUI 层的数据源）
- **且** 不需要额外的 merge 步骤

#### Scenario: TUI Layer 参与统一渲染
- **当** Scene 执行渲染时
- **则** TUI Layer 作为一个普通层参与渲染
- **且** 通过 render_weight 控制层级顺序
- **且** 默认 TUI 层在图形层之上（render_weight: 100 > 0）

#### Scenario: TUI Buffer 清空和更新
- **当** 应用在每帧开始时需要更新 TUI 内容
- **则** 调用 `scene.tui_buffer_mut().reset()` 清空 buffer
- **且** 然后渲染新的 Widget 内容到 buffer
- **且** Scene::draw() 使用最新的 buffer 数据

### Requirement: Layer 精灵层管理
渲染系统 SHALL 使用 `Layer` 管理精灵集合，去除 `is_pixel` 标记，所有 Sprite 都视为 pixel sprite。

#### Scenario: Layer 创建和管理
- **当** 创建 Layer 时
- **则** 使用 `Layer::new(name)` 创建
- **且** Layer 提供 `add(sprite, tag)`、`get(tag)` 方法管理精灵
- **且** 不需要 `is_pixel` 字段区分精灵类型

#### Scenario: Layer 渲染
- **当** Scene 执行渲染时
- **则** 按 render_weight 排序所有 Layer
- **且** 每个 Layer 内部按 sprite 的 render_weight 排序渲染
- **且** 渲染结果提交给 Adapter

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
- **且** Widget 渲染到 tui layer 的 buffer
- **且** 不使用 Sprite 来渲染字符内容
- **且** 职责清晰：Widget = TUI, Sprite = 图形

### Requirement: Adapter draw_all 统一接口
渲染 Adapter SHALL 接收统一的参数：`layers: &mut Vec<Layer>`，保持多层支持。

#### Scenario: draw_all 参数结构
- **当** Scene 调用 Adapter 渲染时
- **则** 传递 `current_buffer: &Buffer`
- **且** 传递 `previous_buffer: &Buffer`
- **且** 传递 `layers: &mut Vec<Layer>`（多层列表）
- **且** 保持多层扩展能力

#### Scenario: 文本模式 Adapter 行为
- **当** CrosstermAdapter 接收 draw_all 调用时
- **则** 渲染所有层到终端
- **且** 按 render_weight 顺序渲染
- **且** 行为与之前一致

#### Scenario: 图形模式 Adapter 行为
- **当** 图形 Adapter（SDL/Glow/WGPU/Web）接收 draw_all 调用时
- **则** 遍历所有层，合并到 RenderCell 数组
- **且** 按 render_weight 顺序渲染（tui 层在 sprite 层之上）
- **且** 单次 draw call 渲染所有内容

### Requirement: 多层扩展支持
渲染系统 SHALL 保持 `layers: Vec<Layer>` 结构，允许应用添加自定义层。

#### Scenario: 添加自定义层
- **当** 应用需要添加额外的渲染层时
- **则** 使用 `scene.add_layer(name)` 添加新层
- **且** 通过 `scene.set_layer_weight(name, weight)` 设置层级顺序
- **且** 新层与默认层平等参与渲染

#### Scenario: 层级顺序控制
- **当** 渲染多个层时
- **则** 按 render_weight 从大到小排序（大的在上层）
- **且** 默认 tui 层 weight=100，sprite 层 weight=0
- **且** 应用可以自定义层的 weight 来控制顺序
