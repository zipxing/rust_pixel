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

### Decision 7: 统一 4096 纹理块系统与 tex/block 语义

**选择：** 建立统一的块索引系统 (Block Index System)，明确 `tex` 字段语义为"块索引 (0-255)"

#### 7.1 4096×4096 统一纹理布局

RustPixel 使用单一 4096×4096 纹理存储所有字符和符号，分为 256 个块 (Block 0-255)：

```
4096×4096 统一纹理
├── Sprite 区域 (Block 0-159): 160 块，每块 256×256px
│   ├── Block 0: Sprite texture 0 (基础 ASCII/符号)
│   ├── Block 1: Sprite texture 1
│   └── ...
├── TUI 区域 (Block 160-169): 10 块，每块 256×512px
│   ├── Block 160: 数学符号 (∀∃∈∞...)
│   ├── Block 161: 箭头符号 (←→↑↓...)
│   └── ...
├── Emoji 区域 (Block 170-175): 6 块，每块 256×512px
│   ├── Block 170: 表情符号
│   └── ...
├── CJK 区域 (Block 176-239): 64 块，每块 256×256px
│   ├── 16 列 × 4 行布局
│   ├── 每块 8×8 个汉字 (64 字符)
│   ├── 每个汉字 32×32px
│   └── 总计 4096 个汉字
└── 保留区域 (Block 240-255): 16 块
```

#### 7.2 CJK 块系统重新设计

**背景：**
- 旧设计：32 个线性块 (176-207)，每块 128×1 字符 (4096×32px)，布局极扁平
- 新设计：64 个方形块 (176-239)，每块 8×8 字符 (256×256px)，布局统一

**新 CJK 布局：**
```
CJK 区域: 2048×1024px (4096 个汉字)
┌─────────────────────────────────────────────────────────────┐
│ 16 列 × 4 行 = 64 blocks                                     │
│ 每个 block: 256×256px (8×8 个 32×32px 汉字)                  │
├─────────────────────────────────────────────────────────────┤
│ Block 176 │ Block 177 │ ... │ Block 191 │  ← Row 0         │
│ Block 192 │ Block 193 │ ... │ Block 207 │  ← Row 1         │
│ Block 208 │ Block 209 │ ... │ Block 223 │  ← Row 2         │
│ Block 224 │ Block 225 │ ... │ Block 239 │  ← Row 3         │
└─────────────────────────────────────────────────────────────┘
```

**块索引计算算法：**
```rust
// symbol_map.rs: cjk_idx() 方法
pub fn cjk_idx(&self, symbol: &str) -> Option<(u8, u8)> {
    let ch = symbol.chars().next()?;
    let (pixel_x, pixel_y) = self.cjk.get(&ch).copied()?;

    // 1. 像素坐标转换为字符网格位置
    let char_col = pixel_x / 32;         // 0-127 (全局列)
    let char_row = (pixel_y - 3072) / 32; // 0-31  (全局行)

    // 2. 转换为块位置 (16 列 × 4 行)
    let block_col = char_col / 8;  // 0-15 (块列)
    let block_row = char_row / 8;  // 0-3  (块行)
    let block = (block_row * 16 + block_col) as u8;

    // 3. 计算块内位置 (8×8 网格)
    let in_block_col = char_col % 8;  // 0-7
    let in_block_row = char_row % 8;  // 0-7
    let idx = (in_block_row * 8 + in_block_col) as u8;

    // 返回 (block_index, symbol_index)
    Some((176 + block, idx))
}
```

**常量更新：**
```rust
// symbol_map.rs: layout module
pub const CJK_BLOCK_START: usize = 176;
pub const CJK_BLOCKS: u32 = 64;           // 32 → 64
pub const CJK_BLOCK_COLS: u32 = 16;       // 新增
pub const CJK_BLOCK_ROWS: u32 = 4;        // 新增
pub const CJK_SYMBOLS_PER_BLOCK: u32 = 64; // 128 → 64
pub const CJK_CHARS_PER_BLOCK_ROW: u32 = 8;
pub const CJK_CHARS_PER_BLOCK_COL: u32 = 8;
pub const CJK_BLOCK_END: usize = 239;     // 207 → 239
```

#### 7.3 tex 字段语义统一

**问题：**
- 旧文档描述 `tex` 为"纹理文件索引 (0-3)"，指向 4 个 PNG 文件
- 实际代码中 `get_cell_info()` 返回的是块索引 (0-255)
- CellInfo 的第二个 u8 字段语义混乱 (有时是 0-3，有时是 160+)

