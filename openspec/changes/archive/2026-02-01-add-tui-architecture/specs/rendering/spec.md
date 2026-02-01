## ADDED Requirements

### Requirement: Sprite 符号使用 Unicode 私有使用区

渲染系统 MUST 使用 Unicode 私有使用区 (U+E000~U+E0FF) 作为 Sprite 符号的索引映射，避免与标准 Unicode 字符冲突。

#### 场景：Sprite 符号映射
- **当** 系统生成 Sprite 符号字符时
- **则** 使用 `cellsym(idx)` 生成 U+E000 + idx 的 Unicode 字符
- **且** UTF-8 编码为 3 字节：`0xEE 0x80~0x83 0x80~0xBF`

#### 场景：Sprite 符号解析
- **当** 系统解析字符串中的 Sprite 符号时
- **则** 使用 `symidx(symbol)` 识别私有使用区字符
- **且** 检测 UTF-8 第一个字节为 `0xEE`，第二个字节高 6 位为 `0x20`
- **且** 从后两个字节提取 8 位索引：`((byte[1] & 3) << 6) + (byte[2] & 0x3F)`

#### 场景：避免 Unicode 字符冲突
- **当** 用户在 TUI 模式显示数学公式如 "∀x ∈ ℝ: x² ≥ 0"
- **则** 数学符号正常显示，不会被错误映射到 Sprite 纹理
- **当** 用户在 TUI 模式显示箭头符号如 "→ ↑ ↓ ←"
- **则** 箭头符号正常显示，不会被错误映射到 Sprite 纹理

#### 场景：符号集扩展性
- **当** 未来需要扩展 Sprite 符号数量时
- **则** 可以使用 U+E100~U+E1FF, U+E200~U+E2FF 等连续区域
- **且** BMP 私有使用区提供 6400 个码点 (U+E000~U+F8FF)
- **且** 扩展无需修改 UTF-8 编码逻辑（仍为 3 字节）

### Requirement: 统一符号纹理与四区域布局（Block-Based）

渲染系统 MUST 支持统一的 4096x4096 符号纹理（`symbols.png`），使用 block-based 布局，包含四个区域：Sprite 符号（16x16 像素）、TUI 符号（16x32 像素）、Emoji（32x32 彩色）、CJK 汉字（32x32）。

#### 场景：统一纹理布局（10 Sprite Rows）
- **当** 系统加载符号纹理时
- **则** 纹理尺寸为 4096x4096 像素
- **且** 顶部 2560 像素（行 0-2559）包含 Sprite 符号，每个字符 16x16 像素
- **且** 行 2560-3071（512px）包含 TUI + Emoji 符号（16 blocks）
- **且** 行 3072-4095（1024px）包含 CJK 汉字

#### 场景：Sprite 符号区域布局（Block 0-159）
- **当** 解析 Sprite 区域（行 0-2559）时
- **则** 该区域使用 block-based 布局：10 rows × 16 blocks/row = 160 blocks
- **且** 每个 block 包含 16×16 个字符（256×256 像素）
- **且** 每个字符占用 16 像素宽 × 16 像素高
- **且** Sprite 字符总数为 160 × 256 = 40,960 个字符
- **且** Sprite 符号在统一符号数组中占用线性索引 0-40959
- **且** 索引计算公式：`linear_index = texidx * 256 + symidx`（texidx: 0-159）

#### 场景：TUI 符号区域布局（Block 160-169）
- **当** 解析 TUI 区域（行 2560-3071）时
- **则** 该区域使用 block-based 布局：10 blocks
- **且** 每个 block 包含 16×16 个字符（256×512 像素）
- **且** 每个字符占用 16 像素宽 × 32 像素高
- **且** TUI 字符总数为 10 × 256 = 2,560 个字符
- **且** TUI 符号在统一符号数组中占用线性索引 40960-43519
- **且** 索引计算公式：`linear_index = 40960 + (texidx - 160) * 256 + symidx`（texidx: 160-169）

#### 场景：Emoji 区域布局（Block 170-175）
- **当** 解析 Emoji 区域（行 2560-3071，x=2560 开始）时
- **则** 该区域使用 block-based 布局：6 blocks
- **且** 每个 block 包含 8×16 个 Emoji（256×512 像素）
- **且** 每个 Emoji 占用 32 像素宽 × 32 像素高
- **且** Emoji 总数为 6 × 128 = 768 个
- **且** Emoji 在统一符号数组中占用线性索引 43520-44287
- **且** 索引计算公式：`linear_index = 43520 + (texidx - 170) * 128 + symidx`（texidx: 170-175）
- **且** Emoji 使用 RGBA 彩色格式

