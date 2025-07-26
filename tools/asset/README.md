# RustPixel Asset Packer

A command-line tool for packing multiple images into texture atlases and generating corresponding `.pix` metadata files for use with the RustPixel engine.

## Features

- 🖼️ **Efficient Image Packing**: Uses the MaxRects bin packing algorithm for optimal space utilization
- 📏 **Smart Size Optimization**: Automatically adjusts image sizes to multiples of 8 pixels for grid alignment
- 🎨 **Texture Atlas Generation**: Creates unified texture atlases from multiple input images
- 📝 **PIX Metadata Generation**: Generates `.pix` files containing texture coordinate information
- 🎮 **Symbol Integration**: Seamlessly integrates with existing RustPixel symbol textures
- ⚡ **High-Quality Scaling**: Uses Lanczos3 filtering for image resizing

## Installation

From the project root directory:

```bash
cd tools/asset
cargo build --release
```

## Usage

### Basic Usage

```bash
cargo run --release -- <INPUT_FOLDER> <OUTPUT_FOLDER>
```

### Examples

```bash
# Pack images from sprites folder to assets folder
cargo run --release -- ./sprites ./assets

# Pack images with specific paths
cargo run --release -- /path/to/images /path/to/output
```

### Help

```bash
cargo run -- --help
# or run without arguments to see usage information
cargo run
```

## Input Requirements

- **Input Folder**: Should contain image files (PNG, JPEG, GIF, BMP, etc.)
- **Base Texture**: Requires `assets/pix/symbols.png` to exist as the base symbol texture
- **Supported Formats**: Any format supported by the `image` crate

## Output

The tool generates:

1. **`texture_atlas.png`**: A combined texture atlas containing all input images
2. **`*.pix` files**: Metadata files for each input image containing:
   - Texture dimensions (width, height)
   - Texture ID (always 255 for atlas textures)
   - Texture coordinate mapping data

## How It Works

1. **Image Loading**: Loads all valid images from the input folder
2. **Size Adjustment**: Adjusts image dimensions to multiples of 8 pixels
3. **Downscaling**: Reduces images to half size for better packing efficiency
4. **Bin Packing**: Uses MaxRects algorithm to efficiently pack images
5. **Atlas Creation**: Combines base symbol texture with packed images
6. **Metadata Generation**: Creates `.pix` files with texture coordinate information

## Technical Details

### Image Processing

- Images are automatically padded to multiples of 8 pixels
- All images are scaled to 50% of their adjusted size
- High-quality Lanczos3 filtering is used for scaling
- Images are packed using the best-area-fit heuristic

### Texture Atlas Layout

```
┌─────────────────────────────────────┐
│         Symbol Texture              │  ← 128px height
│         (assets/pix/symbols.png)    │
├─────────────────────────────────────┤
│                                     │
│         Packed Images               │  ← 896px height
│                                     │
│                                     │
└─────────────────────────────────────┘
           1024px width
```

### PIX File Format

Each `.pix` file contains:
```
width=<W>,height=<H>,texture=255
<texture_coordinate_data>
```

Where texture coordinates are organized in a grid format suitable for the RustPixel sprite system.

## Dependencies

- `image`: Image loading and processing
- `log`/`log4rs`: Logging functionality
- `lab`/`deltae`: Color space operations (inherited from RustPixel)
- `rust_pixel`: Core RustPixel engine integration

## Features

- `sdl`: Enable SDL2 backend support
- `term`: Enable terminal backend support

## Error Handling

The tool provides comprehensive error handling for:

- Invalid or missing input folders
- Unsupported image formats
- Insufficient packing space
- File I/O errors
- Image processing errors

## Performance

- Efficient memory usage through streaming image processing
- Fast bin packing algorithm with O(n²) complexity
- Minimal disk I/O with batch processing

## Compatibility

- Works with all image formats supported by the `image` crate
- Compatible with RustPixel engine's texture system
- Cross-platform support (Windows, macOS, Linux)

## Contributing

This tool is part of the RustPixel project. For contributions, please refer to the main project repository.

## License

This project is licensed under the MIT OR Apache-2.0 license - see the LICENSE file in the main project directory for details. 
