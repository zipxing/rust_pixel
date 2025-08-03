# RustPixel UI Framework

基于 rust_pixel 字符渲染引擎的简单实用UI框架，专为开发编辑器应用、画廊查看器等字符界面应用而设计。

## 🎯 设计目标

- **简单易用**: 提供直观的API，快速构建UI应用
- **功能完整**: 包含常用UI组件，满足基本应用需求  
- **高度可定制**: 支持主题、样式、布局的灵活配置
- **性能优化**: 基于rust_pixel的高效字符渲染

## 🏗️ 核心架构

### Widget系统
```rust
pub trait Widget {
    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()>;
    fn handle_event(&mut self, event: &UIEvent, ctx: &mut Context) -> UIResult<bool>;
    fn update(&mut self, dt: f32, ctx: &mut Context) -> UIResult<()>;
    // ... 其他方法
}
```

### 布局系统
- **LinearLayout**: 垂直/水平线性布局
- **GridLayout**: 网格布局  
- **FreeLayout**: 自由定位布局

### 事件系统
- 统一的事件处理机制
- 支持键盘、鼠标交互
- 组件间通信支持

### 主题系统
- 内置暗色、亮色、终端主题
- 组件状态样式支持（hover、focus、pressed、disabled）
- 可扩展的主题定义

## 📦 组件库

### 基础组件

#### Label - 文本显示
```rust
let label = Label::new("Hello World!")
    .with_style(Style::default().fg(Color::Green))
    .with_align(TextAlign::Center)
    .with_wrap(true);
```

#### Button - 按钮交互
```rust
let button = Button::new("Click Me")
    .with_button_style(ButtonStyle::Normal)
    .on_click(|| println!("Button clicked!"));
```

#### TextBox - 文本输入
```rust
let textbox = TextBox::new()
    .with_placeholder("Enter text...")
    .with_max_length(100)
    .on_changed(|text| println!("Text: {}", text));
```

#### List - 列表选择
```rust
let mut list = List::new()
    .with_selection_mode(SelectionMode::Single)
    .on_selection_changed(|indices| println!("Selected: {:?}", indices));

list.add_text_item("Item 1");
list.add_text_item("Item 2");
```

#### Tree - 树形结构
```rust
let mut tree = Tree::new()
    .with_lines(true)
    .on_selection_changed(|node_id| println!("Selected: {:?}", node_id));

let root = tree.add_root_node("Root");
tree.add_child_node(root, "Child 1");
```

#### Panel - 容器组件
```rust
let mut panel = Panel::new()
    .with_border(BorderStyle::Single)
    .with_title("My Panel")
    .with_layout(Box::new(LinearLayout::vertical()));

panel.add_child(Box::new(label));
panel.add_child(Box::new(button));
```

## 🚀 快速开始

### 1. 创建应用
```rust
use rust_pixel::ui::*;

fn main() -> UIResult<()> {
    let mut app = UIAppBuilder::new(80, 25)
        .with_title("My App")
        .with_theme("dark")
        .build()?;
    
    // 创建界面
    let root = create_interface();
    app.set_root_widget(Box::new(root));
    
    // 运行应用
    app.run_simple()?;
    Ok(())
}
```

### 2. 构建界面
```rust
fn create_interface() -> Panel {
    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 80, 25))
        .with_border(BorderStyle::Single)
        .with_title("Hello UI Framework")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1)));
    
    // 添加组件
    panel.add_child(Box::new(
        Label::new("Welcome to RustPixel UI!")
            .with_style(Style::default().fg(Color::Green))
    ));
    
    panel.add_child(Box::new(
        Button::new("Start")
            .on_click(|| println!("Application started!"))
    ));
    
    panel
}
```

## 🎨 主题定制

### 使用内置主题
```rust
app.set_theme("dark")?;    // 暗色主题
app.set_theme("light")?;   // 亮色主题  
app.set_theme("terminal")?; // 终端主题
```

