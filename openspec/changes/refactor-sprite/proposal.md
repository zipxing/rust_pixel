# Refactor: 统一 Sprite 架构 - Widget + Sprite 二元模型

## Why

当前 rust_pixel 的渲染架构存在概念重叠和复杂性：

1. **三种 Sprite 概念**：Normal Sprite（字符对齐）、Pixel Sprite（像素精确）、UI Buffer，职责边界不清晰
2. **Normal Sprite 与 Widget 功能重叠**：两者都用于渲染 TUI 内容（字符），导致 API 混乱
3. **命名不够直观**：Panel 作为核心容器，命名不够形象；Sprites 作为类名不够清晰
4. **架构不够纯粹**：TUI 内容和图形内容混合在同一个层级管理

这导致：
- 开发者需要理解三种不同的 Sprite 类型
- TUI 渲染有两套 API（Normal Sprite 和 Widget）
- 代码维护成本高，概念不够清晰

## What Changes

### 核心理念
**去除 Normal Sprite 概念，建立 Widget + Sprite 二元模型：**
- **Widget**：专门处理 TUI 内容（文本、UI 组件）
- **Sprite**：专门处理图形内容（像素、图片、动画），即原来的 Pixel Sprite

### 架构变更

**旧架构（三种 Sprite）**
```
Panel
├── Normal Sprites (字符对齐) ❌ 冗余
├── Pixel Sprites  (像素精确) ✅
└── UI Buffer → mainbuffer    ❌ 特殊处理
```

**新架构（Widget + Sprite）**
```
Stage（舞台）
├── TUI Sprite (mainbuffer 载体) ← 所有 Widgets 渲染到这里
│   └── UIApp / 独立 Widgets
└── Sprites (都是 pixel sprites)
    └── 图形模式：完整功能（旋转、透明、缩放、像素定位）
    └── 文本模式：退化使用（字符对齐，忽略图形特性）
```

### 关键改动

1. **Panel → Stage**
   - 更形象的命名：舞台承载所有演出内容
   - 简化为两个核心容器：`tui_sprite` 和 `sprites`

2. **Sprites → SpriteLayer**
   - 更清晰的命名：精灵层包含多个精灵
   - 去除 `is_pixel` 标记（所有 Sprite 都是 pixel sprite）

3. **去除 Normal Sprite**
   - 所有 TUI 内容通过 Widget 系统渲染
   - Sprite 专注于图形渲染
   - 去除 `set_color_str` 等字符渲染 API（保留给 Widget）

4. **TUI Sprite 作为 mainbuffer 载体**
   - `stage.tui_sprite.content` 就是 mainbuffer
   - 所有 Widget 渲染到这个 buffer
   - 与 Sprite 系统平等对待，统一渲染

5. **Sprite 在不同模式下的行为**
   - **图形模式**：完整功能（像素定位、旋转、透明、缩放）
   - **文本模式**：退化使用（字符对齐，忽略 `angle`/`alpha`/`scale` 属性）

## Impact

### Affected Specs
- rendering（重大修改）
- ui（轻微影响，Widget 系统保持不变）

### Affected Code
**核心渲染系统**
- `src/render/panel.rs` → `src/render/stage.rs`
  - 重命名 Panel → Stage
  - 简化为 `tui_sprite` + `sprites` 两个容器
  - 统一渲染流程

- `src/render/sprite/sprites.rs` → `src/render/sprite/sprite_layer.rs`
  - 重命名 Sprites → SpriteLayer
  - 去除 `is_pixel` 字段

- `src/render/sprite.rs`
  - 去除字符渲染相关 API（`set_color_str` 等）
  - 所有 Sprite 都是 pixel sprite，简化代码

- `src/render/adapter.rs`
  - `draw_all()` 接收 `tui_buffer: &Buffer` 和 `sprites: &mut SpriteLayer`
  - 简化渲染逻辑

