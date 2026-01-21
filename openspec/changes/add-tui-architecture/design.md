## Context

rust_pixel 目前支持文本模式（终端）和图形模式（SDL/OpenGL/WGPU/WebGL），但在图形模式下缺乏对 TUI（Terminal User Interface）风格界面的良好支持。终端字符通常是瘦高的（16x32 像素），而图形模式使用的符号是正方形的（16x16 像素）。这导致在图形模式下无法真实模拟终端 UI 的视觉效果。

**约束条件：**
- 必须保持文本模式完全向后兼容
- 必须保持单次 draw call 的高性能渲染
- 必须支持 TUI 和游戏精灵的混合渲染
- 使用统一的 2048x2048 符号纹理，包含 Sprite、TUI 和 Emoji 三个区域

**相关方：**
- 游戏开发者：需要在图形模式下使用 TUI 界面
- UI 框架用户：需要正确的字符宽高比和鼠标交互
- 性能敏感应用：需要保持高效的渲染性能

## Goals / Non-Goals

**Goals:**
- 在图形模式下支持瘦高字符（16x32）的 TUI 渲染
- 提供清晰的 TUI 层和游戏精灵层分离
- 实现双坐标系统，正确处理 TUI 和游戏区域的鼠标事件
- TUI 层永远渲染在最上层，确保界面可见性
- 保持单次 draw call 的渲染性能
- 使用统一纹理简化纹理管理

**Non-Goals:**
- 不改变文本模式的任何行为
- 不引入复杂的窗口管理或布局系统
- 不支持可变宽度字符（如 CJK 全角字符的特殊处理）
- 不实现专业 GUI 框架的高级特性（如矢量绘制、富文本排版）

## Decisions

### Decision 1: 统一符号纹理与区域划分

**选择：** 使用统一的 2048x2048 `symbols.png` 纹理，包含三个区域：Sprite 符号（16x16）、TUI 符号（16x32）、Emoji（32x32 彩色）

**布局规划（向后兼容设计）：**
```
2048x2048 纹理布局（Grid-Based，128×128 grid units）：

纹理物理布局：
┌─────────────────────────────────────────────────────────────┐
│                     2048 x 2048 像素                         │
│                     128 x 128 grid units                     │
│                     每个 grid unit = 16 x 16 像素            │
├─────────────────────────────────────────────────────────────┤
│ Sprite 区域（grid 行 0-95）                     │ 1536px 高  │
│ - 6 rows × 8 blocks/row = 48 blocks                         │
│ - 每 block: 16×16 chars, 16×16px each                       │
│ - Block 0-47: 12,288 sprites                                │
│ - 线性索引：0-12287                                          │
├─────────────────────────────────────────────────────────────┤
│ TUI + Emoji 区域（grid 行 96-127）              │ 512px 高   │
│                                                             │
│ TUI 区域 (Cols 0-79, 5 blocks):                             │
│ - Block 48-52: 1280 TUI chars (16x32px each)                │
│ - 每 block: 16×16 chars = 256 chars                         │
│ - TUI 字符占用 1×2 grid units (16x32 像素)                  │
│                                                             │
│ Emoji 区域 (Cols 80-127, 3 blocks):                         │
│ - Block 53-55: 384 Emoji (32x32px each)                     │
│ - 每 block: 8×16 chars = 128 emojis                         │
│ - Emoji 占用 2×2 grid units (32x32 像素)                    │
└─────────────────────────────────────────────────────────────┘

Block 规格：
- Sprite blocks (0-47):  16×16 chars/block, 16×16px each, 256 chars/block
- TUI blocks (48-52):    16×16 chars/block, 16×32px each, 256 chars/block
- Emoji blocks (53-55):  8×16 chars/block, 32×32px each, 128 chars/block

符号索引分配总结：
- Block 0-47:   Sprite 游戏精灵（16x16，单色）  - 12,288 个
- Block 48-52:  TUI 文本字符（16x32，单色）     - 1,280 个
- Block 53-55:  预制 Emoji（32x32，彩色）       - 384 个
```

