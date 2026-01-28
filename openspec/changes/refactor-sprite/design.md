# 设计文档：Sprite 架构统一重构

## Context

rust_pixel 当前的渲染架构存在三种 Sprite 概念：
1. **Normal Sprite**：字符对齐，使用 `set_color_str()` 等 API，merge 到 mainbuffer
2. **Pixel Sprite**：像素精确，使用 `set_graph_sym()` 等 API，绕过 mainbuffer 直接渲染
3. **UI Buffer**：UIApp 渲染到独立 buffer，最终 merge 到 mainbuffer

经过深入分析，发现 Normal Sprite 本质上就是为了渲染 TUI 内容（字符），这与 Widget 系统的目标完全一致。因此存在功能重叠和概念冗余。

**约束条件：**
- 必须保持文本模式和图形模式的功能不变
- 必须保持高性能渲染
- 必须简化概念，降低理解成本
- 迁移成本可控

**相关方：**
- 游戏开发者：需要清晰的 API 和概念
- UI 框架用户：Widget 系统保持不变
- 性能敏感应用：渲染性能不能降低

## Goals / Non-Goals

**Goals:**
- 建立清晰的 Widget + Sprite 二元模型
- 去除 Normal Sprite 概念，简化架构
- 更好的命名：Panel → Scene, Sprites → Layer
- 保持 `layers: Vec<Layer>` 结构，支持多层扩展
- 默认两层：tui 层（TUI 内容）+ sprite 层（图形精灵）
- 统一 Sprite 类型（都是 pixel sprite，在文本模式下退化使用）

**Non-Goals:**
- 不改变渲染性能特征
- 不改变 Widget 系统的实现
- 不改变文本模式和图形模式的视觉效果
- 不引入新的渲染特性

## Decisions

### Decision 1: 建立 Widget + Sprite 二元模型

**选择：** 去除 Normal Sprite，建立清晰的二元模型：
- **Widget**：专门处理 TUI 内容（文本、UI 组件）
- **Sprite**：专门处理图形内容（像素、图片、动画）

**理由：**
- Normal Sprite 的 `set_color_str()` 功能与 Widget 的 `render()` 功能重叠
- 两者都是为了渲染字符/文本内容到 buffer
- 统一到 Widget 系统，职责更清晰
- Sprite 专注图形渲染，API 更纯粹

**替代方案：**
1. 保持三种 Sprite（现状）
   - ❌ 概念复杂，学习成本高
   - ❌ API 重叠，维护成本高

2. 统一所有 Sprite，Widget 也变成特殊 Sprite
   - ❌ Widget 系统已经很完善，不应该强制改造
   - ❌ 增加 Widget 系统复杂度

3. 当前方案（Widget + Sprite）
   - ✅ 职责清晰，概念简单
   - ✅ API 不重叠，易于理解
   - ✅ 代码简化，维护成本低

### Decision 2: Panel → Scene 命名

**选择：** 将核心容器 `Panel` 重命名为 `Scene`（场景）

**理由：**
- "场景"是游戏引擎的通用术语（Unity/Godot 都用 Scene）
- Scene 包含多个 Layer（层），概念清晰
- 比 Stage（舞台）更通用，不仅限于表演隐喻

**结构定义：**
```rust
pub struct Scene {
    // 双缓冲（用于 diff 优化）
    pub buffers: [Buffer; 2],
    pub current: usize,

    // 层索引
    pub layer_tag_index: HashMap<String, usize>,

    // 多层支持
    pub layers: Vec<Layer>,

    // 渲染顺序索引
    pub render_index: Vec<(usize, i32)>,
}
```

**关键变化：**
- `Panel` → `Scene` 重命名
- 保持 `layers: Vec<Layer>` 结构，支持多层扩展
- 默认初始化两层：tui 层和 sprite 层

### Decision 3: Sprites → Layer 命名

**选择：** 将 `Sprites` 重命名为 `Layer`（层）

**理由：**
- `Sprites`（复数）作为类名不够清晰，容易与单个 `Sprite` 混淆
- `Layer` 更简洁，明确表示"这是一个渲染层"
- Layer 概念在渲染系统中很常见，易于理解

**结构定义：**
```rust
pub struct Layer {
    pub name: String,
    // 去除 is_pixel 字段 - 所有 sprite 都是 pixel sprite
    pub is_hidden: bool,
    pub sprites: Vec<Sprite>,
    pub tag_index: HashMap<String, usize>,
    pub render_index: Vec<(usize, i32)>,
    pub render_weight: i32,
}
```

**关键变化：**
- `Sprites` → `Layer` 重命名
- 去除 `is_pixel: bool` - 不再需要区分 Normal 和 Pixel Sprite
- 简化逻辑，所有 Sprite 统一处理

### Decision 4: TUI Layer 作为 mainbuffer 载体

