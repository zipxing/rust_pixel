# 动态字体目录 / Dynamic Font Directory

此目录用于存放动态字体渲染所需的 TTF/OTF 字体文件。

This directory stores TTF/OTF font files for dynamic font rendering.

## 默认字体 / Default Font

将你的等宽字体文件命名为 `default.ttf` 放在此目录下。

Place your monospace font file named `default.ttf` in this directory.

## 推荐字体 / Recommended Fonts

以下是一些支持 CJK 字符的开源等宽字体：

Here are some open-source monospace fonts with CJK support:

1. **Sarasa Gothic (更纱黑体)**
   - https://github.com/be5invis/Sarasa-Gothic
   - 推荐: `sarasa-mono-sc-regular.ttf` (简体中文)

2. **Noto Sans Mono CJK**
   - https://github.com/googlefonts/noto-cjk
   - 推荐: `NotoSansMonoCJKsc-Regular.otf`

3. **Source Han Mono (思源等宽)**
   - https://github.com/adobe-fonts/source-han-mono
   - 推荐: `SourceHanMono-Regular.otf`

4. **LXGW WenKai Mono (霞鹜文楷等宽)**
   - https://github.com/lxgw/LxgwWenKai
   - 推荐: `LXGWWenKaiMono-Regular.ttf`

## 使用方法 / Usage

```rust
use rust_pixel::render::GlyphRenderer;

// 从默认路径加载
let renderer = GlyphRenderer::with_default_font(".", 1.0)?;

// 从指定路径加载
let renderer = GlyphRenderer::from_file("path/to/font.ttf", 1.0)?;

// 从字节数据加载
let font_data = include_bytes!("path/to/font.ttf");
let renderer = GlyphRenderer::from_bytes(font_data, 1.0)?;
```

## 字体要求 / Font Requirements

- **格式**: TTF 或 OTF
- **类型**: 等宽字体 (Monospace) 效果最佳
- **字符集**: 至少包含 ASCII (U+0020-U+007E)
- **CJK 支持**: 如需中文显示，需包含 CJK Unified Ideographs

## 注意事项 / Notes

- 字体文件不包含在 git 仓库中，需要自行下载
- 字体文件大小建议不超过 10MB
- 首次使用时会预加载 ASCII 字符到缓存
