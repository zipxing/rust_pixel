## 1. 基础设施

- [ ] 1.1 创建 `apps/mdpt/src/chart/mod.rs`，定义 `ChartRenderer` trait 和公共类型（ChartData, ChartConfig）
- [ ] 1.2 创建 `apps/mdpt/src/chart/braille.rs`，实现 Braille 点阵工具（坐标→Braille 字符映射、点阵 Buffer 操作）
- [ ] 1.3 在 `slide.rs` 的 SlideElement 枚举中添加 `LineChart`、`BarChart`、`PieChart`、`MermaidDiagram` 变体
- [ ] 1.4 在 markdown 解析流程中识别 `linechart`、`barchart`、`piechart`、`mermaid` 代码块并生成对应 SlideElement

## 2. 图表数据解析

- [ ] 2.1 实现简化 key-value 格式解析器（`title:`, `x:`, `y:`, `labels:`, `values:`, `width:`, `height:`, `radius:` 等字段）
- [ ] 2.2 实现数组解析（`[item1, item2, ...]` 格式，支持数值和字符串）
- [ ] 2.3 添加解析错误处理，格式错误时降级为普通代码块显示

## 3. 折线图

- [ ] 3.1 创建 `apps/mdpt/src/chart/line_chart.rs`，实现 `LineChart` 结构体和 `ChartRenderer` trait
- [ ] 3.2 实现坐标轴绘制（Box Drawing 字符 ─│┌└，Y 轴刻度标签，X 轴标签）
- [ ] 3.3 实现数据点到 Braille 点阵的映射和折线连接（Bresenham 直线算法在 Braille 子像素上）
- [ ] 3.4 实现多系列支持（y, y2, y3... 不同颜色）和图例
- [ ] 3.5 实现自动缩放（Y 轴 min/max 计算、合适的刻度间隔）

## 4. 直方图

- [ ] 4.1 创建 `apps/mdpt/src/chart/bar_chart.rs`，实现 `BarChart` 结构体和 `ChartRenderer` trait
- [ ] 4.2 实现垂直柱状图渲染（Block Elements ▁▂▃▄▅▆▇█，1/8 精度高度）
- [ ] 4.3 实现柱状图标签（底部标签、顶部数值）
- [ ] 4.4 实现 Y 轴刻度和自动缩放
- [ ] 4.5 实现多色柱状图（按调色板循环着色）

## 5. 饼图

- [ ] 5.1 创建 `apps/mdpt/src/chart/pie_chart.rs`，实现 `PieChart` 结构体和 `ChartRenderer` trait
- [ ] 5.2 实现 Braille 圆形绘制（极坐标遍历，判断点是否在圆内）
- [ ] 5.3 实现扇区划分（角度计算，每个点判定所属扇区，着色）
- [ ] 5.4 实现图例渲染（右侧显示标签、颜色块、百分比）
- [ ] 5.5 处理小扇区（< 2% 的最小显示弧度保证）

## 6. Mermaid 流程图（最小子集）

- [ ] 6.1 创建 `apps/mdpt/src/chart/mermaid.rs`，定义 MermaidGraph, MermaidNode, MermaidEdge 类型
- [ ] 6.2 实现 `graph TD/LR` 语法解析（节点 `A[text]`，边 `A --> B`，边标签 `A -->|label| B`）
- [ ] 6.3 实现简单层次布局（拓扑排序分层 + 同层均匀分布）
- [ ] 6.4 实现节点矩形渲染（Box Drawing）和边线渲染（│↓─→ + 标签）
- [ ] 6.5 不支持的 mermaid 类型降级为代码块显示

## 7. 集成到 slide_builder

- [ ] 8.1 在 `slide_builder.rs` 的 `build_slide_page()` 中添加图表元素渲染分支
- [ ] 8.2 实现图表尺寸自适应（根据 slide 可用宽度和 margin 计算）
- [ ] 8.3 确保图表在 Pause 步进中正确处理（图表作为整体出现或隐藏）

## 8. 测试与验证

- [ ] 9.1 创建 demo slide（`apps/mdpt/assets/demo_charts.md`）包含各类图表示例
- [ ] 9.2 终端模式测试：`cargo pixel r mdpt t`
- [ ] 9.3 图形模式测试：`cargo pixel r mdpt glow`
- [ ] 9.4 验证 WASM 编译：`cd apps/mdpt/wasm && make`
- [ ] 9.5 测试边界情况：空数据、单值、极大/极小值、超长标签