#### 场景：CJK 汉字区域布局（行 3072-4095）
- **当** 解析 CJK 区域（行 3072-4095）时
- **则** 该区域使用网格布局：128 列 × 32 行 = 4,096 个汉字
- **且** 每个汉字占用 32 像素宽 × 32 像素高
- **且** CJK 符号在统一符号数组中占用线性索引 44288-48383
- **且** 索引计算公式：`pixel_x = (cjk_idx % 128) * 32, pixel_y = 3072 + (cjk_idx / 128) * 32`

#### 场景：文本模式保持不变
- **当** 应用在文本模式运行时
- **则** 加载相同的纹理但只使用 Sprite 符号
- **且** 所有渲染使用终端字符单元，与之前相同

### Requirement: 预制 Emoji 支持

渲染系统 MUST 支持预制 Emoji 的映射、渲染和双宽字符（wcwidth=2）处理，为 TUI 模式提供常用 Emoji 显示能力。

#### 场景：Emoji 映射表
- **当** 系统初始化时
- **则** 创建 `EMOJI_MAP: HashMap<String, u16>`，将 Emoji 字符映射到纹理索引
- **且** 预制 768 个最常用 Emoji（表情、符号、食物、自然等）
- **且** Emoji 索引范围为 43520-44287（Emoji 区域，Block 170-175）
- **且** 未映射的 Emoji 显示为空白或占位符

#### 场景：Emoji 识别
- **当** `set_stringn` 处理字符串时
- **则** 使用 `unicode-width` crate 检测字符宽度
- **且** 使用 `is_prerendered_emoji()` 检查是否为预制 Emoji
- **且** wcwidth=2 且在映射表中的字符识别为 Emoji

#### 场景：Emoji 双宽字符处理
- **当** 渲染预制 Emoji 时
- **则** Emoji 占用 2 个 Cell 宽度（符合终端 wcwidth=2）
- **且** 第一个 Cell 存储 Emoji 字符和符号索引
- **且** 第二个 Cell 设为空白（占位）
- **且** 渲染时 Emoji 显示为 2 倍字符宽度

#### 场景：未预制 Emoji 处理
- **当** 遇到 wcwidth=2 但未在映射表中的 Emoji 时
- **则** 显示为空白占位符
- **且** 仍然占用 2 个 Cell 宽度
- **且** 不影响其他字符的正常显示

#### 场景：Emoji 纹理坐标计算
- **当** 渲染 Emoji（线性索引 43520-44287）时
- **则** Block 计算：`texidx = 170 + (emoji_idx - 43520) / 128`
- **且** Block 内索引：`symidx = (emoji_idx - 43520) % 128`
- **且** 纹理 X 坐标 = `2560 + (texidx - 170) * 256 + (symidx % 8) * 32`
- **且** 纹理 Y 坐标 = `2560 + (symidx / 8) * 32`
- **且** 源纹理尺寸为 32x32 像素
- **且** 目标渲染尺寸为 `cell_width * 2` × `cell_height`

#### 场景：Emoji 常用分类
- **当** 选择预制 Emoji 集时
- **则** 包含表情与情感类（😀😊😂🤔😭🥺等，约 100 个）
- **且** 包含符号与标志类（✅❌⚠️🔥⭐🌟等，约 80 个）
- **且** 包含箭头与指示类（➡️⬅️⬆️⬇️等，约 50 个）
- **且** 包含食物与饮料类（🍕🍔🍩🍰🍭等，约 50 个）
- **且** 包含自然与动物类（🌈🌸🍀🐱🐶等，约 50 个）
- **且** 包含对象与工具类（📁📂📊🔧💻等，约 50 个）
- **且** 包含活动与运动类（⚽🏀🎮🎯等，约 30 个）
- **且** 预留空间 358 个，供用户自定义或未来扩展

### Requirement: 区域感知的符号索引计算

渲染系统 MUST 根据渲染层（TUI、Emoji 或 Sprite）自动计算正确的纹理符号索引，确保从统一纹理的正确区域采样。

