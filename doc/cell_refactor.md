# Cell 重构计划

## 目标

消除 Cell 中 tex 字段的混乱，通过 symbol 字符串唯一确定字符类型。

---

## 核心设计

### 1. Cell 结构（已移除 tex）

```rust
pub struct Cell {
    pub symbol: String,    // 完全决定渲染信息
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    pub scale_x: f32,
    pub scale_y: f32,
}
```

### 2. Glyph 结构（新增）

```rust
pub struct Glyph {
    pub block: u8,   // 纹理 block 索引
    pub idx: u8,     // 符号索引
    pub width: u8,   // 宽度倍数 (1 或 2)
    pub height: u8,  // 高度倍数 (1 或 2)
}

impl Glyph {
    pub fn sprite(block: u8, idx: u8) -> Self;  // 1x1 (16x16)
    pub fn tui(block: u8, idx: u8) -> Self;     // 1x2 (16x32)
    pub fn emoji(block: u8, idx: u8) -> Self;   // 2x2 (32x32)
    pub fn cjk(block: u8, idx: u8) -> Self;     // 2x2 (32x32)
    pub fn is_double_height(&self) -> bool;
    pub fn is_double_width(&self) -> bool;
}
```

### 3. Buffer 增加 mode 字段

```rust
pub enum BufferMode {
    Tui,     // 标准 Unicode
    Sprite,  // PUA 编码
}

pub struct Buffer {
    pub mode: BufferMode,
    pub content: Vec<Cell>,
    pub area: Rect,
}
```

### 4. PUA 编码方案（Sprite 模式）

- 范围：U+F0000 ~ U+F9FFF (40960个字符，160个block × 256个符号)
- 使用 Supplementary Private Use Area-A (U+F0000-U+FFFFF)
- 编码：`0xF0000 + block * 256 + idx`
- Block 0: U+F0000-U+F00FF
- Block 1: U+F0100-U+F01FF
- ...
- Block 159: U+F9F00-U+F9FFF

### 5. get_glyph() — 核心方法（返回 Glyph，包含尺寸信息）

```rust
impl Cell {
    /// 完全由 symbol 决定 Glyph
    pub fn get_glyph(&self) -> Glyph {
        // 1. PUA → Sprite (1x1)
        if let Some(ch) = self.symbol.chars().next() {
            if let Some((block, idx)) = decode_pua(ch) {
                return Glyph::sprite(block, idx);
            }
        }

        // 2. Emoji → Emoji (2x2)
        if let Some((block, idx)) = symbol_map.emoji_idx(&self.symbol) {
            return Glyph::emoji(block, idx);
        }

        // 3. CJK → CJK (2x2)
        if let Some((block, idx)) = symbol_map.cjk_idx(&self.symbol) {
            return Glyph::cjk(block, idx);
        }

        // 4. TUI char → TUI (1x2)
        if let Some((block, idx)) = symbol_map.tui_idx(&self.symbol) {
            return Glyph::tui(block, idx);
        }

        Glyph::sprite(0, 32)  // fallback: space
    }

    /// 兼容方法，返回 (block, idx)
    pub fn get_glyph_info(&self) -> (u8, u8) {
        let glyph = self.get_glyph();
        (glyph.block, glyph.idx)
    }
}
```

### 6. Glyph 缓存机制

- `set_symbol()` 时自动调用 `compute_glyph()`，将 (block, idx, width, height) 缓存到 Cell.glyph
- 渲染时 `get_cell_info()` 直接读 glyph 缓存，不再解析 symbol 字符串
- 查找顺序: PUA(Sprite 1x1) → Emoji(2x2) → CJK(2x2) → TUI(1x2) → fallback(space)

### 7. Buffer.mode 的作用

- **Tui 模式**: symbol 是标准 Unicode（ASCII/Box/Braille/Emoji/CJK）
- **Sprite 模式**: symbol 是 PUA 编码，通过 cellsym_block(block, idx) 构造
- **渲染时不需要 mode**: 完全由 symbol → glyph 缓存决定

### 8. Buffer 设置内容 API 总览

