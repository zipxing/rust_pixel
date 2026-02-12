# 2.1.0

- MDPT and all apps now support fullscreen mode
- Removed SDL and Glow (OpenGL) backend adapters, keeping only WGPU and terminal mode
- Web mode now uses WGPU (with WebGL fallback) instead of Glow
- Auto-generated cover slide for MDPT based on YAML front matter (title, author, theme)
- Added CJK character support in GPU texture atlas
- Fixed command line argument parsing for project path and app-specific args
```
cargo pixel r mdpt wg -r apps/mdpt/ demo.md
```

---

# 2.0.0 - MDPT: A TUI Without a Terminal Emulator

## 🎯 Killer App: MDPT (Markdown Presentation Tool)

**A Markdown-first presentation toolkit with a self-rendered TUI.**

MDPT demonstrates RustPixel's unique capability: rendering a full-featured terminal UI in a native GPU window, completely independent of any terminal emulator.

### Why MDPT?

Unlike terminal-based presenters (presenterm, slides), MDPT:
- **No terminal emulator** — Runs in a native window with GPU rendering
- **Consistent rendering** — Same look across all platforms
- **Rich transitions** — GPU shader effects impossible in terminals
- **True graphics** — Not limited by terminal cell constraints

### Features

| Feature | Description |
|---------|-------------|
| **GPU-Accelerated Transitions** | 6 transition effects (dissolve, circle, wipe, etc.) powered by shaders |
| **Code Highlighting** | 100+ languages with dynamic line-by-line reveal `{1-4\|6-10\|all}` |
| **Text Animations** | Spotlight, Wave, FadeIn, Typewriter effects |
| **Charts** | Line charts, bar charts, pie charts, Mermaid diagrams |
| **Column Layouts** | Flexible multi-column content arrangement |
| **PETSCII/SSF Images** | Native pixel-art and animation support |
| **Full CJK Support** | Chinese, Japanese, Korean text rendering |
| **Incremental Display** | Step-by-step content reveal with pause markers |

### Usage

```bash
cargo pixel r mdpt wg -r . assets/demo.md  # WGPU mode
cargo pixel r mdpt t -r . assets/demo.md   # Terminal mode
cargo pixel r mdpt w -r                    # Web mode
```

## 🎮 pixel_basic: Built-in BASIC Interpreter

**Write games in familiar BASIC syntax — perfect for beginners or quick prototyping!**

### Features

- **Classic BASIC syntax** with line numbers
- **Game hooks**: `ON_INIT (1000)`, `ON_TICK (2000)`, `ON_DRAW (3500)`
- **Graphics**: `PLOT x, y, char, fg, bg` / `BOX` / `CLS`
- **Input**: `KEY("W")`, `KEY("SPACE")`
- **Arrays**: `DIM arr(100)`
- **Control flow**: `GOTO`, `GOSUB/RETURN`, `FOR/NEXT`, `IF/THEN`
- **Math**: `RND()`, `INT()`, `ABS()`
- **Strings**: `STR$()`, `LEN()`, `MID$()`
- **Coroutines**: `YIELD`, `WAIT` — write game logic naturally

### Example

```basic
10 REM SNAKE GAME
20 X = 20: Y = 10
30 DIM BX(100): DIM BY(100)
40 YIELD
50 GOTO 40

1000 REM ON_INIT
1010 BOX 0, 0, 60, 24, 1
1020 RETURN

2000 REM ON_TICK
2010 IF KEY("W") THEN DY = -1: DX = 0
2020 X = X + DX: Y = Y + DY
2030 RETURN
```

### Usage

```bash
cargo pixel r basic_snake t      # Run BASIC Snake game
```

See `pixel_basic/` for the interpreter source and `apps/basic_snake/` for a complete example.

## 🎨 Logo Animation Enhancement

### Jitter Hold Frames

Improved logo animation with frame-holding behavior for jitter effects:

- **Jitter persists for 4 frames** instead of changing every frame
- Uses deterministic random seeds based on held stage
- Smoother visual experience with less flickering

```rust
const JITTER_HOLD_FRAMES: u32 = 4;
let held_stage = stage / JITTER_HOLD_FRAMES;
```

