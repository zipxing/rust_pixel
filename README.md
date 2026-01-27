![logo](./screen-shot/logo.png)

<div align="center">

![License] [![Latest Version]][crates.io] ![Downloads] [![API Documentation]][docs.rs] ![MSRV]

[License]: https://img.shields.io/badge/license-Apache2.0-blue.svg
[Latest Version]: https://img.shields.io/crates/v/rust_pixel.svg
[crates.io]: https://crates.io/crates/rust_pixel
[Downloads]: https://img.shields.io/crates/d/rust_pixel.svg
[API Documentation]: https://docs.rs/rust_pixel/badge.svg
[docs.rs]: https://docs.rs/rust_pixel
[MSRV]: https://img.shields.io/badge/rust-1.71+-brightgreen.svg?&logo=rust

**Tile-first. Retro-ready. Write Once, Run Anywhere—2D Engine!**

[Change Log] | [Principle] | [Coding] | [FAQ] | [TODO] | [Roadmap] | [Online Demo]

[Change Log]: doc/change.md
[Principle]: doc/principle.md
[Coding]: doc/coding.md
[FAQ]: doc/faq.md
[TODO]: doc/todo.md
[Roadmap]: doc/roadmap_2026.md
[Online Demo]: https://zipxing.github.io/rust_pixel

</div>

---

## Core Philosophy

<table>
<tr>
<td width="33%" align="center">

### Everything is Tiles

Cell → Buffer → Sprite → Panel

Unified rendering abstraction

High performance - One texture, one draw call 

</td>
<td width="33%" align="center">

### Write Once, Run Anywhere

Terminal | Desktop | Web

TUI in native windows — no terminal emulator required

One codebase, multiple targets

</td>
<td width="33%" align="center">

### Quick Start

`app!` macro scaffolding

Built-in BASIC interpreter

Model-Render-Game pattern

</td>
</tr>
</table>

---

## Rendering Modes

| Mode | Backend | Output |
|------|---------|--------|
| **Terminal** | crossterm | ASCII / Unicode / Emoji |
| **SDL** | SDL2 + OpenGL | PETSCII / Custom Symbols |
| **Glow** | winit + OpenGL | PETSCII / Custom Symbols |
| **WGPU** | winit + wgpu | PETSCII / Custom Symbols |
| **Web** | WASM + WebGL | PETSCII / Custom Symbols |

---

## Showcase

