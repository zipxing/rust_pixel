# FAQ

## Installation

### How to install on Windows?
Use WSL2 with Windows 11. RustPixel's terminal mode and WGPU mode both work well under WSL2.

### What's the minimum Rust version?
Rust 1.71+. The engine uses features stabilized in that version.

### How to install cargo-pixel?
```bash
cargo install rust_pixel
cargo pixel          # First run clones repo to ~/rust_pixel_work
```
Or from local source:
```bash
cargo install --path . --force
```

## Rendering

### Why only WGPU and terminal mode? What happened to SDL/Glow?
As of v2.1, SDL2 and Glow (OpenGL) backends have been removed. WGPU provides a modern, unified GPU API (Vulkan/Metal/DX12/WebGPU) that covers all platforms including web. This simplifies the codebase significantly — one GPU pipeline instead of three.

### How does web rendering work without a separate backend?
Web mode uses the same WGPU pipeline as desktop. When compiled to WASM, wgpu automatically falls back to WebGL2 if WebGPU is unavailable in the browser. No separate adapter code needed.

### What's the difference between `term` and `wgpu` mode?
- **term**: Renders directly to a terminal emulator using crossterm. Characters are Unicode text, colors are ANSI. Works over SSH.
- **wgpu**: Opens a native window with GPU rendering. Characters are glyphs from a texture atlas. Supports scaling, rotation, alpha blending, shader effects.

Both modes share the same Model code. Only the Render implementation differs.

### How does the texture atlas work?
A single PNG image contains all glyphs arranged in a 16x16 grid of blocks (256 blocks total). Each block contains 16x16 glyph cells. The GPU renders all characters in one draw call using instanced rendering.

Block assignment:
- 0-159: Sprite glyphs (PETSCII, ASCII, custom game art)
- 160-169: TUI characters (box drawing, symbols)
- 170-175: Emoji
- 176-239: CJK characters

### Why does MDPT use an 8192x8192 texture?
Standard games use 4096x4096 (16px per glyph cell). For fullscreen presentation at high DPI, 16px cells look blurry. MDPT uses 8192x8192 (32px per glyph cell) for crisp text rendering. The engine auto-detects cell size from the atlas dimensions at startup.

### What is the 4-stage GPU pipeline?
1. **Data → RenderBuffer**: Collect cells from Buffer/Sprites into `Vec<RenderCell>`
2. **RenderBuffer → RenderTexture**: Draw to off-screen textures (RT0-RT3)
3. **RT Operations**: Blend/copy between textures (for transitions)
4. **RT → Screen**: Composite final output

RT convention: RT0/RT1 = transition sources, RT2 = main scene, RT3 = overlay/effects.

### How do GPU transitions work?
Slide transitions use the RT pipeline: previous frame goes to RT0, current frame to RT1, a shader blends them into RT3 based on progress (0.0→1.0). Seven GPU effects are available: squares, heart, noise, rotate, bounce, dissolve, ripple. Five CPU effects also exist (slide, wipe, dissolve) that operate on cell buffers directly.

## Application Development

### How is an app structured?
Every app has a Model (game logic) and a Render (drawing). The `app!` macro generates scaffolding:
```rust
use rust_pixel::app;
app!(MyGame);
```
This creates `MyGameGame` struct, `init_game()`, `run()`, and WASM exports.

### How do I differentiate graphics vs terminal code?
```rust
#[cfg(any(feature = "wgpu", target_arch = "wasm32"))]
{
    // GPU-only code: set_graph_sym, set_alpha, etc.
}

#[cfg(not(any(feature = "wgpu", target_arch = "wasm32")))]
{
    // Terminal-only code: load .txt assets, etc.
}
```

### How do Model and Render communicate?
Use events for loose coupling:
```rust
// Model side
event_emit("MyApp.ScoreChanged");

// Render side (in handle_event)
if event_check("MyApp.ScoreChanged", "update_score") {
    // redraw score display
}
```

### How do command line arguments work?
`cargo pixel r myapp wg -r <project_path> <extra_args...>` translates to:
```
cargo run -p myapp --features wgpu --release -- <project_path> <extra_args...>
```
- `args[1]` = project/asset directory path (used by `get_project_path()`)
- `args[2]+` = app-specific arguments

If only one arg is passed, it's the project path. If none, uses `CARGO_MANIFEST_DIR` or `.`.

### How do I create a standalone project?
```bash
cargo pixel c myapp ..
cd ../myapp
cargo pixel r myapp wg -r
```
The standalone project depends on rust_pixel as a crate and has its own Cargo.toml.

## MDPT (Markdown Presentation Tool)

### How is MDPT different from terminal presenters like presenterm?
MDPT renders a complete TUI in a GPU window — no terminal emulator needed. This gives consistent rendering across platforms, GPU shader transitions, and true graphics capabilities not limited by terminal cells.

### How do I run a presentation?
```bash
cargo pixel r mdpt wg -r apps/mdpt/ demo.md
```

### What markdown features does MDPT support?
- Headings, paragraphs, lists (nested, ordered/unordered)
- Code blocks with syntax highlighting (100+ languages)
- Dynamic code highlighting with `{1-4|6-10|all}` syntax
- Tables with column alignment
- Block quotes with GitHub alert types (`[!note]`, `[!caution]`, etc.)
- Column layouts (`<!-- column_layout: [1, 1] -->`)
- Incremental display (`<!-- pause -->`)
- Text animations (spotlight, wave, fadein, typewriter)
- Charts (line, bar, pie) and Mermaid flowcharts
- Image support (.pix, .ssf)
- CJK text and emoji

### How does the auto-generated cover page work?
If the YAML front matter has a `title` field, a cover slide is automatically inserted at index 0. It displays the title (large, centered), author, and a dim config summary line. No need to write a manual first slide.

## UI Framework

### What widgets are available?
Label, Button, TextBox, Panel, List, Tree, ScrollBar, Table, and more. All work in both terminal and GPU modes.

### How does the layout system work?
Panels support `FreeLayout` (manual x/y positioning), `VBoxLayout`, and `HBoxLayout`. Widgets have bounds (`Rect`) and can be nested inside panels.

### Can I use the UI framework for non-game apps?
Yes. MDPT itself is a pure UI application — no game logic, just document rendering with the UI widget system.