**选择：** 创建 "tui" 层，包含一个全屏 buffer sprite 作为 mainbuffer 载体

**理由：**
- TUI 内容也是一个 Layer，概念统一
- 明确 mainbuffer 的归属：它是 tui layer 中 "buffer" sprite 的 content
- 保持多层扩展能力，未来可添加更多层

**默认层结构：**
```
Scene
├── layers[0]: "tui"      (render_weight: 100, 上层)
│   └── sprites[0]: "buffer"  → TUI buffer 载体
└── layers[1]: "sprite"   (render_weight: 0, 下层)
    ├── sprites[0]: "player"
    ├── sprites[1]: "enemy"
    └── ...
```

**渲染流程：**
```rust
impl Scene {
    pub fn new() -> Self {
        let (width, height) = (180, 80);
        let size = Rect::new(0, 0, width, height);

        let mut layers = vec![];
        let mut layer_tag_index = HashMap::new();

        // TUI 层 - 包含全屏 buffer sprite
        let mut tui_layer = Layer::new("tui");
        tui_layer.render_weight = 100;  // 在上层
        let tui_sprite = Sprite::new(0, 0, width, height);
        tui_layer.add(tui_sprite, "buffer");
        layers.push(tui_layer);
        layer_tag_index.insert("tui".to_string(), 0);

        // Sprite 层 - 图形精灵
        let mut sprite_layer = Layer::new("sprite");
        sprite_layer.render_weight = 0;  // 在下层
        layers.push(sprite_layer);
        layer_tag_index.insert("sprite".to_string(), 1);

        Scene {
            buffers: [Buffer::empty(size), Buffer::empty(size)],
            current: 0,
            layer_tag_index,
            layers,
            render_index: vec![],
        }
    }

    pub fn draw(&mut self, ctx: &mut Context) -> io::Result<()> {
        // 1. Widget 系统渲染到 tui layer 的 buffer sprite
        // （在应用层完成，例如 model.ui_app.render_into(scene.tui_buffer_mut())）

        // 2. 渲染所有层
        self.update_render_index();
        for idx in &self.render_index {
            if !self.layers[idx.0].is_hidden {
                self.layers[idx.0].render_all_to_buffer(
                    &mut ctx.asset_manager,
                    &mut self.buffers[self.current]
                );
            }
        }

        // 3. 统一提交到 adapter
        let cb = &self.buffers[self.current];
        let pb = &self.buffers[1 - self.current];
        ctx.adapter.draw_all(cb, pb, &mut self.layers, ctx.stage)?;

        // 4. 交换缓冲区
        self.buffers[1 - self.current].reset();
        self.current = 1 - self.current;

        Ok(())
    }

    // 便捷方法：获取 TUI buffer
    pub fn tui_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.layers[0].get("buffer").content
    }

    // 便捷方法：添加图形精灵到 sprite 层
    pub fn add_sprite(&mut self, sp: Sprite, tag: &str) {
        self.layers[1].add(sp, tag);
    }

    // 便捷方法：获取图形精灵
    pub fn get_sprite(&mut self, tag: &str) -> &mut Sprite {
        self.layers[1].get(tag)
    }
}
```

**使用示例：**
```rust
// 应用层代码
impl Render for MyRender {
    fn draw(&mut self, ctx: &mut Context, model: &mut MyModel, _dt: f32) {
        // 1. 清空 TUI buffer
        self.scene.tui_buffer_mut().reset();

        // 2. 渲染 UIApp 到 TUI buffer
        model.ui_app.render_into(self.scene.tui_buffer_mut())?;

        // 3. 或者使用独立 Widget
        let label = Label::new("Score: 100");
        label.render(self.scene.tui_buffer_mut(), ctx)?;

        // 4. Scene 统一渲染
        self.scene.draw(ctx)?;
    }
}
```

### Decision 5: Sprite 在不同模式下的行为

**选择：** 所有 Sprite 都是 pixel sprite，在文本模式下退化使用

**图形模式行为：**
```rust
impl Sprite {
    // 完整功能
    pub fn set_angle(&mut self, angle: f64) { /* 旋转 */ }
    pub fn set_alpha(&mut self, alpha: u8) { /* 透明 */ }
    pub fn set_scale(&mut self, x: f32, y: f32) { /* 缩放 */ }
    pub fn set_position(&mut self, x: u16, y: u16) { /* 像素定位 */ }

    // 像素级渲染
    pub fn set_graph_sym(&mut self, x: u16, y: u16, tex_id: u16, sym_id: u16, color: Color) {
        // 设置 16x16 图形符号
    }
}
```