**解决方案：**
- 保持字段名 `tex` 不变（向后兼容）
- 统一语义为"块索引 (Block Index, 0-255)"
- 更新所有文档和注释

**Cell 结构更新：**
```rust
// cell.rs
pub struct Cell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,

    /// Texture block index (0-255) in the 4096x4096 unified texture.
    ///
    /// Block ranges:
    /// - 0-159: Sprite region
    /// - 160-169: TUI region
    /// - 170-175: Emoji region
    /// - 176-239: CJK region
    /// - 240-255: Reserved
    pub tex: u8,
}
```

**CellInfo 类型明确：**
```rust
/// Cell rendering information: (symbol_index, block_index, fg_color, bg_color, modifier)
///
/// - symbol_index (u8): Index within the block (0-255)
/// - block_index (u8): Texture block index (0-255):
///   - 0-159: Sprite blocks
///   - 160-169: TUI blocks
///   - 170-175: Emoji blocks
///   - 176-239: CJK blocks
///   - 240-255: Reserved
pub type CellInfo = (u8, u8, Color, Color, Modifier);
```

**块索引解析优先级：**
```rust
// cell.rs: get_cell_info() 方法
pub fn get_cell_info(&self) -> CellInfo {
    // 1. 优先检查 Emoji (Block 170-175)
    if let Some((block, idx)) = get_symbol_map().emoji_idx(&self.symbol) {
        return (idx, block, self.fg, self.bg, self.modifier);
    }

    // 2. 检查 CJK (Block 176-239)
    if let Some((block, idx)) = get_symbol_map().cjk_idx(&self.symbol) {
        return (idx, block, self.fg, self.bg, self.modifier);
    }

    // 3. 兜底：使用 self.tex 作为块索引 (通常是 Sprite 0-159)
    (symidx(&self.symbol), self.tex, self.fg, self.bg, self.modifier)
}
```

**Buffer API 文档更新：**
```rust
// buffer.rs
/// Set string with block index
/// * `tex` - Texture block index (0-255):
///   - For normal text: typically 0 (Sprite block 0)
///   - For special symbols: use appropriate block index
///   - For Emoji/TUI/CJK: block is auto-determined by `get_cell_info()`
pub fn set_stringn_tex(&mut self, x: u16, y: u16, s: &str, w: u16, tex: u8, style: Style)
```

#### 7.4 SymbolIndex 枚举统一

**背景：**
- `SymbolIndex::Cjk` 使用 `(u16, u16)` 与其他变体不一致
- 其他变体都是 `(u8, u8)` 格式

**统一修改：**
```rust
// symbol_map.rs
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolIndex {
    Sprite(u8, u8),  // (block, index)
    Tui(u8, u8),     // (block, index)
    Emoji(u8, u8),   // (block, index)
    Cjk(u8, u8),     // (block, index) - 从 (u16, u16) 改为 (u8, u8)
    NotFound,
}
```

#### 7.5 优势总结

**1. 块大小统一**
- Sprite 块：256×256px
- CJK 块：256×256px (新)
- 简化纹理管理和加载逻辑

**2. 语义清晰**
- `tex` 字段明确表示"块索引"
- `CellInfo` 第二个字段统一为 `block_index`
- 消除"纹理文件索引"的误导性概念

**3. 扩展性强**
- 64 个 CJK 块提供更细粒度的管理
- 16 个保留块 (240-255) 可用于未来扩展
- 统一的块索引系统易于添加新区域

**4. 代码一致性**
- `SymbolIndex` 枚举所有变体都是 `(u8, u8)`
- 所有 `*_idx()` 方法都返回 `Option<(u8, u8)>`
- 块索引计算算法清晰统一

**5. 性能优化**
- 方形块布局提高缓存局部性
- 块内索引计算简单高效 (位运算)
- 减少纹理切换次数

#### 7.6 兼容性说明

**向后兼容：**
- `tex` 字段名保持不变
- 现有代码中 `tex = 0` 或 `tex = 1` 等用法仍然有效
- `get_cell_info()` 自动处理 Emoji/CJK 的块索引解析

**文档迁移：**
- 所有"纹理索引"描述改为"块索引"
- 所有"texture index"改为"block index"
- 更新示例代码中的注释

**测试验证：**
- 12/12 symbol_map 测试通过
- CJK 字符索引计算正确
- Emoji/TUI 块索引解析正常

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

