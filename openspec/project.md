# Project Context

## Purpose
RustPixel 是一个 **Tile-first, Retro-ready** 的 2D 游戏引擎，核心理念：

1. **Everything is Tiles** — Scene > Layer > Sprite > Buffer > Cell 统一渲染抽象，单纹理单 draw call 高性能渲染
2. **Write Once, Run Anywhere** — 同一套代码可运行于 Terminal、Desktop (WGPU)、Web (WebGL2/WebGPU)
3. **TUI in GPU Windows** — 在原生 GPU 窗口中渲染完整 TUI，无需终端模拟器

**杀手级应用 MDPT**：Markdown-first 演示工具，在原生窗口中渲染字符 UI，支持 GPU shader 转场、代码高亮、图表、CJK 文本等。

**内置 BASIC 解释器**：pixel_basic 提供经典 BASIC 语法，快速原型与入门学习。

参考：`README.md`、`doc/architecture.md`、`doc/faq.md`。

## Tech Stack
- **Language & Build**: Rust 1.71+，Cargo workspace
- **Render Backends**（两套后端，统一 Adapter trait）:
  - `CrosstermAdapter`：终端模式（ASCII/Unicode/Box/Braille/Emoji/CJK）
  - `WinitWgpuAdapter`：桌面 GPU 模式（Vulkan/Metal/DX12）
  - `WgpuWebAdapter`：Web 模式（WebGL2/WebGPU，通过 wasm32 target 自动启用）
- **Feature Flags**:
  - `term`：终端模式（crossterm）
  - `wgpu`：桌面 GPU 模式（wgpu + winit）
  - `web`：Web 模式（自动检测 wasm32，使用 wgpu）
- **Assets & Tools**: `.pix`（PETSCII 图片）、`.ssf`（序列帧动画）、`.esc`（终端转义序列）；FFmpeg 用于 GIF → SSF 转换
- **CLI**: `cargo-pixel`（创建/运行/打包/资源处理一体化）
- **Demos**: `apps/*`（mdpt、tetris、snake、tower、petview、poker、palette、basic_snake 等）

## Project Conventions

### Code Style
- 模块清晰、命名语义化：类型用名词，函数用动词短语；避免 1-2 字母变量名
- 明确所有权边界，优先借用；避免无谓 `clone()`；热路径避免临时分配
- 公共 API 明确错误返回类型；在 CLI/WASM 边界用 `anyhow` 兜底（讲座建议）
- 条件编译集中在模块边界，避免在业务逻辑散布复杂 `#[cfg]`

### Architecture Patterns
- **Game Loop 三层**：`Model`（状态/逻辑）、`Render`（渲染）、`Game`（调度）
- **Adapter 模式**：统一 `Adapter` trait，按 Feature/平台选择 `CrosstermAdapter`、`WinitWgpuAdapter`、`WgpuWebAdapter`
- **渲染层级**：Scene > Layer (render_weight 排序) > Sprite > Buffer > Cell
- **Cell/Glyph 系统**：
  - Cell: symbol + fg + bg + modifier + scale
  - Glyph: block + idx + width + height（缓存在 Cell 中，避免渲染时字符串解析）
  - PUA 编码：Sprite 使用 U+F0000~U+F9FFF（160 blocks × 256 symbols）
- **GPU 4-Stage Pipeline**：
  1. Data → RenderBuffer（Buffer + Layers → Vec\<RenderCell\>）
  2. RenderBuffer → RenderTexture
  3. RT Operations（过渡效果混合）
  4. RT → Screen（final composition）
- **统一纹理图集**（4096×4096 或 8192×8192）：
  - Block 0-159: Sprite glyphs（PETSCII/ASCII/custom）
  - Block 160-169: TUI characters
  - Block 170-175: Emoji
  - Block 176-239: CJK characters
  - 单纹理绑定，单 draw call，零纹理切换
- **UI 框架**：Widget trait + Layout (Free/VBox/HBox) + UIPage（多页面 + 转场）
- **资产系统**：AssetManager 统一加载 `.pix`/`.esc`/`.ssf`；桌面同步读取，Web 异步 fetch