| 分类 | 方法 | 说明 |
|------|------|------|
| **核心（内部）** | `set_stringn(x, y, string, style)` | 按 grapheme cluster 拆分 string，逐个设置 cell；emoji 强制占 2 格，CJK 由 unicode-width 返回 2；多宽字符后续 cell 自动 reset；x,y 为绝对坐标 |
| **字符串设置** | `set_str(x, y, string, style)` | 相对坐标（自动加 area.x/y）；TUI 传 Unicode，Sprite 传 `cellsym_block()` 编码的 PUA |
| | `set_color_str(x, y, string, fg, bg)` | `set_str` 的便捷封装，分开传 fg/bg 颜色 |
| | `set_petscii_str(x, y, string, fg, bg)` | 自动将 ASCII 转换为 PETSCII PUA 符号（C64 复古字体） |
| | `set_default_str(string)` | 在 (0,0) 用默认样式写入 |
| | `set_graph_sym(x, y, texture_id, sym, fg)` | 图形模式快捷方法，内部调用 `set_str(x, y, cellsym_block(texture_id, sym), ..)` |
| | `set_string(x, y, string, style)` | 绝对坐标版本（一般不直接用） |
| **样式/颜色** | `set_style(area, style)` | 区域批量设置样式 |
| | `set_fg(color)` | 全 buffer 设置前景色 |
| **图形绘制** | `set_border(borders, border_type, style)` | TUI 边框 |
| | `draw_circle(x0, y0, radius, sym, fg)` | 画圆 |
| | `draw_line(x1, y1, x2, y2, sym, fg)` | 画线 |

#### 调用示例

```rust
// Sprite 模式（图形）: 使用 cellsym_block 构造 PUA
buf.set_graph_sym(x, y, 1, 65, Color::White);            // block=1, idx=65
buf.set_str(x, y, cellsym_block(1, 65), style);          // 等价写法

// TUI 模式（文本）: 直接传 Unicode
buf.set_str(x, y, "Hello", style);
buf.set_color_str(x, y, "Hello", Color::Green, Color::Reset);

// PETSCII 风格: ASCII 自动转 C64 字体
buf.set_petscii_str(x, y, "HELLO", Color::White, Color::Reset);

// 带 Modifier（如 GLOW 光晕）:
buf.set_str(x, y, cellsym_block(1, 65),
    Style::default().fg(color).bg(Color::Reset).add_modifier(Modifier::GLOW));
```

#### 已移除的旧 API

- `set_str_tex()` / `set_string_tex()` — 已删除，用 `set_str` + `cellsym_block` 替代
- `set_stringn` 的 `width` 参数 — 已注释，调用方从未使用（总是传 usize::MAX）
- `set_stringn` 的返回值 `(u16, u16)` — 已移除，无调用方使用

### 9. 资源文件加载改动

```rust
// pix.rs - 图形模式资源
let mut sp = Buffer::empty_sprite(size);  // 使用 Sprite 模式
sp.set_str(col, y, cellsym_block(tex, idx), style);

// seq_frame.rs - 根据 texture_id 选择模式
let mut sp = if self.texture_id >= 256 {
    Buffer::empty(size)       // TUI 模式 (ESC/UTF8)
} else {
    Buffer::empty_sprite(size) // Sprite 模式
};
sp.set_str(x, y, cellsym_block(tex, idx), style);
```

### 10. PETSCII 风格字符串渲染

```rust
// 方法1：使用 ascii_to_petscii 转换
use rust_pixel::render::ascii_to_petscii;
let petscii = ascii_to_petscii("HELLO WORLD");
buf.set_str(0, 0, petscii, style);

// 方法2：使用便捷方法 set_petscii_str
buf.set_petscii_str(0, 0, "HELLO WORLD", Color::White, Color::Reset);
```

这两种方式都会查询 sprite region 的 extras 映射，将 ASCII 字符转换为 PETSCII PUA 符号，
实现复古 C64 风格的文字渲染。

---

## 实施进度

### 阶段 1：准备工作 ✅

- [x] 1.1 扩展 PUA 编码函数 `cellsym_block(block, idx)`
- [x] 1.2 扩展 PUA 解码函数 `decode_pua(ch) -> Option<(u8, u8)>`
- [x] 1.3 添加 `is_pua_sprite()` 检测函数

### 阶段 2：修改 Buffer ✅

