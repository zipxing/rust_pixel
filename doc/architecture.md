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
Cell { symbol, fg, bg, modifier, scale_x, scale_y, tile (graphics_mode only) }
```

### Cell and Tile

`Cell` is the fundamental rendering unit. The `symbol` string fully determines what gets rendered.

```rust
pub struct Cell {
    pub symbol: String,    // Fully determines rendering
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    pub scale_x: f32,
    pub scale_y: f32,
    #[cfg(graphics_mode)]
    tile: Tile,            // Cached tile info (resolved from symbol)
}
```

**Tile caching**: `set_symbol()` automatically calls `compute_tile()`, caching the Tile in the Cell. Rendering reads the cached tile directly — no symbol map lookup at render time.

`Tile` describes how to render a symbol using mipmap textures:
```rust
pub struct MipUV {
    pub layer: u16,  // Layer index in Texture2DArray
    pub x: f32,      // Normalized UV x (0.0-1.0)
    pub y: f32,      // Normalized UV y (0.0-1.0)
    pub w: f32,      // Normalized UV width
    pub h: f32,      // Normalized UV height
}

pub struct Tile {
    pub cell_w: u8,       // Cell width (1=normal, 2=wide like CJK/Emoji)
    pub cell_h: u8,       // Cell height (1=single, 2=tall like TUI/CJK)
    pub is_emoji: bool,   // Pre-rendered emoji (no color modulation)
    pub mips: [MipUV; 3], // 3 mipmap levels: [high, mid, low]
}
```

Cell sizes in grid units:
- **Sprite**: cell_w=1, cell_h=1
- **TUI**: cell_w=1, cell_h=2
- **Emoji**: cell_w=2, cell_h=2
- **CJK**: cell_w=2, cell_h=2

**Mipmap selection**: At render time, the engine calculates actual pixel size and selects the appropriate mip level:
- `per_unit >= 48` → mip0 (high resolution, for fullscreen)
- `per_unit >= 24` → mip1 (medium resolution)
- `per_unit < 24` → mip2 (low resolution, for small windows)

**Symbol lookup**: `LayeredSymbolMap` maps symbol strings to `Tile`. Lookup order: PUA Sprite → Emoji → CJK → TUI → fallback (space)

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
- Rendering uses `LayeredSymbolMap` to look up `Tile` with mipmap UV coordinates

### Symbol → Tile Mapping

`LayeredSymbolMap` (loaded from `layered_symbol_map.json`) maps symbol strings directly to `Tile`:

```
cell.symbol (String)         →    Tile { cell_w, cell_h, is_emoji, mips[3] }
─────────────────────────         ─────────────────────────────────────────
"A", "█", "─", "⠿"           →    TUI tile (cell_w=1, cell_h=2)
"中", "国"                    →    CJK tile (cell_w=2, cell_h=2)
"😀", "🎮"                    →    Emoji tile (cell_w=2, cell_h=2, is_emoji=true)
PUA "\u{F0000}"...           →    Sprite tile (cell_w=1, cell_h=1)
```

**Sprite PUA encoding**: Sprite symbols use **Supplementary Private Use Area-A** (Plane 15):
```
Range: U+F0000 ~ U+F9FFF (40960 codepoints)
Encoding: 0xF0000 + block * 256 + idx
Blocks: 160 blocks × 256 symbols each
```

**No Unicode conflict**: PUA Plane 15 is completely separate from standard characters (Plane 0), Emoji (Plane 0+1), and CJK extensions (Plane 2).

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
- **Symbols shader** — instanced rendering of tiles from Texture2DArray
- **Transition shader** — blends two RTs with effects (dissolve, wipe, etc.)
- **General2D shader** — final composition to screen

## Texture2DArray Architecture

Layered texture system using GPU Texture2DArray:
- Multiple 4096×4096 layers packed with symbols
- 3-level mipmaps for different display scales (mip0/mip1/mip2)
- Sprite, TUI, Emoji, CJK symbols all in the same array
- Single texture binding, instanced rendering, one draw call

The engine auto-selects mipmap level based on actual render size, ensuring crisp rendering from small windows to fullscreen high-DPI displays.

## Asset System

Three custom formats + standard image/audio:
- `.pix` — PETSCII images (cells with color)
- `.ssf` — PETSCII animations (frame sequences)
- `.esc` — Terminal escape-sequence graphics

### Asset Loading: Native vs WASM

| Aspect | Native (Desktop WGPU) | WASM (Web) |
|--------|----------------------|-------------|
| Entry point | `main()` → `run()` | JS `import("./pkg/pixel.js")` |
| File I/O | `std::fs::read` (sync) | `fetch()` (async) |
| Texture layers | Rust `image::open()` directly | JS decodes PNG → extracts RGBA → copies into WASM memory |
| Symbol map | Rust reads JSON file | JS fetches JSON text → passes into WASM |
| App content (e.g., MD) | `std::fs::read_to_string()` | URL param `?data=` → JS fetch → `WASM_APP_DATA` |
| WGPU init | Synchronous (`pollster::block_on`) | **Async** (`await init_from_cache_async()`) |
| Runtime assets (.pix/.ssf) | Sync `fs::read` | JS fetch → global queue → drained per tick |

### WASM Loading Timeline

The entire flow is **orchestrated by JavaScript**; the Rust/WASM side passively receives data.

```
index.js                                      Rust (WASM)
────────                                      ───────────
1. import("./pkg/pixel.js")
   └─ Load compiled .wasm module