#### 场景：Sprite 层符号索引计算（Block 0-159）
- **当** 渲染 Pixel Sprites 的字符时
- **则** 使用 Sprite 区域的 block-based 索引计算
- **且** 线性索引计算：`linear_index = texidx * 256 + symidx`（texidx: 0-159）
- **且** Block 位置：`block_x = texidx % 16, block_y = texidx / 16`
- **且** 纹理坐标：`pixel_x = block_x * 256 + (symidx % 16) * 16, pixel_y = block_y * 256 + (symidx / 16) * 16`
- **且** 最终索引范围为 0-40959

#### 场景：TUI 层符号索引计算（Block 160-169）
- **当** 渲染 Main Buffer（TUI 层）的字符时
- **则** 使用 TUI 区域的 block-based 索引计算
- **且** 线性索引计算：`linear_index = 40960 + (texidx - 160) * 256 + symidx`（texidx: 160-169）
- **且** Block 位置：`block_num = texidx - 160`（0-9）
- **且** 纹理坐标：`pixel_x = block_num * 256 + (symidx % 16) * 16, pixel_y = 2560 + (symidx / 16) * 32`
- **且** 最终索引范围为 40960-43519

#### 场景：Emoji 层符号索引计算（Block 170-175）
- **当** 渲染 Emoji 字符时
- **则** 使用 Emoji 区域的 block-based 索引计算
- **且** 线性索引计算：`linear_index = 43520 + (texidx - 170) * 128 + symidx`（texidx: 170-175）
- **且** Block 位置：`block_num = texidx - 170`（0-5）
- **且** 纹理坐标：`pixel_x = 2560 + block_num * 256 + (symidx % 8) * 32, pixel_y = 2560 + (symidx / 8) * 32`
- **且** 最终索引范围为 43520-44287

#### 场景：CJK 层符号索引计算（行 3072-4095）
- **当** 渲染 CJK 汉字时
- **则** 使用 CJK 区域的网格索引计算
- **且** 线性索引计算：`linear_index = 44288 + cjk_idx`
- **且** 纹理坐标：`pixel_x = (cjk_idx % 128) * 32, pixel_y = 3072 + (cjk_idx / 128) * 32`
- **且** 最终索引范围为 44288-48383

#### 场景：渲染层自动区分
- **当** `render_main_buffer` 生成 RenderCell 时
- **则** 自动使用 TUI 区域索引计算
- **当** `render_pixel_sprites` 生成 RenderCell 时
- **则** 自动使用 Sprite 区域索引计算
- **且** 应用层代码无需关心区域差异

#### 场景：Cell 数据结构保持不变
- **当** Cell 存储字符信息时
- **则** 仍使用 `symbol`（字符串）、`tex`（区块索引 0-255）、`fg`、`bg` 字段
- **且** 不需要添加新的区域标识字段
- **且** 现有的 `Cell::get_cell_info()` 方法保持不变
- **且** 区域逻辑完全在渲染层处理

### Requirement: 统一坐标系统（水平共享，垂直转换）

输入系统 MUST 在鼠标事件中使用统一的坐标系统，按 16 像素宽度计算，水平方向 TUI 和 Sprite 共享，垂直方向 TUI 需要除以 2。

#### 场景：统一坐标计算
- **当** 在图形模式下发生鼠标事件时
- **则** `MouseEvent.column` 使用 16 像素宽度计算：`column = pixel_x / 16`
- **且** `MouseEvent.row` 使用 16 像素高度计算：`row = pixel_y / 16`
- **且** 坐标基于 Sprite 坐标系（16x16 像素）

#### 场景：TUI 层坐标转换
- **当** TUI 组件（如 Button、Modal）处理鼠标事件时
- **则** 水平方向直接使用：`column_tui = MouseEvent.column`（16 像素宽度相同）
- **且** 垂直方向除以 2：`row_tui = MouseEvent.row / 2`（TUI 是 32 像素高）
- **且** 命中测试准确对应 TUI 字符单元（16x32 像素）

#### 场景：Sprite 层直接使用坐标
- **当** 游戏代码处理 Pixel Sprite 的鼠标事件时
- **则** 直接使用 `MouseEvent.column` 和 `MouseEvent.row`
- **且** 无需任何坐标转换
- **且** 坐标准确对应 Sprite 层（16x16 像素）的字符单元
- **且** 向后兼容，现有 Sprite 代码无需修改

### Requirement: TUI 层渲染优先级

