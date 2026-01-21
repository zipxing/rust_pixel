# 动态字体渲染 (Dynamic Font Rendering)

## Why

当前图形模式使用 2048x2048 统一符号纹理 (`symbols.png`)，其中 TUI 区域 (Block 48-52) 提供 640 个预渲染字符。这存在两个主要问题：

- **CJK 支持有限**: 640 个字符无法覆盖数量庞大的中日韩字符集（2万+字符）
- **缩放后不清晰**: 虽然图形模式已支持任意缩放，但预渲染的位图纹理在放大时会出现模糊或锯齿（这是位图缩放的固有限制）

这限制了 rust_pixel 在需要完整 Unicode 支持和高质量文本显示的应用场景中的可用性。

## What Changes

- 添加基于 `fontdue` 库的动态字体光栅化功能
- **移除 TUI 预渲染区域** (Block 48-52)，所有 TUI 文本字符改用动态渲染
- 实现简化的混合渲染模式:
  1. **Sprite 符号** (U+E000~U+E0FF): 使用 Sprite 区域 (Block 0-47) - 静态
  2. **预渲染 Emoji**: 使用 Emoji 区域 (Block 53-55) - 静态
  3. **所有文本字符**: 动态光栅化（ASCII + CJK + Unicode）
- 添加独立的动态纹理图集（1024x1024 @ 1x，随 DPI 缩放），采用 block-based 布局
- 添加带 LRU 缓存的延迟加载机制（默认 1000 字符）
- **两次渲染方案**: 静态图集一次 draw call，动态图集一次 draw call
- **DPI 感知光栅化**: 根据窗口 scale_factor 动态调整字体渲染尺寸

**性能特征:**
- 首次字符渲染: ~0.05-0.1ms (含缓存)
- 缓存字符渲染: 0ms (等同于静态纹理)
- 两次 draw call 额外开销: < 0.1ms/frame
- 启动时预加载常用字符

**清晰度优势:**
- 按实际显示分辨率光栅化，而非缩放位图
- 放大后仍保持锐利边缘，无模糊或锯齿
- HiDPI/Retina 显示器原生支持

**架构简化:**
- 移除 Block 48-52 预渲染区域，减少一个字符来源
- 释放空间可用于扩展 Sprite 或 Emoji
- 代码逻辑更简洁

## Impact

**影响的规范:**
- `rendering` - 新增动态字体渲染能力

**影响的代码:**
- `src/render/adapter/*/render_symbols.rs` - 所有图形模式适配器的符号渲染
- `src/render/graph.rs` - 添加 `GlyphRenderer` 和动态纹理管理
- `src/asset.rs` - 字体资源加载和管理
- `Cargo.toml` - 添加 `fontdue = "0.8"`、`lru = "0.12"` 和 `unicode-width` 依赖

**破坏性变更:**
- Block 48-52 (TUI 预渲染区域) 将不再使用
- 依赖 `tui_char_index()` 的代码需要移除

**迁移:**
- 现有应用无需修改即可继续工作
- 应用可通过加载 TTF 字体获得完整 CJK 支持
