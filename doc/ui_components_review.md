# RustPixel TUI 组件体系 Review 报告

**日期**: 2026-03-04  
**审查范围**: `src/ui/` 全部代码 (~7,611 行) + `apps/ui_demo/`  
**审查人**: 小龙虾SG 🦞

---

## 1. 总览

### 1.1 代码规模

| 模块 | 行数 |
|------|------|
| 核心框架 (`src/ui/*.rs`) | 1,777 |
| 组件库 (`src/ui/components/`) | 5,834 |
| Demo 应用 (`apps/ui_demo/`) | ~380 |
| **合计** | **~7,991** |

### 1.2 架构总图

```
┌──────────────────────────────────────────────────┐
│  UIPage (app.rs)                                 │
│  ┌─ root_widget: Box<dyn Widget>                 │
│  ├─ EventDispatcher (event.rs)                   │
│  ├─ ThemeManager (theme.rs)                      │
│  ├─ Buffer (内置渲染缓冲)                         │
│  └─ render_into() / render() → Buffer            │
├──────────────────────────────────────────────────┤
│  Widget Trait (widget.rs)                        │
│  ├─ BaseWidget + WidgetState                     │
│  ├─ Container Trait (extends Widget)             │
│  ├─ impl_widget_base! 宏                         │
│  └─ next_widget_id() (AtomicU32)                 │
├──────────────────────────────────────────────────┤
│  Layout (layout.rs)         Theme (theme.rs)     │
│  ├─ LinearLayout (H/V)     ├─ dark/light/terminal│
│  ├─ GridLayout             ├─ ComponentStyle      │
│  └─ FreeLayout             └─ 5态优先级解析       │
├──────────────────────────────────────────────────┤
│  19 Components (components/)                     │
│  ├─ 基础: Label Button TextBox Checkbox          │
│  │        ToggleSwitch Radio Slider ProgressBar  │
│  │        Dropdown                               │
│  ├─ 容器: Panel Tabs Modal                       │
│  ├─ 数据: List Table Tree ScrollBar              │
│  │        PresentList PresentTable               │
│  └─ 反馈: Toast                                  │
├──────────────────────────────────────────────────┤
│  Utilities                                       │
│  ├─ text_util.rs (wrap/width/truncate, CJK感知)  │
│  └─ BufferTransition (9种转场效果)                │
└──────────────────────────────────────────────────┘
```

---

## 2. 现有组件详情

### 2.1 基础输入组件

| 组件 | 行数 | 功能 | 亮点 |
|------|------|------|------|
| **Button** | 266 | 点击、样式、回调 | 背景/文字/边框三层渲染 |
| **TextBox** | 448 | 单行输入、光标、占位符 | 密码模式、max_length |
| **Checkbox** | 151 | 勾选、标签 | `[x]`/`[ ]` 渲染 |
| **ToggleSwitch** | 151 | 开关切换 | `[ON ]`/`[OFF]` 动画 |
| **RadioGroup** | 233 | 单选组 | `(●)`/`( )` 渲染 |
| **Slider** | 182 | 拖拽滑块 | min/max/step |
| **Dropdown** | 298 | 下拉选择 | 展开/收起、键盘导航 |

### 2.2 数据展示组件

| 组件 | 行数 | 功能 | 亮点 |
|------|------|------|------|
| **Label** | 527 | 文本显示+动画 | Spotlight/Wave/FadeIn/Typewriter 4种动画 |
| **List** | 393 | 列表选择 | 单选/多选模式、内置滚动条 |
| **Table** | 499 | 表格展示 | 列对齐、行选择、行激活回调 |
| **Tree** | 464 | 树形结构 | 展开/折叠、连线、节点管理 |
| **PresentList** | 189 | 展示型列表 | 有序/无序、多级缩进 |
| **PresentTable** | 226 | 展示型表格 | 列对齐、自动列宽计算 |
| **ProgressBar** | 152 | 进度条 | 百分比显示、自定义字符 |

### 2.3 容器组件

