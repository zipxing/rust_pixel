---
title: MDPT Demo
theme: dark
transition: dissolve
title_animation: typewriter
code_theme: base16-ocean.dark
margin: 4
---

# Welcome to MDPT

A Markdown Presentation Tool built with RustPixel

Press Space or Right arrow to advance

---

## Features

* GPU-accelerated slide transitions (6 types)
* Code syntax highlighting (100+ languages)
* Text animations (Spotlight, Wave, FadeIn, Typewriter)
* .pix/.ssf image support (planned)

<!-- pause -->

* Column layouts
* Incremental display with pause
* Table rendering
* WASM/Web deployment
* 中文支持吗

---

## Code Highlighting

```rust +line_numbers
use rust_pixel::game::Model;

fn main() {
    let message = "Hello from MDPT!";
    println!("{}", message);

    for i in 0..5 {
        println!("  Slide {}", i);
    }
}
```

---

## Tables

| Feature       | Status   | Priority |
|:--------------|:--------:|---------:|
| Transitions   | Done     | High     |
| Highlighting  | Done     | High     |
| Column Layout | Done     | Medium   |
| Animations    | Planned  | Medium   |
| Images        | Planned  | Low      |

---

<!-- column_layout: [1, 1] -->
<!-- column: 0 -->

### Left Column

This demonstrates the
column layout feature.

Content flows in the
left column area.

* Item A
* Item B

<!-- column: 1 -->

### Right Column

The right column gets
equal width here.

Useful for comparisons
or side-by-side content.

* Item X
* Item Y

<!-- reset_layout -->

---

## Nested Lists

* RustPixel Engine
  * Rendering
    * Terminal (crossterm)
    * Graphics (wgpu/glow/sdl)
    * Web (WASM)
  * UI Framework
    * 17 widget types
    * Layout system
* MDPT Presentation
  * Markdown parsing
  * Code highlighting
  * Slide transitions

---

<!-- jump_to_middle -->

# Thank You!

Built with RustPixel - press q to quit