**理由：**
- **向后兼容**：Sprite 区域保持原有布局不变（Block 0-47），现有游戏无需修改
- 单个纹理简化纹理管理，无需多个纹理绑定
- Grid-based 管理（128×128 grid）便于统一坐标计算
- 三个区域明确分离，避免符号索引冲突
- 2048x2048 纹理大小适中，所有现代 GPU 都支持
- Sprite 区域容量充足（12,288 个），满足大型游戏需求
- TUI + Emoji 放在底部，不影响现有 Sprite 布局
- 保持高效的 GPU 纹理采样性能

**替代方案：**
- 独立 TUI 纹理文件 → 需要管理两个纹理，增加加载和绑定开销
- 运行时缩放 16x16 符号 → 视觉效果差，失真明显
- 更大的统一纹理 → 超出常见 GPU 纹理限制

### Decision 2: 统一坐标系统（水平共享，垂直转换）

**选择：** `MouseEvent` 只提供 `(column, row)`，按 16 像素宽度计算

**坐标计算：**
```rust
// 基础坐标（所有 adapter）
column = pixel_x / 16   // 16 像素宽度（TUI 和 Sprite 共享）
row = pixel_y / 16      // 16 像素高度（Sprite 坐标系）

// TUI 层使用
column_tui = column     // 水平方向相同（都是 16 像素宽）
row_tui = row / 2       // 垂直方向转换（TUI 是 32 像素高）

// Sprite 层使用
sprite_col = column     // 直接使用
sprite_row = row        // 直接使用
```

**理由：**
- 水平统一：TUI 和 Sprite 都是 16 像素宽，`column` 通用无需转换
- 垂直简单：TUI 高度是 Sprite 的 2 倍，简单除以 2 即可
- 向后兼容：Sprite 层代码完全不需要修改，直接使用 `column/row`
- TUI 转换直观：只需要 `row / 2`，符合 32/16 = 2 的比例关系
- API 简洁：只有一套坐标，减少复杂度

**替代方案：**
- 双坐标系统（`column/row` + `column_tui/row_tui`）→ API 复杂，增加认知负担
- TUI 坐标为主 → Sprite 层需要修改，向后兼容性差

### Decision 3: TUI 层渲染顺序

**选择：** Main Buffer（TUI 层）在 `generate_render_buffer` 中最后添加到 `RenderCell` 数组

**实现代码（graph.rs:1433）：**
```rust
// 先渲染 Pixel Sprites
render_pixel_sprites(pixel_spt, rx, ry, |...| { ... });

// 最后渲染 Main Buffer (TUI)
render_main_buffer(cb, width, rx, ry, true, &mut rfunc);
```

**理由：**
- GPU 按顺序渲染，后添加的在上层
- 确保 TUI 界面（如菜单、对话框）始终可见
- 无需修改 shader 或引入深度测试

**替代方案：**
- 使用 Z-index 或深度缓冲 → 增加渲染复杂度，违背简单原则
- 应用层控制渲染顺序 → 容易出错，不够健壮

### Decision 4: 符号尺寸配置

**选择：** 使用统一的全局配置，TUI 高度为 Sprite 的 2 倍：
```rust
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();   // 16 pixels (both Sprite and TUI)
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();  // 16 pixels (Sprite)

// TUI dimensions derived from Sprite:
// TUI_WIDTH = PIXEL_SYM_WIDTH         // 16 pixels (same as Sprite)
// TUI_HEIGHT = PIXEL_SYM_HEIGHT * 2   // 32 pixels (double Sprite height)

// Emoji dimensions:
// EMOJI_WIDTH = PIXEL_SYM_WIDTH * 2   // 32 pixels
// EMOJI_HEIGHT = PIXEL_SYM_HEIGHT * 2 // 32 pixels
```

**实际实现（graph.rs:115-121）：**
```rust
/// Symbol width (in pixels) resolved from the symbol atlas (16 pixels)
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();

/// Symbol height (in pixels) resolved from the symbol atlas (16 pixels for Sprite)
/// Note:
/// - Sprite layer: uses this value directly (16 pixels)
/// - TUI layer: uses double this value (32 pixels = PIXEL_SYM_HEIGHT * 2)
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();
```

**理由：**
- 简化配置：TUI 宽度与 Sprite 相同，高度固定为 2 倍关系
- 减少全局变量：无需额外的 `PIXEL_TUI_WIDTH/HEIGHT`
- 关系明确：TUI 高度 = Sprite 高度 × 2，直观易懂
- 代码简洁：在需要时直接计算 `PIXEL_SYM_HEIGHT * 2.0`

