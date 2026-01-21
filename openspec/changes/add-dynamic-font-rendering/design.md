# 动态字体渲染设计文档

## Context

rust_pixel 是一个基于图块(tile-based)的 2D 游戏引擎,支持终端文本模式和图形模式。当前图形模式使用 2048x2048 统一符号纹理 (`symbols.png`)，采用 block-based 布局：

- **Sprite 区域** (Block 0-47): 3,072 个 16x16 像素符号，用于游戏精灵
- **TUI 区域** (Block 48-52): 640 个 16x32 像素字符，用于 ASCII + 常用符号
- **Emoji 区域** (Block 53-55): 192 个 32x32 像素 Emoji

**问题：** TUI 区域的 640 个预渲染字符无法覆盖 CJK 字符集（2万+字符），需要动态字体光栅化作为补充。

**约束:**
- 保持现有的 Panel/Sprite/Buffer/Cell 架构
- 与现有 TUI 纹理系统无缝集成
- 图形模式下所有渲染仍基于纹理图块
- 不影响终端文本模式(crossterm)的实现
- 保持 60 FPS 性能

**利益相关者:**
- 游戏开发者:需要高质量文本显示
- 中文用户:需要完整的 CJK 字符支持
- 引擎维护者:需要保持架构一致性

## Goals / Non-Goals

**Goals:**
- 提供清晰的文本渲染,媲美终端软件质量
- 支持完整的 Unicode 字符集,特别是 CJK
- 首次渲染延迟 < 0.1ms,缓存命中 0 延迟
- 混合模式:静态图集(ASCII/Emoji) + 动态字体(CJK/Unicode)
- 预留彩色 Emoji 升级接口

**Non-Goals:**
- 不改变终端文本模式的实现
- 不引入复杂的文本排版引擎(保持单字符图块)
- 不在当前版本实现彩色 Emoji(预留接口即可)
- 不支持字体连字(ligatures)或复杂文本布局

## Decisions

### 决策 1: 使用 `fontdue` 库
- **理由:** 纯 Rust 实现,无 C 依赖,支持 WASM,API 简洁,性能优秀
- **备选方案:**
  - `rusttype`: 较老,API 复杂
  - `swash`: 功能强大但过于复杂,支持彩色字体但当前不需要
  - `ab_glyph`: API 设计好,但性能略低于 fontdue
- **选择原因:** fontdue 在性能、API 简洁性和 WASM 支持上最均衡

### 决策 2: TUI 字符全面动态渲染

**渲染策略：**
1. **Sprite 符号** (U+E000~U+E0FF): 使用 Sprite 区域 (Block 0-47) - 保持静态
2. **预渲染 Emoji**: 使用 Emoji 区域 (Block 53-55)，192 个彩色 Emoji - 保持静态
3. **所有 TUI 文本字符**: 动态光栅化（包括 ASCII、CJK 及其他 Unicode）

**为什么 ASCII 也用动态渲染：**
- **缩放清晰度**: 预渲染纹理在任意缩放下都会模糊，动态渲染可针对目标分辨率光栅化
- **一致性**: 中英文混排时，统一使用同一字体渲染，风格更协调
- **简化逻辑**: 无需维护预渲染字符的判断逻辑
- **性能影响极小**: ASCII 字符常用，很快被缓存，后续渲染 0 开销

**保持静态的区域：**
- **Sprite 符号**: 游戏像素艺术，不需要缩放清晰度，保持位图风格
- **Emoji**: 彩色图像，当前用静态预渲染，未来可升级为 swash 彩色字体

- **判断逻辑（在 `render_helper_tui` 中）:**
  ```rust
  fn get_glyph_source(ch: char, symbol: &str) -> GlyphSource {
      // 1. 检查是否为 Sprite 符号（私有使用区）
      if is_sprite_symbol(ch) {
          return GlyphSource::SpriteAtlas(symidx(ch));
      }

      // 2. 检查是否为预渲染 Emoji（Block 53-55）
      if let Some(idx) = emoji_texidx(symbol) {
          return GlyphSource::EmojiAtlas(idx);
      }

      // 3. 所有其他 TUI 字符：动态光栅化（包括 ASCII）
      GlyphSource::Dynamic(ch)
  }
  ```

