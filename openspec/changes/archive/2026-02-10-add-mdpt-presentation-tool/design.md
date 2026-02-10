## Context

需要在 RustPixel 中新增一个基于 Markdown 的演示应用 mdpt。该应用作为 `apps/mdpt/` 独立存在，遵循标准 app 结构（`app!()` 宏），不集成进 cargo-pixel 工具链。充分利用 RustPixel 的图形模式能力（GPU 转场、文本动画、不依赖终端），同时保持终端模式可用。参照 presenterm 的 Markdown 解析和代码高亮方案，但渲染层完全替换为 RustPixel 引擎。

**约束条件：**
- 遵循标准 RustPixel app 结构（`app!()` 宏，model.rs + render_terminal.rs + render_graphics.rs）
- 通过 `cargo pixel r mdpt g` / `cargo pixel r mdpt t` 运行，与 snake、tetris 等 app 一致
- Markdown 解析需支持 front matter、标题分页、代码块、列表、表格等常用元素
- 图形模式下利用 BufferTransition 和 GpuTransition 实现幻灯片转场
- v1 支持 .pix/.ssf 图片嵌入（通过 asset2sprite 加载），预留 ImageProvider 扩展接口
- 支持 WASM/Web 部署（包含 wasm/ 目录，与其他 app 一致）

## Goals / Non-Goals

**Goals:**
- 解析标准 Markdown 文件为幻灯片序列（`---` 或 `# Heading` 分页）
- 支持代码块语法高亮（100+ 语言，通过 syntect）
- Slide 切换时自动应用转场效果（可配置类型）
- 标题支持文本动画效果（Typewriter、FadeIn 等）
- 支持 front matter 配置（主题、转场类型、动画等）
- 终端模式和图形模式均可运行（`cargo pixel r mdpt t` / `cargo pixel r mdpt g`）
- 支持 .pix/.ssf 图片和动画嵌入（通过 Markdown `![alt](path)` 语法）
- 预留 ImageProvider 扩展接口（后续对接 AI 像素图等）
- 支持 WASM/Web 部署（浏览器中运行演示）
- 支持 presenterm 风格的 Comment Commands 扩展标签：
  - `<!-- end_slide -->` 显式分页
  - `<!-- pause -->` 逐步展示
  - `<!-- column_layout: [w1, w2] -->` / `<!-- column: N -->` / `<!-- reset_layout -->` 多列布局
  - `<!-- jump_to_middle -->` 内容垂直居中
- 代码块支持 `+line_numbers` 标记显示行号

**Non-Goals:**
- v1 不实现 PNG/JPG 等通用图片渲染（仅支持 .pix/.ssf RustPixel 原生格式）
- 不实现实时 Markdown 编辑/热重载
- 不实现 PDF/HTML 导出
- 不实现演讲者备注模式
- 不实现代码执行（`+exec`）

## Decisions

### Decision 1: Markdown 解析库 —— comrak

**选择：** 使用 `comrak` 解析 Markdown

**理由：**
- presenterm 已验证其适用于演示场景
- 支持 CommonMark + 扩展（front matter、表格、删除线、脚注等）
- Rust 原生实现，无 FFI 依赖
- 成熟稳定，社区活跃

**替代方案：**
- `pulldown-cmark` → 更轻量但扩展支持不足（无 front matter、表格对齐信息不完整）
- `markdown-rs` → 相对较新，社区较小

### Decision 2: 代码高亮 —— syntect

**选择：** 使用 `syntect` 进行语法高亮

**理由：**
- 支持 100+ 编程语言的语法定义
- 内置 bat 的主题集（包括常用暗色/亮色主题）
- presenterm 已验证其稳定性
- 输出颜色可直接映射到 RustPixel 的 `Color::Rgba`

**替代方案：**
- `tree-sitter` → 更精准但引入 C 依赖，增加编译复杂度
- 自定义高亮 → 工作量过大，不必要

### Decision 3: Slide 模型架构

**选择：** `Markdown → Vec<SlideContent> → Vec<UIPage>` 三层架构

