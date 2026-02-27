# RustPixel Architecture

RustPixel is a 2D game engine supporting text-mode and GPU-mode rendering across desktop, web, and terminal platforms.

## Rendering Backends

Two backends, unified interface:

```
                    Adapter trait
                        |
          +-------------+-------------+
          |                           |
   CrosstermAdapter            WGPU Pipeline
   (Terminal I/O)                    |
                          +----------+----------+
                          |                     |
                   WinitWgpuAdapter      WgpuWebAdapter
                   (Desktop: winit)     (Web: wasm + canvas)
                   Vulkan/Metal/DX12    WebGPU / WebGL2
```

Feature flags:
```toml
term = ["crossterm", ...]    # Terminal mode
wgpu = ["wgpu", "winit", ...] # Desktop GPU mode
web  = ["image"]              # Web mode (auto-detects wasm32, uses wgpu)
```

## Game Loop: Model-Render-Game

Every app implements `Model` (logic) and `Render` (drawing), orchestrated by `Game`:

```rust
pub trait Model {
    fn init(&mut self, ctx: &mut Context);
    fn handle_event(&mut self, ctx: &mut Context, dt: f32);
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32);
    fn handle_input(&mut self, ctx: &mut Context, dt: f32);
    fn handle_auto(&mut self, ctx: &mut Context, dt: f32);
}

pub trait Render {
    type Model: Model;
    fn init(&mut self, ctx: &mut Context, m: &mut Self::Model);
    fn handle_event(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
    fn handle_timer(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
    fn draw(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
}
```

Per-frame call order:
```
Model.update(dt)
  ├── handle_event()
  ├── handle_timer()
  ├── handle_input()
  └── handle_auto()

Render.update(dt)
  ├── handle_event()
  ├── handle_timer()
  └── draw()
```

## Rendering Hierarchy

```
Scene
├── buffers[2]          # Double-buffered (text mode diff rendering)
├── layers[]            # Sorted by render_weight
│   ├── "tui"           # TUI content sprites (weight: 100)
│   └── "sprite"        # Game object sprites (weight: 0)
└── layer_tag_index     # Name → index mapping
```

Each Layer holds Sprites. Each Sprite holds a Buffer of Cells:
```
Cell { symbol, fg, bg, modifier, scale_x, scale_y, glyph (cached) }
```

### Cell and Glyph

`Cell` is the fundamental rendering unit. The `symbol` string fully determines what gets rendered — no separate texture field.

```rust
pub struct Cell {
    pub symbol: String,    // Fully determines rendering
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    pub scale_x: f32,
    pub scale_y: f32,
    // glyph: Glyph (cached, computed on set_symbol)
}
```

`Glyph` describes how to find and render a symbol in the texture atlas:
```rust
pub struct Glyph {
    pub block: u8,   // Texture block index
    pub idx: u8,     // Symbol index within block
    pub width: u8,   // Width multiplier (1 or 2)
    pub height: u8,  // Height multiplier (1 or 2)
}
```

Glyph sizes (in PIXEL_SYM_WIDTH/HEIGHT units):
- **Sprite**: width=1, height=1 → 16×16 pixels
- **TUI**: width=1, height=2 → 16×32 pixels
- **Emoji**: width=2, height=2 → 32×32 pixels
- **CJK**: width=2, height=2 → 32×32 pixels

**Glyph caching**: `set_symbol()` automatically calls `compute_glyph()`, caching (block, idx, width, height) in the Cell. Rendering reads the cached glyph directly via `get_cell_info()` — no symbol string parsing at render time.

Lookup order: PUA Sprite (1×1) → Emoji (2×2) → CJK (2×2) → TUI (1×2) → fallback (space)

### Buffer

```rust
pub enum BufferMode {
    Tui,     // Standard Unicode (ASCII, Box, Braille, Emoji, CJK)
    Sprite,  // PUA-encoded sprite symbols
}

pub struct Buffer {
    pub mode: BufferMode,
    pub content: Vec<Cell>,
    pub area: Rect,
}
```