### Decision 8: 统一 GPU 渲染管线架构

**选择：** 建立以 Buffer → RenderBuffer → RenderTexture → Screen 为核心的四阶段渲染管线

**背景分析：**
经过深入分析渲染流程，所有渲染源头本质上都是 Buffer：
- TUI 内容：Widget 渲染到 Buffer
- Sprite 内容：每个 Sprite 包含一个 Buffer (content 字段)

因此，渲染管线可以统一为四个清晰的阶段。

#### 8.1 四阶段渲染管线

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          统一 GPU 渲染管线架构                                │
└─────────────────────────────────────────────────────────────────────────────┘

阶段1: 数据源 → RenderBuffer
┌─────────────┐     ┌─────────────┐
│ TUI Buffer  │────►│             │
└─────────────┘     │ RenderBuffer│ ← Vec<RenderCell>
┌─────────────┐     │  (统一格式)  │
│ Sprite Layer│────►│             │
└─────────────┘     └──────┬──────┘
                           │
阶段2: RenderBuffer → RT   │
                           ▼
              ┌────────────────────────┐
              │  draw_to_rt(rbuf, rt)  │
              └────────────────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
           ┌────┐       ┌────┐       ┌────┐
           │RT0 │       │RT1 │       │RT2 │  ...
           └────┘       └────┘       └────┘

阶段3: RT 运算 (可选)
              ┌─────────────────────────────┐
              │  blend_rts(src1, src2, dst) │
              │  copy_rt(src, dst)          │
              │  clear_rt(rt)               │
              └─────────────────────────────┘
                           │
                           ▼
                        ┌────┐
                        │RT3 │ ← 运算结果
                        └────┘

阶段4: RT[] → Screen
              ┌─────────────────────────────┐
              │  present([                  │
              │    RtComposite(RT2, ...),   │
              │    RtComposite(RT3, ...),   │
              │  ])                         │
              └─────────────────────────────┘
                           │
                           ▼
                      ┌────────┐
                      │ Screen │
                      └────────┘
```

#### 8.2 各阶段 API 设计

**阶段1: 数据转换**
```rust
/// 将 Buffer + Layers 转换为 GPU 渲染格式
fn generate_render_buffer(
    buffer: &Buffer,           // TUI buffer
    previous_buffer: &Buffer,  // 上一帧 (用于 diff 优化)
    layers: &mut Vec<Layer>,   // Sprite 层
    stage: u32,                // 当前阶段 (logo 显示等)
    base: &AdapterBase,        // 适配器基础数据
) -> Vec<RenderCell>;
```

**阶段2: 渲染到 RT**
```rust
/// 将 RenderBuffer 渲染到指定的 RenderTexture
fn draw_render_buffer_to_texture(
    &mut self,
    rbuf: &[RenderCell],  // GPU 渲染数据
    rt: usize,            // 目标 RT 索引 (0-3)
    debug: bool,          // 调试模式
);

/// 将 Buffer 直接渲染到 RT (便捷方法)
fn draw_buffer_to_texture(&mut self, buf: &Buffer, rt: usize);
```

**阶段3: RT 运算**
```rust
/// 使用 GPU shader 混合两个 RT 到目标 RT
fn blend_rts(
    &mut self,
    src1: usize,    // 源 RT 1
    src2: usize,    // 源 RT 2
    target: usize,  // 目标 RT
    effect: usize,  // 效果类型 (0=Mosaic, 1=Heart, ...)
    progress: f32,  // 过渡进度 (0.0-1.0)
);

/// 复制 RT 内容
fn copy_rt(&mut self, src: usize, dst: usize);

/// 清空 RT
fn clear_rt(&mut self, rt: usize);
```

**阶段4: 输出到屏幕**
```rust
/// 将 RT 合成链输出到屏幕
fn present(&mut self, composites: &[RtComposite]);

/// 使用默认设置输出 (RT2 全屏 + RT3 覆盖层)
fn present_default(&mut self);
```

#### 8.3 RtComposite 合成配置

```rust
/// RT 合成配置
#[derive(Clone, Debug)]
pub struct RtComposite {
    pub rt: usize,              // RT 索引 (0-3)
    pub viewport: Option<Rect>, // 输出视口，None = 全屏
    pub blend: BlendMode,       // 混合模式
    pub alpha: u8,              // 透明度 (0-255)
}