### 自定义主题
```rust
let mut theme = Theme::new("custom");

let button_style = ComponentStyle::new(
    Style::default().fg(Color::White).bg(Color::Blue)
)
.with_hover(Style::default().fg(Color::Yellow).bg(Color::Blue))
.with_focus(Style::default().fg(Color::Black).bg(Color::White));

theme.set_style("button", button_style);
app.theme_manager_mut().register_theme(theme);
```

## 📱 示例应用

### 文件浏览器
位于 `apps/ui_demo/` 目录，演示了完整的文件浏览器界面：

- 📁 左侧文件树导航
- 📄 右侧文件信息面板
- 👁️ 文件预览区域
- 🔧 操作按钮组

```bash
cd apps/ui_demo
cargo run
```

### 主要特性展示
- **树形结构**: 文件夹层次展示
- **多面板布局**: 灵活的界面划分
- **交互响应**: 点击、选择、按钮回调
- **主题支持**: 切换不同视觉风格

## 🔧 高级功能

### 事件处理
```rust
// 自定义事件处理
widget.handle_event(&UIEvent::Input(input_event), &mut ctx)?;

// 组件间通信
event_dispatcher.emit_event(WidgetEvent::ValueChanged(id, value).into());
```

### 布局约束
```rust
let constraints = LayoutConstraints {
    min_width: 10,
    max_width: 50,
    weight: 2.0,
    ..Default::default()
};

panel.add_child_with_constraints(widget, constraints);
```

### 滚动支持
```rust
let scrollbar = ScrollBar::vertical()
    .with_value(0.5)
    .with_page_size(0.2)
    .on_value_changed(|value| println!("Scroll: {}", value));
```

## 🛠️ 开发指南

### 自定义组件
```rust
pub struct MyWidget {
    base: BaseWidget,
    // 自定义字段
}

impl Widget for MyWidget {
    impl_widget_base!(MyWidget, base);
    
    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()> {
        // 自定义渲染逻辑
        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent, ctx: &mut Context) -> UIResult<bool> {
        // 自定义事件处理
        Ok(false)
    }
}
```

### 与rust_pixel集成
```rust
// 在游戏循环中集成UI
impl Model for MyAppModel {
    fn handle_input(&mut self, ctx: &mut Context, dt: f32) {
        for event in &ctx.input_events {
            self.ui_app.handle_input_event(event.clone());
        }
    }
    
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        self.ui_app.update(dt)?;
    }
}

impl Render for MyAppRender {
    fn update(&mut self, ctx: &mut Context, model: &mut MyAppModel, dt: f32) {
        if model.ui_app.should_render() {
            model.ui_app.render()?;
            
            // 将UI buffer内容渲染到主buffer
            let ui_buffer = model.ui_app.buffer();
            // ... 复制buffer内容
            
            model.ui_app.frame_complete();
        }
    }
}
```

## 🎯 适用场景

- **📝 文本编辑器**: 代码编辑、配置文件编辑
- **📁 文件管理器**: 文件浏览、批量操作  
- **🖼️ 图片查看器**: ASCII艺术展示、图片管理
- **⚙️ 系统工具**: 配置界面、监控面板
- **🎮 游戏UI**: 菜单界面、设置面板

## 📈 性能特点

- **轻量级**: 基于字符渲染，内存占用小
- **高效**: 增量渲染，只更新变化区域
- **跨平台**: 支持终端、图形、Web多种后端
- **响应快**: 60FPS流畅交互体验

## 🔮 发展路线

- [ ] 更多组件：Menu、TabView、ProgressBar
- [ ] 拖拽支持：组件拖拽、文件拖拽
- [ ] 动画系统：过渡效果、加载动画
- [ ] 数据绑定：MVVM模式支持
- [ ] 插件系统：组件扩展机制

---

**RustPixel UI Framework** - 让字符界面开发变得简单而强大！