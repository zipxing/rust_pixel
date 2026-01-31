# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RustPixel is a 2D game engine and rapid prototyping toolkit supporting both text and graphics rendering modes. It can compile to native binaries, FFI libraries, and WASM for web deployment.

- **Text Mode**: Uses `crossterm` for terminal rendering with ASCII/Unicode/Emoji
- **Graphics Mode**: Uses `wgpu`, `glow`, or `sdl2` for hardware-accelerated rendering with PETSCII/custom symbols
- **Web Mode**: Compiles to WASM with WebGL backend

## Build Commands

### Using cargo-pixel CLI

First-time setup (install from crates.io):
```bash
cargo install rust_pixel
cargo pixel  # First run clones repo to ~/rust_pixel_work
```

Or install from local source:
```bash
cargo install --path . --force
```

### Running Applications

```bash
# Run in terminal mode
cargo pixel run <app> term
cargo pixel r <app> t  # shorthand

# Run in SDL/graphics mode
cargo pixel run <app> sdl
cargo pixel r <app> s  # shorthand

# Run in Glow mode (winit + OpenGL)
cargo pixel r <app> glow
cargo pixel r <app> g  # shorthand

# Run in WGPU mode (winit + wgpu)
cargo pixel r <app> wgpu
cargo pixel r <app> wg  # shorthand

# Run in web mode (starts server on localhost:8080)
cargo pixel run <app> web
cargo pixel r <app> w  # shorthand
cargo pixel r <app> w --webport 8081  # custom port
cargo pixel r <app> w -r  # release mode

# Examples
cargo pixel r snake t
cargo pixel r tetris s -r
cargo pixel r tower w
cargo pixel r petview g -r
```

### Creating New Applications

```bash
# Create app in ./apps/ using template
cargo pixel c mygame

# Create standalone app in different directory
cargo pixel c myapp ..
cd ../myapp
cargo pixel r myapp t
```

### Tools

```bash
# palette: Terminal UI color tool
cargo pixel r palette t -r

# edit: Character art editor
cargo pixel edit term . assets/logo.txt
cargo pixel e t . assets/logo.txt
cargo pixel edit sdl . assets/logo.pix
cargo pixel e s . assets/logo.pix

# petii: Convert images to PETSCII art
cargo pixel p assets/image.png > output.pix
cargo pixel p assets/image.png 40 25 > output.pix

# ssf: Preview PETSCII animations
cargo pixel ssf . assets/animation.ssf

# cg: Convert GIF to PETSCII animation
cargo pixel cg assets/input.gif assets/output.ssf 40 25

# symbol: Generate symbol atlas from image
cargo pixel sy image.png 8

# ttf: Generate font texture from TTF
cargo pixel tf font.ttf output.png 8
```

### Testing

Run tests for specific apps or the core library:
```bash
# Test core library
cargo test

# Test specific app
cd apps/<app_name>
cargo test
```

## Architecture

### Core Design Pattern: Model-Render-Game

Every RustPixel application follows this pattern:

```
Game (orchestrator)
├── Model (game logic and state)
│   ├── init()
│   ├── update()
│   ├── handle_event()
│   ├── handle_timer()
│   ├── handle_input()
│   └── handle_auto()
└── Render (visual output)
    ├── init()
    ├── draw()
    ├── handle_event()
    └── handle_timer()
```

### Project Structure

```
rust_pixel/
├── src/                    # Core engine code
│   ├── algorithm/          # Pathfinding, disjoint-set, etc.
│   ├── event/              # Event system and timers
│   ├── render/             # Rendering subsystem
│   │   ├── adapter/        # Platform-specific adapters
│   │   │   ├── cross_adapter.rs      # Terminal mode
│   │   │   ├── sdl_adapter.rs        # SDL2 mode
│   │   │   ├── winit_glow_adapter.rs # Winit + OpenGL
│   │   │   ├── winit_wgpu_adapter.rs # Winit + WGPU
│   │   │   └── web_adapter.rs        # WASM/WebGL
│   │   ├── buffer.rs       # Screen buffer
│   │   ├── cell.rs         # Base rendering unit
│   │   ├── panel.rs        # Layered drawing surface
│   │   └── sprite.rs       # Sprite management
│   ├── ui/                 # Terminal UI framework
│   ├── util/               # Common utilities
│   ├── asset.rs            # Asset loading (.pix, .ssf, .esc)
│   ├── audio.rs            # Audio playback
│   ├── context.rs          # Shared runtime state
│   ├── game.rs             # Game loop implementation
│   ├── init.rs             # Asset initialization (GameConfig, texture loading)
│   ├── macros.rs           # app! macro for scaffolding applications
│   └── lib.rs              # Main entry point and re-exports
├── apps/                   # Demo games and applications
│   ├── template/           # Template for new apps
│   ├── tetris/
│   ├── snake/
│   ├── poker/
│   │   ├── ffi/            # C FFI bindings example
│   │   └── wasm/           # WASM bindings example
│   └── ...
├── tools/                  # Utilities
│   ├── cargo-pixel/        # CLI build tool
│   ├── asset/              # Asset packer
│   ├── edit/               # Character art editor
│   ├── petii/              # Image to PETSCII converter
│   ├── ssf/                # Animation format tool
│   └── ...
└── Cargo.toml              # Workspace configuration
```