- **Tui mode**: symbol is standard Unicode
- **Sprite mode**: symbol is PUA-encoded, constructed via `cellsym_block(block, idx)`
- Rendering is mode-independent: fully determined by the cached glyph

### Symbol → Glyph Mapping (Two-Layer System)

The engine has two numbering systems: **Unicode codepoints** (stored in `cell.symbol`) and **atlas block indices** (in `Glyph`). `compute_glyph()` maps between them:

```
cell.symbol (Unicode)              Glyph (atlas block + idx)
───────────────────────            ──────────────────────────
PUA U+F0000 + block*256 + idx  →   Block 0-159,   idx   (decode PUA)
Real Emoji characters           →   Block 170-175, idx   (symbol_map lookup)
Real CJK characters             →   Block 176-239, idx   (symbol_map lookup)
Real ASCII/Box/Braille chars    →   Block 160-169, idx   (symbol_map lookup)
```

**Sprite PUA encoding**: Sprite symbols have no standard Unicode codepoint, so they use **Supplementary Private Use Area-A** (Plane 15) as artificial encoding:
```
Range: U+F0000 ~ U+F9FFF (40960 codepoints)
Encoding: 0xF0000 + block * 256 + idx
Blocks: 160 blocks × 256 symbols each
```

**No Unicode conflict**: Sprite PUA uses Plane 15 (U+F0000-U+F9FFF), completely separate from standard characters (Plane 0), Emoji (Plane 0+1), and CJK extensions (Plane 2). `compute_glyph()` can unambiguously determine type by codepoint range.

## GPU Rendering Pipeline (4-Stage)

Graphics mode uses a unified 4-stage pipeline (shared by desktop and web):

```
Stage 1: Data → RenderBuffer
  Buffer (TUI) + Layers (Sprites) → generate_render_buffer() → Vec<RenderCell>

Stage 2: RenderBuffer → RenderTexture
  draw_render_buffer_to_texture(rbuf, rt_index)
  Main scene → RT2,  Transition sources → RT0, RT1

Stage 3: RT Operations (optional)
  blend_rts(src1, src2, target, effect, progress)   # Transitions
  copy_rt(src, dst)

Stage 4: RT → Screen
  present(composites: &[RtComposite])
  Default: RT2 fullscreen + RT3 overlay
```

GPU shaders:
- **Symbols shader** — instanced rendering of glyphs from texture atlas
- **Transition shader** — blends two RTs with effects (dissolve, wipe, etc.)
- **General2D shader** — final composition to screen

## Texture Atlas

Single texture atlas, 256 blocks organized as:
```
Block 0-159:   Sprite glyphs (PETSCII, ASCII, custom)
Block 160-169: TUI characters
Block 170-175: Emoji
Block 176-239: CJK characters
```

One texture binding, one draw call, zero texture switching.

Atlas size is configurable per app. The default is 4096x4096. Apps that need fullscreen high-DPI rendering (e.g. MDPT) use 8192x8192 for crisper text. The engine auto-detects cell size from the atlas dimensions at startup. Different glyph regions have different cell sizes to match character widths:

```
               4096 atlas          8192 atlas
TUI cells:     16x32              32x64           (half-width)
CJK cells:     32x32              64x64           (full-width)
Sprite cells:  16x16              32x32
```

Even at 8192, fullscreen on high-DPI displays may show slight edge blurring — the tradeoff between atlas size (VRAM) and rendering sharpness.

## Asset System

Three custom formats + standard image/audio:
- `.pix` — PETSCII images (cells with color)
- `.ssf` — PETSCII animations (frame sequences)
- `.esc` — Terminal escape-sequence graphics

Loading flow:
```
AssetManager
├── Native: file I/O (sync)
└── Web: JavaScript fetch (async)

States: Loading → Parsing → Ready
```

## Event System

```rust
// Custom events (Model ↔ Render decoupling)
event_register("Block.RedrawTile", "draw_tile");
event_emit("Block.RedrawTile");
if event_check("Block.RedrawTile", "draw_tile") { ... }

// Timers
timer_register("Block.TestTimer", 0.1, "test_timer");
timer_fire("Block.TestTimer", 0);

// Input events
context.input_events: Vec<Event>  // Key, Mouse, Window
```