**替代方案：**
- 独立的 `PIXEL_TUI_WIDTH/HEIGHT` → 增加全局变量，关系不够明确
- 单一尺寸 + 缩放因子 → 不够直观，容易混淆
- 运行时查表 → 性能开销，不必要的复杂度

### Decision 5: 渲染管线集成

**选择：** 修改 `render_main_buffer` 使用 TUI 符号尺寸，但仍合并到统一的 `RenderCell` 数组

**实际实现（graph.rs:1073-1157）：**
```rust
pub fn render_main_buffer<F>(
    buf: &Buffer,
    width: u16,
    rx: f32,
    ry: f32,
    use_tui: bool,  // true: TUI (16×32), false: Sprite (16×16)
    mut f: F,
) where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, usize, usize, u16),
{
    // ... 遍历 buffer 中的每个 cell
    // 根据 use_tui 标志选择正确的符号区域和尺寸
}
```

**理由：**
- 保持单次 draw call 的高性能
- 复用现有的实例化渲染管线
- 最小化 shader 修改（已支持可变尺寸的 `RenderCell.w/h`）

**替代方案：**
- 分离 TUI 和 Sprite 的 draw call → 性能下降，违背设计目标
- 使用多个 render pass → 过度设计，不符合简单原则

### Decision 6: Sprite 符号使用 Unicode 私有使用区

**选择：** 使用 U+E000~U+E0FF (Private Use Area) 作为 Sprite 符号的 Unicode 映射范围

**背景：**
之前版本使用数学符号区域 (U+2200~U+22FF) 作为 Sprite 符号的索引。这导致了一个问题：当用户在 TUI 模式下想显示真实的数学符号（如 ∀∃∈∞≈≤≥⊕⊗）时，这些字符会被错误地映射到 Sprite 纹理索引。

**实际实现（cell.rs:229-233）：**
```rust
/// returns a cellsym string by index
/// 256 unicode chars mark the index of a symbol in a SDL texture
/// unicode: 0xE000 ~ 0xE0FF (Private Use Area)
/// maps to a 3 byte UTF8: 11101110 100000xx 10xxxxxx
pub fn cellsym(idx: u8) -> String {
    // U+E000 + idx
    let codepoint = 0xE000u32 + idx as u32;
    char::from_u32(codepoint).unwrap().to_string()
}
```

**实际实现（cell.rs:254-268）：**
```rust
fn symidx(symbol: &String) -> u8 {
    let sbts = symbol.as_bytes();
    // Private Use Area: U+E000~U+E0FF
    // UTF-8: 11101110 100000xx 10xxxxxx (0xEE 0x80~0x83 0x80~0xBF)
    if sbts.len() == 3 && sbts[0] == 0xEE && (sbts[1] >> 2 == 0x20) {
        let idx = ((sbts[1] & 3) << 6) + (sbts[2] & 0x3f);
        return idx;
    }
    // fallback to CELL_SYM_MAP for common ASCII chars
    if let Some(idx) = CELL_SYM_MAP.get(symbol) {
        ret = *idx;
    }
    ret
}
```

**理由：**
1. **永不冲突** - Unicode Private Use Area (PUA) 专门为应用程序自定义使用保留，Unicode 标准永远不会在此分配字符
2. **编码简单** - 仍然是 3 字节 UTF-8 编码，与之前方案一致，无性能损失
3. **容量充足** - BMP PUA 有 6400 个码点 (U+E000~U+F8FF)，当前只用 256 个 (U+E000~U+E0FF)，未来可扩展
4. **TUI 兼容性** - TUI 模式可以自由显示数学符号、箭头符号等标准 Unicode 字符，不会与 Sprite 索引冲突

**使用场景对比：**

| 场景 | 旧方案 (U+2200~U+22FF) | 新方案 (U+E000~U+E0FF) |
|------|----------------------|----------------------|
| Sprite 渲染 | ✅ 可以工作 | ✅ 可以工作 |
| TUI 显示数学公式 | ❌ 冲突，无法显示 | ✅ 正常显示 |
| TUI 显示箭头符号 | ✅ 可以显示 | ✅ 可以显示 |
| 符号集扩展 | ⚠️ 仅 256 个数学符号 | ✅ PUA 有 6400 个码点 |