### Application Structure

Each app follows this layout:

```
apps/my_game/
├── src/
│   ├── lib.rs              # Uses app! macro
│   ├── main.rs             # Native entry point
│   ├── model.rs            # Game state and logic
│   ├── render_terminal.rs  # Terminal rendering
│   └── render_graphics.rs  # Graphics rendering
├── lib/                    # Optional: core algorithms as library
│   └── src/
│       └── lib.rs
├── ffi/                    # Optional: C FFI bindings
├── wasm/                   # Optional: WASM bindings
├── assets/                 # Game assets
├── build.rs                # Build script for asset embedding
└── Cargo.toml
```

### Key Abstractions

**Panel**: Multi-layer drawing surface supporting both text and graphics modes
- Contains multiple layers, each holding sprites
- Handles z-order rendering and dirty region tracking
- Unified API across all rendering backends

**Sprite**: Positioned drawable element
- Contains a Buffer with Cells
- Can be static or animated
- Supports pixel-perfect positioning in graphics mode

**Cell**: Smallest rendering unit
- In text mode: a Unicode character with foreground/background colors
- In graphics mode: a glyph from a texture atlas (PETSCII, ASCII, or custom)

**Buffer**: 2D array of Cells with efficient diff-based updates

**Context**: Shared runtime state passed through the game loop
- Contains the active Adapter
- Manages input events, timers, and global state
- Provides access to AssetManager for loading resources

### Rendering Adapters

Each adapter implements platform-specific rendering:

- **CrossAdapter** (terminal): Direct terminal I/O with crossterm
- **SdlAdapter** (SDL2): Hardware-accelerated OpenGL via SDL2
- **WinitGlowAdapter** (Winit + Glow): Portable OpenGL via winit
- **WinitWgpuAdapter** (WGPU): Modern GPU API (Vulkan/Metal/DX12)
- **WebAdapter** (Web): WebGL via WASM

All adapters expose the same `Adapter` trait interface, allowing games to work across platforms without modification.

### GPU Rendering Pipeline (4-Stage Architecture)

Graphics mode adapters use a unified 4-stage rendering pipeline:

```
┌─────────────────────────────────────────────────────────────────┐
│ Stage 1: Data Sources → RenderBuffer                            │
│   Buffer (TUI) ─┬─→ generate_render_buffer() → Vec<RenderCell>  │
│   Layers (Sprites) ─┘                                           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Stage 2: RenderBuffer → RenderTexture (RT)                      │
│   draw_render_buffer_to_texture(rbuf, rt_index, debug)          │
│   - Main scene → RT2                                            │
│   - Transition sources → RT0, RT1                               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Stage 3: RT Operations (Optional)                               │
│   blend_rts(src1, src2, target, effect, progress)               │
│   copy_rt(src, dst)                                             │
│   clear_rt(rt)                                                  │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Stage 4: RT[] → Screen                                          │
│   present(composites: &[RtComposite])                           │
│   present_default()  // RT2 fullscreen + RT3 overlay            │
└─────────────────────────────────────────────────────────────────┘
```

**RenderTexture (RT) Usage Convention:**
- **RT0**: Source image 1 (for transitions)
- **RT1**: Source image 2 (for transitions)
- **RT2**: Main scene content (TUI + Sprites)
- **RT3**: Overlay layer (transition results, effects)