/// 混合模式
pub enum BlendMode {
    Normal,   // 正常 alpha 混合
    Add,      // 加法混合
    Multiply, // 乘法混合
    Screen,   // 滤色混合
}

// 使用示例
adapter.present(&[
    RtComposite::fullscreen(2),                    // RT2 全屏
    RtComposite::with_viewport(3, game_area)       // RT3 在游戏区域
        .blend(BlendMode::Normal)
        .alpha(200),
]);
```

#### 8.4 RenderTexture 用途约定

| RT 索引 | 用途 | 说明 |
|---------|------|------|
| RT0 | 源图像1 | 用于过渡效果的源图像 |
| RT1 | 源图像2 | 用于过渡效果的目标图像 |
| RT2 | 主场景 | 游戏主要内容 (TUI + Sprites) |
| RT3 | 叠加层 | 过渡效果结果/特效层 |

#### 8.5 典型渲染流程

**普通游戏帧：**
```rust
// Scene::draw() 内部流程
fn draw(&mut self, ctx: &mut Context) {
    // 阶段1: 合并所有层数据
    let rbuf = generate_render_buffer(
        &self.tui_buffers[self.current],
        &self.tui_buffers[1 - self.current],
        &mut self.layers,
        ctx.stage,
        ctx.adapter.get_base(),
    );

    // 阶段2: 渲染到 RT2
    ctx.adapter.draw_render_buffer_to_texture(&rbuf, 2, false);

    // 阶段4: 输出到屏幕 (跳过阶段3，无特效)
    ctx.adapter.present_default();
}
```

**带过渡效果的帧 (如 petview)：**
```rust
// 阶段2: 渲染两个源图像到 RT0 和 RT1
adapter.draw_buffer_to_texture(&source_image, 0);
adapter.draw_buffer_to_texture(&target_image, 1);

// 阶段3: 使用 GPU shader 混合到 RT3
adapter.blend_rts(0, 1, 3, effect_type, progress);

// 阶段4: 合成输出
adapter.present(&[
    RtComposite::fullscreen(2),  // 主场景
    RtComposite::fullscreen(3),  // 过渡效果
]);
```

#### 8.6 架构优势

1. **统一数据源** - Buffer 是唯一的渲染源，概念简单
2. **灵活的 RT 操作** - RT 之间可以任意运算 (blend, copy, clear)
3. **清晰的输出控制** - present() 接收 RT 数组，按顺序合成
4. **解耦** - 每个阶段职责单一，易于理解和维护
5. **高性能** - 批量渲染，最小化 GPU 状态切换
6. **可扩展** - 易于添加新的 RT 效果和混合模式

#### 8.7 与现有代码的对应关系

| 新 API | 现有实现 | 位置 |
|--------|----------|------|
| `generate_render_buffer()` | `generate_render_buffer()` | graph.rs |
| `draw_render_buffer_to_texture()` | `draw_render_buffer_to_texture()` | adapter.rs |
| `blend_rts()` | `render_advanced_transition()` | adapter.rs |
| `copy_rt()` | `copy_render_texture()` | adapter.rs |
| `present()` | `present()` | adapter.rs |
| `present_default()` | `present_default()` | adapter.rs |

#### 8.8 调用链与 App 自定义渲染

**核心调用链：**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              渲染调用链                                      │
└─────────────────────────────────────────────────────────────────────────────┘

Game Loop (game.rs)
       │
       ▼
   game.tick()
       │
       ├──► model.update()           ← 游戏逻辑更新
       │
       └──► render.draw(ctx, model, dt)  ← App 的 Render::draw()
                    │                       App 完全控制这里！
                    ▼
            ┌───────────────────────────────────────────┐
            │  App 的 draw() 实现                        │
            │                                           │
            │  方式1: 默认流程                           │
            │  self.scene.draw(ctx)?;                   │
            │                                           │
            │  方式2: 自定义流程                         │
            │  ctx.adapter.draw_buffer_to_rt(...);      │
            │  ctx.adapter.blend_rts(...);              │
            │  ctx.adapter.present(...);                │
            └───────────────────────────────────────────┘
```

**关键设计原则：Adapter 不调用 App，App 调用 Adapter！**

App 的 `Render::draw()` 拥有完全的渲染控制权：
- 可以调用 `scene.draw(ctx)` 走默认流程
- 可以直接调用 `ctx.adapter.xxx()` 自定义流程
- 可以混合使用（先调 scene.draw，再额外调 adapter API）