## UI Framework

Character-based UI system (`src/ui/`):

- **Widgets**: Label, Button, TextBox, Panel, List, Tree, ScrollBar, Table, etc.
- **Layout**: FreeLayout (manual positioning), VBoxLayout, HBoxLayout
- **UIPage**: Multi-page container with transition support
- **Theme**: Configurable styling

```rust
let mut panel = Panel::new()
    .with_bounds(Rect::new(0, 0, 80, 24))
    .with_layout(Box::new(FreeLayout));
panel.enable_canvas(80, 24);       // Direct buffer drawing
panel.add_child(Box::new(label));   // Widget children

let mut page = UIPage::new(80, 24);
page.set_root_widget(Box::new(panel));
page.start();
```

## app! Macro

`app!(Block)` generates all scaffolding:

```rust
use rust_pixel::app;
app!(Block);

// Expands to:
// - BlockGame struct wrapping Game<BlockModel, BlockRender>
// - init_game() and run() functions
// - WASM exports (new, tick, key_event, wasm_init_pixel_assets) for web
// - Conditional render module selection (render_terminal vs render_graphics)
```

## Creating a Project

```bash
# Create in apps/ subdirectory
cargo pixel c myapp

# Create standalone project
cargo pixel c myapp ..
cd ../myapp
```

### Project Structure

```
myapp/
├── src/
│   ├── main.rs              # Binary entry: calls myapp::run()
│   ├── lib.rs               # app!(MyApp) macro
│   ├── model.rs             # Game state and logic
│   ├── render_terminal.rs   # Terminal rendering
│   └── render_graphics.rs   # GPU rendering
├── lib/src/lib.rs           # Optional: core algorithms (for FFI/WASM reuse)
├── assets/                  # Game assets (.pix, .ssf, .png, ...)
└── Cargo.toml
```

### Running

```bash
cargo pixel r myapp t        # Terminal mode
cargo pixel r myapp wg       # WGPU desktop mode
cargo pixel r myapp w        # Web mode (localhost:8080)
cargo pixel r myapp wg -r    # Release build
```

## Coding Example

A minimal app with sprites, events, timers, and particles:

### Model (model.rs)

```rust
use rust_pixel::{
    context::Context,
    event::{event_emit, Event, KeyCode},
    game::Model,
    util::ParticleSystem,
};

pub const APPW: u16 = 80;
pub const APPH: u16 = 40;

pub struct MyModel {
    pub score: u32,
    pub pats: ParticleSystem,
}

impl Model for MyModel {
    fn init(&mut self, _ctx: &mut Context) {
        self.pats.fire_at(10.0, 10.0);
        event_emit("MyApp.Redraw");
    }

    fn handle_input(&mut self, ctx: &mut Context, _dt: f32) {
        let es = ctx.input_events.clone();
        for e in &es {
            if let Event::Key(key) = e {
                match key.code {
                    KeyCode::Char('n') => {
                        self.score += 1;
                        event_emit("MyApp.Redraw");
                    }
                    _ => {}
                }
            }
        }
        ctx.input_events.clear();
    }

    fn handle_auto(&mut self, _ctx: &mut Context, dt: f32) {
        self.pats.update(dt as f64);
    }

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}
}
```

### Render (render_graphics.rs)