**Key APIs:**
```rust
// Stage 1: Generate render data
let rbuf = generate_render_buffer(current_buf, prev_buf, layers, stage, base);

// Stage 2: Render to texture
adapter.draw_render_buffer_to_texture(&rbuf, 2, false);  // Main scene to RT2

// Stage 3: RT operations (optional, for transitions)
adapter.blend_rts(0, 1, 3, effect_type, progress);  // Blend RT0+RT1 → RT3

// Stage 4: Output to screen
adapter.present_default();  // Or: adapter.present(&[...composites])
```

**RtComposite Configuration:**
```rust
RtComposite {
    rt: usize,           // RT index (0-3)
    viewport: Option<Rect>,  // None = fullscreen
    blend: BlendMode,    // Normal, Add, Multiply, Screen
    alpha: u8,           // 0-255 transparency
}
```

### The app! Macro

The `app!` macro in `src/macros.rs` scaffolds the application entry point:
- Generates the `{Name}Game` struct wrapping `Game<Model, Render>`
- Creates `init_game()` and `run()` functions
- Provides WASM exports (`new()`, `tick()`, `key_event()`, `wasm_init_pixel_assets()`) for web builds
- Conditionally compiles `render_terminal` or `render_graphics` modules based on build features

Example usage in app's `lib.rs`:
```rust
use rust_pixel::app;
app!(MyGame);
```

### Features and Conditional Compilation

```toml
# Cargo.toml features
default = ["log4rs", "crossterm", "rodio", "image"]
term = ["log4rs", "crossterm", "rodio", "image"]
sdl = ["log4rs", "rodio", "sdl2", "image"]
glow = ["log4rs", "rodio", "winit", "glutin", "glutin-winit", "raw-window-handle", "image"]
wgpu = ["log4rs", "rodio", "wgpu", "bytemuck", "pollster", "winit", "glutin", "glutin-winit", "raw-window-handle", "image"]
web = ["image"]
base = ["log4rs"]  # Minimal build for FFI/WASM libraries
```

The `cargo-pixel` tool automatically sets these features based on the mode argument.

### Asset System

RustPixel uses custom asset formats:
- **.pix**: PETSCII images (binary format with color info)
- **.ssf**: PETSCII animations (sequence of .pix frames)
- **.esc**: Escape-sequence based terminal graphics
- Standard formats: .png, .jpg, .mp3 for audio

Assets are loaded via `AssetManager` in `Context`, which supports both synchronous and asynchronous loading for WASM compatibility.

### Event System

Two event mechanisms:
1. **Input events**: Keyboard, mouse, window events stored in `context.input_events`
2. **Timer events**: Global timer system via `event::timer_register()` / `timer_update()`

Events are processed in the Model's `handle_event()` and `handle_timer()` methods.

## Development Workflow

### Adding a New Game

1. Create from template:
   ```bash
   cargo pixel c my_game
   ```

2. Implement game logic in `apps/my_game/src/`:
   - `model.rs`: Define state struct and implement `Model` trait
   - `render_terminal.rs`: Implement terminal rendering
   - `render_graphics.rs`: Implement graphics rendering
   - `lib.rs`: Call `pixel_game!(MyGame)` macro

3. Add assets to `apps/my_game/assets/`

4. Test in different modes:
   ```bash
   cargo pixel r my_game t
   cargo pixel r my_game s
   cargo pixel r my_game w
   ```

### Building for Web

Apps with WASM support have a `wasm/` directory:

```bash
cd apps/my_game/wasm
make run  # Builds WASM and starts local server
```

The WASM build uses `wasm-pack` and integrates with JavaScript via the web template in `web-templates/`.

### Creating FFI Bindings

Apps with FFI support have an `ffi/` directory with:
- `cbindgen.toml`: Configuration for C header generation
- `Makefile`: Build C/Python examples

```bash
cd apps/poker/ffi
make run  # Builds library and runs C++/Python examples
```

### Extending the Engine

To add a new rendering adapter:
1. Create `src/render/adapter/my_adapter.rs`
2. Implement the `Adapter` trait
3. Add feature flags in `Cargo.toml`
4. Update `cargo-pixel` to support the new mode

To add new asset formats:
1. Add loading logic to `src/asset.rs`
2. Update `AssetManager` to handle the format
3. Add conversion tools to `tools/` if needed

## Notes

- The main branch is `main`
- Minimum Rust version: 1.71+
- The engine uses a fixed 60 FPS game loop (configurable via `GAME_FRAME` constant)
- Graphics mode supports PETSCII character sets and custom symbol atlases
- The terminal UI system works in both terminal and graphics modes
- All rendering is done through the unified Panel/Sprite/Buffer abstraction