渲染系统 MUST 确保在图形模式下，TUI 层（Main Buffer）始终渲染在所有 Pixel Sprite 层之上。

#### 场景：TUI 覆盖在游戏精灵之上
- **当** 场景同时包含 Pixel Sprite 和 TUI 元素时
- **则** 所有 Pixel Sprite 首先渲染
- **且** TUI 层（Main Buffer）最后渲染
- **且** TUI 元素显示在所有游戏对象之上

#### 场景：RenderCell 数组中的渲染顺序
- **当** 生成 RenderCell 数组时
- **则** Pixel Sprite 单元首先添加
- **且** Main Buffer（TUI）单元最后添加
- **且** GPU 按数组顺序渲染，确保正确的分层

### Requirement: TUI 符号尺寸配置

系统 MUST 使用统一的符号尺寸配置，TUI 高度为 Sprite 高度的 2 倍。

#### 场景：统一尺寸配置
- **当** 系统初始化时
- **则** `PIXEL_SYM_WIDTH` 设置为 16.0 像素（Sprite 和 TUI 共享）
- **且** `PIXEL_SYM_HEIGHT` 设置为 16.0 像素（Sprite 基准高度）
- **且** TUI 宽度 = `PIXEL_SYM_WIDTH`（16 像素）
- **且** TUI 高度 = `PIXEL_SYM_HEIGHT * 2`（32 像素）

#### 场景：TUI 渲染使用派生尺寸
- **当** 渲染 Main Buffer（TUI 层）时
- **则** 宽度直接使用 `PIXEL_SYM_WIDTH`
- **且** 高度使用 `PIXEL_SYM_HEIGHT * 2.0`
- **且** 无需额外的全局变量

#### 场景：Sprite 渲染使用基准尺寸
- **当** 渲染 Pixel Sprite 时
- **则** 直接使用 `PIXEL_SYM_WIDTH` 和 `PIXEL_SYM_HEIGHT`
- **且** 现有的精灵渲染不受影响

### Requirement: 单次绘制调用性能

渲染系统 MUST 通过将 TUI 和 Sprite 渲染单元合并到统一的 RenderCell 数组中，保持单次绘制调用的性能。

#### 场景：统一渲染管线
- **当** 渲染同时包含 TUI 和 Sprite 的帧时
- **则** 所有 RenderCell（TUI 和 Sprite）位于单个数组中
- **且** GPU 在一次实例化绘制调用中处理所有单元
- **且** 渲染性能与当前系统相当

#### 场景：着色器中的可变单元尺寸
- **当** 着色器处理不同尺寸的 RenderCell 时
- **则** 每个单元的 `w` 和 `h` 字段正确指定其大小
- **且** TUI 单元（16x32）和 Sprite 单元（16x16）在同一遍中正确渲染

### Requirement: TUI 架构始终启用

系统 SHALL在图形模式下始终启用 TUI 架构，支持 TUI（Main Buffer）和游戏精灵（Pixel Sprites）的混合渲染，无需配置。

#### 场景：启动时初始化 TUI 架构
- **当** 应用在图形模式下启动时
- **则** 统一纹理加载，包含 TUI 和 Sprite 符号区域
- **且** `PIXEL_TUI_*` 和 `PIXEL_SYM_*` 尺寸都被初始化
- **且** 鼠标事件包含 TUI 和 Sprite 两套坐标
- **且** 渲染管线支持 TUI 和 Sprite 混合渲染

#### 场景：应用选择渲染方式
- **当** 应用仅使用 Pixel Sprite（无 Main Buffer 内容）时
- **则** TUI 层渲染为空（无额外开销）
- **且** 应用的工作方式与之前完全相同
- **当** 应用使用 Main Buffer 渲染 TUI 元素时
- **则** TUI 元素使用 16x32 瘦字符渲染
- **且** TUI 层显示在所有 Pixel Sprite 之上

### Requirement: UI 组件坐标转换

UI 组件 MUST 在 Main Buffer 中渲染时，正确转换鼠标坐标以匹配 TUI 字符单元（16x32 像素）。

#### 场景：UI 组件鼠标命中测试
- **当** UI 组件（如 Button）接收鼠标事件时
- **则** 水平方向直接使用：`column_tui = mouse_event.column`
- **且** 垂直方向除以 2：`row_tui = mouse_event.row / 2`
- **且** 命中测试正确识别 TUI 渲染组件上的点击
- **且** 组件准确响应用户交互

