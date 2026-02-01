# refactor-sprite 变更归档

## 归档信息

- **归档日期**: 2026-02-01
- **OpenSpec ID**: refactor-sprite
- **完成度**: 94%
- **状态**: ✅ 核心功能完成，可编译运行
- **测试状态**: ✅ 所有应用正常运行

## 变更摘要

对 rust_pixel 游戏引擎进行了全面的渲染架构重构，建立了清晰的 Widget + Sprite 二元模型，统一了 GPU 渲染管线，并提供了灵活的自定义渲染能力。

## 核心成就

### 1. Panel → Scene, Sprites → Layer 命名重构

将核心渲染概念重命名为更直观的术语：
- `Panel` → `Scene` (场景容器)
- `Sprites` → `Layer` (渲染层)
- layers[0] "main" → "tui" (TUI 内容层)
- layers[1] "pixel" → "sprite" (图形精灵层)

### 2. 统一 4096 纹理块系统

建立了统一的块索引系统 (Block Index System)：
- Sprite 区域 (Block 0-159): 160 块，每块 256×256px
- TUI 区域 (Block 160-169): 10 块，每块 256×512px
- Emoji 区域 (Block 170-175): 6 块，每块 256×512px
- CJK 区域 (Block 176-239): 64 块，每块 256×256px
- 保留区域 (Block 240-255): 16 块

### 3. 四阶段 GPU 渲染管线

建立了清晰的四阶段渲染管线：

```
阶段1: 数据源 → RenderBuffer
        TUI Buffer + Sprite Layer → Vec<RenderCell>

阶段2: RenderBuffer → RT
        draw_to_rt(rbuf, rt) → RT0/RT1/RT2/RT3

阶段3: RT 运算 (可选)
        blend_rts(), copy_rt(), clear_rt()

阶段4: RT[] → Screen
        present([RtComposite...])
```

### 4. 统一 RT API

在 Adapter trait 中添加了统一的 RT 操作 API：

```rust
fn draw_render_buffer_to_texture(&mut self, rbuf: &[RenderCell], rt: usize, debug: bool);
fn blend_rts(&mut self, src1: usize, src2: usize, target: usize, effect: usize, progress: f32);
fn copy_rt(&mut self, src: usize, dst: usize);
fn clear_rt(&mut self, rt: usize);
fn set_rt_visible(&mut self, rt: usize, visible: bool);
fn present(&mut self, composites: &[RtComposite]);
fn present_default(&mut self);
```

### 5. Viewport 辅助方法

提供了便捷的 viewport 计算和创建方法：

**RtComposite 方法：**
- `fullscreen(rt)` - 全屏显示
- `with_viewport(rt, vp)` - 自定义 viewport
- `centered(rt, w, h, cw, ch)` - 居中显示
- `at_position(rt, x, y, w, h)` - 指定位置
- `x(val)`, `y(val)`, `offset(dx, dy)` - 链式位置调整

**Context 方法：**
- `centered_viewport(cell_w, cell_h)` - 计算居中 viewport
- `centered_rt(rt, cell_w, cell_h)` - 创建居中 RtComposite (最便捷 API)
- `canvas_size()` - 获取画布尺寸
- `ratio()` - 获取 DPI 缩放比例

### 6. App 自定义渲染支持

App 的 `Render::draw()` 拥有完全的渲染控制权：

**模式1：默认流程 (大多数 app)**
```rust
fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    self.scene.draw(ctx).unwrap();
}
```

**模式2：自定义 present**
```rust
fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    self.scene.draw_to_rt(ctx).unwrap();
    let rt3 = ctx.centered_rt(3, CELL_W, CELL_H);
    ctx.adapter.present(&[RtComposite::fullscreen(2), rt3]);
}
```

**模式3：完全自定义**
```rust
fn draw(&mut self, ctx: &mut Context, model: &mut Model, dt: f32) {
    ctx.adapter.buf2rt(&self.buffer, 0);
    ctx.adapter.blend_rts(0, 1, 3, effect, progress);
    ctx.adapter.present(&[RtComposite::fullscreen(3)]);
}
```

## 文件清单

### 核心修改

| 文件 | 变更 |
|------|------|
| `src/render/panel.rs` → `src/render/scene.rs` | Panel 重命名为 Scene |
| `src/render/sprite/sprites.rs` → `src/render/sprite/layer.rs` | Sprites 重命名为 Layer |
| `src/render/graph.rs` | 添加 RtComposite, BlendMode, viewport 辅助方法 |
| `src/render/adapter.rs` | 统一 RT API 定义 |
| `src/render/adapter/gl/pixel.rs` | OpenGL present 实现，viewport 位置支持 |
| `src/render/adapter/winit_glow_adapter.rs` | Glow adapter RT API 实现 |
| `src/render/adapter/winit_wgpu_adapter.rs` | WGPU adapter RT API 实现，viewport 位置支持 |
| `src/render/adapter/sdl_adapter.rs` | SDL adapter RT API 实现 |
| `src/render/adapter/web_adapter.rs` | Web adapter RT API 实现 |
| `src/context.rs` | 添加 viewport 辅助方法 |