| 组件 | 行数 | 功能 | 亮点 |
|------|------|------|------|
| **Panel** | 590 | 通用容器 | Canvas模式、边框(5种)、标题、水平/垂直分割线 |
| **Tabs** | 244 | 标签页 | 实现Container trait、标签栏渲染 |
| **Modal** | 316 | 模态弹窗 | 半透明背景、内容/按钮区域 |

### 2.4 辅助组件

| 组件 | 行数 | 功能 | 亮点 |
|------|------|------|------|
| **ScrollBar** | 341 | 滚动条 | 水平/垂直、步进/翻页 |
| **Toast** | 164 | 通知提示 | Info/Success/Warning/Error 4种类型、自动隐藏 |

---

## 3. 核心框架评估

### 3.1 Widget 系统 ✅ 设计良好

- **trait 架构清晰**: Widget(基础) + Container(容器) 分层合理
- **BaseWidget + 宏**: `impl_widget_base!` 减少样板代码
- **WidgetState 六态**: visible/enabled/focused/hovered/pressed/dirty
- **ID 生成**: AtomicU32 线程安全
- **hit_test**: 统一的点击检测

### 3.2 Layout 系统 ✅ 够用

- **三种策略**: Linear(H/V)、Grid、Free
- **LayoutConstraints**: min/max 尺寸 + weight 权重分配
- **Padding**: all/horizontal/vertical 快捷构造
- **不足**: 缺少 Flex/Wrap 布局（组件溢出时不会自动换行）

### 3.3 事件系统 ⚠️ 基本可用但有短板

- **三层事件**: Input(原始) → Widget(组件) → App(应用)
- **EventDispatcher**: focus/hover 跟踪 + VecDeque 队列
- **WidgetValue**: None/Bool/Int/Float/String/Point 通用值类型
- **短板**: 无焦点链(Tab导航)、无事件冒泡、无事件拦截机制

### 3.4 主题系统 ✅ 设计不错

- **三套内置主题**: dark/light/terminal
- **五态样式**: normal → hovered → focused → pressed → disabled (优先级递增)
- **ComponentStyle**: 统一的组件样式结构
- **ThemeManager**: 主题切换 + 事件通知

### 3.5 UIPage / App 框架 ✅ 多页面支持

- **每页独立 Buffer**: 支持多页面同时存在
- **BufferTransition**: 9种转场效果 (Wipe/Slide/Dissolve/Blinds/Checkerboard/Typewriter)
- **render_into()**: 零拷贝渲染到目标 Buffer
- **wasm32 兼容**: Instant::now() 条件编译

### 3.6 Demo 应用 ✅ 已有

`apps/ui_demo/` 三页展示：
- Page 1: Button, TextBox, List, ProgressBar, Spotlight 动画
- Page 2: Wave/FadeIn/Typewriter 动画 + 文字修饰符
- Page 3: Tree, Checkbox, ToggleSwitch, Slider, Radio, Dropdown, Table

页面间 TransitionState 自动循环 9 种转场效果。

---

## 4. 缺失分析

### 4.1 P0 — 缺失组件/功能（影响基本可用性）

#### 4.1.1 Focus Chain / Tab 导航
**现状**: EventDispatcher 只跟踪单个 `focused_widget`，没有 `focus_next()` / `focus_prev()`  
**影响**: 用户无法用 Tab 键在表单组件间切换，键盘操作效率低  
**建议**: 在 EventDispatcher 中维护 `focus_order: Vec<WidgetId>`，支持 Tab/Shift+Tab 循环导航

#### 4.1.2 TextArea（多行文本编辑）
**现状**: TextBox 仅支持单行  
**影响**: 无法实现配置编辑器、日志查看器等常见场景  
**建议**: 新增 TextArea 组件，支持多行编辑、行号、选区、滚动

#### 4.1.3 Menu / ContextMenu
**现状**: 完全没有菜单组件  
**影响**: TUI 应用标配，缺少会让应用显得不完整  
**建议**: MenuBar(顶部菜单栏) + ContextMenu(右键菜单)，支持快捷键绑定

### 4.2 P1 — 重要增强

#### 4.2.1 ScrollView 滚动容器
**现状**: ScrollBar 存在但没有通用滚动容器  
**影响**: 内容超出视口时缺少统一的滚动方案  
**建议**: ScrollView 容器组件，自动管理内容偏移 + 内嵌 ScrollBar