**App 渲染模式示例：**

**模式1：默认流程（大多数 app）**
```rust
impl Render for SnakeRender {
    fn draw(&mut self, ctx: &mut Context, model: &mut SnakeModel, dt: f32) {
        // 一行搞定，Scene 内部处理所有 4 个阶段
        self.scene.draw(ctx).unwrap();
    }
}
```

**模式2：扩展流程（petview 过渡效果）**
```rust
impl Render for PetViewRender {
    fn draw(&mut self, ctx: &mut Context, model: &mut PetViewModel, dt: f32) {
        // 主场景正常渲染
        self.scene.draw(ctx).unwrap();

        // 额外：在 handle_timer 中调用 blend_rts 处理过渡
        // scene.draw 内部的 present_default 会输出 RT2 + RT3
    }
}
```

**模式3：完全自定义流程（分屏渲染）**
```rust
impl Render for SplitViewRender {
    fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
        // 跳过 scene.draw()，完全自己控制
        let left = Rect::new(0, 0, 400, 600);
        let right = Rect::new(400, 0, 400, 600);

        // 阶段2: 两个 buffer 渲染到 RT0 的不同区域
        ctx.adapter.clear_rt(0);
        ctx.adapter.draw_buffer_to_rt(&self.left_buf, 0, Some(left));
        ctx.adapter.draw_buffer_to_rt(&self.right_buf, 0, Some(right));

        // 阶段4: 输出
        ctx.adapter.present(&[RtComposite::fullscreen(0)]);
    }
}
```

**模式4：多 RT 组合（复杂特效）**
```rust
impl Render for EffectRender {
    fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
        // 阶段2: 渲染多个源到不同 RT
        ctx.adapter.draw_sprite_to_rt(&self.background, 0, None);
        ctx.adapter.draw_sprite_to_rt(&self.foreground, 1, None);

        // 阶段3: RT 运算
        ctx.adapter.blend_rts(0, 1, 3, model.effect, model.progress);

        // 主场景渲染到 RT2
        self.scene.draw(ctx).unwrap();

        // 阶段4: 自定义合成
        ctx.adapter.present(&[
            RtComposite::fullscreen(2),              // 主场景
            RtComposite::fullscreen(3).alpha(128),   // 特效叠加，半透明
        ]);
    }
}
```

**设计优势：**
1. **App 不需要实现 Adapter trait** - 只需在 draw() 中组合调用
2. **渐进式复杂度** - 简单 app 一行代码，复杂 app 自由组合
3. **完全的灵活性** - App 可以控制 4 阶段的任何步骤
4. **向后兼容** - 现有 app 无需修改，scene.draw() 仍然有效

#### 8.9 Viewport 辅助方法与坐标系统

**背景：**
在 present() 阶段自定义 RT 的显示位置时，需要处理多种坐标系统的转换，这是一个容易出错且繁琐的过程。我们添加了一系列辅助方法来简化这一流程。

##### 8.9.1 坐标系统说明

RustPixel 涉及四种坐标系统：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              坐标系统层次                                    │
└─────────────────────────────────────────────────────────────────────────────┘

1. Cell 坐标 (逻辑单元)
   ┌───────────────────────────────┐
   │  (0,0)  (1,0)  (2,0) ...     │  ← 字符/符号网格
   │  (0,1)  (1,1)  (2,1) ...     │    例如：40×25 cells
   │   ...                         │
   └───────────────────────────────┘
   用途：游戏逻辑、Buffer 操作

2. Pixel 坐标 (纹理像素)
   ┌───────────────────────────────┐
   │  Cell × sym_size             │  ← 每个 cell 对应 sym_w×sym_h 像素
   │  例如：40×8 = 320px 宽       │    (通常 sym_w=8, sym_h=8)
   │       25×8 = 200px 高        │
   └───────────────────────────────┘
   用途：RT 纹理尺寸、精灵定位

3. Canvas 坐标 (屏幕像素)
   ┌───────────────────────────────┐
   │  Pixel × ratio               │  ← HiDPI 缩放后的实际屏幕像素
   │  例如：320×2.0 = 640px       │    ratio_x, ratio_y 通常 > 1.0
   │       200×2.0 = 400px        │
   └───────────────────────────────┘
   用途：Viewport 定位、屏幕布局

