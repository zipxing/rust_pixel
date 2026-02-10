## Why

RustPixel 目前缺少一个演示/幻灯片应用。社区中已有 presenterm 等终端演示工具，但它们完全依赖终端环境，无法利用 GPU 加速的页面转场效果和丰富的文本动画。RustPixel 的图形模式恰好提供了：
1. **不依赖终端的 TUI 渲染** —— 图形模式下内置字符渲染引擎，脱离终端限制
2. **丰富的页面转场** —— 9 种 Buffer 级转场 + 7 种 GPU 着色器转场
3. **丰富的文本动画** —— Label 组件支持 Spotlight/Wave/FadeIn/Typewriter 四种动画
4. **跨平台** —— 同一套代码可运行在 Terminal、Desktop (Glow/WGPU)、Web (WASM)
5. **原生资产支持** —— .pix/.ssf 格式的 PETSCII 图片和动画可直接嵌入演示

mdpt (Markdown Presentation Tool) 作为一个独立 app，将这些能力整合为一个开箱即用的演示应用。

## What Changes

- **新增 `apps/mdpt/`** —— 基于 RustPixel 的 Markdown 演示应用，遵循标准 app 结构（`app!()` 宏）
  - Markdown 解析（comrak）→ Slide 模型 → RustPixel UIPage 渲染
  - 代码块语法高亮（syntect）
  - Slide 间自动应用 BufferTransition / GpuTransition 转场
  - 标题/正文支持 Label 文本动画（可通过 front matter 配置）
  - 支持 presenterm 风格的 Comment Commands（`<!-- pause -->`、`<!-- column_layout -->`、`<!-- end_slide -->`、`<!-- jump_to_middle -->`）
  - 代码块支持 `+line_numbers` 行号显示
  - 支持 .pix/.ssf 图片和动画嵌入（通过 Markdown `![alt](path.pix)` 语法，使用 RustPixel asset2sprite 加载）
  - 预留 ImageProvider trait 扩展接口，后续可对接 AI 像素图生成
- **WASM/Web 支持** —— 包含 `wasm/` 目录，可编译为 WebAssembly 在浏览器中运行演示
- **新增依赖** —— `comrak`（Markdown 解析）、`syntect`（语法高亮）添加到 `apps/mdpt/Cargo.toml`
- **不修改 cargo-pixel 工具链** —— mdpt 作为普通 app，通过 `cargo pixel r mdpt g` 运行

## Impact

- Affected specs: 新增 `mdpt-presentation` capability
- Affected code:
  - `apps/mdpt/` —— **CREATE** 全新 app 目录
  - `Cargo.toml`（workspace root）—— 自动包含（`"apps/*"` glob 已覆盖）
- Compatibility: 纯新增功能，不影响现有代码
- Migration: 无需迁移
