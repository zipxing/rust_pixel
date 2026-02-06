# UIPage 渲染架构文档

本文档描述 RustPixel UI 框架中 UIPage 的设计以及文本模式/图形模式的渲染流程。

---

## 1. UIPage 架构

UIPage 是 UI 框架的核心容器，每个页面包含独立的 widget 树和内置 Buffer，支持多页面转场。

```
┌─────────────────────────────────────────────────────────────┐
│                         UIPage                               │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐                                    │
│  │    root_widget      │  ← Widget 树 (Panel, Label, etc.)  │
│  │    ├─ Panel         │                                    │
│  │    │  ├─ Label      │                                    │
│  │    │  ├─ Button     │                                    │
│  │    │  └─ List       │                                    │
│  │    └─ ...           │                                    │
│  └─────────────────────┘                                    │
│                                                              │
│  ┌─────────────────────┐                                    │
│  │      buffer         │  ← 内置 Buffer (width × height)    │
│  │  ┌─────────────────┐│                                    │
│  │  │ Cell Cell Cell  ││                                    │
│  │  │ Cell Cell Cell  ││                                    │
│  │  │ Cell Cell Cell  ││                                    │
│  │  └─────────────────┘│                                    │
│  └─────────────────────┘                                    │
│                                                              │
│  event_dispatcher    ← 事件分发                              │
│  theme_manager       ← 主题管理                              │
└─────────────────────────────────────────────────────────────┘
```

### 核心方法

| 方法 | 说明 |
|------|------|
| `render()` | 渲染 widgets 到内置 buffer |
| `render_into(target)` | 渲染 widgets 到指定 buffer (零拷贝) |
| `buffer()` | 获取内置 buffer 的不可变引用 |
| `buffer_mut()` | 获取内置 buffer 的可变引用 |

---

## 2. 文本模式渲染流程

### 2.1 单页面渲染 (无转场)

```
┌─────────────────────────────────────────────────────────────┐
│                    Text Mode - Single Page                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────┐                                           │
│   │   UIPage    │                                           │
│   │  (widgets)  │                                           │
│   └──────┬──────┘                                           │
│          │ render_into()                                    │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ tui_buffer  │  ← Scene 的 TUI buffer                    │
│   └──────┬──────┘                                           │
│          │ diff (前后帧比较)                                 │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │  Terminal   │  ← crossterm 输出                         │
│   └─────────────┘                                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 多页面转场渲染

```
┌─────────────────────────────────────────────────────────────┐
│                Text Mode - Multi-Page Transition             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │  UIPage A   │     │  UIPage B   │                       │
│   │  (widgets)  │     │  (widgets)  │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │ render()          │ render()                     │
│          ▼                   ▼                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │  buffer_A   │     │  buffer_B   │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │                   │                              │
│          └─────────┬─────────┘                              │
│                    ▼                                        │
│          ┌─────────────────────┐                            │
│          │  BufferTransition   │  ← progress (0.0~1.0)      │
│          │  (Wipe/Slide/...)   │                            │
│          └──────────┬──────────┘                            │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │    output_buffer    │                            │
│          └──────────┬──────────┘                            │
│                     │ merge()                               │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │     tui_buffer      │                            │
│          └──────────┬──────────┘                            │
│                     │ diff                                  │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │      Terminal       │                            │
│          └─────────────────────┘                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 文本模式流程总结

```
Step 1: Widget 渲染
   UIPage.render() → 将 widget 树渲染到内置 buffer

Step 2: 转场混合 (可选)
   BufferTransition.transition(from, to, dst, progress)
   - from: 源页面 buffer
   - to:   目标页面 buffer
   - dst:  输出 buffer
   - progress: 0.0 (全显示from) → 1.0 (全显示to)

Step 3: 输出到 Scene
   tui_buffer.merge(source_buffer, 255, true)

Step 4: 差分渲染
   Scene.draw() → 只更新变化的字符到终端
```

---

## 3. 图形模式渲染流程

### 3.1 单页面渲染 (无转场)

```
┌─────────────────────────────────────────────────────────────┐
│                  Graphics Mode - Single Page                 │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │   UIPage    │     │   Sprites   │                       │
│   │  (widgets)  │     │  (游戏对象) │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │ render_into()     │                              │
│          ▼                   │                              │
│   ┌─────────────┐            │                              │
│   │ tui_buffer  │            │                              │
│   └──────┬──────┘            │                              │
│          │                   │                              │
│          └─────────┬─────────┘                              │
│                    ▼                                        │
│          ┌─────────────────────┐                            │
│          │    RenderBuffer     │  ← 合并 TUI + Sprites      │
│          └──────────┬──────────┘                            │
│                     │ draw_render_buffer_to_texture()       │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │    RT2 (主场景)     │                            │
│          └──────────┬──────────┘                            │
│                     │ present_default()                     │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │       Screen        │                            │
│          └─────────────────────┘                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 多页面转场渲染 (CPU BufferTransition)

```
┌─────────────────────────────────────────────────────────────┐
│           Graphics Mode - CPU BufferTransition               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │  UIPage A   │     │  UIPage B   │                       │
│   │  (widgets)  │     │  (widgets)  │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │ render()          │ render()                     │
│          ▼                   ▼                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │  buffer_A   │     │  buffer_B   │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │                   │                              │
│          └─────────┬─────────┘                              │
│                    ▼                                        │
│          ┌─────────────────────┐                            │
│          │  BufferTransition   │  ← 字符级混合              │
│          └──────────┬──────────┘                            │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │    output_buffer    │                            │
│          └──────────┬──────────┘                            │
│                     │ merge → tui_buffer                    │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │    RenderBuffer     │                            │
│          └──────────┬──────────┘                            │
│                     │                                       │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │   RT2 → Screen      │                            │
│          └─────────────────────┘                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 GPU 转场渲染 (GpuTransition)