```rust
use crate::model::{MyModel, APPW, APPH};
use rust_pixel::{
    asset2sprite,
    context::Context,
    event::{event_check, event_register, timer_fire, timer_register},
    game::{Model, Render},
    render::panel::Panel,
    render::sprite::Sprite,
    render::style::Color,
};

pub struct MyRender {
    pub panel: Panel,
}

impl MyRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();
        panel.add_sprite(Sprite::new(0, 0, APPW, APPH), "back");
        panel.add_sprite(Sprite::new(0, 0, 10, 5), "card");

        // Register event and timer
        event_register("MyApp.Redraw", "redraw");
        timer_register("MyApp.Timer", 0.1, "tick");
        timer_fire("MyApp.Timer", 0);

        Self { panel }
    }
}

impl Render for MyRender {
    type Model = MyModel;

    fn init(&mut self, ctx: &mut Context, data: &mut Self::Model) {
        ctx.adapter.init(APPW, APPH, 1.0, 1.0, "myapp".to_string());
        self.panel.init(ctx);
    }

    fn handle_event(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        if event_check("MyApp.Redraw", "redraw") {
            let card = self.panel.get_sprite("card");
            asset2sprite!(card, ctx, "card.pix");
            card.set_pos(5, 5);
        }
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _data: &mut Self::Model, _dt: f32) {
        if event_check("MyApp.Timer", "tick") {
            timer_fire("MyApp.Timer", 0);
        }
    }

    fn draw(&mut self, ctx: &mut Context, data: &mut Self::Model, _dt: f32) {
        // Draw particles
        self.panel.draw_objpool(&mut data.pats.particles, |sprite, particle| {
            sprite.set_pos(particle.obj.loc[0] as u16, particle.obj.loc[1] as u16);
        });
        self.panel.draw(ctx).unwrap();
    }
}
```

### Key Points

- **Model and Render should be loosely coupled** — use events for communication
- Use `#[cfg(any(feature = "wgpu", target_arch = "wasm32"))]` to differentiate graphics vs text mode
- Use `set_graph_sym()` for GPU glyph rendering, `set_default_str()` / `set_color_str()` for text
- `asset2sprite!` loads `.pix` / `.ssf` / `.esc` assets into sprites
- `draw_objpool()` manages drawing particle systems and pooled game objects

## Conditional Compilation

Build aliases (set in build.rs):
```
graphics_mode    = wgpu feature OR wasm32 target
wgpu_backend     = wgpu feature AND NOT wasm32
wgpu_web_backend = wasm32 target
cross_backend    = NOT graphics_mode (terminal)
```

Usage in code:
```rust
#[cfg(any(feature = "wgpu", target_arch = "wasm32"))]
{
    // Graphics-only code
    sprite.set_graph_sym(0, 0, 1, 83, Color::Indexed(14));
    sprite.set_alpha(200);
}

#[cfg(not(any(feature = "wgpu", target_arch = "wasm32")))]
{
    // Terminal-only code
    asset2sprite!(sprite, ctx, "back.txt");
}
```

## Source Layout

```
src/
├── game.rs                    # Game loop, Model/Render traits
├── context.rs                 # Shared runtime state
├── init.rs                    # Asset initialization, GameConfig
├── macros.rs                  # app! macro
├── event/                     # Event system, timers
├── render/
│   ├── adapter.rs             # Adapter trait
│   ├── adapter/
│   │   ├── cross_adapter.rs   # Terminal backend
│   │   ├── winit_wgpu_adapter.rs  # Desktop GPU backend
│   │   ├── wgpu_web_adapter.rs    # Web GPU backend
│   │   ├── winit_common.rs    # Shared window/input handling
│   │   └── wgpu/              # Shared WGPU pipeline
│   │       ├── pixel.rs       # Render texture management
│   │       ├── render_symbols.rs      # Instanced glyph shader
│   │       ├── render_transition.rs   # Transition effects
│   │       └── render_general2d.rs    # Final composition
│   ├── buffer.rs              # Cell buffer (BufferMode, diff tracking, set_str API)
│   ├── cell.rs                # Cell + Glyph (PUA encoding, glyph caching)
│   ├── scene.rs               # Scene container
│   ├── sprite/                # Sprite + Layer
│   ├── graph.rs               # Graphics data structures
│   └── effect.rs              # Transition types
├── ui/                        # UI framework
│   ├── widget.rs              # Widget trait
│   ├── app.rs                 # UIPage
│   ├── layout.rs              # Layout system
│   └── components/            # Button, Label, TextBox, List, ...
├── asset.rs                   # Asset loading (.pix, .ssf, .esc)
├── audio.rs                   # Audio playback
└── util/                      # Rect, math, particle system
```
