# 实现任务清单

## 1. 核心功能实现
- [ ] 1.1 添加 `fontdue`、`lru` 和 `unicode-width` 依赖到 Cargo.toml
- [ ] 1.2 创建 `GlyphSource` 枚举和 `GlyphRenderer` 结构体
- [ ] 1.3 实现 `DynamicTextureAtlas` - block-based 动态纹理图集管理
- [ ] 1.4 实现 LRU 缓存策略（默认 1000 字符）
- [ ] 1.5 实现 `TextureUploader` trait 和纹理部分更新逻辑
- [ ] 1.6 实现 DPI 感知光栅化（根据 scale_factor 调整字体大小和图集尺寸）

## 2. 适配器集成
- [ ] 2.1 在 `render_symbols.rs` 中修改 `render_helper_tui()` 集成字形来源判断
- [ ] 2.2 在 `WinitWgpuAdapter` 中实现 `WgpuTextureUploader`
- [ ] 2.3 在 `WinitGlowAdapter` 中实现 `GlowTextureUploader`
- [ ] 2.4 在 `SdlAdapter` 中实现 `SdlTextureUploader`
- [ ] 2.5 在 `WebAdapter` 中实现 `WebGlTextureUploader`

## 3. 资源管理
- [ ] 3.1 在 `AssetManager` 中添加字体加载支持
- [ ] 3.2 添加默认 CJK TTF 字体到 assets（如 Noto Sans CJK）
- [ ] 3.3 实现字体资源的配置（允许用户指定自定义字体）

## 4. 测试和优化
- [ ] 4.1 编写字形缓存的单元测试
- [ ] 4.2 编写 CJK 字符渲染正确性测试
- [ ] 4.3 性能基准测试（首次渲染 vs 缓存命中）
- [ ] 4.4 常用字符预加载优化（ASCII + 高频中文）
- [ ] 4.5 缩放清晰度测试（1x, 1.5x, 2x 缩放下的渲染质量）

## 5. 文档和示例
- [ ] 5.1 更新 CLAUDE.md 添加动态字体说明
- [ ] 5.2 创建 CJK 文本渲染示例应用
- [ ] 5.3 添加 API 文档注释
- [ ] 5.4 更新 TUI 架构文档说明动态字体集成
