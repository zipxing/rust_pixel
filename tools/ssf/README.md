# PixelSSF - SSF Sequence Frame Player

A command-line tool for playing RustPixel's SSF (Sequence Frame) animation files.

## Features

- **Multi-mode Support**: Works in both terminal mode and graphics mode (SDL/Winit/WGPU)
- **Interactive Controls**: Full playback control with keyboard shortcuts
- **Multiple Playback Options**: Auto-play, manual frame stepping, loop modes
- **Speed Control**: Adjustable playback speed from 0.5x to 20x
- **Cross-platform**: Runs on Windows, macOS, and Linux

## Usage

### Using cargo-pixel (Recommended)

```bash
# Play specific SSF file using cargo-pixel
cargo pixel r pixel_ssf wg -r . assets/sdq/fire.ssf    # WGPU graphics mode
cargo pixel r pixel_ssf wg -r . sdq/fire.ssf           # (will auto-add "assets/" prefix)

# Other rendering modes
cargo pixel r pixel_ssf sdl -r . sdq/fire.ssf          # SDL graphics mode
cargo pixel r pixel_ssf winit -r . sdq/fire.ssf        # Winit graphics mode  
cargo pixel r pixel_ssf term -r . sdq/fire.ssf         # Terminal mode
```

### Direct cargo run

```bash
# Play specific SSF file directly (path relative to assets/ directory)
cargo run -p pixel_ssf --features wgpu --release -- sdq/fire.ssf

# Build for different modes
cargo run -p pixel_ssf --features sdl -- sdq/fire.ssf     # SDL graphics mode
cargo run -p pixel_ssf --features winit -- sdq/fire.ssf   # Winit graphics mode  
cargo run -p pixel_ssf --features wgpu -- sdq/fire.ssf    # WGPU graphics mode
cargo run -p pixel_ssf --features term -- sdq/fire.ssf    # Terminal mode (default)
```

## Controls

| Key | Action |
|-----|--------|
| `Space` | Toggle auto play/pause |
| `←` / `→` | Previous/Next frame |
| `R` | Reset to first frame |
| `L` | Toggle loop mode |
| `+` / `=` | Increase speed (faster playback) |
| `-` | Decrease speed (slower playback) |
| `Q` | Quit |

## Available SSF Files

The tool works with SSF files in the `assets/` directory:

- `assets/sdq/fire.ssf` - Fire effect animation  
- `assets/sdq/ball.ssf` - Ball animation
- `assets/sdq/cube.ssf` - Cube animation
- `assets/sdq/1.ssf` - Animation sequence 1
- `assets/sdq/2.ssf` - Animation sequence 2

When specifying files, you can omit the `assets/` prefix as it's added automatically.

## Building

```bash
# Terminal mode (minimal dependencies)
cargo build -p pixel_ssf --features term

# Graphics modes
cargo build -p pixel_ssf --features sdl
cargo build -p pixel_ssf --features winit  
cargo build -p pixel_ssf --features wgpu
```

## SSF File Format

SSF (Sequence Frame) files are compressed animation files used by RustPixel. They support:
- Multiple frame formats (ASCII, Unicode, PETSCII)
- Frame-by-frame compression with gzip
- Texture mapping for graphics modes
- Variable frame rates
- Different color modes and symbol sets

For more information about the SSF format, see the RustPixel documentation.

## Troubleshooting

- **File not found**: Make sure the SSF file exists in the `assets/` directory
- **No animation**: Check that the file loaded successfully - you should see "SSF loaded: X frames" in the logs  
- **Controls not working**: Ensure the window has focus and you're using the correct key combinations 