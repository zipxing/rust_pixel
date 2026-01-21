# 实现任务清单

## 1. 核心功能实现
- [ ] 1.1 添加 `fontdue`、`lru` 和 `unicode-width` 依赖到 Cargo.toml
- [ ] 1.2 创建 `GlyphSource` 枚举（移除 TuiAtlas）和 `GlyphRenderer` 结构体
- [ ] 1.3 实现 `DynamicTextureAtlas` - block-based 动态纹理图集管理
- [ ] 1.4 实现 LRU 缓存策略（默认 1000 字符）
- [ ] 1.5 实现 `TextureUploader` trait 和纹理部分更新逻辑
- [ ] 1.6 实现 DPI 感知光栅化（根据 scale_factor 调整字体大小和图集尺寸）

## 2. 移除 TUI 预渲染区域
- [ ] 2.1 移除 `tui_char_index()` 相关代码
- [ ] 2.2 更新 `render_helper_tui()` 判断逻辑（仅保留 Sprite/Emoji/Dynamic 三种来源）
- [ ] 2.3 更新 symbols.png 相关常量定义（Block 48-52 标记为未使用）
- [ ] 2.4 清理 TUI 预渲染相关的测试代码

## 3. 适配器集成（两次渲染方案）
- [ ] 3.1 实现 `categorize_cells()` 按纹理分类（static_cells / dynamic_cells）
- [ ] 3.2 修改渲染循环支持两次 draw call
- [ ] 3.3 在 `WinitWgpuAdapter` 中实现 `WgpuTextureUploader`
- [ ] 3.4 在 `WinitGlowAdapter` 中实现 `GlowTextureUploader`
- [ ] 3.5 在 `SdlAdapter` 中实现 `SdlTextureUploader`
- [ ] 3.6 在 `WebAdapter` 中实现 `WebGlTextureUploader`

## 4. 资源管理
- [ ] 4.1 在 `AssetManager` 中添加字体加载支持
- [ ] 4.2 添加默认 CJK TTF 字体到 assets（如 Noto Sans CJK 子集）
- [ ] 4.3 实现字体资源的配置（允许用户指定自定义字体）

## 5. 测试和优化
- [ ] 5.1 编写字形缓存的单元测试
- [ ] 5.2 编写 CJK 字符渲染正确性测试
- [ ] 5.3 性能基准测试（首次渲染 vs 缓存命中 vs 两次 draw call 开销）
- [ ] 5.4 常用字符预加载优化（ASCII + 高频中文）
- [ ] 5.5 缩放清晰度测试（1x, 1.5x, 2x 缩放下的渲染质量）

## 6. 文档和示例
- [ ] 6.1 更新 CLAUDE.md 添加动态字体说明
- [ ] 6.2 创建 CJK 文本渲染示例应用
- [ ] 6.3 添加 API 文档注释
- [ ] 6.4 更新 TUI 架构文档说明：
  - 动态字体集成
  - Block 48-52 已移除
  - 两次渲染方案