#### 场景：游戏精灵鼠标处理无需转换
- **当** 游戏代码处理 Pixel Sprite 的鼠标事件时
- **则** 直接使用 `mouse_event.column` 和 `mouse_event.row`
- **且** 坐标准确对应 Sprite 层（16x16 像素）
- **且** 向后兼容，现有代码无需修改

### Requirement: TUI 样式修饰符支持

TUI 渲染系统 MUST 在图形模式下支持文本样式修饰符（粗体、斜体、下划线、暗淡、反转、删除线、隐藏），提供与文本模式样式能力的视觉一致性。

#### 场景：RenderCell 修饰符字段支持
- **当** 将 Cell 转换为 RenderCell 进行 TUI 渲染时
- **则** `RenderCell` 包含一个 `modifier` 字段，其中包含 Cell 的修饰符位标志
- **且** 修饰符信息在渲染管线中保留
- **且** GPU 着色器接收每个字符的修饰符数据

#### 场景：粗体文本渲染
- **当** TUI 内容使用 `Style::default().add_modifier(Modifier::BOLD)` 时
- **则** 文本在图形模式下以增加的视觉重量显示
- **且** 粗体效果通过渲染管线中的颜色强度调整实现
- **且** RGB 值乘以 1.3（限制在 1.0 以内）后创建 RenderCell
- **且** 样式提供与普通文本清晰的视觉区分

#### 场景：斜体文本渲染
- **当** TUI 内容使用 `Style::default().add_modifier(Modifier::ITALIC)` 时
- **则** 文本在图形模式下以斜体倾斜显示
- **且** 斜体效果通过着色器中的顶点变换实现
- **且** 倾斜角度提供与普通文本清晰的视觉区分

#### 场景：下划线文本渲染
- **当** TUI 内容使用 `Style::default().add_modifier(Modifier::UNDERLINED)` 时
- **则** 文本在图形模式下显示下划线
- **且** 下划线渲染为字符下方的水平线
- **且** 下划线颜色与前景色匹配

#### 场景：暗淡文本渲染
- **当** TUI 内容使用 `Style::default().add_modifier(Modifier::DIM)` 时
- **则** 文本在图形模式下以降低的不透明度显示
- **且** 暗淡效果通过渲染管线中的 alpha 通道调整实现
- **且** alpha 值乘以 0.6 后创建 RenderCell
- **且** 文本保持可读但视觉上不那么突出

#### 场景：反转文本渲染
- **当** TUI 内容使用 `Style::default().add_modifier(Modifier::REVERSED)` 时
- **则** 前景色和背景色在图形模式下交换
- **且** 颜色交换在创建 RenderCell 前在渲染管线中处理
- **且** 原前景色成为背景色
- **且** 原背景色成为前景色
- **且** 视觉效果与终端反转视频匹配

#### 场景：删除线文本渲染
- **当** TUI 内容使用 `Style::default().add_modifier(Modifier::CROSSED_OUT)` 时
- **则** 文本在图形模式下显示中间的水平线
- **且** 删除线在片段着色器中渲染
- **且** 线条颜色与前景色匹配

#### 场景：隐藏文本渲染
- **当** TUI 内容使用 `Style::default().add_modifier(Modifier::HIDDEN)` 时
- **则** 文本在图形模式下完全透明
- **且** 隐藏效果通过在渲染管线中将 alpha 设置为 0.0 实现
- **且** 字符空间保留但内容不可见

#### 场景：多修饰符组合
- **当** TUI 内容组合多个修饰符（如 BOLD + ITALIC + UNDERLINED）时
- **则** 所有指定的效果在图形模式下同时应用
- **且** 组合效果不会相互干扰
- **且** 视觉结果与预期的终端外观匹配

#### 场景：文本模式兼容性保持
- **当** 应用在文本模式运行时
- **则** 所有修饰符效果继续使用 crossterm ANSI 序列
- **且** 不对现有文本模式样式行为进行更改
- **且** 视觉外观与当前实现相同

#### 场景：闪烁修饰符被忽略
- **当** TUI 内容使用 `Modifier::SLOW_BLINK` 或 `Modifier::RAPID_BLINK` 时
- **则** 闪烁修饰符在图形模式下被忽略
- **且** 文本作为普通文本渲染，无闪烁动画
- **且** 对于不支持的闪烁效果不会生成错误或警告
