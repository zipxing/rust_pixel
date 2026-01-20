# 动态字体渲染设计文档

## Context

rust_pixel 是一个基于图块(tile-based)的 2D 游戏引擎,支持终端文本模式和图形模式。当前图形模式的 TUI 文本渲染使用预制纹理图集,限制了文本清晰度和 CJK 字符支持。需要在保持图块渲染理念的前提下引入动态字体光栅化。

**约束:**
- 保持现有的 Panel/Sprite/Buffer/Cell 架构
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

### 决策 2: 混合渲染策略
- **静态图集:** ASCII (0-127)、PETSCII、静态 Emoji
- **动态字体:** CJK、其他 Unicode、特殊符号
- **理由:**
  - ASCII 使用频率高,静态图集性能最佳
  - Emoji 需要彩色,当前用静态图集,未来可升级
  - CJK 字符集庞大(2万+),必须动态生成
- **判断逻辑:**
  ```rust
  if ch.is_ascii() || emoji_map.contains(ch) {
      use_static_atlas()
  } else {
      use_dynamic_font()
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
- **方案 A (选择):** 每个字符独立纹理
  - 优点:简单,易于缓存管理,支持任意字符组合
  - 缺点:纹理切换较多(现代 GPU 影响小)
- **方案 B (未选择):** 动态纹理图集
  - 优点:减少纹理切换
  - 缺点:复杂,需处理图集满、碎片化等问题
- **选择理由:** 方案 A 的简单性价值更高,现代 GPU 对小纹理切换优化良好

### 决策 5: 字体大小和分辨率
- **字体大小:** 默认 16px(可配置)
- **渲染分辨率:** 1x(与屏幕像素 1:1)
- **理由:**
  - 16px 在 terminal mode 中常见,保持一致
  - 1:1 渲染避免缩放模糊
  - 不同 DPI 通过调整字体大小参数而非缩放纹理

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

### 核心组件

```rust
/// 字形渲染器 - 管理字体和纹理缓存
pub struct GlyphRenderer {
    // 动态字体(用于文本)
    text_font: fontdue::Font,
    text_font_size: f32,

    // 字形纹理缓存 (char -> GPU texture ID)
    glyph_cache: LruCache<char, u32>,

    // 静态 Emoji 图集
    emoji_atlas: TextureAtlas,
    emoji_map: HashMap<char, (u32, UVRect)>,

    // GPU 交互(平台相关)
    texture_uploader: Box<dyn TextureUploader>,
}

impl GlyphRenderer {
    /// 获取字符对应的纹理
    /// 自动判断使用静态图集还是动态渲染
    pub fn get_texture(&mut self, ch: char) -> (u32, UVRect) {
        // 1. 检查是否为 Emoji
        if let Some(&texture) = self.emoji_map.get(&ch) {
            return texture;
        }

        // 2. 检查是否为 ASCII (可选:也可用动态渲染)
        if ch.is_ascii() && self.ascii_atlas.is_some() {
            return self.ascii_atlas.get(ch);
        }

        // 3. 动态光栅化(带缓存)
        let texture_id = self.glyph_cache.get_or_insert_with(ch, || {
            self.rasterize_and_upload(ch)
        });

        (*texture_id, UVRect::full())
    }

    fn rasterize_and_upload(&mut self, ch: char) -> u32 {
        // 使用 fontdue 光栅化
        let (metrics, bitmap) = self.text_font.rasterize(ch, self.text_font_size);

        // 上传到 GPU
        self.texture_uploader.upload_alpha8(
            bitmap,
            metrics.width,
            metrics.height
        )
    }
}

/// 平台相关的纹理上传接口
pub trait TextureUploader {
    fn upload_alpha8(&mut self, data: &[u8], width: usize, height: usize) -> u32;
}
```

### 集成到 Adapter

每个图形适配器需要:
1. 创建 `GlyphRenderer` 实例
2. 在 `draw_cell()` 时调用 `get_texture()`
3. 实现 `TextureUploader` trait

```rust
// 示例: SdlAdapter
impl SdlAdapter {
    fn init(&mut self) {
        // 加载字体
        let font_data = include_bytes!("../assets/NotoSansCJK-Regular.ttf");
        let font = fontdue::Font::from_bytes(font_data, Default::default()).unwrap();

        // 创建渲染器
        self.glyph_renderer = Some(GlyphRenderer::new(
            font,
            16.0, // font size
            Box::new(SdlTextureUploader { texture_creator: &self.texture_creator })
        ));
    }

    fn draw_cell(&mut self, cell: &Cell, x: i32, y: i32) {
        let ch = cell.get_symbol();
        let (texture_id, uv) = self.glyph_renderer.get_texture(ch);

        // 使用 texture_id 渲染...
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