2. fetch("assets/pix/layered_symbol_map.json")
   └─ Get JSON text, parse to obtain layer_files[]

3. Promise.all(layer_files.map(fetch))
   ├─ fetch("assets/pix/layers/layer_0.png")
   ├─ fetch("assets/pix/layers/layer_1.png")     Parallel download
   └─ ...

   For each PNG:
   ├─ createImageBitmap(blob)
   ├─ OffscreenCanvas.drawImage(bitmap)
   └─ getImageData() → Uint8Array (RGBA)

4. Concatenate all layers into one buffer:
   allLayerData = concat(layer0, layer1, ...)

5. wasm_init_pixel_assets(                    ──→  init_pixel_assets_from_wasm()
     "app_name",                                    ├─ Parse symbol_map JSON
     layer_size,                                     ├─ Split concatenated buffer
     layer_count,                                    │   by bytes_per_layer → Vec<Vec<u8>>
     allLayerData,   // concatenated RGBA            ├─ init_layered_symbol_map_from_json()
     symbolMapJson   // JSON string                  └─ init_pixel_assets_inner()
   )                                                     → cache in PIXEL_LAYER_DATA global

6. fetch(urlParams.get("data"))               ──→  wasm_set_app_data(text)
   // e.g., ?data=assets/demo.md                    → WASM_APP_DATA.set(text)

7. PixelGame.new()                            ──→  init_game()
                                                     ├─ [SKIP] init_layered_pixel_assets
                                                     │   (already done by JS in step 5)
                                                     ├─ Model::new() + model.init()
                                                     └─ Render::new()

8. await sg.init_from_cache()                 ──→  WgpuWebAdapter::init_wgpu_from_cache_async()
                                                     ├─ Get <canvas> element
                                                     ├─ wgpu::Instance::new(WEBGPU | GL)
                                                     ├─ await request_adapter()
                                                     ├─ await request_device()
                                                     ├─ surface.configure()
                                                     ├─ with_pixel_layer_data() → build_layered()
                                                     │   └─ Texture2DArray upload
                                                     └─ clear_pixel_layer_data()

9. requestAnimationFrame loop                 ──→  sg.tick(dt)
     60 FPS                                          ├─ process_queued_assets()  // drain queue
                                                     └─ game.on_tick(dt)
