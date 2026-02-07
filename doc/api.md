# RustPixel API Reference

> Based on review of `tower`, `tetris`, `ui_demo`, `petview` apps.

## Table of Contents

- [1. App Macro](#1-app-macro)
- [2. Model Trait](#2-model-trait)
- [3. Render Trait](#3-render-trait)
- [4. Context](#4-context)
- [5. Scene](#5-scene)
- [6. Sprite](#6-sprite)
- [7. Buffer / Cell](#7-buffer--cell)
- [8. Event System](#8-event-system)
- [9. Timer System](#9-timer-system)
- [10. Color / Style](#10-color--style)
- [11. RenderTexture Compositing](#11-rendertexture-compositing)
- [12. UI Widgets](#12-ui-widgets)
- [13. Asset Loading](#13-asset-loading)

---

## 1. App Macro

```rust
// In lib.rs — generates Game struct, init_game(), run(), WASM exports
use rust_pixel::app;
app!(MyGame);
```

**Used by**: all apps

---

## 2. Model Trait

```rust
pub trait Model {
    fn init(&mut self, ctx: &mut Context);
    fn update(&mut self, ctx: &mut Context, dt: f32);      // default impl provided
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32);
    fn handle_event(&mut self, ctx: &mut Context, dt: f32);
    fn handle_input(&mut self, ctx: &mut Context, dt: f32);
    fn handle_auto(&mut self, ctx: &mut Context, dt: f32);
}
```

| Method | Purpose | Example |
|--------|---------|---------|
| `init` | One-time setup | Load assets, create data |
| `handle_input` | Keyboard/mouse | Move piece, fire action |
| `handle_auto` | Autonomous logic | AI, physics, auto-drop |
| `handle_timer` | Timer callbacks | Wave spawning, animations |
| `handle_event` | Custom events | State transitions |

**Used by**: all apps

---

## 3. Render Trait

```rust
pub trait Render {
    type Model: Model;
    fn init(&mut self, ctx: &mut Context, m: &mut Self::Model);
    fn update(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32); // default impl
    fn handle_event(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
    fn handle_timer(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
    fn draw(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
}
```

**Typical init pattern:**

```rust
fn init(&mut self, ctx: &mut Context, _m: &mut Self::Model) {
    ctx.adapter.init(w, h, rx, ry, "title".to_string());
    self.scene.init(ctx);
}
```

| App | adapter.init params |
|-----|---------------------|
| tower | `(18, 15, 1.0, 1.0, "tower")` |
| tetris | `(35, 24, 0.5, 0.5, "tetris")` |
| ui_demo | `(80, 30, 1.0, 1.0, "")` |
| petview | `(40, 25, 1.0, 1.0, "petview")` |

**Used by**: all apps

---

## 4. Context

```rust
pub struct Context {
    pub stage: u32,                 // Frame counter
    pub state: u8,                  // User-defined state
    pub rand: Rand,                 // Random number generator
    pub asset_manager: AssetManager,
    pub input_events: Vec<Event>,   // Input queue (cleared after processing)
    pub adapter: Box<dyn Adapter>,  // Rendering backend
}

// Methods
ctx.cell_width() -> f32
ctx.cell_height() -> f32
ctx.canvas_size() -> (u32, u32)
ctx.ratio() -> (f32, f32)
ctx.centered_viewport(cell_w, cell_h) -> ARect
ctx.centered_rt(rt, cell_w, cell_h) -> RtComposite
```

**Used by**: all apps (`ctx.input_events`, `ctx.state`, `ctx.stage`, `ctx.adapter`)

---

## 5. Scene

Scene is the top-level rendering container, managing layers of sprites with double-buffered rendering.

### Creation & Lifecycle

```rust
let mut scene = Scene::new();
scene.init(&mut ctx);
scene.draw(&mut ctx)?;          // Render + present
scene.draw_to_rt(&mut ctx)?;   // Render to RT2 only (for custom compositing)
```

### Sprite Management (default "sprite" layer)

```rust
scene.add_sprite(Sprite::new(x, y, w, h), "tag");
scene.get_sprite("tag")  -> &mut Sprite
scene.with_sprites(&["a", "b"], |sprites| { ... });
```

### Multi-Layer Management

```rust
scene.add_layer("fx");
scene.add_layer_sprite(sprite, "fx", "explosion");
scene.get_layer_sprite("fx", "explosion") -> &mut Sprite
scene.set_layer_weight("fx", 50);    // z-ordering
scene.active_layer("fx");
scene.deactive_layer("fx");
```

### Object Pool Integration

```rust
scene.creat_objpool_sprites(&pool, w, h, |sprite, obj| { ... });
scene.draw_objpool(&mut pool, |sprite, obj| { ... });
```

### TUI Buffer Access

```rust
scene.tui_buffer_mut() -> &mut Buffer   // For UI widget rendering
scene.current_buffer_mut() -> &mut Buffer
```

| App | Key Usage |
|-----|-----------|
| tower | Object pools for monsters, bullets, towers |
| tetris | Named sprites for grids, blocks |
| ui_demo | `tui_buffer_mut()` for widget rendering |
| petview | `draw_to_rt()` for custom compositing |

---

## 6. Sprite

Sprite is a positioned, transformable drawable element. It `Deref`s to `Buffer`, so all Buffer methods are available directly.

### Creation

```rust
Sprite::new(x: u16, y: u16, width: u16, height: u16) -> Sprite
Sprite::new_half_width(x, y, w, h) -> Sprite    // scale_x = 0.5
```

### Positioning

```rust
sprite.set_pos(x: u16, y: u16)          // Cell position
sprite.set_cell_pos(x: u16, y: u16)     // Same as set_pos
sprite.set_pixel_pos(x: f32, y: f32)    // Pixel position (graphics mode)
sprite.pixel_pos() -> (f32, f32)
```

### Transformation (graphics mode)

```rust
sprite.set_angle(angle: f64)     // Rotation in degrees
sprite.set_alpha(alpha: u8)      // 0–255
sprite.set_scale(scale: f32)     // Uniform scaling
sprite.set_scale_x(sx: f32)
sprite.set_scale_y(sy: f32)
sprite.set_hidden(flag: bool)
sprite.is_hidden() -> bool
```

### Content (via Buffer deref)

```rust
sprite.set_color_str(x, y, text, fg: Color, bg: Color)
sprite.set_graph_sym(x, y, tex_id: u8, sym: u8, fg: Color)
sprite.set_default_str(text)
sprite.draw_line(p0, p1, sym, fg, bg)
sprite.draw_circle(x, y, radius, sym, fg, bg)
sprite.set_border(borders, border_type, style)
```

### Asset Loading

```rust
sprite.set_content_by_asset(
    &mut asset_manager, asset_type, location, frame_idx, off_x, off_y
);
// or use the macro:
asset2sprite!(sprite, ctx, AssetType::ImgPix, "assets/image.pix", 0);
```

| App | Key Operations |
|-----|----------------|
| tower | `set_pos`, `set_angle`, `set_graph_sym`, `set_pixel_pos` |
| tetris | `set_graph_sym`, `set_color_str`, `set_content_by_asset` |
| ui_demo | Buffer operations through `tui_buffer_mut()` |
| petview | `set_alpha`, `set_scale`, `set_hidden`, pixel positioning |

---

## 7. Buffer / Cell

### Buffer

```rust
// Creation
Buffer::empty(area: Rect) -> Buffer
Buffer::filled(area: Rect, &cell: Cell) -> Buffer

// Cell access
buffer.get(x, y) -> &Cell
buffer.get_mut(x, y) -> &mut Cell

// Content setting (relative coordinates)
buffer.set_str(x, y, text, style)
buffer.set_str_tex(x, y, text, style, tex: u8)
buffer.dstr(text)                         // Set at (0,0)

// Convenience
buffer.set_color_str(x, y, text, fg, bg)
buffer.set_graph_sym(x, y, tex_id, sym, fg)

// Shapes
buffer.draw_line(p0, p1, sym, fg, bg)
buffer.draw_circle(x0, y0, radius, sym, fg, bg)

// Operations
buffer.reset()
buffer.set_fg(color: Color)
buffer.merge(&other, alpha: u8, fast: bool)
buffer.blit(dx, dy, &other, other_part, alpha)
buffer.diff(&other) -> Vec<(u16, u16, &Cell)>
```

### Cell

```rust
cell.set_symbol(s: &str) -> &mut Cell
cell.set_char(ch: char) -> &mut Cell
cell.set_texture(tex: u8) -> &mut Cell
cell.set_fg(color: Color) -> &mut Cell
cell.set_bg(color: Color) -> &mut Cell
cell.set_style(style: Style) -> &mut Cell

// Helpers
cellsym(idx: u8) -> String              // Private Use Area U+E000+idx
tui_symidx(s: &str) -> Option<(u8, u8)> // Symbol → (tex, idx)
```

---

## 8. Event System

### Input Events

```rust
// Event enum
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

// KeyEvent
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,       // Press, Repeat, Release
}

// Common KeyCode variants
KeyCode::Char(c), KeyCode::Enter, KeyCode::Esc,
KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down,
KeyCode::Tab, KeyCode::F(1..12)

// MouseEvent
pub struct MouseEvent {
    pub kind: MouseEventKind,     // Down, Up, Drag, Moved
    pub column: u16,              // 8px unit
    pub row: u16,                 // 8px unit (÷2 for TUI row)
    pub modifiers: KeyModifiers,
}
```

### Typical Pattern

```rust
fn handle_input(&mut self, ctx: &mut Context, _dt: f32) {
    for e in &ctx.input_events {
        match e {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => ctx.state = 1,
                KeyCode::Up => { /* move */ },
                _ => {}
            },
            Event::Mouse(mouse) => { /* handle mouse */ },
        }
    }
    ctx.input_events.clear();
}
```

### Global Event System

```rust
event_register("grid_changed", "redraw_grid");
event_emit("grid_changed");
event_check("grid_changed", "redraw_grid") -> bool
```

**Used by**: all apps

---

## 9. Timer System

```rust
// Register a timer (fires after `time` seconds)
timer_register("wave", 5.0, "spawn_wave");

// Fire immediately with data
timer_fire("effect", value);

// Query
timer_stage("wave") -> u32        // Current countdown
timer_rstage("wave") -> u32       // Remaining time
timer_percent("wave") -> f32      // 0.0–1.0
timer_exdata("wave") -> Option<Vec<u8>>

// Control
timer_set_time("wave", 3.0);
timer_cancel("wave", nocall: bool);
```

### Handling in Model

```rust
fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {
    if event_check("wave", "spawn_wave") {
        self.spawn_next_wave();
    }
}
```

**Used by**: tower (wave timer), tetris (auto-drop), petview (animations)

---

## 10. Color / Style

### Color

```rust
pub enum Color {
    Reset,
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, Gray,
    DarkGray, LightRed, LightGreen, LightYellow,
    LightBlue, LightMagenta, LightCyan, White,
    Indexed(u8),                  // 256-color palette
    Rgba(u8, u8, u8, u8),        // True color + alpha
}

color.get_rgba() -> (u8, u8, u8, u8)
```

### Style

```rust
let style = Style::default()
    .fg(Color::Yellow)
    .bg(Color::Black)
    .add_modifier(Modifier::BOLD);
```

**Used by**: all apps

---

## 11. RenderTexture Compositing

Advanced GPU rendering pipeline for graphics mode.

### RT Convention

| RT | Purpose |
|----|---------|
| RT0 | Transition source 1 |
| RT1 | Transition source 2 |
| RT2 | Main scene content |
| RT3 | Overlay / effects |

### RtComposite

```rust
RtComposite::fullscreen(rt: usize) -> RtComposite
RtComposite::with_viewport(rt, viewport: ARect) -> RtComposite

// Builder methods
.blend(BlendMode)      // Normal, Add, Multiply, Screen
.alpha(u8)
.offset(dx, dy)
.scale(sx, sy)
.scale_uniform(s)
```

### Adapter Methods

```rust
adapter.draw_render_buffer_to_texture(&rbuf, rt_index, debug);
adapter.blend_rts(src1, src2, target, effect, progress);
adapter.copy_rt(src, dst);
adapter.clear_rt(rt);
adapter.present(&[RtComposite]);   // Custom compositing
adapter.present_default();          // RT2 fullscreen + RT3 overlay
```

### Helper

```rust
ctx.centered_rt(rt, cell_w, cell_h) -> RtComposite
```

**Used by**: petview (multi-RT compositing), ui_demo (transitions)

---

## 12. UI Widgets

Terminal UI framework that works in both terminal and graphics modes.

### UIPage (App Container)

```rust
let mut page = UIPage::new(width, height);
page.set_root_widget(Box::new(root_panel));
page.set_theme("dark")?;
page.handle_input_event(event);
page.update(dt)?;
page.render()?;
page.render_into(&mut buffer)?;   // Zero-copy into existing buffer
page.buffer() -> &Buffer
```

### Panel (Container)

```rust
Panel::new()
    .with_bounds(Rect::new(x, y, w, h))
    .with_border(BorderStyle::Rounded)
    .with_title("Title")
    .with_layout(Box::new(LinearLayout::vertical().with_spacing(1)))
panel.add_child(Box::new(widget));
```

### Label

```rust
Label::new("text")
    .with_style(style)
    .with_spotlight(highlight_style, width, speed)
    .with_wave(amplitude, frequency, speed)
    .with_fade_in(char_interval, loop_mode)
    .with_typewriter(char_interval, loop_mode, sound)
```

### Button

```rust
Button::new("Click me")
    .with_style(style)
    .on_click(|_| { /* action */ })
```

### TextBox

```rust
TextBox::new()
    .with_placeholder("Enter text...")
    .with_style(style)
```

### List

```rust
List::new()
    .with_selection_mode(SelectionMode::Single)
    .with_style(style)
list.add_text_item("item");
```

### ProgressBar

```rust
ProgressBar::new()
    .with_value(0.75)            // 0.0–1.0
    .with_fill_style(style)
```

### Table

```rust
Table::new()
    .with_columns(vec![
        Column::new("Name", 20).align(ColumnAlign::Left),
        Column::new("Value", 10).align(ColumnAlign::Right),
    ])
    .with_header(true)
    .with_header_style(style)
table.set_rows(vec![TableRow { cells, enabled }]);
```

### Other Widgets

| Widget | Description |
|--------|-------------|
| Checkbox | Toggle on/off |
| ToggleSwitch | Animated toggle |
| Slider | Value range selector |
| RadioGroup | Single-choice group |
| Dropdown | Select from list |
| Tabs | Tab navigation container |
| Modal | Overlay dialog |
| Toast | Temporary notification |
| Tree | Hierarchical tree view |
| Scrollbar | Scroll indicator |

### Layout

```rust
LinearLayout::vertical().with_spacing(1).with_alignment(Alignment::Center)
LinearLayout::horizontal().with_spacing(2)
```

**Used by**: ui_demo (full showcase), stockai (data tables)

---

## 13. Asset Loading

### Asset Types

| Format | Type | Description |
|--------|------|-------------|
| `.pix` | `AssetType::ImgPix` | PETSCII image |
| `.ssf` | `AssetType::SsfAnim` | PETSCII animation |
| `.esc` | `AssetType::ImgEsc` | Terminal escape art |
| `.png/.jpg` | Standard | Via `image` crate |

### Loading

```rust
// Macro (most common)
asset2sprite!(sprite, ctx, AssetType::ImgPix, "assets/image.pix", 0);

// Manual
sprite.set_content_by_asset(
    &mut ctx.asset_manager,
    AssetType::SsfAnim,
    "assets/anim.ssf",
    frame_index,
    offset_x,
    offset_y,
);

// Async loading (WASM)
ctx.asset_manager.load("assets/data.bin");
if ctx.asset_manager.is_loaded("assets/data.bin") {
    let data = ctx.asset_manager.get("assets/data.bin");
}
```

**Used by**: tetris (block art), petview (image viewing), tower (sprites)

---

## Quick Reference: Most Common API by Frequency

| Rank | API | Used By |
|------|-----|---------|
| 1 | `app!()` | ALL |
| 2 | `Scene::new/init/draw` | ALL |
| 3 | `Sprite::new` + `set_pos` | ALL |
| 4 | `ctx.input_events` + `Event::Key` | ALL |
| 5 | `adapter.init(w, h, rx, ry, title)` | ALL |
| 6 | `set_graph_sym` / `set_color_str` | tower, tetris, petview |
| 7 | `timer_register` / `event_check` | tower, tetris, petview |
| 8 | `set_alpha` / `set_angle` / `set_hidden` | tower, petview |
| 9 | `UIPage` + Panel/Label/Button | ui_demo |
| 10 | `RtComposite` / `present` | petview, ui_demo |