**文本模式行为：**
```rust
impl Sprite {
    // 退化：忽略图形特性
    pub fn set_angle(&mut self, angle: f64) { /* 忽略 */ }
    pub fn set_alpha(&mut self, alpha: u8) { /* 忽略 */ }
    pub fn set_scale(&mut self, x: f32, y: f32) { /* 忽略 */ }
    pub fn set_position(&mut self, x: u16, y: u16) { /* 字符对齐 */ }

    // 使用字符渲染（内部实现）
    pub fn set_graph_sym(&mut self, x: u16, y: u16, tex_id: u16, sym_id: u16, color: Color) {
        // 映射到字符渲染
        // 注意：应用层应该使用 Widget 而不是 Sprite 来渲染 TUI 内容
    }
}
```

**理由：**
- 统一的 Sprite 类型，简化代码
- 不需要 `is_pixel` 标记
- 文本模式下自然降级，不需要特殊处理
- 编译时通过 `#[cfg(graphics_mode)]` 控制行为

### Decision 6: 去除 Sprite 的字符渲染 API

**选择：** 去除 `Sprite::set_color_str()` 等字符渲染 API

**理由：**
- 这些 API 与 Widget 系统功能重叠
- TUI 内容应该通过 Widget 或直接操作 buffer
- Sprite 专注于图形渲染，API 更纯粹

**迁移方案：**
```rust
// 旧代码（Normal Sprite）
let mut sprite = Sprite::new(0, 0, 80, 24);
sprite.set_color_str(10, 0, "Title", Color::Yellow, Color::Reset);
panel.add_sprite(sprite, "border");

// 新代码（方式 1：使用 Widget）
let label = Label::new("Title")
    .with_style(Style::default().fg(Color::Yellow));
stage.add_widget_to_tui(Box::new(label));

// 新代码（方式 2：直接操作 buffer）
stage.tui_sprite.content.set_string(
    10, 0, "Title",
    Style::default().fg(Color::Yellow)
);

// 新代码（方式 3：使用 UIApp）
model.ui_app.render_into(&mut stage.tui_sprite.content)?;
```

## Implementation Plan

### Phase 1: 核心重构（1-2 天）

**1.1 重命名核心类型**
- `src/render/panel.rs` → `src/render/scene.rs`
- `src/render/sprite/sprites.rs` → `src/render/sprite/layer.rs`
- 更新所有导入和引用

**1.2 修改 Scene 结构**
```rust
pub struct Scene {
    pub buffers: [Buffer; 2],
    pub current: usize,
    pub layer_tag_index: HashMap<String, usize>,
    pub layers: Vec<Layer>,
    pub render_index: Vec<(usize, i32)>,
}

impl Scene {
    pub fn new() -> Self {
        let (width, height) = (180, 80);
        let size = Rect::new(0, 0, width, height);

        let mut layers = vec![];
        let mut layer_tag_index = HashMap::new();

        // TUI 层
        let mut tui_layer = Layer::new("tui");
        tui_layer.render_weight = 100;
        let tui_sprite = Sprite::new(0, 0, width, height);
        tui_layer.add(tui_sprite, "buffer");
        layers.push(tui_layer);
        layer_tag_index.insert("tui".to_string(), 0);

        // Sprite 层
        let sprite_layer = Layer::new("sprite");
        layers.push(sprite_layer);
        layer_tag_index.insert("sprite".to_string(), 1);

        Scene {
            buffers: [Buffer::empty(size), Buffer::empty(size)],
            current: 0,
            layer_tag_index,
            layers,
            render_index: vec![],
        }
    }
}
```

**1.3 修改 Layer**
- 去除 `is_pixel: bool` 字段
- 简化构造函数：`Layer::new(name)` 不再需要 `is_pixel` 参数

**1.4 修改 Sprite**
- 标记 `set_color_str()` 等 API 为 deprecated（或直接移除）
- 在文本模式下，图形 API 退化为空操作

**1.5 更新 Adapter 接口**
```rust
pub trait Adapter {
    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        layers: &mut Vec<Layer>,     // 多层支持
        stage: u32,
    ) -> Result<(), Box<dyn Error>>;
}
```

**1.6 更新所有 Adapter 实现**
- `src/render/adapter/cross_adapter.rs`
- `src/render/adapter/sdl_adapter.rs`
- `src/render/adapter/winit_glow_adapter.rs`
- `src/render/adapter/winit_wgpu_adapter.rs`
- `src/render/adapter/web_adapter.rs`

### Phase 2: 应用迁移（2-3 天）

**2.1 迁移 ui_demo（作为示例）**
```rust
// apps/ui_demo/src/render_graphics.rs
pub struct UiDemoRender {
    pub scene: Scene,  // panel → scene
}

impl Render for UiDemoRender {
    fn draw(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        // 清空 TUI buffer
        self.scene.tui_buffer_mut().reset();

        // 渲染 UIApp 到 TUI buffer
        model.ui_app.render_into(self.scene.tui_buffer_mut())?;

        // Scene 统一渲染
        self.scene.draw(ctx)?;
    }
}
```