- [x] 2.1 Buffer 增加 `mode: BufferMode` 字段
- [x] 2.2 添加 `Buffer::empty_sprite()` 构造函数
- [x] 2.3 添加 `is_tui()` / `is_sprite()` 方法

### 阶段 3：修改 Cell ✅

- [x] 3.1 Cell 移除 tex 字段
- [x] 3.2 添加 `get_glyph()` 方法返回 Glyph
- [x] 3.3 添加 `get_glyph_info()` 兼容方法
- [x] 3.4 更新 `get_cell_info()` 使用 `get_glyph()`
- [x] 3.5 更新 `set_texture()` 转换为 PUA
- [x] 3.6 更新 `reset()` / `is_blank()`

### 阶段 4：修改渲染管线 graph.rs ✅

- [x] 4.1 添加 Glyph 结构体
- [x] 4.2 更新 render_helper_with_scale 使用 glyph_height 参数
- [x] 4.3 移除 block 范围检查 (sh.1 >= 160 && sh.1 < 170)
- [x] 4.4 使用 glyph.is_double_width() 替代 texidx >= 170 检查

### 阶段 5：修改上层代码 ✅

- [x] 5.1 Sprite::new() 默认使用 Buffer::empty_sprite() (Sprite mode)
- [x] 5.2 新增 Sprite::new_tui() 使用 Buffer::empty() (TUI mode)
- [x] 5.3 set_border() 改为查表 sprite_extras，不再硬编码 tex=1
- [x] 5.4 Apps 无需修改

### 阶段 6：简化 API ✅

- [x] 6.1 移除 `set_str_tex()` 和 `set_string_tex()` 公开方法
- [x] 6.2 `set_stringn()` 移除 tex 参数，直接设置 symbol
- [x] 6.3 更新 `set_graph_sym()` 和 `draw_line()` 使用 `cellsym_block`
- [x] 6.4 更新 pix.rs: 使用 `Buffer::empty_sprite()` + `cellsym_block(tex, idx)`
- [x] 6.5 更新 seq_frame.rs: 根据 texture_id 选择 Buffer 模式
- [x] 6.6 更新 tools/edit 和 apps/petview 使用 `cellsym_block`
- [x] 6.7 `set_stringn()` 移除 width 参数（注释保留），调用方从未实际使用
- [x] 6.8 `set_stringn()` 移除返回值 `(u16, u16)`，无调用方使用

### 阶段 7：清理（待完成）

- [ ] 7.1 移除 symidx() 函数
- [ ] 7.2 简化 symbol_map 相关代码
- [x] 7.3 更新文档：Buffer API 总览、Glyph 缓存机制、已移除 API 说明

---

## 关键规则

1. **Symbol 决定一切**: 渲染时完全由 symbol 确定 Glyph (block, idx, width, height)
2. **Glyph 包含尺寸**: 不再需要通过 block 范围推断尺寸
3. **Buffer 单一类型**: mode=Tui 或 mode=Sprite，不能混用
4. **Sprite 模式**: symbol 必须是 PUA 编码
5. **TUI 模式**: symbol 是标准 Unicode（ASCII/Box/Braille/Emoji/CJK）
6. **调用者构造 PUA**: 使用 `cellsym_block(tex, idx)` 构造正确的 PUA 符号
7. **set_str 无 tex 参数**: `set_str(x, y, cellsym_block(tex, idx), style)` 替代旧的 `set_str_tex`

## Block 范围（渲染用）

| 类型 | Block 范围 | 尺寸 | 来源 |
|------|-----------|------|------|
| Sprite | 0-159 | 1×1 | 从 Supplementary PUA-A 解码 |
| TUI | 160-169 | 1×2 | symbol_map 查表 |
| Emoji | 170-175 | 2×2 | symbol_map 查表 |
| CJK | 176-239 | 2×2 | symbol_map 查表 |

## Glyph 尺寸（以 PIXEL_SYM_WIDTH/HEIGHT 为单位）

| 类型 | width × height | 像素大小 |
|------|---------------|---------|
| Sprite | 1×1 | 16×16 |
| TUI | 1×2 | 16×32 |
| Emoji | 2×2 | 32×32 |
| CJK | 2×2 | 32×32 |