### 应用迁移

所有应用已迁移到新架构：
- `apps/petview/` - 完整使用新 RT API 和 viewport 辅助方法
- `apps/snake/`, `apps/tetris/`, `apps/tower/` 等 - 迁移到 Scene/Layer

### 文档

| 文件 | 内容 |
|------|------|
| `openspec/changes/refactor-sprite/design.md` | 完整设计文档 (1500+ 行) |
| `openspec/changes/refactor-sprite/tasks.md` | 任务清单 |
| `openspec/changes/refactor-sprite/specs/rendering/spec.md` | 规格说明 |
| `CLAUDE.md` | 架构说明更新 |

## 设计文档亮点

### 8.9 Viewport 辅助方法与坐标系统

详细说明了四种坐标系统及其转换：
1. Cell 坐标 (40×25 逻辑单元)
2. Pixel 坐标 (320×200 纹理像素)
3. Canvas 坐标 (屏幕像素，含 HiDPI 缩放)
4. NDC 坐标 (-1 到 +1，GPU 着色器使用)

NDC 平移公式：
```
tx = (2 * vp_x + vp_w - canvas_w) / canvas_w
ty = (canvas_h - 2 * vp_y - vp_h) / canvas_h
```

### Section 9: 完整渲染 API 参考

包含：
- 渲染流程总览图
- 核心 API 速查表 (16 个方法)
- 四种典型使用场景示例
- 调试技巧

## 进度总结

| Phase | 完成度 | 状态 |
|-------|--------|------|
| Phase 1: 核心重构 | 100% | ✅ 完成 |
| Phase 2: 应用迁移 | 100% | ✅ 完成 |
| Phase 3: 测试/文档 | 100% | ✅ 完成 |
| Phase 4: 发布准备 | 0% | ⬜ 待定 |
| Phase 5: GPU 渲染管线 | 100% | ✅ 完成 |

**总完成度: 94%** (功能完成，发布可选)

## 关键代码片段

### RtComposite 链式 API

```rust
// 创建居中 RT，然后调整位置
let rt3 = ctx.centered_rt(3, 40, 25).x(0);  // 左对齐
let rt3 = ctx.centered_rt(3, 40, 25).offset(-10, 0);  // 左移 10px

// 使用 present 合成
ctx.adapter.present(&[
    RtComposite::fullscreen(2),
    rt3,
]);
```

### Viewport 位置生效 (NDC 平移)

```rust
// gl/pixel.rs 和 winit_wgpu_adapter.rs
let tx = (2.0 * vp_x + vp_w - canvas_w) / canvas_w;
let ty = (canvas_h - 2.0 * vp_y - vp_h) / canvas_h;

let mut transform = UnifiedTransform::new();
transform.scale(vp_w / canvas_w, vp_h / canvas_h);
transform.translate(tx, ty);
```

## 后续工作建议

### 可选发布准备

1. **版本号更新**: Cargo.toml version = "2.0.0"
2. **创建 Git Tag**: `git tag -a v2.0.0 -m "Scene/Layer Architecture"`
3. **发布到 crates.io**: `cargo publish`

### 未来优化方向

1. **BlendMode 实现**: 目前只有 Normal 模式，可扩展 Add/Multiply/Screen
2. **RT Alpha 支持**: RtComposite.alpha 字段已预留，待实现
3. **更多转场效果**: blend_rts 的 effect 参数可扩展更多类型

## 经验总结

### 成功经验

1. **渐进式重构**: 保持 deprecated API 兼容，逐步迁移
2. **统一抽象层**: RtComposite 很好地封装了 RT 操作细节
3. **坐标系统文档**: 详细的坐标转换说明减少了使用错误
4. **链式 API**: `.x().y().offset()` 提供了优雅的调整方式

### 遇到的挑战

1. **借用检查器**: `ctx.centered_rt()` 不能直接在 `present()` 参数中使用
2. **Viewport 位置不生效**: 初始实现忘记了 NDC 平移
3. **坐标系统复杂性**: Cell/Pixel/Canvas/NDC 四层转换容易混淆

## 结论

refactor-sprite OpenSpec 实现已基本完成 (94%)，核心功能全部实现并可运行。这次重构为 rust_pixel 引擎建立了更清晰的渲染架构，提供了强大的自定义渲染能力，并通过详尽的文档和辅助 API 降低了使用门槛。

**主要成就**:
- ✅ Panel → Scene, Sprites → Layer 命名重构
- ✅ 统一 4096 纹理块系统
- ✅ 四阶段 GPU 渲染管线
- ✅ 统一 RT API (blend_rts, copy_rt, present 等)
- ✅ Viewport 辅助方法 (ctx.centered_rt 等)
- ✅ App 自定义渲染支持
- ✅ 完整文档 (design.md 1500+ 行)

---

**归档人**: Claude Opus 4.5
**最后审核**: 2026-02-01
**OpenSpec 状态**: ✅ 功能完成，可归档