4. NDC 坐标 (归一化设备坐标)
   ┌───────────────────────────────┐
   │  范围：-1.0 到 +1.0          │  ← GPU 着色器使用
   │  原点：屏幕中心              │    OpenGL/WGPU 标准
   │  Y 轴：向上为正              │
   └───────────────────────────────┘
   用途：GPU 顶点变换、present() 内部
```

**坐标转换公式：**
```rust
// Cell → Pixel
pixel_w = cell_w * sym_w / ratio_x
pixel_h = cell_h * sym_h / ratio_y

// Pixel → Canvas (viewport)
// 对于居中定位：
vp_x = (canvas_w - pixel_w) / 2
vp_y = (canvas_h - pixel_h) / 2

// Canvas → NDC (present 内部使用)
// 用于 viewport 位置转换：
tx = (2 * vp_x + vp_w - canvas_w) / canvas_w
ty = (canvas_h - 2 * vp_y - vp_h) / canvas_h
```

##### 8.9.2 RtComposite 辅助方法

**基础创建方法：**
```rust
// graph.rs 中定义

/// 全屏显示 RT
pub fn fullscreen(rt: usize) -> Self

/// 在指定 viewport 显示 RT
pub fn with_viewport(rt: usize, viewport: ARect) -> Self

/// 在指定位置和尺寸显示 RT
pub fn at_position(rt: usize, x: i32, y: i32, w: u32, h: u32) -> Self

/// 居中显示 (需要提供画布尺寸)
pub fn centered(rt: usize, vp_w: u32, vp_h: u32, canvas_w: u32, canvas_h: u32) -> Self

/// 从 cell 尺寸创建居中 viewport (高级方法)
pub fn centered_cells(
    rt: usize,
    cell_w: u16, cell_h: u16,
    sym_w: f32, sym_h: f32,
    rx: f32, ry: f32,
    canvas_w: u32, canvas_h: u32,
) -> Self
```

**链式修改方法：**
```rust
/// 设置 viewport 的 x 位置
pub fn x(mut self, x: i32) -> Self

/// 设置 viewport 的 y 位置
pub fn y(mut self, y: i32) -> Self

/// 相对偏移 viewport 位置
pub fn offset(mut self, dx: i32, dy: i32) -> Self

/// 设置混合模式 (预留)
pub fn blend(mut self, mode: BlendMode) -> Self

/// 设置透明度 (预留)
pub fn alpha(mut self, alpha: u8) -> Self
```

**使用示例：**
```rust
// 创建居中 RT，然后左移到 x=0
let rt3 = RtComposite::centered(3, 320, 200, 640, 480).x(0);

// 创建居中 RT，然后相对偏移
let rt3 = RtComposite::centered(3, 320, 200, 640, 480).offset(-50, 0);
```

##### 8.9.3 Context 辅助方法 (推荐使用)

为了避免用户手动获取所有渲染参数，Context 提供了更高层的辅助方法：

```rust
// context.rs 中定义 (仅 graphics_mode)

/// 从 cell 尺寸计算居中 viewport
/// 自动获取 sym_w/h, ratio, canvas_size
pub fn centered_viewport(&mut self, cell_w: u16, cell_h: u16) -> ARect

/// 创建居中的 RtComposite (最便捷的 API)
pub fn centered_rt(&mut self, rt: usize, cell_w: u16, cell_h: u16) -> RtComposite

/// 获取画布尺寸 (width, height)
pub fn canvas_size(&mut self) -> (u32, u32)

/// 获取 DPI 缩放比例 (ratio_x, ratio_y)
pub fn ratio(&mut self) -> (f32, f32)
```

**最佳实践示例：**
```rust
impl Render for MyRender {
    fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
        // 渲染场景到 RT2
        self.scene.draw_to_rt(ctx).unwrap();

        // 方式1: 最简单 - 使用 ctx.centered_rt()
        let rt3 = ctx.centered_rt(3, CELL_W, CELL_H);
        ctx.adapter.present(&[
            RtComposite::fullscreen(2),
            rt3,
        ]);

        // 方式2: 自定义位置 - 先获取居中，再调整
        let rt3 = ctx.centered_rt(3, CELL_W, CELL_H).x(0);  // 左对齐
        ctx.adapter.present(&[
            RtComposite::fullscreen(2),
            rt3,
        ]);