**替代方案：**
- U+F0000~U+FFFFD (PUA-A): 4 字节 UTF-8，编码复杂，性能稍差
- U+100000~U+10FFFD (PUA-B): 4 字节 UTF-8，编码复杂，性能稍差

### Decision 7: 区域感知的符号索引计算（方案 C）

**选择：** 在渲染层分离处理 TUI 和 Sprite 区域的符号索引计算，不修改 Cell 数据结构

**核心思想：**
- Main Buffer 使用 TUI 区域时，通过 `tui_symidx()` 映射到 Block 48-52
- Pixel Sprites 始终使用 Sprite 区域（Block 0-47）
- 索引计算在 `render_main_buffer` 和 `push_render_buffer` 中实现

**数据流程：**
```
Cell.symbol → symidx() / tui_symidx() → (block, index)
Cell.tex    → 区块索引 (TUI: 48-52, Sprite: 0-47, Emoji: 53-55)
                ↓
     render_main_buffer (use_tui=true/false)
                ↓
   TUI区域计算 / Sprite区域计算 / Emoji区域计算
                ↓
        纹理 grid 坐标
                ↓
           RenderCell.texsym
```

**实际实现 - 索引计算公式（graph.rs:771-815）：**

Sprite 区域（Block 0-47，grid 行 0-95）：
```rust
// Block-based layout: 6 rows × 8 blocks/row = 48 blocks
// 每个 block: 16×16 chars = 256 chars
if texidx <= 47 {
    let x = symidx % 16 + (texidx % 8) * 16;  // grid column (0-127)
    let y = symidx / 16 + (texidx / 8) * 16;  // grid row (0-95)
    texsym = y * 128 + x;  // 线性 grid 索引
}
```

TUI 区域（Block 48-52，grid 行 96-127，cols 0-79）：
```rust
// 5 blocks horizontally
// 每个 TUI 字符占用 1×2 grid units (16x32 像素)
if texidx >= 48 && texidx <= 52 {
    let col_offset = (texidx - 48) * 16;
    let row_offset = 96;
    let r = symidx / 16;
    let c = symidx % 16;
    let grid_col = col_offset + c;
    let grid_row = row_offset + r * 2;  // ×2 因为 TUI 是双高度
    texsym = grid_row * 128 + grid_col;
}
```

Emoji 区域（Block 53-55，grid 行 96-127，cols 80-127）：
```rust
// 3 blocks horizontally
// 每个 Emoji 占用 2×2 grid units (32x32 像素)
if texidx >= 53 && texidx <= 55 {
    let col_offset = 80 + (texidx - 53) * 16;
    let row_offset = 96;
    let r = symidx / 8;   // 8 emojis per row in block
    let c = symidx % 8;
    let grid_col = col_offset + c * 2;  // ×2 因为 Emoji 是双宽度
    let grid_row = row_offset + r * 2;  // ×2 因为 Emoji 是双高度
    texsym = grid_row * 128 + grid_col;
}
```

**理由：**
1. **最小侵入性**：无需修改 Cell 结构，保持向后兼容
2. **职责清晰**：区域逻辑集中在渲染层，应用层无感知
3. **性能优化**：符号索引计算 O(1) 复杂度，仅在渲染时执行
4. **易于维护**：区域差异处理逻辑集中在 push_render_buffer 函数中
5. **自然分层**：Main Buffer 和 Pixel Sprites 本就是不同渲染层

**替代方案及弊端：**

**方案 A：扩展 Cell.tex 语义（使用高位标记区域）**
- ❌ 破坏 tex 字段原有语义
- ❌ 限制区块数量（最多 128 个）
- ❌ 需修改所有设置 tex 的代码

**方案 B：添加独立区域标识字段**
- ❌ 增加 Cell 内存占用
- ❌ 序列化/反序列化需要更新
- ❌ 所有创建 Cell 的代码需要设置 region

**方案 C 的优势（当前选择）：**
- ✅ 零内存开销：不修改 Cell 结构
- ✅ 零迁移成本：现有代码完全兼容
- ✅ 自动区域识别：渲染层自动计算正确索引
- ✅ 性能无损：索引计算仅在渲染时执行

### Decision 8: 预制 Emoji 支持