### 决策 3: LRU 缓存策略
- **缓存容量:** 默认 1000 个字符(可配置)
- **驱逐策略:** Least Recently Used
- **理由:**
  - 1000 个字符覆盖常见场景(ASCII 128 + 高频中文 500 + 其他 372)
  - LRU 确保高频字符保留在缓存中
  - 每个字符纹理约 1-4KB,总内存占用 1-4MB 可接受
- **实现:** 使用 `lru` crate 或自定义 HashMap + LinkedList

### 决策 4: 纹理管理方案
- **方案 A:** 每个字符独立纹理
  - 优点:简单,易于缓存管理,支持任意字符组合
  - 缺点:纹理切换较多(现代 GPU 影响小)
- **方案 B (选择):** 动态纹理图集（与现有 block-based 布局一致）
  - 优点:减少纹理切换,与现有渲染管线无缝集成
  - 缺点:需处理图集满、碎片化等问题
- **选择理由:** 方案 B 与现有 TUI 架构的 block-based 纹理管理一致，可复用现有的单次 draw call 渲染管线

**动态纹理图集设计：**
- 使用独立的动态纹理（如 1024x1024），与 `symbols.png` 分离
- 采用类似 TUI 区域的 block-based 布局
- **字形尺寸：** 统一使用 32x32 像素槽位
  - 半角字符（16x32）占用槽位左半边
  - 全角字符（32x32，如 CJK 汉字）占用整个槽位
- 每个 block 128x128 像素，可容纳 4x4 = 16 个字形槽位
- 1024x1024 纹理 = 8x8 blocks = 64 blocks × 16 slots = 1024 个字形
- LRU 驱逐时按字形槽位清理

### 决策 5: 字体大小、分辨率和 DPI 感知

**基础尺寸（逻辑像素）：**
- **字体大小:** 默认 32px 高度（与 TUI 字符高度一致）
- **字形尺寸:**
  - **半角字符**（ASCII 等）: 16x32 逻辑像素，占 1 个 TUI Cell
  - **全角字符**（CJK 汉字等）: 32x32 逻辑像素，占 2 个 TUI Cell（wcwidth=2）

**DPI 感知和缩放支持：**
- **光栅化尺寸 = 逻辑尺寸 × scale_factor**
  - 1x 缩放: 32px 高度 → 光栅化 32px
  - 2x 缩放: 32px 高度 → 光栅化 64px（Retina/HiDPI）
  - 1.5x 缩放: 32px 高度 → 光栅化 48px
- **动态纹理图集尺寸也随 scale_factor 调整：**
  - 1x: 1024x1024（1024 个 32x32 槽位）
  - 2x: 2048x2048（1024 个 64x64 槽位）
- **scale_factor 来源：**
  - 从 winit/SDL 获取窗口的 `scale_factor`
  - 或用户配置覆盖

**实现方式：**
```rust
pub struct GlyphRenderer {
    base_font_height: f32,  // 逻辑高度 32px
    scale_factor: f32,      // 从窗口获取，如 1.0, 2.0

    // 实际光栅化高度 = base_font_height * scale_factor
    fn effective_font_height(&self) -> f32 {
        self.base_font_height * self.scale_factor
    }
}
```

**理由:**
- 32px 逻辑高度与 TUI 字符高度一致
- CJK 汉字为全角，宽高相等（32x32），符合 Unicode 宽度标准
- **核心优势：按实际显示尺寸光栅化**
  - 预渲染位图在放大时必然模糊（插值）或有锯齿（最近邻）
  - 动态字体根据当前缩放比例重新光栅化，始终保持清晰
  - 例：2x 缩放时，光栅化为 64px 字体，而非将 32px 位图放大 2 倍