```

### Base Texture Loading

**Native** (`src/init.rs` — `init_layered_pixel_assets()`):

1. Locate pix directory (try `{project_path}/assets/pix/`, fall back to `./assets/pix/`).
2. Read `layered_symbol_map.json` via `std::fs::read_to_string()`.
3. For each layer PNG in `layer_files`, call `image::open()` → `.to_rgba8()` → `.into_raw()`.
4. Cache the raw RGBA `Vec<Vec<u8>>` into the `PIXEL_LAYER_DATA` global.

**WASM** (`web-templates/index.js` → `src/init.rs` — `init_pixel_assets_from_wasm()`):

1. JS fetches `layered_symbol_map.json` and parses it to obtain `layer_files`.
2. JS fetches all layer PNGs **in parallel** using `Promise.all`.
3. For each PNG, JS decodes via `createImageBitmap()` + `OffscreenCanvas`, then calls `getImageData()` to extract raw RGBA pixels.
4. All layer pixel data is **concatenated** into a single `Uint8Array` and passed across the JS/WASM boundary in one call.
5. On the Rust side, the concatenated buffer is split back into per-layer `Vec<Vec<u8>>` by `bytes_per_layer = layer_size * layer_size * 4`.
6. The result is cached in the same `PIXEL_LAYER_DATA` global.

**Why concatenate then split?** A single `Uint8Array` crossing the JS/WASM boundary is far more efficient than multiple transfers (only one memory copy).

### GPU Initialization and Texture Upload

**Native** (`src/render/adapter/winit_wgpu_adapter.rs`):

Everything is synchronous. `pollster::block_on()` drives the async WGPU calls:

```rust
fn create_wgpu_window_and_resources(&mut self) {
    let adapter = pollster::block_on(instance.request_adapter(...));
    let (device, queue) = pollster::block_on(adapter.request_device(...));
    with_pixel_layer_data(|data| {
        builder.build_layered(device, queue, data.layer_size, &layer_refs)
    });
    clear_pixel_layer_data();  // free CPU memory
}
```

**WASM** (`src/render/adapter/wgpu_web_adapter.rs`):

Browser APIs are inherently async. The Game object is created **before** the GPU is ready:

```rust
pub async fn init_wgpu_from_cache_async(&mut self) {
    let adapter = instance.request_adapter(&opts).await?;
    let (device, queue) = adapter.request_device(&desc).await?;
    with_pixel_layer_data(|data| {
        builder.build_layered(device, queue, data.layer_size, &layer_refs)
    });
    clear_pixel_layer_data();  // free CPU memory
}
```

JS awaits this: `await sg.init_from_cache()`. The inverted ordering (Game created → GPU initialized later) is the opposite of native mode.

Both paths converge at `WgpuRenderCoreBuilder::build_layered()` which calls `WgpuTextureArray::from_layers()` to upload layers into a GPU Texture2DArray via `queue.write_texture()` per layer.

### CPU-Side Layer Data Lifecycle

Both native and WASM paths cache raw RGBA layer data in `PIXEL_LAYER_DATA` (a global `OnceLock<Mutex<PixelLayerData>>`) before the GPU is ready. For a 4096×4096×6-layer texture set, this is ~384 MB of CPU memory — only needed during the one-time GPU upload.

```
Phase 1: Cache layer data
  ├─ Native: image::open() → Vec<Vec<u8>> → PIXEL_LAYER_DATA
  └─ WASM:   JS fetch + decode → wasm_init_pixel_assets() → PIXEL_LAYER_DATA

Phase 3: GPU upload + immediate release
  ├─ with_pixel_layer_data(|data| { build_layered(...) })
  │   └─ WgpuTextureArray::from_layers() uploads to GPU while lock is held
  └─ clear_pixel_layer_data()
      └─ Clears Vec<Vec<u8>> inside the Mutex → frees ~384 MB CPU memory

Resize: Texture reuse (no CPU data needed)
  ├─ old_core.take_symbol_texture_array() → extracts WgpuTextureArray
  └─ builder.build_with_texture(device, queue, tex_array) → reuses GPU texture
      └─ Only pipelines, buffers, and render textures are recreated