**选择：** 使用预制 Emoji 图集 + HashMap 映射，而不是动态字体渲染

**核心思想：**
- 预先渲染 175 个最常用 Emoji 到纹理图集
- 使用 `EMOJI_MAP: HashMap<String, (u8, u8)>` 将 Emoji 字符映射到 (block, index)
- 未映射的 Emoji 显示为空白或占位符
- 复用现有的预制纹理渲染管线

**实际实现（cell.rs:70-165）：**

```rust
/// Emoji mapping table for pre-rendered Emoji in the unified texture
static ref EMOJI_MAP: HashMap<String, (u8, u8)> = {
    let mut map = HashMap::new();
    // Start from Block 53, Index 0
    let mut block = 53u8;
    let mut idx = 0u8;

    // Emotions & Faces (50)
    let emotions = ["😀", "😊", "😂", "🤣", "😍", ...];
    for emoji in &emotions {
        map.insert(emoji.to_string(), (block, idx));
        idx += 1;
        if idx == 128 { // Each Emoji block holds 128 emojis
            idx = 0;
            block += 1;
        }
    }
    // ... 其他分类
    map
};

pub fn is_prerendered_emoji(symbol: &str) -> bool {
    EMOJI_MAP.contains_key(symbol)
}

pub fn emoji_texidx(symbol: &str) -> Option<(u8, u8)> {
    EMOJI_MAP.get(symbol).copied()
}
```

**wcwidth=2 处理（graph.rs:1134-1139）：**
```rust
// Handle Emoji rendering in TUI mode
// Emoji (Block >= 53) are 32x32 source pixels,
// while TUI chars are 16x32 source pixels.
if use_tui && texidx >= 53 {
    s2.w *= 2;  // Emoji 占用 2 格宽度
    // Skip next cell rendering to avoid overlap
    if (i + 1) % width as usize != 0 {
        skip_next = true;
    }
}
```

**理由：**
1. **实现简单** - 复用现有预制纹理机制，无需引入字体渲染库
2. **性能最优** - 预制纹理性能最好，无运行时光栅化开销
3. **风格统一** - 可以使用统一风格的 Emoji 集（如 Twemoji, Noto Emoji）
4. **足够实用** - 175 个精选常用 Emoji，覆盖 90%+ 的使用场景
5. **易于扩展** - Block 53-55 提供 384 个 Emoji 位置

**Emoji 选择标准（384 个总容量）：**
- **表情与情感**（50 个）：😀😊😂🤣😍🥰😘😎🤔😭🥺😤😡🤯😱 等
- **符号与标志**（30 个）：✅❌⚠️🔥⭐🌟✨💫🎯🚀⚡💡🔔📌🔗🔒 等
- **箭头与指示**（20 个）：➡️⬅️⬆️⬇️↗️↘️↙️↖️🔄🔃 等
- **食物与饮料**（20 个）：🍕🍔🍟🍿🍩🍪🍰🎂🍭🍫☕🍺🍷 等
- **自然与动物**（20 个）：🌈🌸🌺🌻🌲🌳🍀🐱🐶🐭🐹🦊🐻 等
- **对象与工具**（20 个）：📁📂📄📊📈📉🔧🔨⚙️🖥️💻⌨️🖱️ 等
- **活动与运动**（15 个）：⚽🏀🏈⚾🎮🎲🎯🎨🎭🎪 等
- **预留空间**（209 个）：供用户自定义或未来扩展

**替代方案及弊端：**

**方案 A：动态字体渲染（终端模拟器方法）**
- ✅ 支持无限 Emoji
- ✅ 自动使用系统 Emoji 字体
- ❌ 需要集成 FreeType/fontdue/rusttype
- ❌ 需要实现字形缓存系统
- ❌ 实现复杂度高
- ❌ 运行时光栅化有性能开销

**方案 B：完全不支持 Emoji**
- ✅ 实现最简单
- ❌ 用户体验差
- ❌ 无法在 TUI 中使用 Emoji（文本模式可以）

**方案 C：预制 Emoji（当前选择）**
- ✅ 简单、高效、实用
- ✅ 覆盖 95%+ 使用场景
- ⚠️ 仅支持预定义的 Emoji 集
- ⚠️ 需要手工维护 EMOJI_MAP