**2.2 迁移 snake**
- 边框和消息文字：改为直接操作 `scene.tui_buffer_mut()`
- 游戏画面：使用 `scene.add_sprite()` 添加到 sprite 层

**2.3 迁移 tetris**
- 同 snake

**2.4 迁移其他应用**
- tower
- poker
- 其他所有 apps

### Phase 3: 测试和文档（1 天）

**3.1 完整测试**
- 所有应用在文本模式下运行测试
- 所有应用在图形模式下运行测试
- 验证性能无降低

**3.2 更新文档**
- `CLAUDE.md` - 更新架构说明
- `README.md` - 更新示例代码
- `doc/` - 更新技术文档

**3.3 创建迁移指南**
- 详细的 API 迁移步骤
- 示例代码对比
- 常见问题解答

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_creation() {
        let scene = Scene::new();
        assert_eq!(scene.layers.len(), 2);  // tui + sprite
        assert_eq!(scene.layers[0].name, "tui");
        assert_eq!(scene.layers[1].name, "sprite");
    }

    #[test]
    fn test_layer_add_sprite() {
        let mut layer = Layer::new("test");
        let sprite = Sprite::new(10, 10, 20, 20);
        layer.add(sprite, "sprite1");
        assert_eq!(layer.sprites.len(), 1);
    }

    #[test]
    fn test_tui_buffer_rendering() {
        let mut scene = Scene::new();
        scene.tui_buffer_mut().set_string(
            0, 0, "Test",
            Style::default().fg(Color::White)
        );
        // 验证 buffer 内容
    }

    #[test]
    fn test_add_sprite_to_sprite_layer() {
        let mut scene = Scene::new();
        let sprite = Sprite::new(10, 10, 32, 32);
        scene.add_sprite(sprite, "player");
        assert_eq!(scene.layers[1].sprites.len(), 1);
    }
}
```

### Integration Tests
- 运行所有 apps 在文本模式和图形模式下
- 验证渲染输出一致
- 验证性能不降低

### Manual Tests
- 交互测试：鼠标点击、键盘输入
- 视觉测试：渲染效果对比
- 性能测试：FPS 监控

## Rollout Plan

### Week 1: 核心重构
- Day 1-2: Phase 1 完成
- Day 3: 内部测试

### Week 2: 应用迁移
- Day 1: ui_demo + snake
- Day 2: tetris + tower
- Day 3: 其他应用

### Week 3: 测试和发布
- Day 1: 完整测试
- Day 2: 文档更新
- Day 3: 发布新版本

## Success Metrics

- ✅ 所有应用成功迁移
- ✅ 测试全部通过
- ✅ 性能无降低（FPS ≥ 原版本）
- ✅ 代码行数减少（去除 Normal Sprite 相关代码）
- ✅ 概念更简单（Widget + Sprite 二元模型）
- ✅ API 更清晰（无重叠功能）

## Risks and Mitigations

### Risk 1: 迁移成本高
**缓解措施：**
- 提供详细迁移指南
- 创建自动化迁移脚本（可选）
- 逐步迁移，一个 app 一个 app

### Risk 2: 性能降低
**缓解措施：**
- 渲染流程保持不变，只是概念重组
- 进行性能测试和监控
- 必要时优化热点代码

### Risk 3: 功能缺失
**缓解措施：**
- 仔细审查所有使用场景
- 保留必要的兼容 API（deprecated）
- 充分的测试覆盖

## Open Questions

1. **是否保留兼容 API？**
   - 可以保留 deprecated 的 `set_color_str()` 一段时间
   - 或直接移除，强制迁移

2. **是否需要 `stage.add_widget_to_tui()` 辅助方法？**
   - 或者让用户直接操作 `stage.tui_sprite.content`

3. **文档级别？**
   - 需要多详细的迁移指南？
   - 是否需要视频教程？

## Conclusion

这次重构将简化 rust_pixel 的渲染架构，建立清晰的 Widget + Sprite 二元模型：
- **Widget** 专注 TUI 渲染（文本、UI 组件）→ 渲染到 tui layer
- **Sprite** 专注图形渲染（像素、图片、动画）→ 添加到 sprite layer
- **Scene** 作为统一容器，包含多个 Layer
- **Layer** 统一的层概念，支持扩展

通过去除 Normal Sprite 概念和更好的命名（Panel→Scene, Sprites→Layer），架构将更加纯粹和易于理解。

### 新旧命名对照

| 旧命名 | 新命名 | 说明 |
|--------|--------|------|
| Panel | Scene | 场景容器 |
| Sprites | Layer | 渲染层 |
| layers[0] "main" | layers[0] "tui" | TUI 内容层 |
| layers[1] "pixel" | layers[1] "sprite" | 图形精灵层 |
| is_pixel: bool | 去除 | 不再区分 |