### Testing Strategy
- 示例驱动：以 `apps/*` 作为端到端案例与回归
- 烟雾测试：`cargo pixel r <app> <mode>`（mode: t/wg/w）验证后端矩阵
- 特性矩阵：本地验证 `term/wgpu/web` 组合；Web 用 localhost:8080 预览
- 算法/纯逻辑模块可添加单测；渲染路径以可视回归为主

### Git Workflow
- 分支：`main` 为稳定分支，功能分支采用简洁语义名（如 `feat/ui-tabs`）
- 提交：建议使用 Conventional Commits（`feat:`, `fix:`, `refactor:` 等）
- 变更提案：较大功能/架构/性能改动优先以 `openspec/changes/*` 提案评审，`openspec validate --strict` 校验后实施

## Domain Context
- **典型场景**：2D 像素风格游戏、Markdown 演示工具、终端工具、字符画浏览器/编辑器、教学/演示
- **资源生态**：PETSCII/ASCII 字符集、`.pix` 图片、`.ssf` 序列帧动画、`.esc` 终端转义序列
- **工具链**：
  - `cargo-pixel`：一键创建/运行/打包（`cargo pixel r <app> <mode> [-r]`）
  - `palette`：颜色生成/分析/转换（终端 UI）
  - `edit`：字符/像素编辑器（终端/图形）
  - `petii`：普通图片 → PETSCII 转换（`cargo pixel p`）
  - `ssf`：GIF → SSF 动画转换与预览（`cargo pixel cg`）
- **Demo Apps**：
  - `mdpt`：Markdown 演示工具（GPU 转场、代码高亮、图表）
  - `tetris`：双人俄罗斯方块（AI 对战）
  - `snake`：PETSCII 动画贪吃蛇
  - `tower`：塔防游戏
  - `petview`：PETSCII 画廊（2000+ 作品，Matrix rain，屏保模式）
  - `poker`/`gin_rummy`：扑克算法（含 FFI/WASM）
  - `basic_snake`：BASIC 语言写的贪吃蛇
  - `palette`：颜色工具

## Important Constraints
- **跨平台**：macOS/Linux/Windows(WSL+Native)/Web；按需启用后端 feature
- **双渲染模式**：终端（crossterm）与图形（wgpu）；保持 Adapter trait 统一
- **统一纹理架构**：单 4096×4096（或 8192×8192）纹理，256 blocks，单 draw call
- **兼容性**：新增能力增量化，避免破坏公共 API
- **性能目标**：60 FPS；实例化渲染 + 双缓冲 diff
- **MSDF/SDF 字体**：高 DPI 下使用距离场渲染保持文字清晰

## External Dependencies
- **渲染与平台**：
  - 终端：`crossterm`
  - GPU：`wgpu`、`winit`
  - Web：`wasm-bindgen`、`web-sys`
- **媒体与工具**：`ffmpeg`（GIF→SSF 工具链）、`image`
- **CLI**：`clap`（`cargo-pixel` 内部）、`cfg_aliases`（Feature 别名）
- **BASIC 解释器**：`pixel_basic`（内置，无外部依赖）

## Running Commands
```bash
cargo pixel r <app> t      # Terminal mode
cargo pixel r <app> wg     # WGPU desktop mode
cargo pixel r <app> w      # Web mode (localhost:8080)
cargo pixel r <app> wg -r  # Release build
cargo pixel c <name>       # Create new app in apps/
cargo pixel c <name> ..    # Create standalone project
```

## Source Layout
```
src/
├── game.rs                    # Game loop, Model/Render traits
├── context.rs                 # Shared runtime state
├── macros.rs                  # app! macro
├── render/
│   ├── adapter.rs             # Adapter trait
│   ├── adapter/               # CrosstermAdapter, WinitWgpuAdapter, WgpuWebAdapter
│   ├── buffer.rs              # Cell buffer
│   ├── cell.rs                # Cell + Glyph (PUA encoding)
│   ├── scene.rs               # Scene container
│   └── sprite/                # Sprite + Layer
├── ui/                        # Widget framework
├── asset.rs                   # Asset loading
└── util/                      # Rect, math, particle system
apps/                          # Demo applications
pixel_basic/                   # BASIC interpreter
```