### Decision 9: TUI 符号映射表

**选择：** 使用独立的 `TUI_CELL_SYM_MAP` 映射 TUI 字符到纹理索引

**实际实现（cell.rs:167-209）：**
```rust
/// TUI Symbol Map for TUI mode (Block 48+)
/// Matches the layout of symbols/tui.txt
static ref TUI_CELL_SYM_MAP: HashMap<String, (u8, u8)> = {
    let syms = concat!(
        " !#$%&()*+,-./01",
        "23456789:;\"'<=>?",
        "@[\\]^_`{|}~⌐¬½¼¡",
        "«»∙·※⦿ABCDEFGHIJ",
        "KLMNOPQRSTUVWXYZ",
        "abcdefghijklmnop",
        "qrstuvwxyz▀▄äà",
        // ... 更多字符
        "┌╭╔┏┘╯╝┛└╰╚┗┤╣",
        "┫├╠┣┬╦┳┴╩┻┼╬╋≋"
    );

    let mut sm = HashMap::new();
    for (i, c) in syms.chars().enumerate() {
        let block = 48 + (i / 256) as u8;
        let idx = (i % 256) as u8;
        sm.insert(c.to_string(), (block, idx));
    }
    sm
};

pub fn tui_symidx(symbol: &str) -> Option<(u8, u8)> {
    TUI_CELL_SYM_MAP.get(symbol).copied()
}
```

**理由：**
- TUI 字符集与 Sprite 字符集不同，需要独立映射
- 支持完整的 ASCII、拉丁扩展、方块绘制字符
- 与 symbols/tui.txt 纹理布局一一对应

## Risks / Trade-offs

### Risk 1: 符号纹理资源大小

**风险：** 统一的 2048x2048 纹理约 4MB（RGBA）

**缓解措施：**
- 使用 PNG 压缩，实际文件约 200-500KB
- 纹理在 GPU 内存中只加载一次
- 所有现代 GPU 都支持 2048x2048 纹理

### Risk 2: 鼠标坐标计算复杂度

**风险：** 双坐标计算可能引入性能开销或精度问题

**缓解措施：**
- 坐标转换是简单的除法运算，开销可忽略
- 在输入事件层一次性计算，后续无额外开销
- 添加单元测试验证坐标精度

### Risk 3: 向后兼容性

**风险：** 现有应用可能受到结构变化影响

**缓解措施：**
- 保留原有 `column/row` 字段，现有代码无需修改
- Sprite 区域布局完全不变（Block 0-47）
- TUI 模式通过 `use_tui` 参数显式启用

## Migration Plan

### Phase 1: 基础设施 ✅ 已完成
1. ✅ 添加 `PIXEL_SYM_WIDTH/HEIGHT` 全局配置
2. ✅ 实现 `tui_symidx()` 和 `TUI_CELL_SYM_MAP`
3. ✅ 实现 `is_prerendered_emoji()` 和 `EMOJI_MAP`

### Phase 2: TUI 渲染支持 ✅ 已完成
1. ✅ 创建统一的 2048x2048 `symbols.png` 资源
2. ✅ 修改 `render_main_buffer` 支持 `use_tui` 参数
3. ✅ 实现 `push_render_buffer` 中的区域索引计算
4. ✅ 调整渲染顺序确保 TUI 在上层

### Phase 3: 应用集成 ✅ 已完成
1. ✅ 更新 UI 组件使用 TUI 坐标
2. ✅ 在 `ui_demo` 中验证
3. ✅ 提供 `Graph::set_use_tui_height()` API

### Rollback Plan
- Phase 1 可随时回滚（仅添加代码，未修改行为）
- Phase 2 需要移除 TUI 符号加载逻辑
- Phase 3 需要恢复 UI 组件的坐标使用

## Open Questions - 已解决

1. **TUI 符号纹理内容：** ✅ 已实现独立的 TUI 字符集，包含完整 ASCII、拉丁扩展和方块绘制字符

2. **混合渲染性能：** ✅ 单次 draw call 保持高效，`RenderCell` 数组合并所有渲染单元

3. **多分辨率支持：** ✅ 复用现有的 `ratio_x/ratio_y` 缩放机制

4. **TUI 模式配置：** ✅ TUI 架构始终启用，通过 `use_tui` 参数控制字符高度