        // 方式3: 完全自定义
        let vp = ctx.centered_viewport(CELL_W, CELL_H);
        let rt3 = RtComposite::with_viewport(3, ARect {
            x: 0,
            y: vp.y,
            w: vp.w,
            h: vp.h,
        });
        ctx.adapter.present(&[
            RtComposite::fullscreen(2),
            rt3,
        ]);
    }
}
```

##### 8.9.4 Viewport 位置生效原理

**问题背景：**
早期实现中，`present()` 仅使用 viewport 的 `w` 和 `h` 进行缩放，忽略了 `x` 和 `y` 位置。

**解决方案：**
在 OpenGL 和 WGPU 的 present 实现中，添加 NDC 坐标平移：

```rust
// gl/pixel.rs 和 winit_wgpu_adapter.rs

// 从 viewport 位置计算 NDC 平移量
let vp_x = viewport.x as f32;
let vp_y = viewport.y as f32;
let vp_w = viewport.w as f32;
let vp_h = viewport.h as f32;
let canvas_w = pixel_canvas_width;
let canvas_h = pixel_canvas_height;

// NDC 平移公式 (将 viewport 中心映射到正确位置)
let tx = (2.0 * vp_x + vp_w - canvas_w) / canvas_w;
let ty = (canvas_h - 2.0 * vp_y - vp_h) / canvas_h;

// 应用变换：先缩放，再平移
let mut transform = UnifiedTransform::new();
transform.scale(vp_w / canvas_w, vp_h / canvas_h);
transform.translate(tx, ty);
```

**公式推导：**
```
NDC 坐标范围: -1.0 到 +1.0, 原点在中心

全屏时 (vp_x=0, vp_w=canvas_w):
  tx = (0 + canvas_w - canvas_w) / canvas_w = 0  ✓

居中时 (vp_x = (canvas_w - vp_w) / 2):
  tx = (2 * (canvas_w - vp_w) / 2 + vp_w - canvas_w) / canvas_w
     = (canvas_w - vp_w + vp_w - canvas_w) / canvas_w
     = 0  ✓

左对齐 (vp_x = 0, vp_w < canvas_w):
  tx = (0 + vp_w - canvas_w) / canvas_w
     = (vp_w - canvas_w) / canvas_w < 0  (向左偏移) ✓
```

##### 8.9.5 注意事项

**借用检查器限制：**
```rust
// ❌ 错误：不能在 present() 参数中调用 ctx.centered_rt()
ctx.adapter.present(&[
    RtComposite::fullscreen(2),
    ctx.centered_rt(3, CELL_W, CELL_H),  // 编译错误！
]);

// ✅ 正确：先计算，再传入
let rt3 = ctx.centered_rt(3, CELL_W, CELL_H);
ctx.adapter.present(&[
    RtComposite::fullscreen(2),
    rt3,
]);
```

**ARect vs Rect：**
- `Rect`: 有裁剪保护，当 `w * h > u16::MAX` 时会调整尺寸
- `ARect`: 无裁剪，直接存储值，用于 viewport (可能超过 u16 范围)

**graphics_mode 条件编译：**
```rust
// viewport 辅助方法仅在图形模式可用
#[cfg(graphics_mode)]
pub fn centered_viewport(&mut self, ...) -> ARect { ... }

// 文本模式不需要这些方法
```

### Section 9: 完整渲染 API 参考

#### 9.1 渲染流程总览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           完整渲染流程                                       │
└─────────────────────────────────────────────────────────────────────────────┘

App.Render::draw(ctx, model, dt)
       │
       ├─── 方式A: 使用 Scene (推荐)
       │    │
       │    └──► scene.draw(ctx)
       │              │
       │              ├── 生成 RenderBuffer
       │              ├── draw_to_rt(RT2)
       │              └── present_default()
       │
       └─── 方式B: 完全自定义
            │
            ├──► 1. 数据准备
            │    ctx.adapter.buf2rt(&buffer, rt)
            │    ctx.adapter.draw_sprite_to_rt(&sprite, rt)
            │
            ├──► 2. RT 运算 (可选)
            │    ctx.adapter.blend_rts(src1, src2, dst, effect, progress)
            │    ctx.adapter.copy_rt(src, dst)
            │    ctx.adapter.clear_rt(rt)
            │
            └──► 3. 输出到屏幕
                 let rt3 = ctx.centered_rt(3, CELL_W, CELL_H);
                 ctx.adapter.present(&[
                     RtComposite::fullscreen(2),
                     rt3,
                 ]);
```

#### 9.2 核心 API 速查表