```
┌─────────────────────────────────────────────────────┐
│ Layer 1: Markdown Parsing (comrak)                  │
│   input.md → Vec<MarkdownElement>                   │
│   - 解析 front matter（主题/转场/动画配置）           │
│   - 按 `---` 或 `# Heading` 分割为 Slide            │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ Layer 2: Slide Model                                │
│   Vec<SlideContent>                                 │
│   SlideContent {                                    │
│     elements: Vec<SlideElement>,                    │
│     transition: Option<TransitionType>,             │
│     animation: Option<AnimationConfig>,             │
│   }                                                 │
│   SlideElement = Title | Paragraph | CodeBlock |    │
│     List | Table | Divider | Image | Pause |        │
│     ColumnLayout | Column | ResetLayout |           │
│     JumpToMiddle                                    │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ Layer 3: RustPixel Rendering                        │
│   每个 SlideContent → 一个 UIPage                    │
│   - Panel 作为容器（带边框/标题）                     │
│   - Label 渲染标题（带动画）                          │
│   - Label/直接 Buffer 渲染正文段落                    │
│   - 代码块：syntect 高亮 → set_color_str 逐行渲染    │
│   - 列表：缩进 + 符号前缀                            │
│   - 表格：Table widget                               │
│   - 图片：asset2sprite 加载 .pix/.ssf → Sprite 渲染  │
└─────────────────────────────────────────────────────┘
```

**理由：**
- 三层解耦：解析/模型/渲染各自独立，易于测试和扩展
- 复用 RustPixel 现有 UI 组件（Panel、Label、Table）
- SlideContent 作为中间表示，与渲染无关，便于后续支持多种输出格式

### Decision 4: 转场效果集成

**选择：** 复用 `ui_demo` 的多页转场模式（TransitionState + BufferTransition）

**数据流：**
```
用户按 →/← 键
    ↓
Model 触发 TransitionState::start(from, to, transition_type)
    ↓
每帧 TransitionState::update(dt) 推进 progress
    ↓
Render 调用 transition.transition(from_buf, to_buf, dst_buf, progress)
    ↓
图形模式下额外可用 GPU 转场：
  adapter.blend_rts(0, 1, 3, effect, progress)
```

**终端模式：** 仅使用 BufferTransition（CPU）
**图形模式：** 可选 BufferTransition 或 GpuTransition

**可用转场类型（16种）：**
- Buffer 级（9种）：WipeLeft/Right/Up/Down, SlideLeft/Right, Dissolve, BlindsH/V, Checkerboard, Typewriter
- GPU 级（7种）：Squares, Heart, Noise, RotateZoom, Bounce, Dispersion, Ripple

### Decision 5: Front Matter 配置

**选择：** YAML front matter 定义演示配置

```yaml
---
title: My Presentation
theme: dark          # dark / light / custom
transition: dissolve # wipe_left / slide_right / dissolve / checkerboard / ...
title_animation: typewriter  # typewriter / fade_in / wave / spotlight / none
code_theme: base16-ocean    # syntect 主题名
margin: 2            # 左右边距（字符数）
---
```

**理由：**
- 与 presenterm 约定一致，用户迁移成本低
- comrak 原生支持 front matter 解析
- YAML 格式简洁直观

### Decision 6: App 结构 —— 标准 app!() 宏

**选择：** 遵循标准 RustPixel app 结构（与 snake、tetris、ui_demo 一致）

```
apps/mdpt/
├── Cargo.toml           # 依赖 rust_pixel + comrak + syntect
├── build.rs             # cfg_aliases
├── src/
│   ├── lib.rs           # app!(Mdpt); 宏调用
│   ├── main.rs          # native 入口
│   ├── model.rs         # MdptModel: slides, navigation, transitions
│   ├── render_terminal.rs  # MdptRender (term)
│   ├── render_graphics.rs  # MdptRender (graphics)
│   ├── parser.rs        # Markdown → Vec<SlideContent>
│   ├── highlight.rs     # syntect 代码高亮
│   ├── slide.rs         # SlideContent, SlideElement 数据结构
│   └── image.rs         # 图片加载（.pix/.ssf）+ ImageProvider trait
├── wasm/                # WASM/Web 构建
│   ├── Cargo.toml       # wasm-bindgen + rust_pixel[web]
│   ├── Makefile          # wasm-pack build + http server
│   ├── src/lib.rs       # WASM 入口（复用 app src）
│   ├── index.html       # Web 页面（canvas）
│   └── index.js         # JS 桥接（资产加载、事件、游戏循环）
└── assets/              # 示例文件
    ├── demo.md          # 示例演示文稿
    └── pix/             # 示例 .pix/.ssf 图片资产