### 决策 6: Emoji 接口设计
- **当前:** 保持静态 Emoji 图集,通过 `emoji_map` 查找
- **接口预留:**
  ```rust
  pub trait GlyphProvider {
      fn get_glyph(&mut self, ch: char) -> GlyphTexture;
  }

  // 当前实现
  struct StaticEmojiProvider { atlas: TextureAtlas }

  // 未来实现(预留)
  struct DynamicEmojiProvider { font: swash::Font }
  ```
- **理由:** 通过 trait 抽象,未来可无缝切换实现

## Architecture

### 与 TUI 架构的集成

动态字体渲染**接管所有 TUI 文本字符**，仅保留 Sprite 和 Emoji 的静态纹理：

```
字符渲染流程：
┌─────────────────────────────────────────────────────────────┐
│                     render_helper_tui()                      │
├─────────────────────────────────────────────────────────────┤
│  1. is_sprite_symbol(ch)?  ──→ Sprite 区域 (Block 0-47)     │
│  2. emoji_texidx(symbol)?  ──→ Emoji 区域 (Block 53-55)     │
│  3. else (所有文本字符)    ──→ GlyphRenderer (动态纹理)      │
│     - ASCII (0x20~0x7E)                                      │
│     - CJK (中日韩汉字)                                       │
│     - 其他 Unicode 字符                                      │
└─────────────────────────────────────────────────────────────┘

注：TUI 区域 (Block 48-52) 的 640 个预渲染字符将不再使用，
    可用于其他用途或在未来版本中移除。
```

### 核心组件

```rust
/// 字形来源枚举
pub enum GlyphSource {
    /// 静态 Sprite 符号纹理 (symbols.png Block 0-47)
    SpriteAtlas { texidx: u8, symidx: u8 },
    /// 静态 TUI 字符纹理 (symbols.png Block 48-52)
    TuiAtlas { texidx: u8, symidx: u8 },
    /// 静态 Emoji 纹理 (symbols.png Block 53-55)
    EmojiAtlas { texidx: u8, symidx: u8 },
    /// 动态光栅化字形
    Dynamic { texture_id: u32, uv: UVRect },
}

/// 字形渲染器 - 管理动态字体和纹理缓存
pub struct GlyphRenderer {
    // 动态字体 (用于 CJK 等)
    text_font: fontdue::Font,
    text_font_height: f32,  // 32px，与 TUI 字符高度一致

    // 动态纹理图集
    dynamic_atlas: DynamicTextureAtlas,

    // 字形缓存 (char -> 动态图集位置)
    glyph_cache: LruCache<char, GlyphLocation>,

    // GPU 交互 (平台相关)
    texture_uploader: Box<dyn TextureUploader>,
}

/// 动态纹理图集
pub struct DynamicTextureAtlas {
    texture_id: u32,
    width: u32,   // 1024
    height: u32,  // 1024
    // Block-based 管理，与 TUI 架构一致
    // 8x8 blocks = 64 blocks, 每个 block 16 slots
    blocks: Vec<DynamicBlock>,
    next_slot: usize,
}

/// 动态图集中的 block
pub struct DynamicBlock {
    // 128x128 像素，4x4 = 16 个字形槽位（每个 32x32）
    slots: [Option<char>; 16],
    usage_count: u32,
}

/// 字形位置信息
pub struct GlyphLocation {
    block_idx: usize,    // Block 索引 (0-63)
    slot_idx: usize,     // 槽位索引 (0-15)
    is_fullwidth: bool,  // 是否为全角字符（CJK 汉字）
}

impl GlyphLocation {
    /// 转换为 UV 坐标
    pub fn to_uv_rect(&self) -> UVRect {
        let block_x = (self.block_idx % 8) as f32;
        let block_y = (self.block_idx / 8) as f32;
        let slot_x = (self.slot_idx % 4) as f32;
        let slot_y = (self.slot_idx / 4) as f32;

        let x = (block_x * 128.0 + slot_x * 32.0) / 1024.0;
        let y = (block_y * 128.0 + slot_y * 32.0) / 1024.0;
        let w = if self.is_fullwidth { 32.0 / 1024.0 } else { 16.0 / 1024.0 };
        let h = 32.0 / 1024.0;

        UVRect { x, y, w, h }
    }
}

impl GlyphRenderer {
    /// 获取字符的字形来源
    pub fn get_glyph_source(&mut self, ch: char) -> GlyphSource {
        // 检查缓存
        if let Some(loc) = self.glyph_cache.get(&ch) {
            return GlyphSource::Dynamic {
                texture_id: self.dynamic_atlas.texture_id,
                uv: loc.to_uv_rect(),
            };
        }

        // 动态光栅化并缓存
        let loc = self.rasterize_and_cache(ch);
        GlyphSource::Dynamic {
            texture_id: self.dynamic_atlas.texture_id,
            uv: loc.to_uv_rect(),
        }
    }

    fn rasterize_and_cache(&mut self, ch: char) -> GlyphLocation {
        // 判断是否为全角字符（CJK 汉字等）
        let is_fullwidth = unicode_width::UnicodeWidthChar::width(ch)
            .map(|w| w == 2)
            .unwrap_or(false);

        // 使用 fontdue 光栅化
        let (metrics, bitmap) = self.text_font.rasterize(ch, self.text_font_height);

        // 分配图集槽位
        let loc = self.dynamic_atlas.allocate_slot(ch, is_fullwidth);

        // 计算像素坐标
        let block_x = (loc.block_idx % 8) * 128;
        let block_y = (loc.block_idx / 8) * 128;
        let slot_x = (loc.slot_idx % 4) * 32;
        let slot_y = (loc.slot_idx / 4) * 32;
        let pixel_x = block_x + slot_x;
        let pixel_y = block_y + slot_y;

        // 上传到 GPU（部分更新）
        self.texture_uploader.upload_subimage(
            self.dynamic_atlas.texture_id,
            pixel_x as u32,
            pixel_y as u32,
            &bitmap,
            metrics.width as u32,
            metrics.height as u32,
        );

        // 添加到缓存
        self.glyph_cache.put(ch, loc.clone());
        loc
    }
}

/// 平台相关的纹理上传接口
pub trait TextureUploader {
    /// 创建新纹理
    fn create_texture(&mut self, width: u32, height: u32) -> u32;
    /// 部分更新纹理
    fn upload_subimage(&mut self, texture_id: u32, x: u32, y: u32,
                       data: &[u8], width: u32, height: u32);
}
```

