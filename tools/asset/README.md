# RustPixel Asset Packer

A command-line tool for packing multiple images into texture atlases and generating corresponding `.pix` metadata files for use with the RustPixel engine.

## Features

- **Efficient Image Packing**: Uses the MaxRects bin packing algorithm for optimal space utilization
- **4096x4096 Texture Atlas**: Supports the new unified texture format with region-aware packing
- **Region Support**: Pack into Sprite, TUI, Emoji, or full atlas regions
- **Smart Size Optimization**: Automatically adjusts image sizes to multiples of 16 pixels for grid alignment
- **PIX Metadata Generation**: Generates `.pix` files containing texture coordinate information
- **Symbol Integration**: Seamlessly integrates with existing RustPixel symbol textures
- **Configurable Scaling**: Custom scale factors for image resizing

## Installation

From the project root directory:

```bash
cd tools/asset
cargo build --release
```

Or run via cargo-pixel:

```bash
cargo pixel r asset t -r <INPUT_FOLDER> <OUTPUT_FOLDER>
```

## Usage

### Basic Usage

```bash
cargo pixel r asset t -r <INPUT_FOLDER> <OUTPUT_FOLDER> [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--symbols <PATH>` | Path to base symbols.png texture (default: `assets/pix/symbols.png`) |
| `--symbol-map <PATH>` | Path to symbol_map.json for auto block detection |
| `--region <MODE>` | Packing region: `sprite` (default) or `full` |
| `--start-block <N>` | Start packing from block N (overrides auto-detect) |
| `--scale <FACTOR>` | Scale factor for images (default: 1.0) |

### Examples

```bash
# Basic usage with auto block detection (recommended)
cargo pixel r asset t -r ./sprites ./output --symbol-map assets/pix/symbol_map.json

# Specify both symbols.png and symbol_map.json
cargo pixel r asset t -r ./sprites ./output \
    --symbols assets/pix/symbols.png \
    --symbol-map assets/pix/symbol_map.json

# Pack into full 4096x4096 atlas
cargo pixel r asset t -r ./images ./output --region full

# Manual start block (overrides auto-detect)
cargo pixel r asset t -r ./icons ./output --start-block 80 --scale 0.5
```

### Auto Block Detection

When `--symbol-map` is provided, the tool reads `symbol_map.json` to detect which blocks are already occupied by existing symbols. It then automatically selects the first free block as the starting point for packing new images.

This ensures that:
- Existing sprite symbols are preserved
- New images don't overwrite built-in characters
- Block allocation is optimized automatically

Example output with symbol map:
```
Block Occupancy Summary:
────────────────────────
  sprite: blocks 0-159, 103 symbols
  tui: blocks 160-169, 256 symbols
  emoji: blocks 170-175, 200 symbols
  cjk: blocks 176-239, 6763 symbols

Sprite region: 1/160 blocks occupied
First free block: 1
Auto-detected start block: 1 (first free block)
```

### Help

```bash
cargo pixel r asset t -r help
# or
cargo pixel r asset t -r --help
```

## 4096x4096 Texture Layout

The asset packer supports the new unified 4096x4096 texture format:

```
┌─────────────────────────────────────────────────────────────┐
│ SPRITE Region (y: 0-2559, 2560px height)                   │
│ • 160 blocks (10 rows × 16 columns)                         │
│ • Block size: 256×256px (16×16 symbols at 16×16px)         │
│ • Total: 40,960 sprites                                     │
├─────────────────────────────────────────────────────────────┤
│ TUI + EMOJI Region (y: 2560-3071, 512px height)            │
│ • TUI (x: 0-2559): 10 blocks, 16×32px symbols              │
│ • Emoji (x: 2560-4095): 6 blocks, 32×32px symbols          │
├─────────────────────────────────────────────────────────────┤
│ CJK Region (y: 3072-4095, 1024px height)                   │
│ • 64 blocks (16 cols × 4 rows), 32×32px symbols            │
└─────────────────────────────────────────────────────────────┘
```

### Block Layout

| Region | Blocks | Y Range | Symbol Size | Total Symbols |
|--------|--------|---------|-------------|---------------|
| Sprite | 0-159 | 0-2559 | 16×16 | 40,960 |
| TUI | 160-169 | 2560-3071 | 16×32 | 2,560 |
| Emoji | 170-175 | 2560-3071 | 32×32 | 768 |
| CJK | 176-239 | 3072-4095 | 32×32 | 4,096 |

## Input Requirements

- **Input Folder**: Should contain image files (PNG, JPEG, GIF, BMP)
- **Base Texture**: Uses `assets/pix/symbols.png` as the base texture (customizable via `--symbols`)
- **Symbol Map** (optional): `assets/pix/symbol_map.json` for auto block detection (`--symbol-map`)
- **Supported Formats**: PNG, JPEG, BMP, GIF

## Output

The tool generates:

1. **`texture_atlas.png`**: A 4096×4096 texture atlas containing all packed images
2. **`*.pix` files**: Metadata files for each input image containing:
   - Texture dimensions (width, height in grid units)
   - Texture ID (255 for atlas textures)
   - Block and symbol index coordinate mapping

## How It Works

1. **Image Loading**: Loads all valid images from the input folder
2. **Size Adjustment**: Adjusts image dimensions to multiples of 16 pixels
3. **Scaling**: Applies optional scale factor (default 1.0)
4. **Bin Packing**: Uses MaxRects algorithm to efficiently pack images into the selected region
5. **Atlas Creation**: Creates 4096×4096 atlas, optionally merging with base symbol texture
6. **Metadata Generation**: Creates `.pix` files with block/symbol coordinate information

## PIX File Format

Each `.pix` file contains:
```
width=<W>,height=<H>,texture=255
<symidx>,<color>,<texidx>,<modifier> ...
```

Where:
- `symidx`: Symbol index within block (0-255)
- `color`: Color index (default 15)
- `texidx`: Block index (0-159 for sprite, etc.)
- `modifier`: Style modifier (default 0)

### Coordinate Calculation

For 4096×4096 texture with 16×16 base symbols:
- Grid: 256×256 cells
- Block: 16×16 cells (256×256 pixels)
- Block index = `(y / 16) * 16 + (x / 16)`
- Symbol index = `(y % 16) * 16 + (x % 16)`

## Dependencies

- `image`: Image loading and processing

## Error Handling

The tool provides comprehensive error handling for:

- Invalid or missing input folders
- Unsupported image formats
- Insufficient packing space
- File I/O errors
- Image processing errors

## Compatibility

- Compatible with RustPixel engine's 4096×4096 texture system
- Works with all image formats supported by the `image` crate
- Cross-platform support (Windows, macOS, Linux)

## License

This project is licensed under the MIT OR Apache-2.0 license - see the LICENSE file in the main project directory for details.