```

**运行方式：**
```bash
cargo pixel r mdpt t demo.md      # 终端模式，播放 demo.md
cargo pixel r mdpt g slides.md    # Glow 图形模式
cargo pixel r mdpt wg talk.md     # WGPU 图形模式
cargo pixel r mdpt w demo.md      # Web 模式（编译 WASM，启动 localhost:8080）
```
md 文件路径通过命令行参数传递（`cargo pixel r` 的 `other` 参数透传给应用）。
若未指定文件，默认加载 `assets/demo.md`。
Web 模式下 md 文件和 .pix/.ssf 资产需部署在 Web 服务器可访问路径下。

**理由：**
- 与 snake、tetris、ui_demo 等 app 结构完全一致
- 使用 `app!()` 宏自动生成入口点和 WASM 导出
- 不污染 cargo-pixel 工具链，保持独立性
- workspace glob `"apps/*"` 自动发现，无需手动注册

### Decision 7: 图片支持 —— .pix/.ssf 原生格式 + ImageProvider 扩展

**选择：** v1 实现 .pix 图片和 .ssf 动画的加载渲染，同时保留 ImageProvider trait 扩展接口

**Markdown 语法：**
```markdown
![Logo](assets/logo.pix)           <!-- 加载 .pix 静态图片 -->
![Animation](assets/dance.ssf)     <!-- 加载 .ssf 动画，自动播放帧 -->
```

**加载机制：** 使用 RustPixel 现有的 `asset2sprite!` 宏和 `AssetManager` 管道

```
Markdown ![alt](path.pix) 解析
    ↓
SlideElement::Image { path, alt, asset_type }
    ↓
渲染时创建 Sprite，调用 asset2sprite!(sprite, ctx, path)
    ↓
AssetManager::load() → PixAsset/SeqFrameAsset::parse()
    ↓
Buffer::blit() 将解析结果复制到 Sprite 内容
    ↓
.ssf 动画：每帧更新 frame_idx，调用 asset2sprite!(sprite, ctx, path, frame_idx)
```

**ImageProvider trait（扩展接口）：**
```rust
pub trait ImageProvider: Send {
    fn load_image(&self, path: &str, max_width: u16, max_height: u16)
        -> Option<ImageContent>;
}

pub struct ImageContent {
    pub buffer: Buffer,
    pub width: u16,
    pub height: u16,
}

/// v1 默认实现：加载 .pix/.ssf 文件
pub struct PixImageProvider;

/// 未识别格式：显示占位符 [Image: alt]
/// 后续可扩展为 PetsciiImageProvider / AiPixelImageProvider
```

**支持的图片格式：**
- `.pix` —— RustPixel PETSCII 静态图片（直接通过 AssetManager 加载）
- `.ssf` —— RustPixel PETSCII 动画序列（自动播放，帧率可配）
- 其他格式 —— 显示 `[Image: alt text]` 占位符

**理由：**
- 完全复用 RustPixel 现有的 AssetManager 和 asset2sprite 管道，零额外依赖
- .pix/.ssf 是 RustPixel 原生格式，所有渲染后端（Terminal/Glow/WGPU/Web）均已支持
- WASM 模式下 AssetManager 自动切换为异步加载（js_load_asset → on_asset_loaded）
- ImageProvider trait 保留扩展性，后续可通过 feature flag 添加 PNG/AI 支持

### Decision 8: WASM/Web 部署

**选择：** 包含 `wasm/` 目录，支持编译为 WebAssembly 在浏览器中运行

**架构：** 与 petview、tower 等 app 的 WASM 支持方式完全一致

```
┌─────────────────────────────────────────────────────┐
│ JavaScript Layer (index.js)                         │
│   1. 加载 symbols.png 纹理 + symbol_map.json        │
│   2. 调用 wasm_init_pixel_assets() 初始化           │
│   3. 创建 MdptGame.new()，调用 init_from_cache()    │
│   4. 事件转发：keyboard/mouse → key_event()         │
│   5. 游戏循环：requestAnimationFrame → tick()       │
│   6. 资产桥接：js_load_asset() → on_asset_loaded()  │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│ Rust/WASM Layer                                     │
│   app!(Mdpt) 宏自动生成 WASM 导出：                  │
│   - new() / tick() / key_event() / init_from_cache()│
│   - on_asset_loaded() / wasm_init_pixel_assets()    │
│   WebAdapter 使用 WebGL2 渲染                       │
└─────────────────────────────────────────────────────┘
```

**Web 模式下的特殊处理：**
- md 文件通过 JavaScript fetch 加载，传递给 Rust 侧解析
- .pix/.ssf 资产通过 `js_load_asset()` 异步加载，AssetManager 自动处理加载状态
- `syntect` 在 WASM 中可正常工作（纯 Rust 实现，无 FFI）
- `comrak` 在 WASM 中可正常工作（纯 Rust 实现）

**Cargo.toml (wasm/)：**
```toml
[dependencies]
rust_pixel = { path = "../../../", default-features = false, features = ["web"] }
comrak = { version = "0.x", default-features = false }
syntect = { version = "5.x", default-features = false, features = ["default-fancy"] }
wasm-bindgen = "0.2"
web-sys = "0.3"
wasm-logger = "0.2"
```

**理由：**
- RustPixel 已有成熟的 WASM 支持（WebAdapter + WebGL2 + 异步资产加载）
- app!() 宏自动生成所有 WASM 导出函数，无需额外代码
- comrak 和 syntect 均为纯 Rust 实现，WASM 兼容性好
- 浏览器中运行演示是非常实用的场景（分享、在线预览）

### Decision 9: Comment Commands —— presenterm 风格扩展标签

**选择：** 通过 HTML 注释实现演示控制命令，兼容 presenterm 语法约定

**支持的 Comment Commands：**

| 命令 | 语法 | 说明 |
|------|------|------|
| 显式分页 | `<!-- end_slide -->` | 替代 `---` 的显式幻灯片分隔符 |
| 暂停展示 | `<!-- pause -->` | 按键后才显示后续内容（逐步展示） |
| 定义列布局 | `<!-- column_layout: [1, 2] -->` | 定义列宽比例 |
| 切换列 | `<!-- column: 0 -->` | 切换到指定列写入内容 |
| 重置布局 | `<!-- reset_layout -->` | 退出列布局，恢复全宽 |
| 垂直居中 | `<!-- jump_to_middle -->` | 后续内容垂直居中显示 |

**代码块扩展标记：**

| 标记 | 语法 | 说明 |
|------|------|------|
| 行号 | `` ```rust +line_numbers `` | 代码块显示行号 |