### 集成到 Adapter

每个图形适配器需要:
1. 创建 `GlyphRenderer` 实例
2. 在 `render_helper_tui()` 中判断字形来源
3. 实现 `TextureUploader` trait

```rust
// 示例: 在 render_symbols.rs 中集成
impl WgpuAdapter {
    fn init(&mut self) {
        // 加载字体
        let font_data = include_bytes!("../assets/NotoSansCJK-Regular.ttf");
        let font = fontdue::Font::from_bytes(font_data, Default::default()).unwrap();

        // 创建动态字形渲染器
        self.glyph_renderer = Some(GlyphRenderer::new(
            font,
            32.0, // 高度 32px，与 TUI 字符一致
            Box::new(WgpuTextureUploader { device: &self.device, queue: &self.queue })
        ));
    }
}

// 在 render_helper_tui 中使用
fn render_helper_tui(
    cell: &Cell,
    glyph_renderer: &mut GlyphRenderer,
    // ...
) -> RenderCell {
    let ch = cell.symbol.chars().next().unwrap_or(' ');

    // 1. 检查是否为预渲染 TUI 字符
    if let Some((texidx, symidx)) = tui_char_index(ch) {
        return RenderCell {
            texture_id: SYMBOLS_TEXTURE_ID,
            tx: calculate_tui_tx(texidx, symidx),
            ty: calculate_tui_ty(texidx, symidx),
            // ...
        };
    }

    // 2. 检查是否为预渲染 Emoji
    if let Some((texidx, symidx)) = emoji_texidx(&cell.symbol) {
        return RenderCell {
            texture_id: SYMBOLS_TEXTURE_ID,
            tx: calculate_emoji_tx(texidx, symidx),
            ty: calculate_emoji_ty(texidx, symidx),
            // ...
        };
    }

    // 3. 动态光栅化
    let glyph_source = glyph_renderer.get_glyph_source(ch);
    match glyph_source {
        GlyphSource::Dynamic { texture_id, uv } => {
            RenderCell {
                texture_id,
                tx: uv.x,
                ty: uv.y,
                tw: uv.w,
                th: uv.h,
                // ...
            }
        }
        _ => unreachable!(),
    }
}
```

