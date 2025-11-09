## 1. Implementation

- [x] 1.1 新增核心 UI 组件（Tabs、Modal/Dialog、~~Tooltip~~、Dropdown/Select、Slider、ProgressBar、ToggleSwitch、Checkbox、Radio、Toast/Notification、~~Image/Icon~~）；基于字符单元渲染，保持简单实用、游戏友好。**完成 9/11 组件，Tooltip 和 Image/Icon 留待后续架构改动后实现。**
- [ ] 1.2 实现布局原语（水平/垂直 Stack、Grid）；保证字符格对齐、间距与伸缩的最小必要配置。**底层 LinearLayout 和 GridLayout 算法已存在，独立容器组件待 TUI/GUI 分层后实现。**
- [ ] 1.3 增强主题（运行时切换、组件变体、高对比度）；默认提供简洁配色方案与常用变体（primary/secondary/ghost）。**待 TUI/GUI 架构改动后实现。**
- [x] 1.4 完善焦点管理与键盘导航（Tab/Shift+Tab、方向键等）；保证基础状态（idle/hover/focus/active/disabled）与键盘等价操作。**基础设施已完成（EventDispatcher、WidgetState），部分组件支持键盘导航。**
- [ ] 1.5 引入事件捕获/冒泡，支持 stop_propagation 与 prevent_default；保持 API 简洁、父子组件解耦。**待 TUI/GUI 架构改动后实现。**
- [ ] 1.6 可达性与一致性：为组件定义语义角色/标签；统一 disabled/readonly 语义；保证键盘操作可达。**部分组件支持 disabled 状态，完整可达性待后续完善。**
- [x] 1.7 在 `apps/ui_demo` 集成并演示新增能力用于验证；不创建测试专用组件或多余演示代码（遵循项目规则）。**所有已实现组件均已在 ui_demo 中演示。**
- [ ] 1.8 更新既有组件以支持变体与焦点可视化；保持向后兼容与最小改动接入。**部分支持，待主题系统完善后统一处理。**

## 2. 阶段性总结

**已完成（Phase 1 - 核心组件）：**
- ✅ 9 个核心交互组件实现并集成到 ui_demo
- ✅ 基础焦点管理和事件分发机制
- ✅ 字符单元渲染，终端和图形模式兼容
- ✅ 鼠标和键盘基础交互支持

**待完成（Phase 2 - 架构增强）：**
- ⏸️ Tooltip、Image/Icon 组件
- ⏸️ 独立布局容器（Stack、Grid）
- ⏸️ 主题系统增强（运行时切换、变体）
- ⏸️ 完整事件捕获/冒泡机制
- ⏸️ 完整可达性支持

**下一步计划：**
进行 TUI/GUI 分层架构改动，完成后再继续完善 UI 系统的高级功能。