**Pause 实现机制：**
```
SlideContent {
  elements: [Title, Paragraph, Pause, CodeBlock, Pause, List]
}
    ↓ 解析为 3 个 "步骤"（steps）
Step 0: [Title, Paragraph]          ← 初始显示
Step 1: [Title, Paragraph, CodeBlock]  ← 按键后追加
Step 2: [Title, Paragraph, CodeBlock, List] ← 再按键追加
```
- Model 中维护 `current_step: usize`，每次 → 键先推进 step，step 到末尾才切换 slide
- 渲染时仅渲染 `elements[0..pause_boundary[current_step]]`

**Column Layout 实现机制：**
```
<!-- column_layout: [1, 2] -->   → ColumnLayout { widths: [1, 2] }
<!-- column: 0 -->               → Column(0)
内容A...                          → 渲染到第 0 列区域
<!-- column: 1 -->               → Column(1)
内容B...                          → 渲染到第 1 列区域
<!-- reset_layout -->            → ResetLayout
后续内容...                        → 恢复全宽渲染
```
- 渲染时根据 `widths` 比例计算每列的像素/字符宽度
- 列内容区域独立维护 y 偏移

**理由：**
- 与 presenterm 语法兼容，降低用户迁移成本
- HTML 注释是合法的 Markdown，不影响其他渲染器显示
- Pause 是演示工具的核心交互，必须支持
- Column Layout 是图文并排的基础需求
- 仅选择最高频的 5 个命令，保持实现简洁

**v1 不支持的 presenterm 命令（后续可扩展）：**
- `<!-- incremental_lists: true -->` — 列表逐项展示
- `<!-- speaker_note: ... -->` — 演讲者备注
- `<!-- include: file.md -->` — 外部文件包含
- `<!-- alignment: center -->` — 文本对齐
- `<!-- newlines: N -->` — 显式空行
- 代码块 `+exec` — 执行代码片段
- 代码块 `{1-4|6-10|all}` — 动态高亮切换

## Risks / Trade-offs

### Risk 1: comrak + syntect 增加编译时间
**缓解：** 仅 apps/mdpt 依赖这两个库，不影响核心 rust_pixel 编译

### Risk 2: 终端模式下动画效果有限
**缓解：** 终端模式下降级为无动画静态渲染，转场使用 BufferTransition（CPU 级别已足够流畅）

### Risk 3: 大型 Markdown 文件解析性能
**缓解：** 一次性解析到内存，Slide 数量通常 < 100 页，内存和解析时间不是问题

### Risk 4: WASM 下 syntect/comrak 包大小
**缓解：** 使用 `default-features = false` 精简依赖；syntect 可选择内嵌较少的语法定义以减小 WASM 体积

### Risk 5: WASM 下 .pix/.ssf 资产异步加载
**缓解：** RustPixel 的 AssetManager 已内置 WASM 异步加载支持（js_load_asset → on_asset_loaded），asset2sprite 宏自动处理加载状态轮询

## Open Questions

1. **Slide 分页规则** —— 同时支持 `---` 和 `<!-- end_slide -->`，是否还需支持 `# Heading` 自动分页？
   - **建议：** v1 支持 `---` 和 `<!-- end_slide -->` 两种分页方式，暂不支持 `# Heading` 自动分页
2. **主题系统** —— 是否需要自定义主题文件？
   - **建议：** v1 内置 dark/light 两个主题（硬编码颜色方案），后续可支持 YAML 主题文件