#### 4.2.2 StatusBar 状态栏
**现状**: 无  
**影响**: 标准 TUI 应用底部状态栏（快捷键提示、状态信息）  
**建议**: 固定在屏幕底部的 StatusBar 组件

#### 4.2.3 Dialog 预设
**现状**: Modal 可用但没有快捷 API  
**影响**: 每次需要确认框都要手动组装 Modal + Button  
**建议**: `Dialog::confirm("title", "message")` / `Dialog::alert()` / `Dialog::prompt()` 快捷构造

#### 4.2.4 Modal 焦点陷阱
**现状**: Modal 打开后键盘事件仍会穿透到背景组件  
**影响**: 用户在 Modal 中按 Tab 可能焦点跑到背景去  
**建议**: Modal 激活时 EventDispatcher 限制焦点范围

### 4.3 P2 — 锦上添花

| 组件 | 说明 |
|------|------|
| **Tooltip** | 鼠标悬停提示，hover 事件已有 |
| **Spinner** | 异步加载指示器 |
| **Splitter** | 可拖拽的分屏面板 |
| **Breadcrumb** | 配合 Tree 导航 |
| **Gauge** | 环形/弧形仪表盘 |
| **Sparkline** | 小型图表 |

---

## 5. 工程质量

### 5.1 测试覆盖 ⚠️ 严重不足

| 模块 | 测试状态 |
|------|----------|
| text_util.rs | ✅ 9 个测试用例 |
| 19 个组件 | ❌ 零测试 |
| widget.rs | ❌ 零测试 |
| layout.rs | ❌ 零测试 |
| event.rs | ❌ 零测试 |
| theme.rs | ❌ 零测试 |
| app.rs | ❌ 零测试 |

**建议优先级**:
1. Layout 算法测试（LinearLayout 权重分配、Grid 边界）
2. EventDispatcher 焦点/hover 状态机测试
3. 各组件 preferred_size 计算测试
4. Widget 渲染到 Buffer 的快照测试

### 5.2 文档

- 核心框架文件有 doc comment ✅
- `app.rs` 有 ASCII 图解 ✅
- 部分组件缺少模块级文档（present_list、present_table）⚠️
- 已有 `doc/uipage_rendering.md` 作为外部文档 ✅

### 5.3 Demo 覆盖

已展示 13/19 组件。未展示：
- ScrollBar（独立）、Modal、Toast、PresentList、PresentTable、Tabs

---

## 6. 建议路线图

### Phase 1 — 补全核心（2周）
- [ ] Focus Chain + Tab/Shift+Tab 导航
- [ ] TextArea 多行编辑器
- [ ] Menu / ContextMenu
- [ ] Modal 焦点陷阱

### Phase 2 — 增强体验（2周）
- [ ] ScrollView 滚动容器
- [ ] StatusBar
- [ ] Dialog 预设 (confirm/alert/prompt)
- [ ] Demo 补全剩余组件展示

### Phase 3 — 质量保证（1周）
- [ ] Layout 算法单元测试
- [ ] EventDispatcher 状态机测试
- [ ] 组件 preferred_size 测试
- [ ] 渲染快照测试

### Phase 4 — 扩展组件（按需）
- [ ] Tooltip、Spinner、Splitter
- [ ] Sparkline、Gauge
- [ ] 事件冒泡 / 信号槽机制

---

## 7. 总体评价

**优点**:
- 架构清晰，trait + macro 组合减少样板代码
- 19 个组件覆盖了常见 TUI 场景的 ~80%
- Label 动画系统（4种效果）和 BufferTransition（9种转场）是亮点
- Panel 的 Canvas 模式提供了灵活的自由绘制能力
- 多页面 + 零拷贝渲染的 UIPage 设计合理
- Unicode/CJK 感知的文本处理

**需改进**:
- 焦点管理是最大短板，影响键盘可用性
- 测试几乎为零，重构风险高
- 缺少 Menu/TextArea/ScrollView 这几个 TUI 标配组件

**一句话**: 架构扎实，组件丰富，但焦点系统和测试覆盖是上线前必须补的短板。
