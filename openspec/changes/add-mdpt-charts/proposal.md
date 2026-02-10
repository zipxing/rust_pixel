## Why

mdpt 目前支持文本、代码块、表格、图片等 slide 元素，但缺乏数据可视化能力。演示文稿中经常需要展示数据图表（折线图、直方图、饼图）和流程/架构图（mermaid）。当前只能通过截图（.pix 文件）嵌入静态图表，无法从文本数据动态生成。

添加字符渲染的图表和 mermaid 支持，可以让 mdpt 在终端和图形模式下都能直接从 markdown 数据描述生成可视化图形，无需外部工具预处理。

## What Changes

- **ADDED** 折线图渲染：在 markdown 中用 ```` ```linechart ```` 代码块描述数据，渲染为字符折线图（Braille 点阵或 ASCII 线条）
- **ADDED** 直方图/柱状图渲染：用 ```` ```barchart ```` 代码块描述数据，渲染为字符柱状图（使用 block 字符 ▁▂▃▄▅▆▇█）
- **ADDED** 饼图渲染：用 ```` ```piechart ```` 代码块描述数据，渲染为字符饼图（使用 Braille 点阵绘制圆形和扇区）
- **ADDED** Mermaid 流程图（最小子集）：用 ```` ```mermaid ```` 代码块描述 `graph TD/LR` 有向流程图，解析为节点和边，渲染为字符框线图。不支持的 mermaid 类型降级为代码块

## Impact

- **Affected specs**: mdpt-presentation（新增 4 个 SlideElement 类型）
- **Affected code**:
  - `apps/mdpt/src/slide.rs` — 新增 SlideElement 变体和解析逻辑
  - `apps/mdpt/src/slide_builder.rs` — 新增图表渲染到 Buffer 的逻辑
  - `apps/mdpt/src/chart/` — 新增图表渲染模块（折线图、直方图、饼图、mermaid 流程图）
  - `apps/mdpt/src/model.rs` — 最小变更，支持新元素类型
- **新增依赖**: 无外部 crate 依赖，纯 Rust 实现字符渲染