```
┌─────────────────────────────────────────────────────────────┐
│              Graphics Mode - GPU Transition                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │  UIPage A   │     │  UIPage B   │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │                   │                              │
│          ▼                   ▼                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │ tui_buffer  │     │ tui_buffer  │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │                   │                              │
│          ▼                   ▼                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │ RenderBuf A │     │ RenderBuf B │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │                   │                              │
│          ▼                   ▼                              │
│   ┌─────────────┐     ┌─────────────┐                       │
│   │    RT0      │     │    RT1      │                       │
│   └──────┬──────┘     └──────┬──────┘                       │
│          │                   │                              │
│          └─────────┬─────────┘                              │
│                    ▼                                        │
│          ┌─────────────────────┐                            │
│          │    GpuTransition    │  ← GLSL Shader             │
│          │  blend_rts(0,1,3)   │  ← 像素级混合              │
│          └──────────┬──────────┘                            │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │    RT3 (结果)       │                            │
│          └──────────┬──────────┘                            │
│                     │ present()                             │
│                     ▼                                       │
│          ┌─────────────────────┐                            │
│          │       Screen        │                            │
│          └─────────────────────┘                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.4 RenderTexture (RT) 分配

| RT | 用途 |
|----|------|
| RT0 | 转场源图像 1 |
| RT1 | 转场源图像 2 |
| RT2 | 主场景内容 (TUI + Sprites) |
| RT3 | 叠加层 / 转场结果 |

---

## 4. BufferTransition 特效类型

### 4.1 字符级转场 (CPU)

| 类型 | 说明 | 示意 |
|------|------|------|
| WipeLeft/Right/Up/Down | 擦除转场 | `[BBBB\|AAAA]` |
| SlideLeft/Right/Up/Down | 滑动转场 | `[AAA→\|←BBB]` |
| Dissolve | 随机溶解 | `A B A B B A` |
| Typewriter | 打字机效果 | `BBBB▌AAAA` |
| BlindsH/V | 百叶窗 | 逐条显示 |
| Checkerboard | 棋盘格 | 交替方块 |

### 4.2 像素级转场 (GPU)

| 类型 | 说明 |
|------|------|
| Squares | 方格渐变 |
| Heart | 心形展开 |
| Noise | 噪点过渡 |
| RotateZoom | 旋转缩放 |
| Bounce | 弹跳波浪 |
| Dispersion | 色散分离 |
| Ripple | 涟漪扩散 |

---

## 5. 代码示例

### 5.1 创建多页面应用

```rust
// 创建多个 UIPage
let mut page1 = UIPage::new(80, 30);
page1.set_root_widget(Box::new(create_page1_widgets()));
page1.start();

let mut page2 = UIPage::new(80, 30);
page2.set_root_widget(Box::new(create_page2_widgets()));
page2.start();
```

### 5.2 执行转场

```rust
// 渲染两个页面到各自 buffer
page1.render();
page2.render();

// 创建转场特效
let transition = TransitionType::SlideLeft.create();

// 执行转场混合
transition.transition(
    page1.buffer(),  // from
    page2.buffer(),  // to
    &mut output_buffer,
    0.5  // 50% 进度
);

// 输出到 tui_buffer
tui_buffer.merge(&output_buffer, 255, true);
```

### 5.3 在 Render 中使用

```rust
fn update(&mut self, ctx: &mut Context, model: &mut Model, _dt: f32) {
    // 获取当前渲染结果 (单页或转场混合)
    let source_buffer = model.get_rendered_buffer();

    // 合并到 TUI buffer
    let tui_buffer = self.scene.tui_buffer_mut();
    tui_buffer.reset();
    tui_buffer.merge(source_buffer, 255, true);

    // 输出到屏幕
    self.scene.draw(ctx);
}
```

---

## 6. 选择指南

| 场景 | 推荐方案 |
|------|----------|
| TUI 页面切换 | BufferTransition (CPU) |
| 游戏场景转场 | GpuTransition (GPU Shader) |
| 简单应用 | 单 UIPage + render_into() |
| 多页向导/PPT | 多 UIPage + BufferTransition |
| 需要华丽效果 | GPU Transition (Heart, Ripple...) |

---

## 7. 性能考虑

- **BufferTransition**: 在 CPU 执行，适合小尺寸 buffer (< 200×60)
- **GpuTransition**: 在 GPU 执行，适合大分辨率或需要复杂特效
- **render_into()**: 零拷贝直接渲染，单页面时性能最优
- **merge()**: 需要一次内存拷贝，多页面转场时必需