```

Key API:

- **`with_pixel_layer_data(closure)`**: Accesses the cached data under a `Mutex` lock. Returns `None` if already cleared.
- **`clear_pixel_layer_data()`**: Called immediately after the first GPU upload. Clears the inner `Vec` while the `OnceLock` shell remains.
- **`build_with_texture()`**: `WgpuRenderCoreBuilder` method that accepts an existing `WgpuTextureArray`. Used by `rebuild_render_core()` during window resize/maximize/fullscreen toggle.
- **`take_symbol_texture_array()`**: Extracts the `WgpuTextureArray` from the old `WgpuRenderCore` before it is dropped, so the GPU texture survives the rebuild.

### Runtime Asset Loading (.pix / .ssf / .esc)

**Native**: Fully synchronous, ready in the same frame:

```rust
let data = std::fs::read(&path)?;
asset.set_data(&data);
asset.parse()?;  // immediately available
```

**WASM**: Asynchronous with a global queue to avoid borrow conflicts:

```
Request:    asset_manager.load("image.pix")
              ↓  triggers JS fetch
            fetch("assets/image.pix")
              ↓  callback fires
            wasm_on_asset_loaded(url, data)  ──→  ASSET_QUEUE.push((url, data))
              ↓  next tick()
            process_queued_assets()
              ↓
            asset.set_data() + parse()       ← resource ready (1+ frames later)
```

The queue mechanism (`src/asset.rs`):

```rust
#[cfg(target_arch = "wasm32")]
thread_local! {
    static ASSET_QUEUE: RefCell<Vec<(String, Vec<u8>)>> = RefCell::new(Vec::new());
}
```

Drained every frame in `tick()`:

```rust
#[cfg(target_arch = "wasm32")]
self.g.context.asset_manager.process_queued_assets();
```

**Key implication**: Runtime-loaded assets in WASM are **not available on the same frame** they are requested. There is at least a 1-frame delay (typically more, depending on network latency).

### Pix Resource Search Path

The `assets/pix/` directory contains the layered texture files (`layers/*.png`) and symbol map (`layered_symbol_map.json`). The engine uses a fallback mechanism to support both workspace apps and standalone projects.

**Search Order:**
1. `{app_path}/assets/pix/` — App-specific (if exists)
2. `./assets/pix/` — Shared root directory (fallback)

**Workspace Layout (rust_pixel/):**
```
rust_pixel/
├── assets/pix/                    # Shared pix resources
│   ├── layers/
│   │   ├── layer_0.png
│   │   └── ...
│   └── layered_symbol_map.json
├── apps/
│   ├── mdpt/
│   │   └── assets/               # App assets (no pix/)
│   ├── tetris/
│   │   └── assets/
│   └── ...
```

All workspace apps share root `assets/pix/`. No duplication needed.

**Standalone Project:**
```
my_game/
└── assets/
    ├── pix/                       # Must include pix/ for standalone
    │   ├── layers/
    │   └── layered_symbol_map.json
    └── ...                        # Other app assets
```

**cargo pixel Commands:**

| Command | Mode | Pix Loading |
|---------|------|-------------|
| `cargo pixel r app t` | Terminal | No pix needed |
| `cargo pixel r app g` | Desktop GPU | Runtime fallback (app → root) |
| `cargo pixel r app w` | Web | Build-time copy (app → root fallback) |

**Web Build Process:**

`cargo pixel r app w` copies assets to `tmp/web_app/`:
1. Copy app's `assets/` directory
2. If `assets/pix/` missing, copy from root `assets/pix/`
3. Start local HTTP server

**Deployment:**

Option 1: Assets alongside executable (default):
```
deploy/
├── my_game(.exe)
└── assets/
    ├── pix/
    │   ├── layers/
    │   └── layered_symbol_map.json
    └── ...
```

Option 2: Specify asset path via command line:
```bash
./my_game /path/to/project    # Looks for /path/to/project/assets/pix/
./my_game .                   # Current directory (default)
```

The first non-flag argument is used as project path. Flags like `-f` (fullscreen) are filtered out.

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
- Use `set_graph_sym()` for GPU sprite rendering, `set_default_str()` / `set_color_str()` for text
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
│   │       ├── render_symbols.rs      # Instanced tile shader
│   │       ├── render_transition.rs   # Transition effects
│   │       └── render_general2d.rs    # Final composition
│   ├── buffer.rs              # Cell buffer (BufferMode, diff tracking, set_str API)
│   ├── cell.rs                # Cell (PUA encoding for sprites)
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