**应用层代码（所有 apps/*）**
- 所有使用 `Panel` 的地方改为 `Stage`
- 所有使用 `add_sprite()` 添加 Normal Sprite 的地方：
  - 改为使用 Widget 系统
  - 或直接操作 `stage.tui_sprite.content` buffer
- Pixel Sprite 相关代码保持不变（已经是新架构）

**具体影响的应用**
- `apps/snake/src/render_*.rs` - 边框和消息使用 Widget 或 buffer 操作
- `apps/tetris/src/render_*.rs` - 同上
- `apps/tower/src/render_*.rs` - 同上
- `apps/ui_demo/src/render_*.rs` - 简化为直接渲染 UIApp 到 tui_sprite
- 其他所有 apps 的 render 模块

### Compatibility
- **不向后兼容**：这是一次架构重构，需要迁移代码
- **文本模式**：行为保持不变，只是 API 变化
- **图形模式**：行为保持不变，概念更清晰

### Migration Guide

**步骤 1：更新导入**
```rust
// 旧代码
use rust_pixel::render::panel::Panel;

// 新代码
use rust_pixel::render::stage::Stage;
```

**步骤 2：更新容器创建**
```rust
// 旧代码
let mut panel = Panel::new();

// 新代码
let mut stage = Stage::new();
```

**步骤 3：迁移 Normal Sprite → Widget**
```rust
// 旧代码（Normal Sprite）
let mut border = Sprite::new(0, 0, 80, 24);
border.set_color_str(10, 0, "Title", Color::Yellow, Color::Reset);
panel.add_sprite(border, "border");

// 新代码（方式 1：使用 Widget）
let title_label = Label::new("Title")
    .with_style(Style::default().fg(Color::Yellow));
stage.add_widget(Box::new(title_label));

// 新代码（方式 2：直接操作 buffer）
stage.tui_sprite.content.set_string(10, 0, "Title",
    Style::default().fg(Color::Yellow));
```

**步骤 4：Pixel Sprite 无需改动**
```rust
// 旧代码（保持不变）
#[cfg(graphics_mode)]
{
    let mut game_sprite = Sprite::new(1, 1, 60, 30);
    stage.add_sprite(game_sprite, "game");  // panel → stage
}
```

**步骤 5：更新 draw 调用**
```rust
// 旧代码
panel.draw(ctx)?;

// 新代码
stage.draw(ctx)?;
```

### Benefits

1. **概念纯粹**
   - Widget = TUI 系统（文本、UI 组件）
   - Sprite = 图形系统（像素、图片、动画）
   - Stage = 舞台（容器）

2. **代码简化**
   - 去掉"普通 Sprite"这个中间概念
   - Widget 和 Sprite 职责清晰，不重叠
   - 减少 API 数量和复杂度

3. **更好的语义**
   - Stage（舞台）比 Panel（面板）更形象
   - SpriteLayer（精灵层）比 Sprites（精灵们）更清晰
   - TUI Sprite 明确表示"TUI 内容的载体"

4. **降级优雅**
   - Sprite 在文本模式下自然退化为字符对齐
   - 不需要 `is_pixel` 标记区分
   - 统一的 Sprite 类型，简化逻辑

5. **统一渲染**
   ```rust
   // 简洁的渲染流程
   Stage::draw() {
       adapter.draw_all(
           tui_sprite.content,  // TUI 内容
           sprites,             // 图形内容
           stage
       )
   }
   ```

## Timeline

- **Phase 1（1-2 天）**：核心重构
  - 重命名 Panel → Stage
  - 重命名 Sprites → SpriteLayer
  - 修改 Stage 结构（tui_sprite + sprites）
  - 更新 adapter 接口

- **Phase 2（2-3 天）**：应用迁移
  - 迁移 ui_demo（示例应用）
  - 迁移 snake
  - 迁移 tetris
  - 迁移其他应用

- **Phase 3（1 天）**：测试和文档
  - 完整测试所有模式
  - 更新文档和示例
  - 更新 CLAUDE.md

**总计：4-6 天**
