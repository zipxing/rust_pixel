## Why
当前 UI 能力以基础组件为主（如 `button`、`label`、`list`、`textbox` 等），在复杂应用中的可用性与一致性不足：缺少更丰富的交互组件、通用布局原语、主题变体与运行时切换、完善的焦点与键盘可达性、事件模型与长列表性能优化等。这限制了上层应用的 UX 表现与开发效率。

## What Changes
- 新增核心 UI 组件：Tabs、Modal/Dialog、Tooltip、Dropdown/Select、Slider、ProgressBar、ToggleSwitch、Checkbox、Radio、Toast/Notification、Image/Icon。
- 新增布局原语：Stack（水平/垂直）、Grid，提供对齐、间距、伸缩等能力。
- 增强主题与变体：运行时主题切换；组件级变体（primary/secondary/ghost 等）；高对比度模式。
- 完善输入与焦点：统一焦点管理；Tab/方向键导航；快捷键与可禁用/只读语义；一致的焦点可视化。
- 事件模型增强：冒泡/捕获；`stop_propagation` 与 `prevent_default` 语义；父子组件解耦的事件订阅。
- 性能与可扩展：虚拟化长列表；批量更新与脏区渲染策略；大数据量交互优化。
- 可达性与一致性：语义化角色/标签；键盘操作等价性；禁用态/只读态的统一标准。
- 兼容性目标：不破坏既有组件 API；新增能力为增量可选接入。

### 范围与原则（Simplicity & Terminal-First）
- 简单实用、游戏友好：不追求专业 GUI 水准，不引入复杂窗口管理、富文本排版或矢量级控件绘制。
- 终端与图形一致：两种后端均基于“字符单元（cell）”渲染，图形模式复用同一字符/符号图集以获得接近一致的视觉。
- 以最小状态集为准：组件仅保证 idle/hover/focus/active/disabled 的基础状态与键盘/鼠标可达性。


## Impact
- Affected specs: ui
- Affected code:
  - `src/ui/*`（布局、主题、焦点/事件、Widget 体系）
  - `src/ui/components/*`（新增组件与变体接入）
  - `src/event/*`（事件冒泡/捕获与键盘导航）
  - `src/render/*`（脏区/批量提交等渲染策略）
- Compatibility: 以增量方式提供新能力，保持既有 API 可用；避免 **BREAKING** 变更。
- Migration: 现有应用无需迁移，按需接入新 UI 组件/能力即可。
- 使用cargo pixel r ui_demo t -r运行demo