### Embedded Logo Data

Logo data now embedded in source code for better standalone project compatibility:

- **New `logo_data.rs`** module contains logo pixel data as static string
- **No external file dependency** — Eliminates `include_str!` compilation errors
- Works reliably in standalone projects without `assets/logo.pix`

## 📦 Charts & Diagrams in MDPT

New chart rendering capabilities using pure character graphics:

- **Line Charts** — Braille dot matrix or ASCII line rendering
- **Bar Charts** — Block characters (▁▂▃▄▅▆▇█) for vertical bars
- **Pie Charts** — Braille dot matrix for circular sectors
- **Mermaid Flowcharts** — `graph TD/LR` support with box drawing

```markdown
\`\`\`linechart
title: Monthly Revenue
x: [Jan, Feb, Mar, Apr, May]
y: [120, 200, 150, 300, 280]
\`\`\`
```

## 🔄 Migration

- Version bump from 1.0.8 to 2.0.0
- All existing apps remain compatible
- MDPT is a new optional app, no breaking changes

---

# 1.0.8 - Texture Upgrade, Code Refactoring & Color Enhancement

## 🖼️ Texture System Upgrade

### **4096x4096 Symbol Texture**
Upgraded from 2048x2048 to 4096x4096 texture atlas:

- **10 Sprite Rows** layout for better organization
- **Chinese Character Support** - CJK glyphs in dedicated region
- **Emoji Support** - Full emoji rendering capability
- **TUI Region** - Dedicated area for terminal UI symbols
- **symbol_map.json** - New mapping file for dynamic symbol lookup

```
4096x4096 Texture Layout:
┌────────────────────────────────┐
│ Row 0-3: PETSCII/ASCII symbols │
│ Row 4-5: Custom game sprites   │
│ Row 6-7: TUI components        │
│ Row 8-9: CJK/Emoji glyphs      │
└────────────────────────────────┘
```

### **Unified Asset Loading**
New unified initialization flow for both native and WASM:

- `init_pixel_assets()` - Loads texture + symbol_map at startup
- `wasm_init_pixel_assets()` - JS passes texture data to WASM
- Texture data cached in `PIXEL_TEXTURE_DATA` for deferred GPU upload

## 🔧 Code Refactoring

### **Modular Architecture**
Reorganized core modules for better maintainability:

- **`src/init.rs`** (NEW): Asset initialization module
  - `GameConfig` - Global game configuration
  - `PixelTextureData` - Texture data caching
  - `init_pixel_assets()` - Native graphics mode initialization
  - `wasm_init_pixel_assets()` - WASM mode initialization

- **`src/macros.rs`** (NEW): Application scaffolding macro
  - `app!` macro (renamed from `pixel_game!`)
  - Cleaner, more concise macro implementation

### **Macro Rename: `pixel_game!` → `app!`**
```rust
// Old (deprecated)
use rust_pixel::pixel_game;
pixel_game!(MyGame);

// New
use rust_pixel::app;
app!(MyGame);
```

## 🎨 Color Enhancement

### **Improved Graphics Mode Colors**
Updated ANSI 16-color palette for better contrast in graphics mode:

| Color | Old Value | New Value |
|-------|-----------|-----------|
| Red | `#800000` | `#CD3131` |
| Green | `#008000` | `#0DBC79` |
| Yellow | `#808000` | `#E5E510` |
| Blue | `#000080` | `#2472C8` |
| Magenta | `#800080` | `#BC3FBC` |
| Cyan | `#008080` | `#11A8CD` |

Colors now match modern terminal color schemes (VS Code style).

## 🔄 Migration

- All 12 demo apps updated to use `app!` macro
- **Breaking Change**: `pixel_game!` macro removed
- Replace `use rust_pixel::pixel_game;` with `use rust_pixel::app;`

---

# 1.0.7 - Major Release: RustPixel UI Framework

## 🎨 What's New

### **RustPixel UI Framework** (NEW)
Complete character-based UI system for building sophisticated applications:

- **7 Core Components**: Label, Button, TextBox, Panel, List, Tree, ScrollBar
- **Advanced Layout**: Automatic positioning, constraint-based sizing
- **Event System**: Mouse/keyboard handling with widget-specific events
- **Theme Support**: Configurable styling and color schemes
- **Demo App**: `apps/ui_demo` showcasing all features

```rust
// Quick Start Example
use rust_pixel::ui::*;

let mut app = UIApp::new(80, 25);
let mut panel = Panel::new();

panel.add_child(Box::new(Label::new("Hello UI!")));
panel.add_child(Box::new(Button::new("Click Me")));

app.set_root(Box::new(panel));
app.run();
```

### **Enhanced Graphics**
- **Sprite Scaling**: Independent X/Y scaling with `set_scale_x()`, `set_scale_y()`
- **TTF Tool Improvements**: Auto-discovery, smart filtering, Unicode support

## 🚀 Use Cases
- **Editors**: Code editors, text viewers with syntax highlighting
- **Development Tools**: Debug consoles, file browsers, config panels
- **Game UI**: Menus, inventory systems, dialogue interfaces
- **Terminal Apps**: Modern CLI tools with mouse support

## 📁 Key Files
- `src/ui/` - Complete UI framework module
- `apps/ui_demo/` - Comprehensive demonstration app
- Enhanced sprite rendering and TTF processing tools

## 🔄 Migration
- **100% Backward Compatible** - Existing code unchanged
- **Additive Features** - New UI framework is optional
- **Zero Breaking Changes** - All existing apps continue to work

---

**Impact**: This release establishes RustPixel as a complete platform for building sophisticated character-based applications, from simple tools to complex editors and games.

**Note**: The new sprite scaling and TTF tool enhancements are specifically designed to support ASCII half-width font rendering for UI elements in graphics mode, enabling crisp terminal-style interfaces with pixel-perfect scaling.

# 1.0.6
- Fix winit mouse bug

# 1.0.5
## ✨ Refactor: Add Cross-Platform Windows Support for `cargo-pixel`

### Summary

This release improves the cross-platform compatibility of `cargo-pixel`, especially on Windows. It includes a full refactor of the command execution and path handling logic to ensure reliable builds across different environments.

### Changes

- ✅ Switched to platform-aware command execution:
  - Uses `cmd /C` on Windows
  - Uses `sh -c` on Unix-like systems
