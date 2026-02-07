## 1. 基础结构搭建

- [ ] 1.1 创建 `apps/mdpt/` 目录结构（Cargo.toml、build.rs、src/lib.rs、src/main.rs）
- [ ] 1.2 配置 Cargo.toml：依赖 rust_pixel + comrak + syntect + serde + serde_yaml
- [ ] 1.3 在 lib.rs 中调用 `app!(Mdpt);` 宏
- [ ] 1.4 创建空的 Model/Render 骨架（MdptModel、MdptRender），确保 `cargo pixel r mdpt t` 能编译运行
- [ ] 1.5 实现命令行参数解析：从 `std::env::args()` 获取 .md 文件路径，未指定时默认加载 `assets/demo.md`

## 2. Markdown 解析

- [ ] 2.1 实现 `src/slide.rs`：定义 SlideContent、SlideElement 枚举（Title/Paragraph/CodeBlock/List/Table/Divider/Image/Pause/ColumnLayout/Column/ResetLayout/JumpToMiddle）
- [ ] 2.2 实现 `src/parser.rs`：使用 comrak 解析 Markdown，按 `---` 和 `<!-- end_slide -->` 分割为 Vec<SlideContent>
- [ ] 2.3 实现 front matter 解析：提取 title、theme、transition、title_animation、code_theme、margin 配置
- [ ] 2.4 实现 Comment Commands 解析：识别 HTML 注释中的 `end_slide`、`pause`、`column_layout`、`column`、`reset_layout`、`jump_to_middle` 命令
- [ ] 2.5 实现代码块扩展标记解析：识别 `+line_numbers` 标记，存储到 CodeBlock 的 `show_line_numbers` 字段
- [ ] 2.6 创建 `apps/mdpt/assets/demo.md` 示例文件，包含标题、段落、代码块、列表、表格、pause、column_layout

## 3. 代码高亮

- [ ] 3.1 实现 `src/highlight.rs`：封装 syntect HighlightLines，将代码字符串转换为 Vec<(Style, String)> 带颜色文本行
- [ ] 3.2 实现 syntect Style → RustPixel Color::Rgba 转换
- [ ] 3.3 支持通过 front matter `code_theme` 选择高亮主题

## 4. 渲染引擎

- [ ] 4.1 实现 `render_terminal.rs`：将 SlideContent 渲染到 Scene 的 tui_buffer
  - 标题：居中，高亮颜色
  - 段落：左对齐，自动换行
  - 代码块：带边框 Panel，逐行 set_color_str（高亮颜色），支持 `+line_numbers` 行号显示
  - 列表：缩进 + `- ` / `1. ` 前缀
  - 表格：使用 Table widget
  - 分隔线：`─` 填充一行
  - Pause：根据 current_step 控制渲染的元素范围
  - ColumnLayout：按 widths 比例分割渲染区域，列内独立 y 偏移
  - JumpToMiddle：计算剩余内容高度，垂直居中定位
- [ ] 4.2 实现 `render_graphics.rs`：与终端模式共享渲染逻辑（通过 tui_buffer）
- [ ] 4.3 实现状态栏：底部显示 `[slide_index / total_slides]` 和导航提示
- [ ] 4.4 实现 dark/light 主题配色（标题色、正文色、代码背景色、边框色）

## 5. 导航与转场

- [ ] 5.1 实现 Model::handle_input：← / → / PageUp / PageDown / Home / End / q 键盘导航（→ 键先推进 pause step，step 到末尾才切换 slide）
- [ ] 5.2 实现 TransitionState 管理（复用 ui_demo 的 TransitionState 模式）
- [ ] 5.3 集成 BufferTransition：Slide 切换时自动应用配置的转场效果
- [ ] 5.4 图形模式下集成 GpuTransition（通过 blend_rts）
- [ ] 5.5 支持通过 front matter 配置转场类型，默认为 Dissolve

## 6. 文本动画

- [ ] 6.1 标题渲染使用 Label widget + 配置的动画效果（Typewriter/FadeIn/Wave/Spotlight）
- [ ] 6.2 支持通过 front matter `title_animation` 配置动画类型
- [ ] 6.3 每次切换到新 Slide 时重置动画状态

## 7. 图片支持

- [ ] 7.1 实现 `src/image.rs`：定义 ImageProvider trait、ImageContent 结构、PixImageProvider 实现
- [ ] 7.2 在 parser.rs 中解析 Markdown `![alt](path)` 为 SlideElement::Image { path, alt, asset_type }
- [ ] 7.3 根据文件扩展名判断 asset_type：.pix → ImgPix，.ssf → ImgSsf，其他 → 显示占位符
- [ ] 7.4 渲染 .pix 图片：创建 Sprite，使用 `asset2sprite!(sprite, ctx, path)` 加载并渲染
- [ ] 7.5 渲染 .ssf 动画：每帧更新 frame_idx，使用 `asset2sprite!(sprite, ctx, path, frame_idx)` 播放
- [ ] 7.6 在 demo.md 中添加 .pix 图片引用示例
- [ ] 7.7 未识别格式显示 `[Image: alt text]` 占位符文本

## 8. WASM/Web 支持

- [ ] 8.1 创建 `apps/mdpt/wasm/` 目录结构（Cargo.toml、Makefile、src/lib.rs、index.html、index.js）
- [ ] 8.2 配置 wasm/Cargo.toml：rust_pixel[web] + comrak + syntect + wasm-bindgen
- [ ] 8.3 实现 wasm/src/lib.rs：复用 app 主代码（与其他 app 的 wasm 入口一致）
- [ ] 8.4 实现 index.js：资产加载（symbols.png + symbol_map.json）、md 文件 fetch、事件转发、游戏循环
- [ ] 8.5 实现 index.html：canvas 元素 + WASM 模块加载
- [ ] 8.6 Makefile：wasm-pack build + http server

## 9. 验证

- [ ] 9.1 `cargo pixel r mdpt t assets/demo.md` —— 终端模式运行正常
- [ ] 9.2 `cargo pixel r mdpt g assets/demo.md` —— Glow 图形模式运行正常
- [ ] 9.3 验证代码块语法高亮显示正确
- [ ] 9.4 验证 Slide 间转场效果正常
- [ ] 9.5 验证标题文本动画正常
- [ ] 9.6 验证键盘导航（←/→/q）正常
- [ ] 9.7 验证未指定文件时默认加载 assets/demo.md
- [ ] 9.8 验证 .pix 图片在 Slide 中正确显示
- [ ] 9.9 验证 .ssf 动画在 Slide 中自动播放
- [ ] 9.10 验证 `<!-- pause -->` 逐步展示正常（→ 键先推进 step）
- [ ] 9.11 验证 `<!-- column_layout -->` 多列布局渲染正确
- [ ] 9.12 验证 `<!-- end_slide -->` 作为分页符正常工作
- [ ] 9.13 验证 `<!-- jump_to_middle -->` 内容垂直居中
- [ ] 9.14 验证代码块 `+line_numbers` 行号显示
- [ ] 9.15 `cd apps/mdpt/wasm && make run` —— Web 模式在浏览器中运行正常
