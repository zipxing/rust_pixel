## 1. Implementation

- [ ] 1.1 新增核心 UI 组件（Tabs、Modal/Dialog、Tooltip、Dropdown/Select、Slider、ProgressBar、ToggleSwitch、Checkbox、Radio、Toast/Notification、Image/Icon）；基于字符单元渲染，保持简单实用、游戏友好。
- [ ] 1.2 实现布局原语（水平/垂直 Stack、Grid）；保证字符格对齐、间距与伸缩的最小必要配置。
- [ ] 1.3 增强主题（运行时切换、组件变体、高对比度）；默认提供简洁配色方案与常用变体（primary/secondary/ghost）。
- [ ] 1.4 完善焦点管理与键盘导航（Tab/Shift+Tab、方向键等）；保证基础状态（idle/hover/focus/active/disabled）与键盘等价操作。
- [ ] 1.5 引入事件捕获/冒泡，支持 stop_propagation 与 prevent_default；保持 API 简洁、父子组件解耦。
- [ ] 1.6 可达性与一致性：为组件定义语义角色/标签；统一 disabled/readonly 语义；保证键盘操作可达。
- [ ] 1.7 在 `apps/ui_demo` 集成并演示新增能力用于验证；不创建测试专用组件或多余演示代码（遵循项目规则）。
- [ ] 1.8 更新既有组件以支持变体与焦点可视化；保持向后兼容与最小改动接入。