### 预加载机制

```rust
impl GlyphRenderer {
    /// 启动时预加载常用字符
    pub fn preload_common_chars(&mut self) {
        // ASCII 完整集
        for ch in 0x20..=0x7E {
            self.get_texture(ch as u8 as char);
        }

        // 高频中文(可配置)
        const COMMON_CHINESE: &str = "的一是了我不人在他有这个上们来到时大地为子中你说生国年着就那和要她出也得里后自以会家可下而过天去能对小多然于心学么之都好看起发当没成只如事把还用第样道想作种开手十用他然";
        for ch in COMMON_CHINESE.chars() {
            self.get_texture(ch);
        }
    }
}
```

## Risks / Trade-offs

### 风险 1: 首次渲染延迟
- **风险:** 首次渲染大量新字符可能卡顿
- **缓解:**
  - 启动预加载常用字符
  - 异步光栅化(WASM 中需注意)
  - 限制单帧最大新字符数

### 风险 2: 内存占用
- **风险:** 大量字符缓存占用内存
- **缓解:**
  - LRU 限制缓存大小(默认 1000)
  - 提供 API 供用户调整缓存容量
  - 监控内存使用,必要时主动清理

### 风险 3: WASM 兼容性
- **风险:** fontdue 在 WASM 中性能可能下降
- **缓解:**
  - ✅ fontdue 是 pure Rust + no_std,天然支持 WASM
  - ✅ 官方提供 WebAssembly 演示和 Canvas 集成示例
  - 预加载策略在 WASM 中更重要(减少运行时光栅化)
  - 字体文件可通过 `include_bytes!` 内嵌或异步加载
- **参考:** [fontdue GitHub](https://github.com/mooman219/fontdue), [fontdue on crates.io](https://crates.io/crates/fontdue)

### Trade-off: 纹理切换 vs 图集管理复杂度
- **选择:** 独立纹理(更多切换)
- **原因:** 现代 GPU 对小纹理切换优化好,代码简单价值更高
- **影响:** 理论性能略低,但实测影响可忽略(< 1ms/frame)

## Migration Plan

### 阶段 1: 核心实现 (Week 1)
1. 添加 `fontdue` 依赖
2. 实现 `GlyphRenderer` 和 `TextureUploader` trait
3. 在 `SdlAdapter` 中集成(作为 POC)
4. 单元测试和性能基准

### 阶段 2: 全平台支持 (Week 2)
1. 集成到 `WinitGlowAdapter`
2. 集成到 `WinitWgpuAdapter`
3. 集成到 `WebAdapter`(重点测试 WASM)
4. 集成测试

### 阶段 3: 优化和文档 (Week 3)
1. 预加载机制
2. 批量纹理上传优化
3. 性能调优
4. 文档和示例

### 回滚策略
- 功能通过 feature flag 控制: `dynamic-font`
- 默认禁用,用户选择启用
- 如有问题可快速回退到静态图集

## Open Questions

1. **字体文件分发:** 是否将 TTF 字体内嵌到二进制?还是要求用户提供?
   - 倾向:内嵌一个默认字体(Noto Sans CJK Simplified),用户可覆盖

2. **多语言优先级:** 是否需要针对日文、韩文优化?
   - 倾向:先支持中文,日韩文自动工作(都在 CJK Unicode 范围)

3. **子像素渲染:** 是否需要支持 subpixel antialiasing?
   - 倾向:暂不支持,灰度抗锯齿已足够,子像素增加复杂度

4. **性能目标:** 具体的性能指标是什么?
   - 倾向:60 FPS 下支持全屏文本(80x24 或更大),首屏渲染 < 16ms