- ✅ Replaced hardcoded shell commands with `std::process::Command` API
- ✅ Fixed path separator issues (`/` vs `\`) by using `PathBuf` to construct paths
- ✅ Cleaned up temporary directory setup and asset copying logic

### Impact

- 🪟 `cargo-pixel` now works reliably on Windows Native
- 🧼 More robust, maintainable, and portable build scripts


# 1.0.4
- Fix stand-alone application web compile bug

# 1.0.3
- Added build.rs, set cfg_aliases
- Fix creat stand-alone application

# 1.0.2
- cargo upgrade: upgrade depend crate version
- toml 0.9 is bad, downgrade to toml 0.8 for cargo.toml parse

# 1.0.1
- Merge tools to cargo-pixel
```
cargo install --path . --force
cargo pixel 
```

# 1.0.0
- Added wgpu render backend
```
cargo pixel r petview wg -r   # wgpu backend (winit + wgpu)
cargo pixel r petview g -r    # glow backend (winit + opengl)
cargo pixel r petview s -r    # sdl backend (sdl2 + opengl)
cargo pixel r petview w -r    # web backend (wasm + webgl)
```
- Remove pixel_macro crate,move to lib.rs

# 0.6.0 / 0.6.1
- Bug fix
- Rename c64.png to symbols.png 
- 0.6.1 fix symbols.png bug

# 0.5.9
- Added pixel_symbol tool, which can dig symbols from a pixel art picture.
```
cargo pixel r pixel_symbol t -r assets/pixel.png 16
```
- Fix petview app web mode bug

# 0.5.8
- Fix a runtime error in linux / wsl

# 0.5.7
- Split the app's render.rs into two files, render_terminal.rs and render_graphics.rs, to make the code logic clearer.
- Added pixel_symbol tool for extract the symbol set used in pixel art picture.

# 0.5.6
- Added pixel asset tool, which can package scattered png images into c64.png and generate pix files corresponding to each image
```
cargo pixel r pixel_asset t ./png_sources ./out
cp out/*.pix apps/city/assets
cp out/texture_atlas.png apps/city/assets/pix
```
- Please refer to apps/city games, now this game supports a better graphics mode
```
cargo pixel r city s -r
```

# 0.5.5
- Update cargo-pixel, added self-update feature. 
- If ~/rust_pixel_work/Cargo.toml version not equal cargo-pixel version, then auto exec `cargo install --path . --force`

# 0.5.4
- Update docs 

# 0.5.3
- Refactored the entire project structure by moving the contents of the rust-pixel directory to the root directory.
- Refactored the way project directories are managed: in development mode, using env::var("CARGO_MANIFEST_DIR") as the starting directory; after release, you can flexibly pass in the path via command line arguments.
- Refactored the cargo-pixel tool and set it as the binary file of rust_pixel, so you can directly install cargo-pixel via cargo install rust_pixel.

# 0.5.2
- Update coding.md and principle.md documents
- Fix cargo-pixel bug, please update cargo-pixel:
```
cargo install --path tools/cargo-pixel --root ~/.cargo
```
- Fix gl transition size

# 0.5.1
- Update petview game, added a online demo
- Update README.md and added a petview demo video

# 0.5.0
- Fixed numerous cargo clippy warnings

# 0.4.9
- Refactored gl/pixel.rs, Use the GlRender trait to wrap several opengl shaders and facilitate further extension of the shader
- Updated the cargo-pixel tool so that when it runs, it first compares the version number in pixel.toml. If it is inconsistent, it prompts to update cargo-pixel.

# 0.4.8
- Refactored the underlying rendering module, abandoned the canvas API of SDL, and replaced it with OpenGL shader,
- Opengl rendering improved engine performance (CPU dropped from 38% to about 15%) 
- Added the ability to use shader to achieve various special effects(coding petview transition)
- Refer to rust-pixel/src/render/adapter/sdl.rs
- Abstracted the repeated code in lib.rs of each application into a procedural macro:
```
pixel_game!(Snake)
```
- Refer to pixel_macro/src/lib.rs

# 0.4.7
- add petview game for petscii arts
- update tpetii tool for convert petscii art to pix files
- added graph mode cell background color

# 0.4.6
- Added linux & windows install guide
- Update readme

# 0.4.5
- Update cargo.toml fix rustdoc bug...

# 0.4.4
- Palette tool work
```
cargo pixel r palette t -r
```

# 0.4.3
- Fix cargo pixel r foobar w bug

# 0.4.2
- Refactor Panel, added layers for rendering
- Refer to apps/palette render.rs
```
impl PaletteRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();
        // creat main layer
        panel.add_layer("main");

        // background
        let gb = Sprite::new(0, 0, PALETTEW, PALETTEH);
        panel.add_layer_sprite(gb, "main", "back");

        // top menu
        let mb = Sprite::new(MENUX, MENUY, MENUW, 1);
        panel.add_layer_sprite(mb, "main", "menu");

        ...

        // creat 6 state layers
        for i in 0..6 {
            panel.add_layer(&format!("{}", i));
            if i != 0 {
                panel.deactive_layer(&format!("{}", i));
            }
        }
        ...
    }

    pub fn draw_menu<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();

        // get layer sprite
        let mb = self.panel.get_layer_sprite("main", "menu");
        ...
    }
```
- Continue writing palette application
```
$ cargo pixel r palette t 
```

# 0.4.1
- Refactor Game and Context, added project_path variable
- Better support for standalone crates using rust_pixel
- Usually, you can use the following commands to creat games or apps that use pixel:
```
$ cargo pixel c games mygame
$ cargo pixel c apps myapp
```
However, these crates are embedded into rust_pixel, 
```
    let mut g = Game::new(m, r, "games/mygame");
    g.init();
```
the project_path is "games/mygame" or "apps/myapp" by default, the assets file path should be "games/mygame/asset/"
if you creat a standalone project with:
```
$ cargo pixel c .. mygame --standalone
$ cargo pixel c .. mygame -s
```
and use "cargo run" in mygame directory, you should use this codes for load assets in "./asset/" path:
```
    let mut g = Game::new_with_project_path(m, r, "games/mygame", ".");
    g.init();
```
The cargo pixel creat <path> <project_name> --standlone command will automatically complete the above work for you.
```
$ cargo pixel c .. mygame --standalone  #Create a standalone crate in ../mygame 
$ cd ../mygame 
$ cargo pixel r mygame t
$ cargo pixel r mygame s

```

# 0.4.0
- Add ColorPro for professional color process
- Support HSL, CMYK, Lab, Lch, OkLab, OkLch ColorSpaces
- Add a terminal application: palette
```
$ cargo pixel r palette t

pub enum ColorSpace {
    SRGBA,
    LinearRGBA,
    CMYK,
    HSLA,
    HSVA,
    HWBA,
    LabA,
    LchA,
    OKLabA,
    OKLchA,
    XYZA,
}
```

# 0.3.3
- Fix cargo-pixel create bug
```
$ cargo pixel c games mygame
$ cargo pixel c apps myapp
```

# 0.3.2
- Rewrite cargo-pixel with rust
- Removed python3 dependence
- Updated readme.md

# 0.3.1
1. Add bezier algorithm, refer to algorithm/bezier.rs
2. Add a demo about keyframe animation and bezier path,refer to games/template/model.rs & render.rs
- model
```
        let in_points = [
            PointF32 { x: 0.0, y: 0.0 },
            PointF32 { x: 1200.0, y: 100.0 },
            PointF32 {
                x: TEMPLATEW as f32 * 16.0,
                y: TEMPLATEH as f32 * 16.0,
            },
        ];
        let num = 100;
        let mut pts = vec![PointF32 { x: 0.0, y: 0.0 }; num];
        draw_bezier_curves(&in_points, &mut pts);
        let mut ks = Vec::new();
        for i in 0..num {
            ks.push((pts[i], i as f64 / num as f64).into());
        }
        self.bezier = AnimationSequence::from(ks);
```
- render
```
        for i in 0..15 {
            let mut pl = Sprite::new(4, 6, 1, 1);
            pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(203));
            pl.set_alpha(255 - 15*(15 - i));
            panel.add_pixel_sprite(pl, &format!("PL{}", i+1));
        }
        ...
        for i in 0..15 {
            let pl = &mut self.panel.get_pixel_sprite(&format!("PL{}", i+1));
            d.bezier.advance_and_maybe_reverse(dt as f64 * 0.1 + 0.01 * i as f64);
            let kf_now = d.bezier.now_strict().unwrap();
            pl.set_pos(kf_now.x as u16, kf_now.y as u16);
            d.bezier.advance_and_maybe_reverse(-0.01 * i as f64);
        }
```
3. Fix some bugs...

# 0.3.0
1. Add particle system
```
    //refer to games/snake/model.rs
    self.pats.fire_at(10.0, 10.0);
    ...
    self.pats.update(dt as f64);
    
    //refer to games/snake/render.rs
    pub fn draw_movie<G: Model>(&mut self, _ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<SnakeModel>().unwrap();

        self.panel.draw_objpool(&mut d.pats.particles, |pl, m| {
            pl.set_pos(m.obj.loc[0] as u16, m.obj.loc[1] as u16);
        });
    }
```
2. Optimize some render APIs for sprite
```
    /// set string content at (x,y) with fg/bg color...
    pub fn set_color_str<S>(&mut self, x: u16, y: u16, string: S, f: Color, b: Color);

    /// set string content at (0,0) with default style...
    pub fn set_default_str<S>(&mut self, string: S);

    /// set graphic model symbol(texture:texture_id, index:sym) at (x,y) with fgcolor...
    pub fn set_graph_sym(&mut self, x: u16, y: u16, texture_id: u8, sym: u8, f: Color);
```

# 0.2.0
1. Add a good template with bin / lib / ffi / wasm for create your own game, app or library
- refer to games/template
- create your own game, run:
```
cargo pixel c <my_game_name> 
```
2. Add global alpha for sprite
```
let sp: Sprite ... ...
sp.set_alpha(100);
```

