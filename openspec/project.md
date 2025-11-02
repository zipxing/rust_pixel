# Project Context

## Purpose
RustPixel 是一个面向 2D 的游戏引擎与快速原型工具集，同时内置一套简单实用的字符/像素 UI 框架。它支持文本模式与图形模式两类渲染路径，可编译为桌面二进制、FFI 动态库，以及 WebAssembly 在浏览器中运行。项目目标：
- 提供统一的 Model/Render/Game 架构与渲染适配层，覆盖 Terminal、SDL/OpenGL、Winit/Glow、Winit/WGPU 与 WebGL。
- 提供可复用的 UI 组件、布局与主题系统，快速构建编辑器、浏览器类工具与游戏内 UI。
- 提供围绕字符画/PETSCII 的资产处理与开发工具，降低入门门槛与工程化成本。

参考：`README.md`、`README_UI_FRAMEWORK.md`、`doc/principle.md`、`rust_pixel_tech_lecture.md`。

## Tech Stack
- Language & Build: Rust 1.71+，Cargo workspace
- Render Backends:
  - Text: crossterm（终端渲染，ASCII/Unicode）
  - Graphics: SDL2 + Glow（OpenGL）、Winit + Glow、Winit + WGPU（Vulkan/Metal/DX12 抽象）
  - Web: WASM + WebGL；`wasm-bindgen`/`web-sys`
- Assets & Tools: 自研 `.pix`/`.txt(ESC)`/`.ssf` 资源格式与工具链（`tools/*`）；FFmpeg 用于 GIF → SSF 转换
- CLI: `cargo-pixel`（创建/运行/打包/资源处理一体化）
- Docs & Demos: `apps/*` 多示例，`README_UI_FRAMEWORK.md` UI 教程

## Project Conventions

### Code Style
- 模块清晰、命名语义化：类型用名词，函数用动词短语；避免 1-2 字母变量名
- 明确所有权边界，优先借用；避免无谓 `clone()`；热路径避免临时分配
- 公共 API 明确错误返回类型；在 CLI/WASM 边界用 `anyhow` 兜底（讲座建议）
- 条件编译集中在模块边界，避免在业务逻辑散布复杂 `#[cfg]`

### Architecture Patterns
- Game Loop 三层：`Model`（状态/逻辑）、`Render`（渲染）、`Game`（调度）
- Adapter 模式：统一 `Adapter` trait，按 Feature/平台选择 `CrosstermAdapter`、`SdlAdapter`、`WinitGlowAdapter`、`WinitWgpuAdapter`、`WebAdapter`
- UI 框架：`Widget` 接口 + 布局（线性/网格/自由）+ 主题系统（暗/亮/终端主题与状态样式）
- 资产系统：`AssetManager` 统一加载/解析/缓存 `.pix`/`.esc`/`.ssf`，桌面直读文件，Web 端异步拉取
- 渲染优化：双缓冲 diff 更新；图形模式使用实例化渲染、纹理图集、过渡与通用 2D 着色器；WGPU 管线可选

### Testing Strategy
- 示例驱动：以 `apps/*` 作为端到端案例与回归；`test/` 目录用于图形/算法验证
- 最小烟雾测试：通过 `cargo pixel r <app> <backend>` 跑一帧或短时运行验证后端矩阵
- 特性矩阵：在本地按需验证 `term/sdl/winit/wgpu/web` 组合；Web 用本地静态服务预览
- 算法/纯逻辑模块可添加单测；渲染路径以可视回归为主

### Git Workflow
- 分支：`main` 为稳定分支，功能分支采用简洁语义名（如 `feat/ui-tabs`）
- 提交：建议使用 Conventional Commits（`feat:`, `fix:`, `refactor:` 等）
- 变更提案：较大功能/架构/性能改动优先以 `openspec/changes/*` 提案评审，`openspec validate --strict` 校验后实施

## Domain Context
- 典型场景：2D 像素风格游戏、终端工具、字符画浏览器/编辑器、教学/演示
- 资源生态：PETSCII/ASCII 字符集、`assets/pix/*` 图集、`.ssf` 序列帧动画
- 工具链：
  - `palette`：颜色生成/分析/转换（终端 UI）
  - `edit`：字符/像素编辑器（终端/图形）
  - `petii`：普通图片 → PETSCII 转换
  - `ssf`：GIF → SSF 动画转换与预览
  - `cargo-pixel`：一键创建/运行/打包
- Demo：`apps/snake`、`apps/tetris`、`apps/tower`、`apps/poker`（含 FFI/WASM）、`apps/palette`、`apps/ui_demo`

## Important Constraints
- 跨平台：macOS/Linux/Windows(WSL)/Web；依赖矩阵尽量最小化（按需启用后端）
- 双渲染模式并存：文本/图形；保持 API 统一与可切换性
- 兼容性：新增能力尽量增量化，避免破坏现有公共 API（OpenSpec 约束）
- 性能目标：在参考设备上 60 FPS；长列表/大量 Cell 使用增量/实例化策略
- 工程约束：不在主工程引入重量级框架；优先“薄抽象 + 简单实现”

## External Dependencies
- 渲染与平台：`crossterm`、`sdl2`/`sdl2_image`/`sdl2_gfx`、`winit`、`glow`、`wgpu`、`wasm-bindgen`/`web-sys`
- 媒体与工具：`ffmpeg`（GIF→SSF 工具链）、`image`
- CLI：`clap`（`cargo-pixel` 内部）、`cfg_aliases`（统一 Feature 别名，经由 `build_support.rs`）

