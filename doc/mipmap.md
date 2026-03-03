# Mipmap Text Rendering in RustPixel

## Background

RustPixel uses **Texture2DArray with 3-level mipmaps** for all symbol rendering. This replaces the previous single texture atlas with MSDF/SDF approach (see `doc/sdf.md` for historical reference), providing crisp rendering at any scale while maintaining single draw call performance.

## Texture2DArray Architecture

| Component | Description |
|-----------|-------------|
| Format | Texture2DArray (multiple 4096×4096 layers) |
| Mipmaps | 3 levels: mip0 (high), mip1 (mid), mip2 (low) |
| Symbols | Sprite, TUI, Emoji, CJK all in same array |
| Rendering | Single texture binding, instanced, one draw call |

## Why Mipmaps?

The challenge: MDPT (Markdown Presentation Tool) needs crisp text from small windows to fullscreen 4K displays. A single resolution texture either wastes memory or looks blurry at certain scales.

**Solution**: Pre-render each symbol at 3 resolutions. The engine auto-selects based on actual render size:

| Condition | Mip Level | Use Case |
|-----------|-----------|----------|
| `per_unit >= 48` | mip0 | Fullscreen, high-DPI |
| `per_unit >= 24` | mip1 | Normal window |
| `per_unit < 24` | mip2 | Small window, thumbnails |

This ensures:
- **No wasted memory**: Only load what's needed for current scale
- **Always crisp**: Appropriate resolution for any window size
- **Single draw call**: All mip levels in same Texture2DArray

## Tile Structure

Each symbol maps to a `Tile` with UV coordinates for all 3 mip levels:

```rust
pub struct MipUV {
    pub layer: u16,  // Layer index in Texture2DArray
    pub x: f32,      // Normalized UV x (0.0-1.0)
    pub y: f32,      // Normalized UV y (0.0-1.0)
    pub w: f32,      // Normalized UV width
    pub h: f32,      // Normalized UV height
}

pub struct Tile {
    pub cell_w: u8,       // 1=normal, 2=wide (CJK/Emoji)
    pub cell_h: u8,       // 1=single, 2=tall (TUI/CJK)
    pub is_emoji: bool,   // Pre-rendered color (no modulation)
    pub mips: [MipUV; 3], // 3 mipmap levels
}
```

## Symbol Map

`layered_symbol_map.json` maps symbol strings to tiles. Compact format (375KB):

```json
{
  "version": 2,
  "layer_count": 15,
  "layer_size": 4096,
  "layer_files": ["layers/layer_0.png", ...],
  "symbols": {
    "A": [1,2, 0,100,200,32,64, 1,50,100,16,32, 2,25,50,8,16],
    ...
  }
}
```

Each symbol is a flat array of 17 numbers:
- `[0-1]`: cell_w, cell_h
- `[2-6]`: mip0 (layer, x, y, w, h)
- `[7-11]`: mip1 (layer, x, y, w, h)
- `[12-16]`: mip2 (layer, x, y, w, h)

## Texture Generation

The `tools/symbols/` Rust tool generates mipmapped textures:

```bash
cargo pixel symbols -o assets/pix
```

### Features

- **macOS CoreText/Quartz**: Native font rendering for TUI/CJK/Emoji
- **3-Level Mipmap**: Each symbol rendered at high/mid/low resolution
- **DP Shelf-Packing**: Efficient texture layer packing
- **Sprite Support**: C64 character ROM PNG integration

### Symbol Types

| Type | Rendering | Cell Size |
|------|-----------|-----------|
| Sprite | C64 ROM bitmap | 1×1 |
| TUI | CoreText + Nerd Font | 1×2 |
| Emoji | Apple Color Emoji | 2×2 |
| CJK | CoreText + PingFang SC | 2×2 |

## File Overview

| File | Role |
|------|------|
| `tools/symbols/` | Rust mipmap texture generator |
| `src/render/symbol_map.rs` | Tile/MipUV structures, JSON parsing |
| `src/render/adapter/wgpu/render_symbols.rs` | Instanced rendering with mip selection |
| `src/init.rs` | Asset loading with fallback logic |
| `assets/pix/` | Generated layers and symbol map |