PETSCII art browser built with RustPixel. Art by [@PETSCIIWORLD](https://x.com/PETSCIIWORLD), transitions by **gltransition**.

https://github.com/user-attachments/assets/4758f2b6-66c2-47ed-997d-a9066be449aa

---

## Quick Start

### Install

```bash
cargo install rust_pixel         # Install cargo-pixel CLI
cargo pixel                      # First run clones workspace to ~/rust_pixel_work
cd ~/rust_pixel_work
```

### Run Demo Games

```bash
cargo pixel r snake t            # Snake - Terminal mode
cargo pixel r snake s            # Snake - SDL mode
cargo pixel r tetris w           # Tetris - Web mode (localhost:8080)
cargo pixel r petview g -r       # Petview - Glow mode (release)
cargo pixel r petview wg -r      # Petview - WGPU mode (release)
```

### Create Your Own Game

```bash
cargo pixel c mygame             # Create in ./apps/mygame
cargo pixel r mygame t           # Run it!

# Or create standalone project
cargo pixel c myapp ..           # Create in ../myapp
cd ../myapp && cargo pixel r myapp s
```

### Write Games in BASIC

RustPixel includes **pixel_basic** - a built-in BASIC interpreter perfect for beginners or quick prototyping!

```bash
cargo pixel r basic_snake t      # Run BASIC Snake game
```

Write game logic in familiar BASIC syntax (`apps/basic_snake/assets/game.bas`):

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

3500 REM ON_DRAW
3510 PLOT X, Y, "@", 10, 0
3520 RETURN
```

**pixel_basic** features:
- Classic BASIC syntax with line numbers
- Game hooks: `ON_INIT (1000)`, `ON_TICK (2000)`, `ON_DRAW (3500)`
- Graphics: `PLOT x, y, char, fg, bg` / `BOX` / `CLS`
- Input: `KEY("W")`, `KEY("SPACE")`
- Arrays: `DIM arr(100)`
- Control flow: `GOTO`, `GOSUB/RETURN`, `FOR/NEXT`, `IF/THEN`
- Math: `RND()`, `INT()`, `ABS()`
- Strings: `STR$()`, `LEN()`, `MID$()`

See `pixel_basic/` for the interpreter source code.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                         Game                            │
│  ┌─────────────────────┐  ┌─────────────────────────┐  │
│  │       Model         │  │        Render           │  │
│  │  ├─ init()          │  │  ├─ init()              │  │
│  │  ├─ handle_input()  │  │  ├─ draw()              │  │
│  │  ├─ handle_auto()   │  │  └─ Panel               │  │
│  │  └─ handle_timer()  │  │      └─ Sprites[]       │  │
│  └─────────────────────┘  │          └─ Buffer      │  │
│                           │              └─ Cells[] │  │
│                           └─────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## Demo Games

<table>
<tr>
<td width="50%">

### Snake
PETSCII animations with smooth gameplay

```bash
cargo pixel r snake s -r    # SDL
cargo pixel r snake t -r    # Terminal
cargo pixel r snake w -r    # Web
```

![Snake](./screen-shot/snake_sdl.gif)

</td>
<td width="50%">

### Tetris
Play against AI

```bash
cargo pixel r tetris s -r   # SDL
cargo pixel r tetris t -r   # Terminal
cargo pixel r tetris w -r   # Web
```

![Tetris](./screen-shot/tetris_sdl.gif)

</td>
</tr>
<tr>
<td width="50%">

### Tower Defense
Pixel-perfect sprite movement

```bash
cargo pixel r tower s -r    # SDL
cargo pixel r tower w -r    # Web
```

![Tower](./screen-shot/tower_sdl.gif)

</td>
<td width="50%">

### Poker / Gin Rummy
Card game algorithms + FFI/WASM demos

```bash
cargo pixel r poker t -r
cargo pixel r gin_rummy t -r
```

![Poker](./screen-shot/ginrummy.png)

</td>
</tr>
</table>

---

## Tools

### Palette - Color Tool
Terminal UI for color manipulation

```bash
cargo pixel r palette t -r
```

![Palette](./screen-shot/palette.gif)

### Edit - Character Art Editor

```bash
cargo pixel e t . assets/logo.txt    # Terminal mode
cargo pixel e s . assets/logo.pix    # SDL mode
```

<table><tr>
<td><img src="./screen-shot/tedit_term.png" alt="Edit Terminal"></td>
<td><img src="./screen-shot/tedit_sdl.png" alt="Edit SDL"></td>
</tr></table>

### Petii - Image to PETSCII Converter

```bash
cargo pixel p assets/lion.png 40 40 > lion.pix
cargo pixel e g . lion.pix
```

<table><tr>
<td><img src="./screen-shot/a.png" alt="Petii Example 1"></td>
<td><img src="./screen-shot/lion.png" alt="Petii Example 2"></td>
</tr></table>

### GIF to PETSCII Animation

```bash
cargo pixel cg input.gif output.ssf 40 25
cargo pixel ssf . output.ssf    # Preview
```

---

## FFI & WASM

RustPixel algorithms can be exported for other languages:

```bash
# C++/Python FFI
cd apps/poker/ffi && make run

# JavaScript WASM
cd apps/poker/wasm && make run
```

---

## Installation Guide

| Platform | Guide |
|----------|-------|
| macOS | [doc/mac.md](doc/mac.md) |
| Linux | [doc/linux.md](doc/linux.md) |
| Windows (WSL) | [doc/win.md](doc/win.md) |
| Windows (Native) | [doc/win-native.md](doc/win-native.md) |

**Requirements:**
- [Nerd Font](https://github.com/ryanoasis/nerd-fonts) (for terminal mode)
- Rust 1.71+
- wasm-pack (for web mode)

---

## Features

- **`app!` macro** - One-line game scaffolding with cross-platform entry points
- **Model/Render pattern** - Clean separation of logic and presentation
- **Event/Timer system** - Built-in messaging mechanism
- **Unified adapter trait** - Same code for all rendering backends
- **OpenGL shaders** - Instance rendering, transitions, 2D effects
- **WGPU shaders** - Modern GPU rendering pipeline
- **Game algorithms** - Pathfinding, object pools, utilities
- **Audio support** - Sound effects and music playback

---

<div align="center">

**Made with Rust**

</div>