| 方法 | 所属 | 用途 |
|------|------|------|
| `scene.draw(ctx)` | Scene | 完整渲染流程 (TUI + Sprites → RT2 → Screen) |
| `scene.draw_to_rt(ctx)` | Scene | 仅渲染到 RT2，不 present |
| `ctx.adapter.buf2rt(&buf, rt)` | Adapter | Buffer 渲染到指定 RT |
| `ctx.adapter.copy_rt(src, dst)` | Adapter | 复制 RT 内容 |
| `ctx.adapter.blend_rts(...)` | Adapter | GPU 混合两个 RT |
| `ctx.adapter.clear_rt(rt)` | Adapter | 清空 RT |
| `ctx.adapter.set_rt_visible(rt, visible)` | Adapter | 设置 RT 可见性 |
| `ctx.adapter.present(&[...])` | Adapter | 合成 RT 并输出到屏幕 |
| `ctx.adapter.present_default()` | Adapter | 默认输出 (RT2 + RT3) |
| `ctx.centered_rt(rt, w, h)` | Context | 创建居中 RtComposite |
| `ctx.centered_viewport(w, h)` | Context | 计算居中 viewport |
| `RtComposite::fullscreen(rt)` | RtComposite | 创建全屏 composite |
| `RtComposite::with_viewport(rt, vp)` | RtComposite | 创建带 viewport 的 composite |
| `composite.x(val)` | RtComposite | 设置 viewport x 位置 |
| `composite.y(val)` | RtComposite | 设置 viewport y 位置 |
| `composite.offset(dx, dy)` | RtComposite | 相对偏移 viewport |

#### 9.3 典型使用场景

**场景1: 普通游戏 (大多数情况)**
```rust
fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    self.scene.draw(ctx).unwrap();
}
```

**场景2: 带图片过渡效果 (如 petview)**
```rust
fn handle_timer(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    // 准备源图像
    ctx.adapter.buf2rt(&source_image, 0);
    ctx.adapter.buf2rt(&target_image, 1);

    // GPU 混合
    ctx.adapter.blend_rts(0, 1, 3, model.effect, model.progress);
}

fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    self.scene.draw_to_rt(ctx).unwrap();

    let rt3 = ctx.centered_rt(3, CELL_W, CELL_H);
    ctx.adapter.present(&[
        RtComposite::fullscreen(2),
        rt3,
    ]);
}
```

**场景3: 分屏渲染**
```rust
fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    // 渲染左半屏
    ctx.adapter.buf2rt(&self.left_buf, 0);

    // 渲染右半屏
    ctx.adapter.buf2rt(&self.right_buf, 1);

    let (cw, ch) = ctx.canvas_size();
    let half_w = cw / 2;

    ctx.adapter.present(&[
        RtComposite::at_position(0, 0, 0, half_w, ch),
        RtComposite::at_position(1, half_w as i32, 0, half_w, ch),
    ]);
}
```

**场景4: Picture-in-Picture (画中画)**
```rust
fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    // 主场景
    self.scene.draw_to_rt(ctx).unwrap();

    // 小窗口渲染到 RT3
    ctx.adapter.buf2rt(&self.pip_buffer, 3);

    let (cw, ch) = ctx.canvas_size();
    let pip_w = cw / 4;
    let pip_h = ch / 4;

    ctx.adapter.present(&[
        RtComposite::fullscreen(2),
        RtComposite::at_position(3, (cw - pip_w - 20) as i32, 20, pip_w, pip_h),
    ]);
}
```

#### 9.4 调试技巧

**查看渲染参数：**
```rust
fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    let (cw, ch) = ctx.canvas_size();
    let (rx, ry) = ctx.ratio();
    log::info!("Canvas: {}x{}, Ratio: {}x{}", cw, ch, rx, ry);

    let vp = ctx.centered_viewport(40, 25);
    log::info!("Viewport: ({},{}) {}x{}", vp.x, vp.y, vp.w, vp.h);
}
```

**测试不同 viewport 位置：**
```rust
// 居中
let rt3 = ctx.centered_rt(3, 40, 25);

// 左上角
let rt3 = ctx.centered_rt(3, 40, 25).x(0).y(0);

// 右下角
let vp = ctx.centered_viewport(40, 25);
let (cw, ch) = ctx.canvas_size();
let rt3 = RtComposite::with_viewport(3, ARect {
    x: (cw - vp.w) as i32,
    y: (ch - vp.h) as i32,
    w: vp.w,
    h: vp.h,
});
```
